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
            num_votes integer,
            updated datetime,
            stable integer
         )",
        NO_PARAMS,
    )?;
    conn.execute(
        "create table if not exists users (
            name text primary key,
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
    let mut stmt = conn.prepare("SELECT id, name, rating, num_votes FROM games order by rating desc")?;
    let games_iter = stmt
        .query_map(NO_PARAMS, |row| Game {
            id: row.get(0),
            name: row.get(1),
            rating: row.get(2),
            votes: row.get(3)
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
        let mut stmt = self.conn.prepare("select name from users where not stable limit 1")?;
        let user: Option<User> = match stmt.query_row(NO_PARAMS, |r| r.get(0)) {
            Ok(u) => Some(u),
            Err(rusqlite::Error::QueryReturnedNoRows) => None,
            Err(e) => bail!(e)
        };
        Ok(user)
    }

    pub fn update_user(&self, user: &User, trusted: bool) -> Result<(), Error> {
        match self.conn.execute("UPDATE users SET stable = 1, trusted = ?1 WHERE name = ?2",
                &[&trusted as &ToSql, user]) {
            Ok(_) => Ok(()),
            Err(err) => bail!(err)
        }
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

    pub fn add_users(&mut self, users: &[&User]) -> Result<(), Error> {
        let tx = self.conn.transaction()?;
        let now = Local::now();
        let zero = 0;
        for user in users {
            tx.execute("insert or ignore into users (name, updated, stable, trusted) values (?1, ?2, ?3, ?4)",
                &[user as &ToSql, &now.to_string(), &zero, &zero])?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn check_user(&self, user: &User) -> Result<Option<bool>, Error> {
        type Row = (bool, bool); // use to get rid of ugly r.get::<_,bool>(0)
        let mut stmt = self.conn.prepare("select stable, trusted from users where name = ?")?;
        let result:Option<bool> = match stmt.query_row(&[user as &ToSql], |r| -> Row { (r.get(0), r.get(1)) }) {
            Err(e) => bail!(e),
            Ok((false, _)) => None, // Unstable
            Ok((_, true)) => Some(true), // trusted
            Ok((_, false)) => Some(false) // not trusted
        };
        Ok(result)
    }

    pub fn update_game(&self, game: &Game) -> Result<(), Error> {
        // TODO rating, n of votes
        match self.conn.execute("UPDATE games SET stable = 1 WHERE id = ?", &[game.id]) {
            Ok(_) => Ok(()),
            Err(err) => bail!(err)
        }
    }
}
