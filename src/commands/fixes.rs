use crate::cli::FixesCommand;
use crate::db::{entity, open_readonly, open_readwrite, warn_if_running};
use crate::error::{ForScoreError, Result};
use crate::itm::{delete_bookmark_from_itm, read_itm, sync_folder_path, write_itm};
use plist::Value;
use rusqlite::Connection;
use std::collections::HashSet;

pub fn handle(cmd: FixesCommand) -> Result<()> {
    match cmd {
        FixesCommand::DuplicateBookmarks { apply } => {
            if apply {
                warn_if_running();
            }

            let conn = if apply {
                open_readwrite()?
            } else {
                open_readonly()?
            };

            // Check database duplicates
            let db_duplicates = find_duplicate_bookmarks(&conn)?;

            // Check ITM file duplicates
            let itm_results = find_itm_duplicates(None)?;

            let has_db_dups = !db_duplicates.is_empty();
            let has_itm_dups = !itm_results.is_empty();

            if !has_db_dups && !has_itm_dups {
                println!("No duplicate bookmarks found.");
                return Ok(());
            }

            // Report database duplicates
            if has_db_dups {
                println!(
                    "Found {} duplicate(s) in database:\n",
                    db_duplicates.len()
                );

                for dup in &db_duplicates {
                    println!(
                        "  {} (ID {}) - pages {}-{} in \"{}\"",
                        dup.title, dup.id, dup.start_page, dup.end_page, dup.score_title
                    );
                    println!("    Duplicate of ID {} (keeping older)", dup.original_id);
                }
                println!();
            }

            // Report ITM duplicates
            if has_itm_dups {
                let total_itm: usize = itm_results.iter().map(|r| r.duplicates.len()).sum();
                println!(
                    "Found {} duplicate(s) in ITM sync files ({} files):\n",
                    total_itm,
                    itm_results.len()
                );

                for result in &itm_results {
                    println!("  {}:", result.pdf_name);
                    for dup in &result.duplicates {
                        println!(
                            "    {} (pages {}-{}) - {} extra copies",
                            dup.title, dup.first_page, dup.last_page, dup.count
                        );
                    }
                }
                println!();
            }

            if apply {
                // Fix database duplicates
                if has_db_dups {
                    println!("Fixing database duplicates...");
                    for dup in &db_duplicates {
                        delete_bookmark(&conn, dup)?;
                    }
                    println!("Deleted {} database duplicate(s).\n", db_duplicates.len());
                }

                // Fix ITM duplicates
                if has_itm_dups {
                    println!("Fixing ITM sync file duplicates...");
                    for result in &itm_results {
                        remove_itm_duplicates(&result.itm_path)?;
                        println!("  Fixed: {}", result.pdf_name);
                    }
                    println!("\nRemoved duplicates from {} ITM file(s).", itm_results.len());
                }
            } else {
                println!("Run with --apply to delete duplicates.");
            }
        }
    }

    Ok(())
}

struct DuplicateBookmark {
    id: i64,
    title: String,
    path: String,
    uuid: Option<String>,
    start_page: i32,
    end_page: i32,
    score_title: String,
    original_id: i64,
}

fn find_duplicate_bookmarks(conn: &Connection) -> Result<Vec<DuplicateBookmark>> {
    // Find bookmarks that have the same score, title, start_page, and end_page
    // Keep the one with the lower ID (older), mark the higher ID (newer) as duplicate
    let mut stmt = conn.prepare(
        "SELECT
            b.Z_PK as id,
            b.ZTITLE as title,
            b.ZPATH as path,
            b.ZUUID as uuid,
            b.ZSTARTPAGE as start_page,
            b.ZENDPAGE as end_page,
            s.ZTITLE as score_title,
            (SELECT MIN(b2.Z_PK) FROM ZITEM b2
             WHERE b2.Z_ENT = ?
             AND b2.ZSCORE = b.ZSCORE
             AND b2.ZTITLE = b.ZTITLE
             AND b2.ZSTARTPAGE = b.ZSTARTPAGE
             AND b2.ZENDPAGE = b.ZENDPAGE) as original_id
         FROM ZITEM b
         JOIN ZITEM s ON b.ZSCORE = s.Z_PK
         WHERE b.Z_ENT = ?
         AND b.Z_PK > (
             SELECT MIN(b2.Z_PK) FROM ZITEM b2
             WHERE b2.Z_ENT = ?
             AND b2.ZSCORE = b.ZSCORE
             AND b2.ZTITLE = b.ZTITLE
             AND b2.ZSTARTPAGE = b.ZSTARTPAGE
             AND b2.ZENDPAGE = b.ZENDPAGE
         )
         ORDER BY score_title, start_page",
    )?;

    let duplicates = stmt
        .query_map(
            [entity::BOOKMARK, entity::BOOKMARK, entity::BOOKMARK],
            |row| {
                Ok(DuplicateBookmark {
                    id: row.get("id")?,
                    title: row.get::<_, Option<String>>("title")?.unwrap_or_default(),
                    path: row.get::<_, Option<String>>("path")?.unwrap_or_default(),
                    uuid: row.get("uuid")?,
                    start_page: row.get::<_, Option<i32>>("start_page")?.unwrap_or(0),
                    end_page: row.get::<_, Option<i32>>("end_page")?.unwrap_or(0),
                    score_title: row
                        .get::<_, Option<String>>("score_title")?
                        .unwrap_or_default(),
                    original_id: row.get("original_id")?,
                })
            },
        )?
        .filter_map(|r| r.ok())
        .collect();

    Ok(duplicates)
}

fn delete_bookmark(conn: &Connection, bookmark: &DuplicateBookmark) -> Result<()> {
    // Delete from database
    conn.execute("DELETE FROM ZITEM WHERE Z_PK = ?", [bookmark.id])?;

    // Delete composer links
    conn.execute(
        "DELETE FROM Z_4COMPOSERS WHERE Z_4ITEMS1 = ?",
        [bookmark.id],
    )?;

    // Delete genre links
    conn.execute("DELETE FROM Z_4GENRES WHERE Z_4ITEMS4 = ?", [bookmark.id])?;

    // Delete from ITM file
    let uuid = bookmark.uuid.as_deref();
    match delete_bookmark_from_itm(&bookmark.path, uuid) {
        Ok(true) => println!("Deleted: {} (ID {}) + ITM", bookmark.title, bookmark.id),
        Ok(false) => println!("Deleted: {} (ID {})", bookmark.title, bookmark.id),
        Err(e) => {
            println!("Deleted: {} (ID {})", bookmark.title, bookmark.id);
            eprintln!("  Warning: Failed to update ITM: {}", e);
        }
    }

    Ok(())
}

// ITM duplicate detection structures
struct ItmDuplicateResult {
    pdf_name: String,
    itm_path: std::path::PathBuf,
    duplicates: Vec<ItmDuplicateInfo>,
}

struct ItmDuplicateInfo {
    title: String,
    first_page: i64,
    last_page: i64,
    count: usize, // number of extra copies (total - 1)
}

fn find_itm_duplicates(file_filter: Option<&str>) -> Result<Vec<ItmDuplicateResult>> {
    let sync_folder = sync_folder_path()?;
    let mut results = Vec::new();

    let entries = std::fs::read_dir(&sync_folder)
        .map_err(|e| ForScoreError::Other(format!("Cannot read sync folder: {}", e)))?;

    for entry in entries.flatten() {
        let path = entry.path();

        // Only process .itm files
        if path.extension().and_then(|s| s.to_str()) != Some("itm") {
            continue;
        }

        let filename = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or_default();

        // Extract PDF name (remove .itm extension)
        let pdf_name = filename.strip_suffix(".itm").unwrap_or(filename);

        // Apply file filter if specified
        if let Some(filter) = file_filter {
            if !pdf_name.to_lowercase().contains(&filter.to_lowercase()) {
                continue;
            }
        }

        let duplicates = find_duplicates_in_itm(&path)?;

        if !duplicates.is_empty() {
            results.push(ItmDuplicateResult {
                pdf_name: pdf_name.to_string(),
                itm_path: path,
                duplicates,
            });
        }
    }

    Ok(results)
}

fn find_duplicates_in_itm(path: &std::path::Path) -> Result<Vec<ItmDuplicateInfo>> {
    let value = match read_itm(&path.to_path_buf()) {
        Ok(v) => v,
        Err(_) => return Ok(Vec::new()),
    };

    let dict = match value {
        Value::Dictionary(d) => d,
        _ => return Ok(Vec::new()),
    };

    let bookmarks = match dict.get("bookmarks") {
        Some(Value::Array(arr)) => arr,
        _ => return Ok(Vec::new()),
    };

    // Count occurrences of each (title, first_page, last_page) combination
    let mut seen: std::collections::HashMap<(String, i64, i64), usize> =
        std::collections::HashMap::new();

    for bookmark in bookmarks {
        if let Value::Dictionary(bm_dict) = bookmark {
            let title = match bm_dict.get("Title") {
                Some(Value::String(t)) => t.clone(),
                _ => continue,
            };

            let first_page = match bm_dict.get("First Page") {
                Some(Value::Integer(p)) => p.as_signed().unwrap_or(0),
                _ => continue,
            };

            let last_page = match bm_dict.get("Last Page") {
                Some(Value::Integer(p)) => p.as_signed().unwrap_or(0),
                _ => continue,
            };

            let key = (title, first_page, last_page);
            *seen.entry(key).or_insert(0) += 1;
        }
    }

    // Find entries with more than one occurrence
    let duplicates: Vec<ItmDuplicateInfo> = seen
        .into_iter()
        .filter(|(_, count)| *count > 1)
        .map(|((title, first_page, last_page), count)| ItmDuplicateInfo {
            title,
            first_page,
            last_page,
            count: count - 1, // extra copies beyond the first
        })
        .collect();

    Ok(duplicates)
}

fn remove_itm_duplicates(path: &std::path::Path) -> Result<()> {
    let value = read_itm(&path.to_path_buf())?;

    let mut dict = match value {
        Value::Dictionary(d) => d,
        _ => return Err(ForScoreError::Other("ITM file is not a dictionary".into())),
    };

    let bookmarks = match dict.get_mut("bookmarks") {
        Some(Value::Array(arr)) => arr,
        _ => return Ok(()), // No bookmarks to dedupe
    };

    // Track seen bookmarks by (title, first_page, last_page)
    let mut seen: HashSet<(String, i64, i64)> = HashSet::new();

    // Retain only the first occurrence of each bookmark
    bookmarks.retain(|bookmark| {
        if let Value::Dictionary(bm_dict) = bookmark {
            let title = match bm_dict.get("Title") {
                Some(Value::String(t)) => t.clone(),
                _ => return true, // Keep bookmarks without title
            };

            let first_page = match bm_dict.get("First Page") {
                Some(Value::Integer(p)) => p.as_signed().unwrap_or(0),
                _ => return true,
            };

            let last_page = match bm_dict.get("Last Page") {
                Some(Value::Integer(p)) => p.as_signed().unwrap_or(0),
                _ => return true,
            };

            let key = (title, first_page, last_page);
            if seen.contains(&key) {
                false // Remove duplicate
            } else {
                seen.insert(key);
                true // Keep first occurrence
            }
        } else {
            true // Keep non-dictionary items
        }
    });

    // Write back
    write_itm(&path.to_path_buf(), &Value::Dictionary(dict))?;

    Ok(())
}
