use failure::{Error, ResultExt, bail};
use reqwest;
use select::document::Document;
use select::predicate::{Name, Class};
use crate::lib::{Game, User};

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

pub fn get_user_average_rating(user: &User) -> Result<f32, Error> {
    Ok(7.5) // TODO: stub
}
