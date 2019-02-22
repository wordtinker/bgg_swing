use crate::db;
use crate::bgg;
use crate::lib::Game;
use failure::{Error, ResultExt, ensure, bail};
use std::fs;
use serde_json::{from_str, to_string_pretty};
use serde_derive::{Serialize, Deserialize};

const CONFIG_FILE_NAME: &str = "app.config";

pub fn create_structure() -> Result<(), Error> {
    // create config file
    let new_conf = to_string_pretty(&Config::new(1000))?;
    fs::write(CONFIG_FILE_NAME, new_conf)?;
    // create db file
    db::initialize()?;
    Ok(())
}

pub fn pull_games(limit: usize, progress: impl Fn(usize) -> ()) -> Result<(), Error> {
    ensure!(limit > 0, "Can't get top.");

    // clear db
    db::drop_all_games()?;
    // Collect games
    for (i, games) in bgg::pull_games(limit).enumerate() {
        // Error will be elevated and next() will be never called again
        let games_on_page = games?;
        db::add_games(games_on_page)?;
        progress(i + 1);
    }
    Ok(())
}

pub fn make_report() -> Result<Vec<Game>, Error> {
    if db::is_stable()? {
        db::get_all_games()
    } else {
        Ok(Vec::new())
    }
}

pub fn config() -> Result<Config, Error> {
    let conf = fs::read_to_string(CONFIG_FILE_NAME)
        .with_context(|_| format!("Can't open: {}", CONFIG_FILE_NAME))?;
    let conf = from_str(&conf)?;
    Ok(conf)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub limit: usize, // number or user ratings for a game
}

impl Config {
    fn new(limit: usize) -> Config {
        Config {limit}
    }
}
