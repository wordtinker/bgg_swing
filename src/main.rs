mod cli;
mod core;
mod db;
mod bgg;
mod lib;

use crate::core::Message;
use cli::Cli;
use structopt::StructOpt;
use failure::Error;
use exitfailure::ExitFailure;
use std::time::Duration;

fn main() -> Result<(), ExitFailure> {
    let cli = Cli::from_args();
    match cli {
        Cli::New { } => create_structure()?,
        Cli::Report { } => make_report()?,
        Cli::Pull { } => pull_games()?,
        Cli::Balance { } => stabilize()?,
        Cli::Review { } => review_users()?
    }
    Ok(())
}

fn create_structure() -> Result<(), Error> {
    core::create_structure()?;
    println!("Created initial structure files.");
    Ok(())
}

fn make_report() -> Result<(), Error> {
    let games = core::make_report()?;
    if games.is_empty() {
        println!("Game list is not stable enough.");
    } else {
        for game in games {
            println!("{}\t{}\t{}", game.id, game.name, game.rating);
        }
    }
    Ok(())
}

fn pull_games() -> Result<(), Error> {
    let config = core::config()?;
    println!("Starting download.");
    core::pull_games(config.limit, |i| {
        println!("Downloaded page: {}", i);
    })?;
    println!("Finished download.");
    Ok(())
}

fn stabilize() -> Result<(), Error> {
    let config = core::config()?;
    println!("Start balancing.");
    core::stabilize(config.attempts, Duration::from_millis(config.delay as u64), |m| match m { // TODO: new format?
        Message::UserProgress(user) => println!("{:?}", user),
        Message::GameProgress(game) => println!("{:?}", game),
        Message::Notification(error) => println!("{:?}", error),
        _ => {} 
    })?;

    Ok(())
}

fn review_users() -> Result<(), Error> {
    // TODO: make unstable again. trusted after 180 untrusted 90
    // any update on user in that mode
    // makes gametable unbalanced
    Ok(())
}
