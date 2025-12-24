use crate::cli::SetlistsCommand;
use crate::db::{open_readonly, open_readwrite, warn_if_running};
use crate::error::Result;
use crate::models::score::{list_scores_in_setlist, resolve_score};
use crate::models::setlist::{
    add_score_to_setlist, create_setlist, delete_setlist, list_setlists, remove_score_from_setlist,
    rename_setlist, reorder_score_in_setlist, resolve_setlist,
};
use crate::output::{output, ToTable};

pub fn handle(cmd: SetlistsCommand) -> Result<()> {
    match cmd {
        SetlistsCommand::Ls { json } => {
            let conn = open_readonly()?;
            let setlists = list_setlists(&conn)?;
            output(&setlists, json);
        }

        SetlistsCommand::Show { identifier, json } => {
            let conn = open_readonly()?;
            let setlist = resolve_setlist(&conn, &identifier)?;
            let mut scores = list_scores_in_setlist(&conn, setlist.id)?;

            // Load metadata (composers, genres, etc.) for each score
            for score in &mut scores {
                score.load_metadata(&conn)?;
            }

            println!("Setlist: {} ({} scores)\n", setlist.title, setlist.score_count);
            output(&scores, json);
        }

        SetlistsCommand::Create { name } => {
            warn_if_running();
            let conn = open_readwrite()?;
            let setlist = create_setlist(&conn, &name)?;
            println!("Created setlist '{}' (ID: {})", setlist.title, setlist.id);
        }

        SetlistsCommand::Rename { identifier, new_name } => {
            warn_if_running();
            let conn = open_readwrite()?;
            let setlist = resolve_setlist(&conn, &identifier)?;
            rename_setlist(&conn, setlist.id, &new_name)?;
            println!("Renamed '{}' to '{}'", setlist.title, new_name);
        }

        SetlistsCommand::Delete { identifier } => {
            warn_if_running();
            let conn = open_readwrite()?;
            let setlist = resolve_setlist(&conn, &identifier)?;
            delete_setlist(&conn, setlist.id)?;
            println!("Deleted setlist '{}'", setlist.title);
        }

        SetlistsCommand::AddScore { setlist, score } => {
            warn_if_running();
            let conn = open_readwrite()?;
            let sl = resolve_setlist(&conn, &setlist)?;
            let sc = resolve_score(&conn, &score)?;
            add_score_to_setlist(&conn, sl.id, sc.id)?;
            println!("Added '{}' to setlist '{}'", sc.title, sl.title);
        }

        SetlistsCommand::RemoveScore { setlist, score } => {
            warn_if_running();
            let conn = open_readwrite()?;
            let sl = resolve_setlist(&conn, &setlist)?;
            let sc = resolve_score(&conn, &score)?;
            remove_score_from_setlist(&conn, sl.id, sc.id)?;
            println!("Removed '{}' from setlist '{}'", sc.title, sl.title);
        }

        SetlistsCommand::Reorder { setlist, score, position } => {
            warn_if_running();
            let conn = open_readwrite()?;
            let sl = resolve_setlist(&conn, &setlist)?;
            let sc = resolve_score(&conn, &score)?;
            reorder_score_in_setlist(&conn, sl.id, sc.id, position)?;
            println!("Moved '{}' to position {} in '{}'", sc.title, position, sl.title);
        }
    }

    Ok(())
}
