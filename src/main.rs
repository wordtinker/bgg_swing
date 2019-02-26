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
use std::io::Write;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use ctrlc;

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
    //
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })?;
    //
    let config = core::config()?;
    println!("Start balancing.");
    let mut stdout = StandardStream::stdout(ColorChoice::Always);
    let mut seen_users: u32 = 0;
    let mut balanced_games: u32 = 0;
    core::stabilize(config, running, |m| match m {
        Message::UserProgress(_) => {
            seen_users += 1;
            if seen_users % 50 == 0 {
                stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green))).unwrap();
                writeln!(&mut stdout, "Found another 50.").unwrap();
            };
        },
        Message::GameProgress(game) => {
            balanced_games += 1;
            stdout.set_color(ColorSpec::new().set_fg(Some(Color::Yellow))).unwrap();
            writeln!(&mut stdout, "{} is balanced.", game.name).unwrap();
        },
        Message::Notification(error) => {
            stdout.set_color(ColorSpec::new().set_fg(Some(Color::Red))).unwrap();
            writeln!(&mut stdout, "{:?}", error).unwrap();
        },
        Message::Info(game) => {
            stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green))).unwrap();
            writeln!(&mut stdout, "About to ask BGG about {}", game.name).unwrap();
        },
        _ => {} 
    })?;
    println!("Seen {} users today and {} balanced games.", seen_users, balanced_games);
    println!("Finished balancing.");
    Ok(())
}

fn review_users() -> Result<(), Error> {
    // TODO: make unstable again. trusted after 180 untrusted 90
    // any update on user in that mode
    // makes gametable unbalanced
    Ok(())
}
