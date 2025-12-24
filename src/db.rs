use crate::error::{ForScoreError, Result};
use rusqlite::{Connection, OpenFlags};
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

/// Core Data epoch: seconds between Unix epoch (1970-01-01) and Core Data epoch (2001-01-01)
const CORE_DATA_EPOCH_OFFSET: i64 = 978307200;

const FORSCORE_CONTAINER: &str =
    "Library/Containers/com.mgsdevelopment.forscore/Data/Library/Preferences/library.4sl";

/// Get the path to the forScore database
pub fn database_path() -> Result<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| ForScoreError::Other("Cannot find home directory".into()))?;
    let path = home.join(FORSCORE_CONTAINER);

    if path.exists() {
        Ok(path)
    } else {
        Err(ForScoreError::DatabaseNotFound)
    }
}

/// Check if forScore is currently running
pub fn is_forscore_running() -> bool {
    Command::new("pgrep")
        .args(["-x", "forScore"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Print a warning if forScore is running
pub fn warn_if_running() {
    if is_forscore_running() {
        eprintln!("WARNING: forScore is currently running. Changes may conflict or be overwritten.");
        eprintln!("         Consider closing forScore before making modifications.\n");
    }
}

/// Open the database in read-only mode
pub fn open_readonly() -> Result<Connection> {
    let path = database_path()?;
    let conn = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    Ok(conn)
}

/// Open the database in read-write mode
pub fn open_readwrite() -> Result<Connection> {
    let path = database_path()?;
    let conn = Connection::open_with_flags(
        &path,
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    Ok(conn)
}

/// Entity type constants from Z_PRIMARYKEY
pub mod entity {
    pub const ITEM: i32 = 4;
    pub const BOOKMARK: i32 = 5;
    pub const SCORE: i32 = 6;
    pub const LIBRARY: i32 = 7;
    pub const META: i32 = 9;
    pub const COMPOSER: i32 = 10;
    pub const DIFFICULTY: i32 = 11;
    pub const GENRE: i32 = 12;
    pub const KEYWORD: i32 = 13;
    pub const LABEL: i32 = 14;
    pub const RATING: i32 = 15;
    pub const PAGE: i32 = 16;
    pub const SETLIST: i32 = 19;
    pub const TRACK: i32 = 22;
}

/// Get current timestamp in Core Data format (seconds since 2001-01-01)
pub fn core_data_timestamp() -> f64 {
    let unix_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs_f64();
    unix_time - CORE_DATA_EPOCH_OFFSET as f64
}

/// Update ZMODIFIED timestamp and increment Z_OPT for an item
pub fn mark_modified(conn: &Connection, item_id: i64) -> Result<()> {
    let timestamp = core_data_timestamp();
    conn.execute(
        "UPDATE ZITEM SET ZMODIFIED = ?, Z_OPT = Z_OPT + 1 WHERE Z_PK = ?",
        rusqlite::params![timestamp, item_id],
    )?;
    Ok(())
}
