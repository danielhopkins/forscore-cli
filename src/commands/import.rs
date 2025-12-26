use crate::cli::ImportCommand;
use crate::db::{mark_modified, open_readonly, open_readwrite, warn_if_running};
use crate::error::{ForScoreError, Result};
use crate::models::key::MusicalKey;
use crate::models::meta::{get_or_create_composer, get_or_create_genre};
use crate::models::score::get_score_by_id;
use csv::Reader;
use std::fs::File;

pub fn handle(cmd: ImportCommand) -> Result<()> {
    match cmd {
        ImportCommand::Csv { file, dry_run } => {
            if !dry_run {
                warn_if_running();
            }

            let conn = if dry_run {
                open_readonly()?
            } else {
                open_readwrite()?
            };

            let csv_file = File::open(&file)?;
            let mut rdr = Reader::from_reader(csv_file);

            let headers = rdr.headers()?.clone();

            // Find column indices
            let id_idx = headers.iter().position(|h| h == "id");
            let title_idx = headers.iter().position(|h| h == "title");
            let composer_idx = headers.iter().position(|h| h == "composer");
            let genre_idx = headers.iter().position(|h| h == "genre");
            let key_idx = headers.iter().position(|h| h == "key");
            let rating_idx = headers.iter().position(|h| h == "rating");
            let difficulty_idx = headers.iter().position(|h| h == "difficulty");

            let id_idx =
                id_idx.ok_or_else(|| ForScoreError::Other("CSV must have 'id' column".into()))?;

            let mut updated = 0;
            let mut errors = 0;

            for result in rdr.records() {
                let record = result?;

                let id: i64 = match record.get(id_idx).and_then(|s| s.parse().ok()) {
                    Some(id) => id,
                    None => {
                        errors += 1;
                        continue;
                    }
                };

                // Verify score exists
                if get_score_by_id(&conn, id).is_err() {
                    eprintln!("Score ID {} not found, skipping", id);
                    errors += 1;
                    continue;
                }

                if dry_run {
                    println!("Would update score ID {}:", id);
                }

                // Update title
                if let Some(idx) = title_idx {
                    if let Some(title) = record.get(idx) {
                        if !title.is_empty() {
                            if dry_run {
                                println!("  title = {}", title);
                            } else {
                                let sort_title = title.to_lowercase();
                                conn.execute(
                                    "UPDATE ZITEM SET ZTITLE = ?, ZSORTTITLE = ? WHERE Z_PK = ?",
                                    rusqlite::params![title, sort_title, id],
                                )?;
                            }
                        }
                    }
                }

                // Update key
                if let Some(idx) = key_idx {
                    if let Some(key_str) = record.get(idx) {
                        if !key_str.is_empty() {
                            if let Ok(key) = MusicalKey::from_string(key_str) {
                                if dry_run {
                                    println!("  key = {}", key.display());
                                } else {
                                    conn.execute(
                                        "UPDATE ZITEM SET ZKEY = ? WHERE Z_PK = ?",
                                        [key.code as i64, id],
                                    )?;
                                }
                            }
                        }
                    }
                }

                // Update rating
                if let Some(idx) = rating_idx {
                    if let Some(rating_str) = record.get(idx) {
                        if let Ok(rating) = rating_str.parse::<i32>() {
                            if rating >= 1 && rating <= 6 {
                                if dry_run {
                                    println!("  rating = {}", rating);
                                } else {
                                    conn.execute(
                                        "UPDATE ZITEM SET ZRATING = ? WHERE Z_PK = ?",
                                        [rating as i64, id],
                                    )?;
                                }
                            }
                        }
                    }
                }

                // Update difficulty
                if let Some(idx) = difficulty_idx {
                    if let Some(diff_str) = record.get(idx) {
                        if let Ok(diff) = diff_str.parse::<i32>() {
                            if diff >= 1 && diff <= 5 {
                                if dry_run {
                                    println!("  difficulty = {}", diff);
                                } else {
                                    conn.execute(
                                        "UPDATE ZITEM SET ZDIFFICULTY = ? WHERE Z_PK = ?",
                                        [diff as i64, id],
                                    )?;
                                }
                            }
                        }
                    }
                }

                // Update composer
                if let Some(idx) = composer_idx {
                    if let Some(composer) = record.get(idx) {
                        if !composer.is_empty() {
                            if dry_run {
                                println!("  composer = {}", composer);
                            } else {
                                let composer_id = get_or_create_composer(&conn, composer)?;
                                conn.execute("DELETE FROM Z_4COMPOSERS WHERE Z_4ITEMS1 = ?", [id])?;
                                conn.execute(
                                    "INSERT INTO Z_4COMPOSERS (Z_4ITEMS1, Z_10COMPOSERS) VALUES (?, ?)",
                                    [id, composer_id],
                                )?;
                            }
                        }
                    }
                }

                // Update genre
                if let Some(idx) = genre_idx {
                    if let Some(genre) = record.get(idx) {
                        if !genre.is_empty() {
                            if dry_run {
                                println!("  genre = {}", genre);
                            } else {
                                let genre_id = get_or_create_genre(&conn, genre)?;
                                conn.execute("DELETE FROM Z_4GENRES WHERE Z_4ITEMS4 = ?", [id])?;
                                conn.execute(
                                    "INSERT INTO Z_4GENRES (Z_4ITEMS4, Z_12GENRES) VALUES (?, ?)",
                                    [id, genre_id],
                                )?;
                            }
                        }
                    }
                }

                // Mark score as modified (update timestamp and version)
                if !dry_run {
                    mark_modified(&conn, id)?;
                }

                updated += 1;
            }

            if dry_run {
                println!(
                    "\nDry run complete. Would update {} scores ({} errors)",
                    updated, errors
                );
            } else {
                println!("Updated {} scores ({} errors)", updated, errors);
            }
        }
    }

    Ok(())
}
