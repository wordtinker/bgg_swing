use failure::{Error, ResultExt, bail};
use reqwest;
use select::document::Document;
use select::predicate::{Name, Class};
use crate::lib::{Game, User};

pub const USER_PAGE_SIZE: u32 = 100;

struct UserIterator {
    game_id: u32,
    page: u32
}

impl UserIterator {
    fn new(game_id: u32) -> UserIterator {
        UserIterator {game_id, page: 0 }
    }
}

impl Iterator for UserIterator {
    type Item = Result<Vec<(User, f64)>, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        self.page += 1;
        // get users for a game
        match get_users_from(self.game_id, self.page) {
            Ok(users) => {
                if users.is_empty() {
                    None
                } else {
                    Some(Ok(users))
                }
            },
            Err(e) => Some(Err(e))
        }
    }
}

fn get_users_from(game_id: u32, page: u32) -> Result<Vec<(User, f64)>, Error> {
    let url =  format!(
        "https://www.boardgamegeek.com/xmlapi2/thing?type=boardgame&id={}&ratingcomments=1&page={}&pagesize={}",
        game_id,
        page,
        USER_PAGE_SIZE
    );
    let resp = reqwest::get(&url)
        .with_context(|_| format!("could not download page `{}`", url))?;
    let doc = Document::from_read(resp)?;
    filter_users(doc)
}

fn filter_users(doc: Document) -> Result<Vec<(User, f64)>, Error> {
    let usertags = doc.find(Name("comment"));

    let mut users = Vec::new();
    for tag in usertags {
        let name = match tag.attr("username") {
            Some(n) => String::from(n),
            _ => bail!("Can't parse username in the user list")
        };
        let rating = match tag.attr("rating") {
            Some(r) => r.parse::<f64>()?,
            _ => bail!("Can't parse user rating in the user list")
        };
        users.push((name, rating));
    }
    Ok(users)
}

struct GameIterator {
    page: u32,
    user_limit: u32,
    seen: Option<Game>
}

impl GameIterator {
    fn new(user_limit: u32) -> GameIterator {
        GameIterator {page: 0 , user_limit, seen: None}
    }
}

impl Iterator for GameIterator {
    type Item = Result<Vec<Game>, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        self.page += 1;
        // get games from a page
        match get_games_from(self.page, self.user_limit) {
            Ok(games) => {
                if games.first() == self.seen.as_ref() || games.is_empty() {
                    None
                } else {
                    self.seen = Some(games[0].clone());
                    Some(Ok(games))
                }
            },
            Err(e) => Some(Err(e))
        }
    }
}

fn get_games_from(page: u32, user_limit: u32) -> Result<Vec<Game>, Error> {
    let url =  format!(
        "https://boardgamegeek.com/search/boardgame/page/{}?advsearch=1&range%5Bnumvoters%5D%5Bmin%5D={}&nosubtypes%5B0%5D=boardgameexpansion",
        page,
        user_limit
    );
    let resp = reqwest::get(&url)
        .with_context(|_| format!("could not download page `{}`", url))?;
    let doc = Document::from_read(resp)?;
    filter_games(doc)
}

fn filter_games(doc: Document) -> Result<Vec<Game>, Error> {
    let links = doc
        .find(Class("collection_table"))
        .flat_map(|t| t.find(Class("collection_objectname")))
        .flat_map(|c| c.find(Name("div")))
        .flat_map(|div| div.find(Name("a")));

    let mut games = Vec::new();
    for link in links {
        match link.attr("href") {
            Some(href) => {
                let id = href_to_id(href)?;
                games.push(Game::new(id, link.text()));
            },
            _ => bail!("Could not find game id.")
        };
    }
    Ok(games)
}

fn href_to_id(href: &str) -> Result<u32, Error> {
    let parts: Vec<&str> = href.rsplit('/').take(2).collect();
    let id = match parts.get(1) {
        Some(x) => x.parse::<u32>()?,
        None => bail!("Can't parse id of the game: {}", href)
    };
    Ok(id)
}


pub fn pull_games(user_limit: u32) -> impl Iterator<Item=Result<Vec<Game>, Error>> {
    GameIterator::new(user_limit)
}

pub fn get_user_average_rating(user: &User) -> Result<f64, Error> {
    let url =  format!("https://boardgamegeek.com/user/{}", user);
    let resp = reqwest::get(&url)
        .with_context(|_| format!("could not download page `{}`", url))?;
    let doc = Document::from_read(resp)?;
    let rating = doc
        .find(Class("profile_block")).skip(3).take(1)
        .flat_map(|pb| pb.find(Name("table"))).skip(5).take(1)
        .flat_map(|t| t.find(Name("tr"))).skip(2).take(1)
        .flat_map(|tr| tr.find(Name("td"))).nth(1);
    let rating = match rating {
        None => bail!("Can't find rating element"),
        Some(r) => r.text().parse::<f64>()?
    };
    Ok(rating)
}

pub fn get_user_ratings(game: &Game) -> impl Iterator<Item=Result<Vec<(User, f64)>, Error>> {
    UserIterator::new(game.id)
}
