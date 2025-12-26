use crate::cli::LibrariesCommand;
use crate::db::{open_readonly, open_readwrite, warn_if_running};
use crate::error::Result;
use crate::models::library::{
    add_score_to_library, list_libraries, remove_score_from_library, resolve_library,
};
use crate::models::score::{list_scores_in_library, resolve_score};
use crate::output::output;

pub fn handle(cmd: LibrariesCommand) -> Result<()> {
    match cmd {
        LibrariesCommand::Ls { json } => {
            let conn = open_readonly()?;
            let libraries = list_libraries(&conn)?;
            output(&libraries, json);
        }

        LibrariesCommand::Show { identifier, json } => {
            let conn = open_readonly()?;
            let library = resolve_library(&conn, &identifier)?;
            let mut scores = list_scores_in_library(&conn, library.id)?;

            // Load metadata (composers, genres, etc.) for each score
            for score in &mut scores {
                score.load_metadata(&conn)?;
            }

            println!(
                "Library: {} ({} scores)\n",
                library.title, library.score_count
            );
            output(&scores, json);
        }

        LibrariesCommand::AddScore { library, score } => {
            warn_if_running();
            let conn = open_readwrite()?;
            let lib = resolve_library(&conn, &library)?;
            let sc = resolve_score(&conn, &score)?;
            add_score_to_library(&conn, lib.id, sc.id)?;
            println!("Added '{}' to library '{}'", sc.title, lib.title);
        }

        LibrariesCommand::RemoveScore { library, score } => {
            warn_if_running();
            let conn = open_readwrite()?;
            let lib = resolve_library(&conn, &library)?;
            let sc = resolve_score(&conn, &score)?;
            remove_score_from_library(&conn, lib.id, sc.id)?;
            println!("Removed '{}' from library '{}'", sc.title, lib.title);
        }
    }

    Ok(())
}
