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
use colored::*;

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
            println!("{}\t{}\t{:.2}\t{}", game.id, game.name, game.rating, game.votes);
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
    let mut seen_users: u32 = 0;
    core::stabilize(config, |m| match m {
        Message::UserProgress(_) => {
            seen_users += 1;
            if seen_users % 50 == 0 {
                println!("Found another 50.")
            };
        },
        Message::GameProgress(game) =>{
            let m = format!("{} is balanced.", game.name);
            println!("{}", m.yellow());
        },
        Message::Notification(error) => {
            let e = format!("{:?}", error);
            eprintln!("{}", e.red());
        } ,
        Message::Info(game) => println!("About to ask BGG about {}", game.name),
        _ => {} 
    })?;
    println!("Seen {} users today.", seen_users);
    println!("Finished balancing.");
    Ok(())
}

fn review_users() -> Result<(), Error> {
    // TODO: make unstable again. trusted after 180 untrusted 90
    // any update on user in that mode
    // makes gametable unbalanced
    Ok(())
}
