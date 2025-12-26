use crate::cli::SetlistsCommand;
use crate::db::{entity, open_readonly, open_readwrite, warn_if_running};
use crate::error::Result;
use crate::models::score::{list_scores_in_setlist, resolve_bookmark, resolve_score};
use crate::models::setlist::{
    add_item_to_setlist, add_score_to_setlist, create_setlist, delete_setlist, list_setlists,
    remove_score_from_setlist, rename_setlist, reorder_score_in_setlist, resolve_setlist,
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

            // Try as score first, then as bookmark
            if let Ok(sc) = resolve_score(&conn, &score) {
                add_score_to_setlist(&conn, sl.id, sc.id)?;

                // Get the UUID that was used (either reused or newly generated)
                let identifier: String = conn
                    .query_row(
                        "SELECT ZUUID FROM ZCYLON WHERE ZSETLIST = ? AND ZITEM = ?",
                        [sl.id, sc.id],
                        |row| row.get(0),
                    )
                    .unwrap_or_default();

                let item = SetlistItem {
                    file_path: sc.path.clone(),
                    title: sc.title.clone(),
                    identifier,
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
            } else if let Ok(bm) = resolve_bookmark(&conn, &score) {
                add_item_to_setlist(&conn, sl.id, bm.id, entity::BOOKMARK)?;

                // Get the UUID that was used
                let identifier: String = conn
                    .query_row(
                        "SELECT ZUUID FROM ZCYLON WHERE ZSETLIST = ? AND ZITEM = ?",
                        [sl.id, bm.id],
                        |row| row.get(0),
                    )
                    .unwrap_or_default();

                let item = SetlistItem {
                    file_path: bm.path.clone(),
                    title: bm.title.clone(),
                    identifier,
                    is_bookmark: true,
                    first_page: bm.start_page.map(|p| p as i64),
                    last_page: bm.end_page.map(|p| p as i64),
                };
                match add_item_to_setlist_file(&sl.title, &item) {
                    Ok(true) => println!("Added bookmark '{}' to setlist '{}' + sync file", bm.title, sl.title),
                    Ok(false) => println!("Added bookmark '{}' to setlist '{}' (already in sync file)", bm.title, sl.title),
                    Err(e) => {
                        println!("Added bookmark '{}' to setlist '{}' (database only)", bm.title, sl.title);
                        eprintln!("Warning: Failed to update sync file: {}", e);
                    }
                }
            } else {
                return Err(crate::error::ForScoreError::Other(format!(
                    "Score or bookmark not found: {}",
                    score
                )));
            }
        }

        SetlistsCommand::RemoveScore { setlist, score } => {
            warn_if_running();
            let conn = open_readwrite()?;
            let sl = resolve_setlist(&conn, &setlist)?;

            // Try as score first, then as bookmark
            let (item_id, item_title) = if let Ok(sc) = resolve_score(&conn, &score) {
                (sc.id, sc.title)
            } else if let Ok(bm) = resolve_bookmark(&conn, &score) {
                (bm.id, bm.title)
            } else {
                return Err(crate::error::ForScoreError::Other(format!(
                    "Score or bookmark not found: {}",
                    score
                )));
            };

            // Get the UUID from ZCYLON before deleting (this is what's in the sync file)
            let identifier: String = conn
                .query_row(
                    "SELECT ZUUID FROM ZCYLON WHERE ZSETLIST = ? AND ZITEM = ?",
                    [sl.id, item_id],
                    |row| row.get(0),
                )
                .unwrap_or_default();

            remove_score_from_setlist(&conn, sl.id, item_id)?;

            // Update sync file
            match remove_item_from_setlist_file(&sl.title, &identifier) {
                Ok(true) => println!("Removed '{}' from setlist '{}' + sync file", item_title, sl.title),
                Ok(false) => println!("Removed '{}' from setlist '{}' (not in sync file)", item_title, sl.title),
                Err(e) => {
                    println!("Removed '{}' from setlist '{}' (database only)", item_title, sl.title);
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

            // Try as score first, then as bookmark
            let (item_id, item_title) = if let Ok(sc) = resolve_score(&conn, &score) {
                (sc.id, sc.title)
            } else if let Ok(bm) = resolve_bookmark(&conn, &score) {
                (bm.id, bm.title)
            } else {
                return Err(crate::error::ForScoreError::Other(format!(
                    "Score or bookmark not found: {}",
                    score
                )));
            };

            reorder_score_in_setlist(&conn, sl.id, item_id, position)?;

            // Rebuild sync file with new order from database
            // Query items with their UUIDs and entity types from ZCYLON
            let mut stmt = conn.prepare(
                "SELECT c.ZITEM, c.ZUUID, c.Z4_ITEM, i.ZPATH, i.ZTITLE, i.ZSTARTPAGE, i.ZENDPAGE
                 FROM ZCYLON c
                 JOIN ZITEM i ON c.ZITEM = i.Z_PK
                 WHERE c.ZSETLIST = ?
                 ORDER BY c.Z_PK"
            )?;
            let mut items: Vec<SetlistItem> = Vec::new();
            let rows = stmt.query_map([sl.id], |row| {
                Ok((
                    row.get::<_, String>(1)?,           // ZUUID
                    row.get::<_, i32>(2)?,              // Z4_ITEM (entity type)
                    row.get::<_, String>(3)?,           // ZPATH
                    row.get::<_, String>(4)?,           // ZTITLE
                    row.get::<_, Option<i32>>(5)?,      // ZSTARTPAGE
                    row.get::<_, Option<i32>>(6)?,      // ZENDPAGE
                ))
            })?;
            for row in rows {
                let (identifier, entity_type, path, title, start_page, end_page) = row?;
                let is_bookmark = entity_type == entity::BOOKMARK;
                items.push(SetlistItem {
                    file_path: path,
                    title,
                    identifier,
                    is_bookmark,
                    first_page: if is_bookmark { start_page.map(|p| p as i64) } else { None },
                    last_page: if is_bookmark { end_page.map(|p| p as i64) } else { None },
                });
            }

            match reorder_setlist_file(&sl.title, &items) {
                Ok(true) => println!(
                    "Moved '{}' to position {} in '{}' + updated sync file",
                    item_title, position, sl.title
                ),
                Ok(false) => println!(
                    "Moved '{}' to position {} in '{}' (no sync file)",
                    item_title, position, sl.title
                ),
                Err(e) => {
                    println!(
                        "Moved '{}' to position {} in '{}' (database only)",
                        item_title, position, sl.title
                    );
                    eprintln!("Warning: Failed to update sync file: {}", e);
                }
            }
        }
    }

    Ok(())
}
