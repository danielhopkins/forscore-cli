mod cli;
mod commands;
mod db;
mod error;
mod itm;
mod models;
mod output;

use clap::Parser;
use cli::{Cli, Commands, SyncCommand};

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn run() -> error::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Scores { command } => commands::scores::handle(command)?,

        Commands::Setlists { command } => commands::setlists::handle(command)?,

        Commands::Libraries { command } => commands::libraries::handle(command)?,

        Commands::Composers { command } => commands::metadata::handle_composers(command)?,

        Commands::Genres { command } => commands::metadata::handle_genres(command)?,

        Commands::Tags { command } => commands::metadata::handle_tags(command)?,

        Commands::Export { command } => commands::export::handle(command)?,

        Commands::Import { command } => commands::import::handle(command)?,

        Commands::Bookmarks { command } => commands::bookmarks::handle(command)?,

        Commands::Info => commands::utils::info()?,

        Commands::Backup { output } => commands::utils::backup(output)?,

        Commands::Sync { command } => match command {
            None => commands::utils::sync_status()?,
            Some(SyncCommand::Log { limit }) => commands::utils::sync_log(limit)?,
            Some(SyncCommand::Trigger) => commands::utils::sync_trigger()?,
        },

        Commands::Fixes { command } => commands::fixes::handle(command)?,
    }

    Ok(())
}
