use crate::error::{ForScoreError, Result};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Library {
    pub id: i64,
    pub title: String,
    pub score_count: i32,
}

/// List all libraries
pub fn list_libraries(conn: &Connection) -> Result<Vec<Library>> {
    let mut stmt = conn.prepare(
        "SELECT l.Z_PK, l.ZTITLE,
                (SELECT COUNT(*) FROM Z_4LIBRARIES z WHERE z.Z_7LIBRARIES = l.Z_PK) as score_count
         FROM ZLIBRARY l
         ORDER BY l.ZTITLE",
    )?;

    let libraries: Vec<Library> = stmt
        .query_map([], |row| {
            Ok(Library {
                id: row.get("Z_PK")?,
                title: row.get("ZTITLE")?,
                score_count: row.get("score_count")?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    Ok(libraries)
}

/// Get library by ID
pub fn get_library_by_id(conn: &Connection, id: i64) -> Result<Library> {
    let mut stmt = conn.prepare(
        "SELECT l.Z_PK, l.ZTITLE,
                (SELECT COUNT(*) FROM Z_4LIBRARIES z WHERE z.Z_7LIBRARIES = l.Z_PK) as score_count
         FROM ZLIBRARY l WHERE l.Z_PK = ?",
    )?;

    stmt.query_row([id], |row| {
        Ok(Library {
            id: row.get("Z_PK")?,
            title: row.get("ZTITLE")?,
            score_count: row.get("score_count")?,
        })
    })
    .map_err(|_| ForScoreError::LibraryNotFound(id.to_string()))
}

/// Get library by name
pub fn get_library_by_name(conn: &Connection, name: &str) -> Result<Library> {
    // Try exact match
    let mut stmt = conn.prepare(
        "SELECT l.Z_PK, l.ZTITLE,
                (SELECT COUNT(*) FROM Z_4LIBRARIES z WHERE z.Z_7LIBRARIES = l.Z_PK) as score_count
         FROM ZLIBRARY l WHERE l.ZTITLE = ?",
    )?;

    if let Ok(library) = stmt.query_row([name], |row| {
        Ok(Library {
            id: row.get("Z_PK")?,
            title: row.get("ZTITLE")?,
            score_count: row.get("score_count")?,
        })
    }) {
        return Ok(library);
    }

    // Try case-insensitive
    let mut stmt = conn.prepare(
        "SELECT l.Z_PK, l.ZTITLE,
                (SELECT COUNT(*) FROM Z_4LIBRARIES z WHERE z.Z_7LIBRARIES = l.Z_PK) as score_count
         FROM ZLIBRARY l WHERE LOWER(l.ZTITLE) = LOWER(?)",
    )?;

    if let Ok(library) = stmt.query_row([name], |row| {
        Ok(Library {
            id: row.get("Z_PK")?,
            title: row.get("ZTITLE")?,
            score_count: row.get("score_count")?,
        })
    }) {
        return Ok(library);
    }

    // Try contains
    let mut stmt = conn.prepare(
        "SELECT l.Z_PK, l.ZTITLE,
                (SELECT COUNT(*) FROM Z_4LIBRARIES z WHERE z.Z_7LIBRARIES = l.Z_PK) as score_count
         FROM ZLIBRARY l WHERE l.ZTITLE LIKE ? LIMIT 2",
    )?;

    let pattern = format!("%{}%", name);
    let libraries: Vec<Library> = stmt
        .query_map([&pattern], |row| {
            Ok(Library {
                id: row.get("Z_PK")?,
                title: row.get("ZTITLE")?,
                score_count: row.get("score_count")?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    match libraries.len() {
        0 => Err(ForScoreError::LibraryNotFound(name.to_string())),
        1 => Ok(libraries.into_iter().next().unwrap()),
        _ => Err(ForScoreError::AmbiguousIdentifier(name.to_string())),
    }
}

/// Resolve library by ID or name
pub fn resolve_library(conn: &Connection, identifier: &str) -> Result<Library> {
    if let Ok(id) = identifier.parse::<i64>() {
        if let Ok(library) = get_library_by_id(conn, id) {
            return Ok(library);
        }
    }
    get_library_by_name(conn, identifier)
}

/// Add a score to a library
pub fn add_score_to_library(conn: &Connection, library_id: i64, score_id: i64) -> Result<()> {
    // Check if already in library
    let exists: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM Z_4LIBRARIES WHERE Z_7LIBRARIES = ? AND Z_4ITEMS3 = ?)",
        [library_id, score_id],
        |row| row.get(0),
    )?;

    if exists {
        return Ok(()); // Already in library
    }

    conn.execute(
        "INSERT INTO Z_4LIBRARIES (Z_7LIBRARIES, Z_4ITEMS3) VALUES (?, ?)",
        [library_id, score_id],
    )?;

    Ok(())
}

/// Remove a score from a library
pub fn remove_score_from_library(conn: &Connection, library_id: i64, score_id: i64) -> Result<()> {
    conn.execute(
        "DELETE FROM Z_4LIBRARIES WHERE Z_7LIBRARIES = ? AND Z_4ITEMS3 = ?",
        [library_id, score_id],
    )?;
    Ok(())
}
