use crate::db::{database_path, open_readonly};
use crate::error::Result;
use chrono::{DateTime, Local};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

/// Show library statistics
pub fn info() -> Result<()> {
    let conn = open_readonly()?;

    let score_count: i64 =
        conn.query_row("SELECT COUNT(*) FROM ZITEM WHERE Z_ENT = 6", [], |row| {
            row.get(0)
        })?;

    let bookmark_count: i64 =
        conn.query_row("SELECT COUNT(*) FROM ZITEM WHERE Z_ENT = 5", [], |row| {
            row.get(0)
        })?;

    let setlist_count: i64 =
        conn.query_row("SELECT COUNT(*) FROM ZSETLIST", [], |row| row.get(0))?;

    let library_count: i64 =
        conn.query_row("SELECT COUNT(*) FROM ZLIBRARY", [], |row| row.get(0))?;

    let composer_count: i64 =
        conn.query_row("SELECT COUNT(*) FROM ZMETA WHERE Z_ENT = 10", [], |row| {
            row.get(0)
        })?;

    let genre_count: i64 =
        conn.query_row("SELECT COUNT(*) FROM ZMETA WHERE Z_ENT = 12", [], |row| {
            row.get(0)
        })?;

    let page_count: i64 = conn.query_row("SELECT COUNT(*) FROM ZPAGE", [], |row| row.get(0))?;

    let track_count: i64 = conn.query_row("SELECT COUNT(*) FROM ZTRACK", [], |row| row.get(0))?;

    // Scores with ratings
    let rated_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM ZITEM WHERE Z_ENT = 6 AND ZRATING IS NOT NULL AND ZRATING > 0",
        [],
        |row| row.get(0),
    )?;

    // Scores with difficulty
    let difficulty_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM ZITEM WHERE Z_ENT = 6 AND ZDIFFICULTY IS NOT NULL AND ZDIFFICULTY > 0",
        [],
        |row| row.get(0),
    )?;

    // Scores with key
    let key_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM ZITEM WHERE Z_ENT = 6 AND ZKEY IS NOT NULL AND ZKEY > 0",
        [],
        |row| row.get(0),
    )?;

    let db_path = database_path()?;

    println!("forScore Library Statistics");
    println!("===========================");
    println!();
    println!("Database: {}", db_path.display());
    println!();
    println!("Content:");
    println!("  Scores:     {:>6}", score_count);
    println!("  Bookmarks:  {:>6}", bookmark_count);
    println!("  Pages:      {:>6}", page_count);
    println!("  Setlists:   {:>6}", setlist_count);
    println!("  Libraries:  {:>6}", library_count);
    println!();
    println!("Metadata:");
    println!("  Composers:  {:>6}", composer_count);
    println!("  Genres:     {:>6}", genre_count);
    println!("  Tracks:     {:>6}", track_count);
    println!();
    println!("Scores with metadata:");
    println!(
        "  With rating:     {:>6} ({:.1}%)",
        rated_count,
        100.0 * rated_count as f64 / score_count as f64
    );
    println!(
        "  With difficulty: {:>6} ({:.1}%)",
        difficulty_count,
        100.0 * difficulty_count as f64 / score_count as f64
    );
    println!(
        "  With key:        {:>6} ({:.1}%)",
        key_count,
        100.0 * key_count as f64 / score_count as f64
    );

    Ok(())
}

/// Backup the database
pub fn backup(output: Option<String>) -> Result<()> {
    let db_path = database_path()?;

    let backup_path = if let Some(out) = output {
        PathBuf::from(out)
    } else {
        let timestamp = Local::now().format("%Y%m%d-%H%M%S");
        let filename = format!("library.4sl.{}.bak", timestamp);
        db_path.parent().unwrap().join(filename)
    };

    fs::copy(&db_path, &backup_path)?;

    // Also copy the WAL files if they exist
    let wal_path = db_path.with_extension("4sl-wal");
    if wal_path.exists() {
        let wal_backup = backup_path.with_extension("4sl-wal");
        fs::copy(&wal_path, &wal_backup)?;
    }

    let shm_path = db_path.with_extension("4sl-shm");
    if shm_path.exists() {
        let shm_backup = backup_path.with_extension("4sl-shm");
        fs::copy(&shm_path, &shm_backup)?;
    }

    println!("Backed up database to: {}", backup_path.display());

    Ok(())
}

/// Show iCloud sync status
pub fn sync_status() -> Result<()> {
    let plist_path = dirs::home_dir()
        .unwrap()
        .join("Library/Containers/com.mgsdevelopment.forscore/Data/Library/Preferences/com.mgsdevelopment.forscore.plist");

    if !plist_path.exists() {
        println!("forScore preferences not found");
        return Ok(());
    }

    // Use plutil to read plist values
    let output = Command::new("plutil")
        .args(["-p", plist_path.to_str().unwrap()])
        .output()?;

    let plist_str = String::from_utf8_lossy(&output.stdout);

    // Parse sync values from plist output
    let mut sync_enabled = false;
    let mut last_sync_date: Option<String> = None;
    let mut last_sync_error: i32 = 0;

    for line in plist_str.lines() {
        if line.contains("&SYNC;syncEnabled") {
            sync_enabled = line.contains("true");
        } else if line.contains("&SYNC;lastSyncDate") {
            // Extract date: "  \"&SYNC;lastSyncDate\" => 2025-12-24 15:02:11 +0000"
            if let Some(pos) = line.find("=>") {
                last_sync_date = Some(line[pos + 3..].trim().to_string());
            }
        } else if line.contains("&SYNC;lastSyncErrorCode") {
            if let Some(pos) = line.find("=>") {
                if let Ok(code) = line[pos + 3..].trim().parse::<i32>() {
                    last_sync_error = code;
                }
            }
        }
    }

    println!("forScore iCloud Sync Status");
    println!("===========================");
    println!();
    println!("Sync Enabled: {}", if sync_enabled { "Yes" } else { "No" });

    if let Some(date_str) = last_sync_date {
        // Parse the date string and convert to local time
        // Format: "2025-12-24 15:02:11 +0000"
        if let Ok(utc_time) = DateTime::parse_from_str(&date_str, "%Y-%m-%d %H:%M:%S %z") {
            let local_time: DateTime<Local> = utc_time.into();
            let now = Local::now();
            let duration = now.signed_duration_since(local_time);

            let ago = if duration.num_days() > 0 {
                format!("{} days ago", duration.num_days())
            } else if duration.num_hours() > 0 {
                format!("{} hours ago", duration.num_hours())
            } else if duration.num_minutes() > 0 {
                format!("{} minutes ago", duration.num_minutes())
            } else {
                "just now".to_string()
            };

            println!(
                "Last Sync:    {} ({})",
                local_time.format("%Y-%m-%d %H:%M:%S"),
                ago
            );
        } else {
            println!("Last Sync:    {}", date_str);
        }
    } else {
        println!("Last Sync:    Never");
    }

    if last_sync_error == 0 {
        println!("Status:       OK");
    } else {
        println!("Status:       Error (code {})", last_sync_error);
    }

    println!();
    println!("Note: Sync can only be triggered manually in the forScore app");
    println!("      (open Sync panel and pull down to refresh)");

    Ok(())
}

/// Show sync log (recently synced files)
pub fn sync_log(limit: usize) -> Result<()> {
    let state_path = dirs::home_dir()
        .unwrap()
        .join("Library/Containers/com.mgsdevelopment.forscore/Data/Library/Preferences/Sync/.syncFolderState");

    if !state_path.exists() {
        println!("No sync state file found");
        return Ok(());
    }

    // Use plutil to convert plist to JSON for easier parsing
    let output = Command::new("plutil")
        .args(["-convert", "json", "-o", "-", state_path.to_str().unwrap()])
        .output()?;

    let json_str = String::from_utf8_lossy(&output.stdout);

    // Parse JSON array
    let entries: Vec<serde_json::Value> = serde_json::from_str(&json_str).unwrap_or_default();

    if entries.is_empty() {
        println!("No sync entries found");
        return Ok(());
    }

    // Convert to sortable structs
    let mut sync_entries: Vec<(f64, String, i64)> = entries
        .iter()
        .filter_map(|e| {
            let modified = e.get("modified")?.as_f64()?;
            let path = e.get("path")?.as_str()?;
            let size = e.get("fileSize")?.as_i64().unwrap_or(0);

            // Clean up path - remove {%SYNC_DIR%}/ prefix and URL decode
            let clean_path = path.strip_prefix("{%SYNC_DIR%}/").unwrap_or(path);
            let decoded = urlencoding::decode(clean_path)
                .unwrap_or_else(|_| clean_path.into())
                .to_string();

            Some((modified, decoded, size))
        })
        .collect();

    // Sort by modification time descending (most recent first)
    sync_entries.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

    println!(
        "Recent Sync Activity (showing {} of {} entries)",
        limit.min(sync_entries.len()),
        sync_entries.len()
    );
    println!("{}", "=".repeat(60));
    println!();

    for (modified, path, size) in sync_entries.into_iter().take(limit) {
        // Convert timestamp to datetime
        let secs = modified as i64;
        let nsecs = ((modified - secs as f64) * 1_000_000_000.0) as u32;

        if let Some(dt) = DateTime::from_timestamp(secs, nsecs) {
            let local: DateTime<Local> = dt.into();
            let now = Local::now();
            let duration = now.signed_duration_since(local);

            let ago = if duration.num_days() > 30 {
                format!("{} months ago", duration.num_days() / 30)
            } else if duration.num_days() > 0 {
                format!("{} days ago", duration.num_days())
            } else if duration.num_hours() > 0 {
                format!("{} hours ago", duration.num_hours())
            } else if duration.num_minutes() > 0 {
                format!("{} mins ago", duration.num_minutes())
            } else {
                "just now".to_string()
            };

            // Format size
            let size_str = if size > 1024 * 1024 {
                format!("{:.1} MB", size as f64 / (1024.0 * 1024.0))
            } else if size > 1024 {
                format!("{:.1} KB", size as f64 / 1024.0)
            } else {
                format!("{} B", size)
            };

            println!("{:<20} {:>10}  {}", ago, size_str, path);
        }
    }

    Ok(())
}

/// Trigger a sync via UI automation
pub fn sync_trigger() -> Result<()> {
    // First check if forScore is running
    let check = Command::new("pgrep").args(["-x", "forScore"]).output()?;

    if !check.status.success() {
        eprintln!("forScore is not running. Please start forScore first.");
        return Ok(());
    }

    // AppleScript to open Sync panel via menu
    // forScore on Mac (iPad app) uses standard menu bar
    let script = r#"
tell application "forScore" to activate
delay 0.5

tell application "System Events"
    tell process "forScore"
        -- Get all menu bar items
        set menuNames to name of every menu bar item of menu bar 1

        -- Look for Tools menu (common location for Sync)
        if menuNames contains "Tools" then
            click menu bar item "Tools" of menu bar 1
            delay 0.2
            try
                click menu item "Sync" of menu "Tools" of menu bar 1
            end try
        else
            -- Try Window menu as fallback
            if menuNames contains "Window" then
                click menu bar item "Window" of menu bar 1
                delay 0.2
                try
                    click menu item "Sync" of menu "Window" of menu bar 1
                end try
            end if
        end if
    end tell
end tell

return "ok"
"#;

    println!("Triggering forScore sync...");

    let output = Command::new("osascript").arg("-e").arg(script).output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("not allowed assistive access")
            || stderr.contains("not allowed to send keystrokes")
        {
            eprintln!("Error: Accessibility permissions required.");
            eprintln!();
            eprintln!("To enable, go to:");
            eprintln!("  System Settings → Privacy & Security → Accessibility");
            eprintln!("  and add your terminal app (Terminal, iTerm2, etc.)");
            eprintln!();
            eprintln!("After enabling, try again.");
            return Ok(());
        }
        eprintln!("Error: {}", stderr);
        return Ok(());
    }

    println!("Sync panel opened in forScore.");
    println!("Pull down on the sync list to trigger a refresh.");

    Ok(())
}
