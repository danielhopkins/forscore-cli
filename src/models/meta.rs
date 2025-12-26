use crate::db::entity;
use crate::error::{ForScoreError, Result};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Composer {
    pub id: i64,
    pub name: String,
    pub score_count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Genre {
    pub id: i64,
    pub name: String,
    pub score_count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Keyword {
    pub id: i64,
    pub name: String,
    pub score_count: i32,
}

/// List all composers
pub fn list_composers(conn: &Connection, unused_only: bool) -> Result<Vec<Composer>> {
    let sql = if unused_only {
        "SELECT m.Z_PK, m.ZVALUE,
                (SELECT COUNT(*) FROM Z_4COMPOSERS c WHERE c.Z_10COMPOSERS = m.Z_PK) as score_count
         FROM ZMETA m WHERE m.Z_ENT = ?
         HAVING score_count = 0
         ORDER BY m.ZVALUE"
    } else {
        "SELECT m.Z_PK, m.ZVALUE,
                (SELECT COUNT(*) FROM Z_4COMPOSERS c WHERE c.Z_10COMPOSERS = m.Z_PK) as score_count
         FROM ZMETA m WHERE m.Z_ENT = ?
         ORDER BY m.ZVALUE"
    };

    let mut stmt = conn.prepare(sql)?;

    let composers: Vec<Composer> = stmt
        .query_map([entity::COMPOSER], |row| {
            Ok(Composer {
                id: row.get("Z_PK")?,
                name: row.get::<_, Option<String>>("ZVALUE")?.unwrap_or_default(),
                score_count: row.get("score_count")?,
            })
        })?
        .filter_map(|r| r.ok())
        .filter(|c| !unused_only || c.score_count == 0)
        .collect();

    Ok(composers)
}

/// Get composer by name
pub fn get_composer_by_name(conn: &Connection, name: &str) -> Result<Composer> {
    let mut stmt = conn.prepare(
        "SELECT m.Z_PK, m.ZVALUE,
                (SELECT COUNT(*) FROM Z_4COMPOSERS c WHERE c.Z_10COMPOSERS = m.Z_PK) as score_count
         FROM ZMETA m WHERE m.Z_ENT = ? AND m.ZVALUE = ?",
    )?;

    stmt.query_row(rusqlite::params![entity::COMPOSER, name], |row| {
        Ok(Composer {
            id: row.get("Z_PK")?,
            name: row.get::<_, Option<String>>("ZVALUE")?.unwrap_or_default(),
            score_count: row.get("score_count")?,
        })
    })
    .map_err(|_| ForScoreError::ComposerNotFound(name.to_string()))
}

/// Rename a composer
pub fn rename_composer(conn: &Connection, old_name: &str, new_name: &str) -> Result<()> {
    let affected = conn.execute(
        "UPDATE ZMETA SET ZVALUE = ? WHERE Z_ENT = ? AND ZVALUE = ?",
        rusqlite::params![new_name, entity::COMPOSER, old_name],
    )?;

    if affected == 0 {
        return Err(ForScoreError::ComposerNotFound(old_name.to_string()));
    }
    Ok(())
}

/// Merge composers: move all scores from source to target, then delete source
pub fn merge_composers(conn: &Connection, source_name: &str, target_name: &str) -> Result<()> {
    let source = get_composer_by_name(conn, source_name)?;
    let target = get_composer_by_name(conn, target_name)?;

    // Update all references
    conn.execute(
        "UPDATE Z_4COMPOSERS SET Z_10COMPOSERS = ? WHERE Z_10COMPOSERS = ?",
        [target.id, source.id],
    )?;

    // Delete source composer
    conn.execute(
        "DELETE FROM ZMETA WHERE Z_PK = ?",
        [source.id],
    )?;

    Ok(())
}

/// List all genres
pub fn list_genres(conn: &Connection, unused_only: bool) -> Result<Vec<Genre>> {
    let sql = "SELECT m.Z_PK, m.ZVALUE2,
                (SELECT COUNT(*) FROM Z_4GENRES g WHERE g.Z_12GENRES = m.Z_PK) as score_count
         FROM ZMETA m WHERE m.Z_ENT = ?
         ORDER BY m.ZVALUE2";

    let mut stmt = conn.prepare(sql)?;

    let genres: Vec<Genre> = stmt
        .query_map([entity::GENRE], |row| {
            Ok(Genre {
                id: row.get("Z_PK")?,
                name: row.get::<_, Option<String>>("ZVALUE2")?.unwrap_or_default(),
                score_count: row.get("score_count")?,
            })
        })?
        .filter_map(|r| r.ok())
        .filter(|g| !unused_only || g.score_count == 0)
        .collect();

    Ok(genres)
}

/// List all keywords (tags)
pub fn list_keywords(conn: &Connection, unused_only: bool) -> Result<Vec<Keyword>> {
    let sql = "SELECT m.Z_PK, m.ZVALUE,
                (SELECT COUNT(*) FROM Z_4KEYWORDS k WHERE k.Z_13KEYWORDS = m.Z_PK) as score_count
         FROM ZMETA m WHERE m.Z_ENT = ?
         ORDER BY m.ZVALUE";

    let mut stmt = conn.prepare(sql)?;

    let keywords: Vec<Keyword> = stmt
        .query_map([entity::KEYWORD], |row| {
            Ok(Keyword {
                id: row.get("Z_PK")?,
                name: row.get::<_, Option<String>>("ZVALUE")?.unwrap_or_default(),
                score_count: row.get("score_count")?,
            })
        })?
        .filter_map(|r| r.ok())
        .filter(|k| !unused_only || k.score_count == 0)
        .collect();

    Ok(keywords)
}

/// Get or create a composer, returning its ID
pub fn get_or_create_composer(conn: &Connection, name: &str) -> Result<i64> {
    // Try to find existing
    if let Ok(composer) = get_composer_by_name(conn, name) {
        return Ok(composer.id);
    }

    // Create new
    let max_pk: i64 = conn.query_row(
        "SELECT COALESCE(MAX(Z_PK), 0) FROM ZMETA",
        [],
        |row| row.get(0),
    )?;

    conn.execute(
        "INSERT INTO ZMETA (Z_PK, Z_ENT, Z_OPT, ZVALUE) VALUES (?, ?, 1, ?)",
        rusqlite::params![max_pk + 1, entity::COMPOSER, name],
    )?;

    // Update Z_PRIMARYKEY
    conn.execute(
        "UPDATE Z_PRIMARYKEY SET Z_MAX = ? WHERE Z_ENT = ?",
        [max_pk + 1, entity::META as i64],
    )?;

    Ok(max_pk + 1)
}

/// Get or create a genre, returning its ID
pub fn get_or_create_genre(conn: &Connection, name: &str) -> Result<i64> {
    // Try to find existing
    let mut stmt = conn.prepare(
        "SELECT Z_PK FROM ZMETA WHERE Z_ENT = ? AND ZVALUE2 = ?",
    )?;

    if let Ok(id) = stmt.query_row(rusqlite::params![entity::GENRE, name], |row| row.get::<_, i64>(0)) {
        return Ok(id);
    }

    // Create new
    let max_pk: i64 = conn.query_row(
        "SELECT COALESCE(MAX(Z_PK), 0) FROM ZMETA",
        [],
        |row| row.get(0),
    )?;

    conn.execute(
        "INSERT INTO ZMETA (Z_PK, Z_ENT, Z_OPT, ZVALUE2) VALUES (?, ?, 1, ?)",
        rusqlite::params![max_pk + 1, entity::GENRE, name],
    )?;

    Ok(max_pk + 1)
}
