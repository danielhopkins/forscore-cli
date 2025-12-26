use crate::cli::FixesCommand;
use crate::db::{entity, open_readonly, open_readwrite, warn_if_running};
use crate::error::Result;
use crate::itm::delete_bookmark_from_itm;
use rusqlite::Connection;

pub fn handle(cmd: FixesCommand) -> Result<()> {
    match cmd {
        FixesCommand::DuplicateBookmarks { dry_run } => {
            if !dry_run {
                warn_if_running();
            }

            let conn = if dry_run {
                open_readonly()?
            } else {
                open_readwrite()?
            };

            let duplicates = find_duplicate_bookmarks(&conn)?;

            if duplicates.is_empty() {
                println!("No duplicate bookmarks found.");
                return Ok(());
            }

            println!("Found {} duplicate bookmark(s):\n", duplicates.len());

            for dup in &duplicates {
                println!(
                    "  {} (ID {}) - pages {}-{} in \"{}\"",
                    dup.title, dup.id, dup.start_page, dup.end_page, dup.score_title
                );
                println!("    Duplicate of ID {} (keeping older)", dup.original_id);
            }

            if dry_run {
                println!("\nDry run - no changes made. Remove --dry-run to delete duplicates.");
            } else {
                println!();
                for dup in &duplicates {
                    delete_bookmark(&conn, dup)?;
                }
                println!("\nDeleted {} duplicate bookmark(s).", duplicates.len());
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
