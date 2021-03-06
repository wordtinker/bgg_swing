use crate::db;
use crate::bgg;
use crate::lib::{Game, User};
use failure::{Error, ResultExt, ensure};
use std::fs;
use serde_json::{from_str, to_string_pretty};
use serde_derive::{Serialize, Deserialize};
use std::thread;
use std::sync::mpsc;
use std::sync::mpsc::{Sender, Receiver, TryRecvError};
use std::time::Duration;
use reqwest::Client;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

const CONFIG_FILE_NAME: &str = "app.config";
const LOWER_BOUND: f64 = 2.0;
const UPPER_BOUND: f64 = 8.0;
const MISS_CHANCE: f32 = 0.5;

pub fn create_structure() -> Result<(), Error> {
    // create config file
    let new_conf = to_string_pretty(&Config::new(1000, 20, 500, 1000, 50000))?;
    fs::write(CONFIG_FILE_NAME, new_conf)?;
    // create db file
    db::initialize()?;
    Ok(())
}

pub fn pull_games(limit: u32, progress: impl Fn(usize) -> ()) -> Result<(), Error> {
    ensure!(limit > 0, "Can't get top.");

    // clear db
    db::drop_all_games()?;
    // Collect games
    for (i, games) in bgg::GameIterator::new(&Client::new(), limit).enumerate() {
        // Error will be elevated and next() will be never called again
        let games_on_page = games?;
        db::add_games(games_on_page)?;
        progress(i + 1);
    }
    Ok(())
}

pub fn make_report() -> Result<Vec<Game>, Error> {
    let conn = db::DbConn::new()?;
    if conn.get_number_of_unstable_games()? == 0 {
        db::get_all_games()
    } else {
        Ok(Vec::new())
    }
}

fn users_prevail(number_of_games: u32, number_of_users: u32) -> bool {
    (number_of_games as f32 * MISS_CHANCE).floor() as u32 * bgg::USER_PAGE_SIZE < number_of_users
}

fn trust(rating: f64) -> bool {
    LOWER_BOUND < rating && rating < UPPER_BOUND
}

fn with_cont(tx: Sender<Message>, rx: Receiver<Order>, mut tkn: RegulationToken,
            continuation: impl Fn(&Sender<Message>, &mut db::DbConn, &Client, &mut RegulationToken) -> ()) -> () {
    // Configure thread
    let mut conn = match db::DbConn::new() {
            Err(e) => {
                tx.send(Message::Err(e)).unwrap();
                return;
            },
            Ok(cn) => cn
    };
    let client = Client::new();
    // Start doing main job
    loop {
        // check if we got stop command
        match rx.try_recv() {
            // Err -> transmitter died i.e. function returned somehow early
            Ok(Order::Stop) | Err(TryRecvError::Disconnected) => {
                break;
            },
            Err(TryRecvError::Empty) => {}
        }
        // check if token stop flag is raised
        if tkn.is_stopped() {
            let e = failure::err_msg("Regulation token stopped the process");
            tx.send(Message::Err(e)).unwrap();
            break;
        }
        thread::sleep(tkn.delay());
        continuation(&tx, &mut conn, &client, &mut tkn);
    }
}

fn stabilize_games(tx: &Sender<Message>, conn: &mut db::DbConn, client: &Client, tkn: &mut RegulationToken) -> () {
    // check if we potentially has a work to do
    let number_of_games = match conn.get_number_of_unstable_games() {
        Err(e) => {
            tx.send(Message::Err(e)).unwrap();
            return;
        },
        Ok(n) => n
    };
    let number_of_users = match conn.get_number_of_unstable_users() {
        Err(e) => {
            tx.send(Message::Err(e)).unwrap();
            return;
        },
        Ok(n) => n
    };
    if users_prevail(number_of_games, number_of_users) {
        thread::sleep(tkn.prevail_for);
        return;
    }
    // find a game to work with
    let gamebox = match conn.get_unstable_game() {
        Err(e) => {
            tx.send(Message::Err(e)).unwrap();
            return;
        },
        Ok(gb) => gb
    };
    // if game is None, there is no more unstable games
    let (mut game, temp) = match gamebox {
        None => {
            tx.send(Message::Stabilized).unwrap();
            return;
        },
        Some(g) => g
    };
    tx.send(Message::Info(game.clone())).unwrap();
    // ask for user ratings
    let mut avg = Avg::new(temp.n, temp.r);
    for (i, page) in bgg::UserIterator::new(&client, game.id, temp.page).enumerate() {
        // save new page to db
        let new_page = temp.page + i as u32;
        match conn.update_page(&game, new_page, avg.n(), avg.result()) {
            Err(e) => {
                tx.send(Message::Err(e)).unwrap();
                return;
            },
            Ok(_) => {}
        };
        let users = match page {
            Err(e) => {
                tx.send(Message::Notification(e)).unwrap();
                tkn.harden(); // wait a bit longer before next request
                return;
            },
            Ok(vec) => {
                tkn.ease();
                vec
            }
        };
        // batch insert them to db
        let usernames: Vec<&User> = users.iter().map(|(u, _)| u).collect();
        match conn.add_users(&usernames) {
            Err(e) => {
                tx.send(Message::Err(e)).unwrap();
                return;
            },
            Ok(_) => {}
        };
        // check user stability and trust
        for (user, rating) in users {
            match conn.check_user(&user) {
                Err(e) => {
                    tx.send(Message::Err(e)).unwrap();
                    return;
                },
                Ok(None) => return, // user is unstable, move along
                Ok(Some(true)) => avg.add(rating),
                Ok(Some(false)) => {} // can't trust, ignore
            }
        }
        // Prevent 429 Too many requests
        thread::sleep(tkn.delay());
    }
    // every user was stable
    // save average and number of users
    game.rating = avg.result();
    game.votes = avg.n();
    match conn.update_game(&game) {
        Err(e) => {
            tx.send(Message::Err(e)).unwrap();
            return;
        },
        Ok(()) => tx.send(Message::GameProgress(game)).unwrap()
    };
}

fn stabilize_users(tx: &Sender<Message>, conn: &mut db::DbConn, client: &Client, tkn: &mut RegulationToken) -> () {
    let user = match conn.get_unstable_user() {
        Err(e) => {
            tx.send(Message::Err(e)).unwrap();
            return;
        },
        Ok(u) => u
    };
    // if user is None but Order::Stop was not recieved, just wait
    let user = match user {
        None => {
            tkn.harden(); // to prevent eternal loop
            return;
        },
        Some(u) => u
    };
    // ask bgg for user stats
    let rating = match bgg::get_user_average_rating(client, &user) {
        Err(e) => {
            tx.send(Message::Notification(e)).unwrap();
            tkn.harden(); // wait a bit longer before next request
            return;
        },
        Ok(rate) => rate
    };
    // save user to db
    match conn.update_user(&user, trust(rating)){
        Err(e) => {
            tx.send(Message::Err(e)).unwrap();
            return;
        },
        Ok(_) => {
            tkn.ease();
            tx.send(Message::UserProgress(user)).unwrap();
        }
    }
}

pub fn stabilize(config: Config, running: Arc<AtomicBool>, mut progress: impl FnMut(Message) -> ()) -> Result<(), Error> {
    // NB. Errors from mpsc channels use unwrap(). If channels fail,
    // the core of the programm is severely damaged, panic is only option. 
    
    
    // First comm network
    let (games_tx, main_rx) = mpsc::channel();
    let users_tx = mpsc::Sender::clone(&games_tx);
    // Second and third comm networks
    let (main_tx1, games_rx) = mpsc::channel();
    let (main_tx2, users_rx) = mpsc::channel();

    // try to balance every game
    // that must be the only source of Message::Stabilized
    let delay_step = Duration::from_millis(config.g_delay as u64);
    let prevail_for = Duration::from_millis(config.prevail_for as u64);
    let g_tkn = RegulationToken::new(config.attempts, delay_step, prevail_for);
    thread::spawn(move || with_cont(games_tx, games_rx, g_tkn, stabilize_games ));
    // try to balance every user
    let delay_step = Duration::from_millis(config.u_delay as u64);
    let u_tkn = RegulationToken::new(config.attempts, delay_step, prevail_for);
    thread::spawn(move || with_cont(users_tx, users_rx, u_tkn, stabilize_users ));

    // This will block main until iterator yields None
    let mut result: Result<(), Error> = Ok(());
    for received in main_rx {
        // handle messages
        match received {
            Message::Err(e) => {
                main_tx1.send(Order::Stop).unwrap_or_default();
                main_tx2.send(Order::Stop).unwrap_or_default();                
                result = Err(e);
            },
            Message::Stabilized => {
                main_tx1.send(Order::Stop).unwrap_or_default();
                main_tx2.send(Order::Stop).unwrap_or_default();
            },
            msg => progress(msg)
        }
        // handle stop signal
        if !running.load(Ordering::SeqCst) {
            main_tx1.send(Order::Stop).unwrap_or_default();
            main_tx2.send(Order::Stop).unwrap_or_default();
        }
    }
    result
}

pub fn config() -> Result<Config, Error> {
    let conf = fs::read_to_string(CONFIG_FILE_NAME)
        .with_context(|_| format!("Can't open: {}", CONFIG_FILE_NAME))?;
    let conf = from_str(&conf)?;
    Ok(conf)
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub struct Config {
    pub limit: u32, // number or user ratings for a game
    pub attempts: u32, // number or errors that thread can handle before stop
    pub u_delay: u32, // ms, delay increase after every failure for stabilize_users
    pub g_delay: u32, // ms, delay increase after every failure for stabilize_games
    pub prevail_for: u32 // ms, sleep time for game thread when users pvevail
}

impl Config {
    fn new(limit: u32, attempts: u32, u_delay: u32, g_delay: u32, prevail_for: u32) -> Config {
        Config {limit, attempts, u_delay, g_delay, prevail_for}
    }
}

#[derive(Debug)]
pub enum Message {
    Err(Error),
    Stabilized,
    UserProgress(User),
    GameProgress(Game),
    Notification(Error),
    Info(Game)
}

enum Order {
    Stop
}

struct RegulationToken {
    limit: u32,
    delay_step: Duration,
    i: u32,
    pub prevail_for: Duration
}

impl RegulationToken {
    fn new(limit: u32, delay_step: Duration, prevail_for: Duration) -> RegulationToken {
        RegulationToken { limit, delay_step, i: 0 , prevail_for}
    }
    fn delay(&self) -> Duration {
        self.delay_step * (self.i + 1)
    }
    fn is_stopped(&self) -> bool {
        self.i >= self.limit
    }
    fn ease(&mut self) -> () {
        if !self.is_stopped() && self.i != 0 {
            self.i -= 1;
        }
    }
    fn harden(&mut self) -> () {
        self.i += 1;
    }
}

struct Avg {
    n: u32,
    val: f64
}

impl Avg {
    fn new(n: u32, val: f64) -> Avg {
        Avg {n, val}
    }
    fn add(&mut self, nmbr: f64) -> () {
        self.n += 1;
        self.val = (nmbr + (self.n - 1) as f64 * self.val) / self.n as f64;
    }
    fn result(&self) -> f64 {
        self.val
    }
    fn n(&self) -> u32 {
        self.n
    }
}
