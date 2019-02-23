use rusqlite::{Connection, NO_PARAMS, OpenFlags};
use rusqlite::types::ToSql;
use failure::{Error, ResultExt, bail};
use chrono::Local;
use crate::lib::{Game, User};

const DB_FILE_NAME: &str = "top.db";

pub fn initialize() -> Result<(), Error> {
    let conn = Connection::open(DB_FILE_NAME)?;
    // create db file
    conn.execute(
        "create table if not exists games (
            id integer primary key,
            name text not null,
            rating real,
            updated datetime,
            stable integer
         )",
        NO_PARAMS,
    )?;
    conn.execute(
        "create table if not exists users (
            id integer primary key,
            updated datetime,
            stable integer,
            trusted integer
         )",
        NO_PARAMS,
    )?;
    Ok(())
}

pub fn drop_all_games() -> Result<(), Error> {
    let conn = Connection::open(DB_FILE_NAME)?;
    conn.execute("delete from games", NO_PARAMS)?;
    Ok(())
}

pub fn add_games(games: Vec<Game>) -> Result<(), Error> {
    let mut conn = Connection::open(DB_FILE_NAME)?;
    let tx = conn.transaction()?;
    let now = Local::now();
    let zero = 0;
    for game in  games {
        tx.execute("insert into games (id, name, updated, stable) values (?1, ?2, ?3, ?4)",
            &[&game.id as &ToSql, &game.name, &now.to_string(), &zero])?;
    }
    tx.commit()?;
    Ok(())
}

pub fn is_stable() -> Result<bool, Error> {
    let conn = Connection::open(DB_FILE_NAME)?;
    let mut stmt = conn.prepare("select count(*) from games where not stable")?;
    let count: u32 = stmt.query_row(NO_PARAMS, |r| r.get(0))?;
    Ok(count == 0)
}

pub fn get_all_games() -> Result<Vec<Game>, Error> {
    let conn = Connection::open(DB_FILE_NAME)?;
    let mut stmt = conn.prepare("SELECT id, name, rating FROM games order by rating desc")?;
    let games_iter = stmt
        .query_map(NO_PARAMS, |row| Game {
            id: row.get(0),
            name: row.get(1),
            rating: row.get(2),
        })?;
    let mut games = Vec::new();
    for game in games_iter {
        games.push(game?);
    }
    Ok(games)
}

pub struct DbConn {
    conn: Connection
}

impl DbConn {
    pub fn new() -> Result<DbConn, Error> {
        let conn = Connection::open_with_flags(
            DB_FILE_NAME,
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX // for multi thread
            )?;
        Ok(DbConn { conn })
    }

    pub fn get_unstable_user(&self) -> Result<Option<User>, Error> {
        return Ok(Some(2584));
        Ok(None) // TODO: stub
    }

    pub fn update_user(&self, user: &User, trusted: bool) -> Result<(), Error> {
        Ok(()) // TODO: stub
    }

    pub fn get_unstable_game(&self) -> Result<Option<Game>, Error> {
        let mut stmt = self.conn.prepare("select id, name from games where not stable order by random() limit 1")?;
        let game: Option<Game> = match stmt.query_row(NO_PARAMS, |r| Game::new(r.get(0), r.get(1))) {
            Ok(g) => Some(g),
            Err(rusqlite::Error::QueryReturnedNoRows) => None,
            Err(e) => bail!(e)
        };
        Ok(game) 
    }

    pub fn add_users(&self, users: &[&User]) -> Result<(), Error> {
        Ok(()) // TODO: stub
    }

    pub fn check_user(&self, user: User) -> Result<Option<bool>, Error> {
        // None - unstable
        // Some(true) - trusted
        // Some(false) - not trusted
        Ok(None) // TODO Stub
    }

    pub fn update_game(&self, game: &Game, rating: f32) -> Result<(), Error> {
        Ok(()) // TODO: stub
    }
}
