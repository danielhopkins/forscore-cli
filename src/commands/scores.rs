use crate::cli::ScoresCommand;
use crate::db::{mark_modified, open_readonly, open_readwrite, warn_if_running};
use crate::error::Result;
use crate::itm::{update_itm, ItmUpdate};
use crate::models::key::MusicalKey;
use crate::models::score::{
    get_score_by_id, list_bookmarks, list_scores, list_scores_in_library, list_scores_in_setlist,
    resolve_score, search_scores,
};
use crate::models::setlist::resolve_setlist;
use crate::models::library::resolve_library;
use crate::models::meta::{get_or_create_composer, get_or_create_genre};
use crate::output::{output, output_score, ToTable};
use std::process::Command;

pub fn handle(cmd: ScoresCommand) -> Result<()> {
    match cmd {
        ScoresCommand::Ls { library, setlist, limit, sort, desc, scores_only, json } => {
            let conn = open_readonly()?;

            let is_filtered = setlist.is_some() || library.is_some();

            let mut scores = if let Some(setlist_id) = setlist {
                let sl = resolve_setlist(&conn, &setlist_id)?;
                list_scores_in_setlist(&conn, sl.id)?
            } else if let Some(library_id) = library {
                let lib = resolve_library(&conn, &library_id)?;
                list_scores_in_library(&conn, lib.id)?
            } else {
                list_scores(&conn, &sort, desc, limit, scores_only)?
            };

            // Apply limit for setlist/library views (they don't support it natively)
            if is_filtered {
                scores.truncate(limit);
            }

            // Load metadata for each score
            for score in &mut scores {
                let _ = score.load_metadata(&conn);
            }

            output(&scores, json);
        }

        ScoresCommand::Search {
            query,
            title,
            composer,
            genre,
            key,
            no_key,
            rating,
            no_rating,
            difficulty,
            limit,
            scores_only,
            json,
        } => {
            let conn = open_readonly()?;

            let key_code = if let Some(k) = key {
                Some(MusicalKey::from_string(&k)?.code)
            } else {
                None
            };

            let mut scores = search_scores(
                &conn,
                query.as_deref(),
                title.as_deref(),
                composer.as_deref(),
                genre.as_deref(),
                key_code,
                no_key,
                rating,
                no_rating,
                difficulty,
                limit,
                scores_only,
            )?;

            // Load metadata for each score
            for score in &mut scores {
                let _ = score.load_metadata(&conn);
            }

            output(&scores, json);
        }

        ScoresCommand::Show { identifier, json } => {
            let conn = open_readonly()?;
            let score = resolve_score(&conn, &identifier)?;
            output_score(&score, json);
        }

        ScoresCommand::Open { identifier } => {
            let conn = open_readonly()?;
            let score = resolve_score(&conn, &identifier)?;

            // Use forScore URL scheme
            let url = format!(
                "forscore://open?path={}",
                urlencoding::encode(&score.path)
            );

            Command::new("open").arg(&url).spawn()?;
            println!("Opening {} in forScore...", score.title);
        }

        ScoresCommand::Edit {
            identifier,
            title,
            composer,
            genre,
            key,
            rating,
            difficulty,
            tags: _,
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

            let score = resolve_score(&conn, &identifier)?;

            if dry_run {
                println!("Dry run - would update score ID {}:", score.id);
            }

            // Update title
            if let Some(new_title) = &title {
                if dry_run {
                    println!("  Title: {} -> {}", score.title, new_title);
                } else {
                    let sort_title = new_title.to_lowercase();
                    conn.execute(
                        "UPDATE ZITEM SET ZTITLE = ?, ZSORTTITLE = ? WHERE Z_PK = ?",
                        rusqlite::params![new_title, sort_title, score.id],
                    )?;
                }
            }

            // Update key
            if let Some(key_str) = &key {
                let key_obj = MusicalKey::from_string(key_str)?;
                if dry_run {
                    println!(
                        "  Key: {} -> {}",
                        score.key.map(|k| k.display()).unwrap_or_default(),
                        key_obj.display()
                    );
                } else {
                    conn.execute(
                        "UPDATE ZITEM SET ZKEY = ? WHERE Z_PK = ?",
                        [key_obj.code as i64, score.id],
                    )?;
                }
            }

            // Update rating
            if let Some(r) = rating {
                if r < 1 || r > 6 {
                    return Err(crate::error::ForScoreError::InvalidRating(r));
                }
                if dry_run {
                    println!(
                        "  Rating: {} -> {}",
                        score.rating.unwrap_or(0),
                        r
                    );
                } else {
                    conn.execute(
                        "UPDATE ZITEM SET ZRATING = ? WHERE Z_PK = ?",
                        [r as i64, score.id],
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
                        score.difficulty.unwrap_or(0),
                        d
                    );
                } else {
                    conn.execute(
                        "UPDATE ZITEM SET ZDIFFICULTY = ? WHERE Z_PK = ?",
                        [d as i64, score.id],
                    )?;
                }
            }

            // Update composer
            if let Some(composer_name) = &composer {
                if dry_run {
                    println!(
                        "  Composer: {} -> {}",
                        score.composers.first().cloned().unwrap_or_default(),
                        composer_name
                    );
                } else {
                    let composer_id = get_or_create_composer(&conn, composer_name)?;

                    // Remove existing composer links
                    conn.execute(
                        "DELETE FROM Z_4COMPOSERS WHERE Z_4ITEMS1 = ?",
                        [score.id],
                    )?;

                    // Add new link
                    conn.execute(
                        "INSERT INTO Z_4COMPOSERS (Z_4ITEMS1, Z_10COMPOSERS) VALUES (?, ?)",
                        [score.id, composer_id],
                    )?;
                }
            }

            // Update genre
            if let Some(genre_name) = &genre {
                if dry_run {
                    println!(
                        "  Genre: {} -> {}",
                        score.genres.first().cloned().unwrap_or_default(),
                        genre_name
                    );
                } else {
                    let genre_id = get_or_create_genre(&conn, genre_name)?;

                    // Remove existing genre links
                    conn.execute(
                        "DELETE FROM Z_4GENRES WHERE Z_4ITEMS4 = ?",
                        [score.id],
                    )?;

                    // Add new link
                    conn.execute(
                        "INSERT INTO Z_4GENRES (Z_4ITEMS4, Z_12GENRES) VALUES (?, ?)",
                        [score.id, genre_id],
                    )?;
                }
            }

            if !dry_run {
                // Mark the score as modified (update timestamp and version)
                mark_modified(&conn, score.id)?;

                // Also update the ITM file for sync
                let mut itm_update = ItmUpdate::new();
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

                match update_itm(&score.path, &itm_update) {
                    Ok(true) => println!("Updated score and ITM: {}", score.title),
                    Ok(false) => println!("Updated score: {} (no ITM file)", score.title),
                    Err(e) => {
                        println!("Updated score: {}", score.title);
                        eprintln!("Warning: Failed to update ITM file: {}", e);
                    }
                }
            }
        }
    }

    Ok(())
}

pub fn handle_bookmarks(score_identifier: &str, json: bool) -> Result<()> {
    let conn = open_readonly()?;
    let score = resolve_score(&conn, score_identifier)?;
    let bookmarks = list_bookmarks(&conn, score.id)?;

    if bookmarks.is_empty() {
        println!("No bookmarks in '{}'", score.title);
    } else {
        output(&bookmarks, json);
    }

    Ok(())
}
