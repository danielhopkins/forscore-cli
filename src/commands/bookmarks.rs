use crate::cli::BookmarksCommand;
use crate::db::{mark_modified, open_readonly, open_readwrite, warn_if_running};
use crate::error::Result;
use crate::itm::{delete_bookmark_from_itm, update_bookmark_in_itm, ItmBookmarkUpdate};
use crate::models::key::MusicalKey;
use crate::models::meta::{get_or_create_composer, get_or_create_genre};
use crate::models::score::{get_bookmark_by_id, list_bookmarks, resolve_score};
use crate::output::output;

pub fn handle(cmd: BookmarksCommand) -> Result<()> {
    match cmd {
        BookmarksCommand::Ls { score, json } => {
            let conn = open_readonly()?;
            let score = resolve_score(&conn, &score)?;
            let bookmarks = list_bookmarks(&conn, score.id)?;

            if bookmarks.is_empty() {
                println!("No bookmarks in '{}'", score.title);
            } else {
                output(&bookmarks, json);
            }
        }

        BookmarksCommand::Show { id, json } => {
            let conn = open_readonly()?;
            let bookmark = get_bookmark_by_id(&conn, id)?;

            if json {
                println!("{}", serde_json::to_string_pretty(&bookmark).unwrap());
            } else {
                println!("ID:         {}", bookmark.id);
                println!("Title:      {}", bookmark.title);
                println!("Path:       {}", bookmark.path);
                if let Some(uuid) = &bookmark.uuid {
                    println!("UUID:       {}", uuid);
                }
                if let (Some(start), Some(end)) = (bookmark.start_page, bookmark.end_page) {
                    if start == end {
                        println!("Page:       {}", start);
                    } else {
                        println!("Pages:      {}-{}", start, end);
                    }
                }
                if let Some(key) = &bookmark.key {
                    println!("Key:        {}", key.display());
                }
                if let Some(rating) = bookmark.rating {
                    println!("Rating:     {} ({})", "★".repeat(rating as usize), rating);
                }
                if let Some(difficulty) = bookmark.difficulty {
                    println!("Difficulty: {}", difficulty);
                }
                if !bookmark.composers.is_empty() {
                    println!("Composers:  {}", bookmark.composers.join(", "));
                }
                if !bookmark.genres.is_empty() {
                    println!("Genres:     {}", bookmark.genres.join(", "));
                }
            }
        }

        BookmarksCommand::Edit {
            id,
            title,
            composer,
            genre,
            key,
            rating,
            difficulty,
            dry_run,
        } => {
            if !dry_run {
                warn_if_running();
            }

            let conn = if dry_run {
                open_readonly()?
            } else {
                open_readwrite()?
            };

            let bookmark = get_bookmark_by_id(&conn, id)?;

            if dry_run {
                println!("Dry run - would update bookmark ID {}:", bookmark.id);
            }

            // Update title
            if let Some(new_title) = &title {
                if dry_run {
                    println!("  Title: {} -> {}", bookmark.title, new_title);
                } else {
                    let sort_title = new_title.to_lowercase();
                    conn.execute(
                        "UPDATE ZITEM SET ZTITLE = ?, ZSORTTITLE = ? WHERE Z_PK = ?",
                        rusqlite::params![new_title, sort_title, bookmark.id],
                    )?;
                }
            }

            // Update key
            if let Some(key_str) = &key {
                let key_obj = MusicalKey::from_string(key_str)?;
                if dry_run {
                    println!(
                        "  Key: {} -> {}",
                        bookmark.key.map(|k| k.display()).unwrap_or_default(),
                        key_obj.display()
                    );
                } else {
                    conn.execute(
                        "UPDATE ZITEM SET ZKEY = ? WHERE Z_PK = ?",
                        [key_obj.code as i64, bookmark.id],
                    )?;
                }
            }

            // Update rating
            if let Some(r) = rating {
                if r < 1 || r > 6 {
                    return Err(crate::error::ForScoreError::InvalidRating(r));
                }
                if dry_run {
                    println!("  Rating: {} -> {}", bookmark.rating.unwrap_or(0), r);
                } else {
                    conn.execute(
                        "UPDATE ZITEM SET ZRATING = ? WHERE Z_PK = ?",
                        [r as i64, bookmark.id],
                    )?;
                }
            }

            // Update difficulty
            if let Some(d) = difficulty {
                if d < 1 || d > 5 {
                    return Err(crate::error::ForScoreError::InvalidDifficulty(d));
                }
                if dry_run {
                    println!(
                        "  Difficulty: {} -> {}",
                        bookmark.difficulty.unwrap_or(0),
                        d
                    );
                } else {
                    conn.execute(
                        "UPDATE ZITEM SET ZDIFFICULTY = ? WHERE Z_PK = ?",
                        [d as i64, bookmark.id],
                    )?;
                }
            }

            // Update composer
            if let Some(composer_name) = &composer {
                if dry_run {
                    println!(
                        "  Composer: {} -> {}",
                        bookmark.composers.first().cloned().unwrap_or_default(),
                        composer_name
                    );
                } else {
                    let composer_id = get_or_create_composer(&conn, composer_name)?;

                    // Remove existing composer links
                    conn.execute(
                        "DELETE FROM Z_4COMPOSERS WHERE Z_4ITEMS1 = ?",
                        [bookmark.id],
                    )?;

                    // Add new link
                    conn.execute(
                        "INSERT INTO Z_4COMPOSERS (Z_4ITEMS1, Z_10COMPOSERS) VALUES (?, ?)",
                        [bookmark.id, composer_id],
                    )?;
                }
            }

            // Update genre
            if let Some(genre_name) = &genre {
                if dry_run {
                    println!(
                        "  Genre: {} -> {}",
                        bookmark.genres.first().cloned().unwrap_or_default(),
                        genre_name
                    );
                } else {
                    let genre_id = get_or_create_genre(&conn, genre_name)?;

                    // Remove existing genre links
                    conn.execute("DELETE FROM Z_4GENRES WHERE Z_4ITEMS4 = ?", [bookmark.id])?;

                    // Add new link
                    conn.execute(
                        "INSERT INTO Z_4GENRES (Z_4ITEMS4, Z_12GENRES) VALUES (?, ?)",
                        [bookmark.id, genre_id],
                    )?;
                }
            }

            if !dry_run {
                // Mark the bookmark as modified
                mark_modified(&conn, bookmark.id)?;

                // Also update the ITM file for sync
                let mut itm_update = ItmBookmarkUpdate::new();
                itm_update.title = title.clone();
                itm_update.composer = composer.clone();
                itm_update.genre = genre.clone();
                if let Some(key_str) = &key {
                    if let Ok(key_obj) = MusicalKey::from_string(key_str) {
                        itm_update.key = Some(key_obj.code as i64);
                    }
                }
                itm_update.rating = rating.map(|r| r as i64);
                itm_update.difficulty = difficulty.map(|d| d as i64);

                // Get the bookmark's UUID for matching in ITM
                let uuid = bookmark.uuid.as_deref();

                match update_bookmark_in_itm(&bookmark.path, uuid, &itm_update) {
                    Ok(true) => println!("Updated bookmark and ITM: {}", bookmark.title),
                    Ok(false) => println!("Updated bookmark: {} (no ITM match)", bookmark.title),
                    Err(e) => {
                        println!("Updated bookmark: {}", bookmark.title);
                        eprintln!("Warning: Failed to update ITM file: {}", e);
                    }
                }
            }
        }

        BookmarksCommand::Delete { id } => {
            warn_if_running();

            let conn = open_readwrite()?;
            let bookmark = get_bookmark_by_id(&conn, id)?;

            // Delete from database
            conn.execute("DELETE FROM ZITEM WHERE Z_PK = ?", [id])?;

            // Delete composer links
            conn.execute("DELETE FROM Z_4COMPOSERS WHERE Z_4ITEMS1 = ?", [id])?;

            // Delete genre links
            conn.execute("DELETE FROM Z_4GENRES WHERE Z_4ITEMS4 = ?", [id])?;

            // Delete from ITM file
            let uuid = bookmark.uuid.as_deref();
            match delete_bookmark_from_itm(&bookmark.path, uuid) {
                Ok(true) => println!("Deleted bookmark and ITM: {}", bookmark.title),
                Ok(false) => println!("Deleted bookmark: {} (no ITM match)", bookmark.title),
                Err(e) => {
                    println!("Deleted bookmark: {}", bookmark.title);
                    eprintln!("Warning: Failed to update ITM file: {}", e);
                }
            }
        }
    }

    Ok(())
}
