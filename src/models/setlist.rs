use crate::db::entity;
use crate::error::{ForScoreError, Result};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Setlist {
    pub id: i64,
    pub title: String,
    pub uuid: Option<String>,
    pub score_count: i32,
}

/// List all setlists
pub fn list_setlists(conn: &Connection) -> Result<Vec<Setlist>> {
    let mut stmt = conn.prepare(
        "SELECT s.Z_PK, s.ZTITLE, s.ZUUID,
                (SELECT COUNT(*) FROM ZCYLON c WHERE c.ZSETLIST = s.Z_PK) as score_count
         FROM ZSETLIST s
         ORDER BY s.ZTITLE",
    )?;

    let setlists: Vec<Setlist> = stmt
        .query_map([], |row| {
            Ok(Setlist {
                id: row.get("Z_PK")?,
                title: row.get("ZTITLE")?,
                uuid: row.get("ZUUID")?,
                score_count: row.get("score_count")?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    Ok(setlists)
}

/// Get setlist by ID
pub fn get_setlist_by_id(conn: &Connection, id: i64) -> Result<Setlist> {
    let mut stmt = conn.prepare(
        "SELECT s.Z_PK, s.ZTITLE, s.ZUUID,
                (SELECT COUNT(*) FROM ZCYLON c WHERE c.ZSETLIST = s.Z_PK) as score_count
         FROM ZSETLIST s WHERE s.Z_PK = ?",
    )?;

    stmt.query_row([id], |row| {
        Ok(Setlist {
            id: row.get("Z_PK")?,
            title: row.get("ZTITLE")?,
            uuid: row.get("ZUUID")?,
            score_count: row.get("score_count")?,
        })
    })
    .map_err(|_| ForScoreError::SetlistNotFound(id.to_string()))
}

/// Get setlist by name
pub fn get_setlist_by_name(conn: &Connection, name: &str) -> Result<Setlist> {
    // Try exact match
    let mut stmt = conn.prepare(
        "SELECT s.Z_PK, s.ZTITLE, s.ZUUID,
                (SELECT COUNT(*) FROM ZCYLON c WHERE c.ZSETLIST = s.Z_PK) as score_count
         FROM ZSETLIST s WHERE s.ZTITLE = ?",
    )?;

    if let Ok(setlist) = stmt.query_row([name], |row| {
        Ok(Setlist {
            id: row.get("Z_PK")?,
            title: row.get("ZTITLE")?,
            uuid: row.get("ZUUID")?,
            score_count: row.get("score_count")?,
        })
    }) {
        return Ok(setlist);
    }

    // Try case-insensitive
    let mut stmt = conn.prepare(
        "SELECT s.Z_PK, s.ZTITLE, s.ZUUID,
                (SELECT COUNT(*) FROM ZCYLON c WHERE c.ZSETLIST = s.Z_PK) as score_count
         FROM ZSETLIST s WHERE LOWER(s.ZTITLE) = LOWER(?)",
    )?;

    if let Ok(setlist) = stmt.query_row([name], |row| {
        Ok(Setlist {
            id: row.get("Z_PK")?,
            title: row.get("ZTITLE")?,
            uuid: row.get("ZUUID")?,
            score_count: row.get("score_count")?,
        })
    }) {
        return Ok(setlist);
    }

    // Try contains
    let mut stmt = conn.prepare(
        "SELECT s.Z_PK, s.ZTITLE, s.ZUUID,
                (SELECT COUNT(*) FROM ZCYLON c WHERE c.ZSETLIST = s.Z_PK) as score_count
         FROM ZSETLIST s WHERE s.ZTITLE LIKE ? LIMIT 2",
    )?;

    let pattern = format!("%{}%", name);
    let setlists: Vec<Setlist> = stmt
        .query_map([&pattern], |row| {
            Ok(Setlist {
                id: row.get("Z_PK")?,
                title: row.get("ZTITLE")?,
                uuid: row.get("ZUUID")?,
                score_count: row.get("score_count")?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    match setlists.len() {
        0 => Err(ForScoreError::SetlistNotFound(name.to_string())),
        1 => Ok(setlists.into_iter().next().unwrap()),
        _ => Err(ForScoreError::AmbiguousIdentifier(name.to_string())),
    }
}

/// Resolve setlist by ID or name
pub fn resolve_setlist(conn: &Connection, identifier: &str) -> Result<Setlist> {
    if let Ok(id) = identifier.parse::<i64>() {
        if let Ok(setlist) = get_setlist_by_id(conn, id) {
            return Ok(setlist);
        }
    }
    get_setlist_by_name(conn, identifier)
}

/// Create a new setlist
pub fn create_setlist(conn: &Connection, name: &str) -> Result<Setlist> {
    let uuid = uuid::Uuid::new_v4().to_string().to_uppercase();

    // Get max Z_PK and Z_OPT
    let max_pk: i64 = conn.query_row("SELECT COALESCE(MAX(Z_PK), 0) FROM ZSETLIST", [], |row| {
        row.get(0)
    })?;

    // Get entity info
    let z_ent = entity::SETLIST;

    conn.execute(
        "INSERT INTO ZSETLIST (Z_PK, Z_ENT, Z_OPT, ZTITLE, ZUUID, ZINDEX, ZMENUINDEX, ZSORT)
         VALUES (?, ?, 1, ?, ?, 0, 0, 0)",
        rusqlite::params![max_pk + 1, z_ent, name, uuid],
    )?;

    // Update Z_PRIMARYKEY
    conn.execute(
        "UPDATE Z_PRIMARYKEY SET Z_MAX = ? WHERE Z_ENT = ?",
        [max_pk + 1, z_ent as i64],
    )?;

    get_setlist_by_id(conn, max_pk + 1)
}

/// Rename a setlist
pub fn rename_setlist(conn: &Connection, setlist_id: i64, new_name: &str) -> Result<()> {
    let affected = conn.execute(
        "UPDATE ZSETLIST SET ZTITLE = ? WHERE Z_PK = ?",
        rusqlite::params![new_name, setlist_id],
    )?;

    if affected == 0 {
        return Err(ForScoreError::SetlistNotFound(setlist_id.to_string()));
    }
    Ok(())
}

/// Delete a setlist (and remove all memberships)
pub fn delete_setlist(conn: &Connection, setlist_id: i64) -> Result<()> {
    // Remove memberships first
    conn.execute("DELETE FROM ZCYLON WHERE ZSETLIST = ?", [setlist_id])?;

    // Delete setlist
    let affected = conn.execute("DELETE FROM ZSETLIST WHERE Z_PK = ?", [setlist_id])?;

    if affected == 0 {
        return Err(ForScoreError::SetlistNotFound(setlist_id.to_string()));
    }
    Ok(())
}

/// Add a score to a setlist
pub fn add_score_to_setlist(conn: &Connection, setlist_id: i64, score_id: i64) -> Result<()> {
    // Check if already in setlist
    let exists: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM ZCYLON WHERE ZSETLIST = ? AND ZITEM = ?)",
        [setlist_id, score_id],
        |row| row.get(0),
    )?;

    if exists {
        return Ok(()); // Already in setlist
    }

    // Get max Z_PK for ordering
    let max_pk: i64 = conn.query_row("SELECT COALESCE(MAX(Z_PK), 0) FROM ZCYLON", [], |row| {
        row.get(0)
    })?;

    // Try to reuse UUID if this score is already in another setlist
    let existing_uuid: Option<String> = conn
        .query_row(
            "SELECT ZUUID FROM ZCYLON WHERE ZITEM = ? AND ZUUID IS NOT NULL LIMIT 1",
            [score_id],
            |row| row.get(0),
        )
        .ok();

    let uuid = existing_uuid.unwrap_or_else(|| uuid::Uuid::new_v4().to_string().to_uppercase());

    // Z4_ITEM should be the entity type (6 for Score), not the score ID
    conn.execute(
        "INSERT INTO ZCYLON (Z_PK, Z_ENT, Z_OPT, ZSETLIST, ZITEM, Z4_ITEM, ZSHUFFLE, ZUUID)
         VALUES (?, 2, 1, ?, ?, ?, 0, ?)",
        rusqlite::params![max_pk + 1, setlist_id, score_id, entity::SCORE, uuid],
    )?;

    Ok(())
}

/// Remove a score from a setlist
pub fn remove_score_from_setlist(conn: &Connection, setlist_id: i64, score_id: i64) -> Result<()> {
    conn.execute(
        "DELETE FROM ZCYLON WHERE ZSETLIST = ? AND ZITEM = ?",
        [setlist_id, score_id],
    )?;
    Ok(())
}

/// Reorder a score within a setlist
pub fn reorder_score_in_setlist(
    conn: &Connection,
    setlist_id: i64,
    score_id: i64,
    new_position: usize,
) -> Result<()> {
    // Get all scores in current order
    let mut stmt =
        conn.prepare("SELECT Z_PK, ZITEM FROM ZCYLON WHERE ZSETLIST = ? ORDER BY Z_PK")?;

    let members: Vec<(i64, i64)> = stmt
        .query_map([setlist_id], |row| Ok((row.get(0)?, row.get(1)?)))?
        .filter_map(|r| r.ok())
        .collect();

    // Find current position
    let current_pos = members.iter().position(|(_, id)| *id == score_id);
    if current_pos.is_none() {
        return Err(ForScoreError::Other(format!(
            "Score {} not in setlist {}",
            score_id, setlist_id
        )));
    }

    // Reorder by deleting and re-inserting with new Z_PK values
    let max_base: i64 = conn.query_row("SELECT COALESCE(MAX(Z_PK), 0) FROM ZCYLON", [], |row| {
        row.get(0)
    })?;

    // Build new order
    let mut new_order: Vec<i64> = members.iter().map(|(_, id)| *id).collect();
    let removed = new_order.remove(current_pos.unwrap());
    let insert_pos = (new_position - 1).min(new_order.len());
    new_order.insert(insert_pos, removed);

    // Delete all memberships for this setlist
    conn.execute("DELETE FROM ZCYLON WHERE ZSETLIST = ?", [setlist_id])?;

    // Re-insert in new order
    for (i, item_id) in new_order.iter().enumerate() {
        let uuid = uuid::Uuid::new_v4().to_string().to_uppercase();
        conn.execute(
            "INSERT INTO ZCYLON (Z_PK, Z_ENT, Z_OPT, ZSETLIST, ZITEM, Z4_ITEM, ZSHUFFLE, ZUUID)
             VALUES (?, 2, 1, ?, ?, ?, 0, ?)",
            rusqlite::params![max_base + 1 + i as i64, setlist_id, item_id, item_id, uuid],
        )?;
    }

    Ok(())
}
