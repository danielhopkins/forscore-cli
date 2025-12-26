use crate::cli::SetlistsCommand;
use crate::db::{open_readonly, open_readwrite, warn_if_running};
use crate::error::Result;
use crate::models::score::{list_scores_in_setlist, resolve_score};
use crate::models::setlist::{
    add_score_to_setlist, create_setlist, delete_setlist, list_setlists, remove_score_from_setlist,
    rename_setlist, reorder_score_in_setlist, resolve_setlist,
};
use crate::output::output;
use crate::setlist_sync::{
    add_item_to_setlist_file, create_setlist_file, delete_setlist_file, remove_item_from_setlist_file,
    rename_setlist_file, reorder_setlist_file, SetlistItem,
};

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

            println!(
                "Setlist: {} ({} scores)\n",
                setlist.title, setlist.score_count
            );
            output(&scores, json);
        }

        SetlistsCommand::Create { name } => {
            warn_if_running();
            let conn = open_readwrite()?;
            let setlist = create_setlist(&conn, &name)?;

            // Create sync file
            match create_setlist_file(&name) {
                Ok(true) => println!("Created setlist '{}' (ID: {}) + sync file", setlist.title, setlist.id),
                Ok(false) => println!("Created setlist '{}' (ID: {}) (sync file exists)", setlist.title, setlist.id),
                Err(e) => {
                    println!("Created setlist '{}' (ID: {}) (database only)", setlist.title, setlist.id);
                    eprintln!("Warning: Failed to create sync file: {}", e);
                }
            }
        }

        SetlistsCommand::Rename {
            identifier,
            new_name,
        } => {
            warn_if_running();
            let conn = open_readwrite()?;
            let setlist = resolve_setlist(&conn, &identifier)?;
            let old_name = setlist.title.clone();
            rename_setlist(&conn, setlist.id, &new_name)?;

            // Rename sync file
            match rename_setlist_file(&old_name, &new_name) {
                Ok(true) => println!("Renamed '{}' to '{}' + updated sync file", old_name, new_name),
                Ok(false) => println!("Renamed '{}' to '{}' (no sync file found)", old_name, new_name),
                Err(e) => {
                    println!("Renamed '{}' to '{}' (database only)", old_name, new_name);
                    eprintln!("Warning: Failed to update sync file: {}", e);
                }
            }
        }

        SetlistsCommand::Delete { identifier } => {
            warn_if_running();
            let conn = open_readwrite()?;
            let setlist = resolve_setlist(&conn, &identifier)?;
            let name = setlist.title.clone();
            delete_setlist(&conn, setlist.id)?;

            // Delete sync file
            match delete_setlist_file(&name) {
                Ok(true) => println!("Deleted setlist '{}' + sync file", name),
                Ok(false) => println!("Deleted setlist '{}' (no sync file found)", name),
                Err(e) => {
                    println!("Deleted setlist '{}' (database only)", name);
                    eprintln!("Warning: Failed to delete sync file: {}", e);
                }
            }
        }

        SetlistsCommand::AddScore { setlist, score } => {
            warn_if_running();
            let conn = open_readwrite()?;
            let sl = resolve_setlist(&conn, &setlist)?;
            let sc = resolve_score(&conn, &score)?;
            add_score_to_setlist(&conn, sl.id, sc.id)?;

            // Update sync file
            let item = SetlistItem {
                file_path: sc.path.clone(),
                title: sc.title.clone(),
                identifier: sc.uuid.clone().unwrap_or_default(),
                is_bookmark: false,
                first_page: None,
                last_page: None,
            };
            match add_item_to_setlist_file(&sl.title, &item) {
                Ok(true) => println!("Added '{}' to setlist '{}' + sync file", sc.title, sl.title),
                Ok(false) => println!("Added '{}' to setlist '{}' (already in sync file)", sc.title, sl.title),
                Err(e) => {
                    println!("Added '{}' to setlist '{}' (database only)", sc.title, sl.title);
                    eprintln!("Warning: Failed to update sync file: {}", e);
                }
            }
        }

        SetlistsCommand::RemoveScore { setlist, score } => {
            warn_if_running();
            let conn = open_readwrite()?;
            let sl = resolve_setlist(&conn, &setlist)?;
            let sc = resolve_score(&conn, &score)?;
            remove_score_from_setlist(&conn, sl.id, sc.id)?;

            // Update sync file
            let identifier = sc.uuid.clone().unwrap_or_default();
            match remove_item_from_setlist_file(&sl.title, &identifier) {
                Ok(true) => println!("Removed '{}' from setlist '{}' + sync file", sc.title, sl.title),
                Ok(false) => println!("Removed '{}' from setlist '{}' (not in sync file)", sc.title, sl.title),
                Err(e) => {
                    println!("Removed '{}' from setlist '{}' (database only)", sc.title, sl.title);
                    eprintln!("Warning: Failed to update sync file: {}", e);
                }
            }
        }

        SetlistsCommand::Reorder {
            setlist,
            score,
            position,
        } => {
            warn_if_running();
            let conn = open_readwrite()?;
            let sl = resolve_setlist(&conn, &setlist)?;
            let sc = resolve_score(&conn, &score)?;
            reorder_score_in_setlist(&conn, sl.id, sc.id, position)?;

            // Rebuild sync file with new order from database
            let scores = list_scores_in_setlist(&conn, sl.id)?;
            let items: Vec<SetlistItem> = scores
                .iter()
                .map(|s| SetlistItem {
                    file_path: s.path.clone(),
                    title: s.title.clone(),
                    identifier: s.uuid.clone().unwrap_or_default(),
                    is_bookmark: false,
                    first_page: None,
                    last_page: None,
                })
                .collect();

            match reorder_setlist_file(&sl.title, &items) {
                Ok(true) => println!(
                    "Moved '{}' to position {} in '{}' + updated sync file",
                    sc.title, position, sl.title
                ),
                Ok(false) => println!(
                    "Moved '{}' to position {} in '{}' (no sync file)",
                    sc.title, position, sl.title
                ),
                Err(e) => {
                    println!(
                        "Moved '{}' to position {} in '{}' (database only)",
                        sc.title, position, sl.title
                    );
                    eprintln!("Warning: Failed to update sync file: {}", e);
                }
            }
        }
    }

    Ok(())
}
