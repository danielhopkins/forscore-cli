//! Setlist sync file handling for forScore
//!
//! forScore syncs setlists via .set sidecar files (gzipped plists).
//! When we modify setlists in the database, we also need to update these files
//! for changes to sync to other devices.

use crate::error::{ForScoreError, Result};
use crate::itm::sync_folder_path;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use plist::{Dictionary, Value};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::PathBuf;

/// URL-encode a setlist name for the filename
fn encode_setlist_name(name: &str) -> String {
    let mut encoded = String::new();
    for c in name.chars() {
        if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' || c == ' ' {
            encoded.push(c);
        } else {
            // URL encode non-ASCII and special characters
            for byte in c.to_string().as_bytes() {
                encoded.push_str(&format!("%{:02X}", byte));
            }
        }
    }
    encoded
}

/// Get the path to a setlist's .set file
pub fn setlist_file_path(name: &str) -> Result<PathBuf> {
    let sync_folder = sync_folder_path()?;
    let filename = format!("{}.set", encode_setlist_name(name));
    Ok(sync_folder.join(filename))
}

/// Read a setlist .set file
fn read_setlist_file(path: &PathBuf) -> Result<Dictionary> {
    if !path.exists() {
        return Err(ForScoreError::Other(format!(
            "Setlist file not found: {}",
            path.display()
        )));
    }

    let file = File::open(path)?;
    let mut decoder = GzDecoder::new(file);
    let mut decompressed = Vec::new();
    decoder.read_to_end(&mut decompressed)?;

    let value: Value = plist::from_bytes(&decompressed)
        .map_err(|e| ForScoreError::Other(format!("Failed to parse setlist plist: {}", e)))?;

    match value {
        Value::Dictionary(d) => Ok(d),
        _ => Err(ForScoreError::Other(
            "Setlist file is not a dictionary".into(),
        )),
    }
}

/// Write a setlist .set file
fn write_setlist_file(path: &PathBuf, dict: &Dictionary) -> Result<()> {
    let mut plist_data = Vec::new();
    plist::to_writer_binary(&mut plist_data, &Value::Dictionary(dict.clone()))
        .map_err(|e| ForScoreError::Other(format!("Failed to serialize setlist plist: {}", e)))?;

    let file = File::create(path)?;
    let mut encoder = GzEncoder::new(file, Compression::default());
    encoder.write_all(&plist_data)?;
    encoder.finish()?;

    Ok(())
}

/// Create a new setlist .set file
pub fn create_setlist_file(name: &str) -> Result<bool> {
    let path = setlist_file_path(name)?;

    if path.exists() {
        return Ok(false); // Already exists
    }

    let mut dict = Dictionary::new();
    dict.insert("title".to_string(), Value::String(name.to_string()));
    dict.insert("items".to_string(), Value::Array(vec![]));
    dict.insert("menuIndex".to_string(), Value::Integer(0.into()));
    dict.insert(
        "kRecoverableDestination".to_string(),
        Value::Integer(4.into()),
    );
    dict.insert(
        "kRecoverablePaddedKeys".to_string(),
        Value::Array(vec![
            Value::String("items".to_string()),
            Value::String("lastPlayed".to_string()),
            Value::String("library".to_string()),
            Value::String("menuIndex".to_string()),
            Value::String("title".to_string()),
        ]),
    );

    write_setlist_file(&path, &dict)?;
    Ok(true)
}

/// Rename a setlist .set file and update its title
pub fn rename_setlist_file(old_name: &str, new_name: &str) -> Result<bool> {
    let old_path = setlist_file_path(old_name)?;

    if !old_path.exists() {
        return Ok(false); // File doesn't exist, nothing to rename
    }

    // Read the file
    let mut dict = read_setlist_file(&old_path)?;

    // Update the title field
    dict.insert("title".to_string(), Value::String(new_name.to_string()));

    // Write to new path
    let new_path = setlist_file_path(new_name)?;
    write_setlist_file(&new_path, &dict)?;

    // Delete old file
    fs::remove_file(&old_path)?;

    // Update any folder files that reference this setlist
    update_folders_for_renamed_setlist(old_name, new_name)?;

    Ok(true)
}

/// Delete a setlist .set file
pub fn delete_setlist_file(name: &str) -> Result<bool> {
    let path = setlist_file_path(name)?;

    if !path.exists() {
        return Ok(false);
    }

    fs::remove_file(&path)?;
    Ok(true)
}

/// Score/bookmark item in a setlist
pub struct SetlistItem {
    pub file_path: String,
    pub title: String,
    pub identifier: String,
    pub is_bookmark: bool,
    pub first_page: Option<i64>,
    pub last_page: Option<i64>,
}

/// Add a score or bookmark to a setlist .set file
pub fn add_item_to_setlist_file(setlist_name: &str, item: &SetlistItem) -> Result<bool> {
    let path = setlist_file_path(setlist_name)?;

    if !path.exists() {
        // Create the file first
        create_setlist_file(setlist_name)?;
    }

    let mut dict = read_setlist_file(&path)?;

    // Get or create items array
    let items = match dict.get_mut("items") {
        Some(Value::Array(arr)) => arr,
        _ => {
            dict.insert("items".to_string(), Value::Array(vec![]));
            match dict.get_mut("items") {
                Some(Value::Array(arr)) => arr,
                _ => return Err(ForScoreError::Other("Failed to create items array".into())),
            }
        }
    };

    // Check if already exists
    for existing in items.iter() {
        if let Value::Dictionary(d) = existing {
            if let Some(Value::String(id)) = d.get("Identifier") {
                if id == &item.identifier {
                    return Ok(false); // Already in setlist
                }
            }
        }
    }

    // Create item dictionary
    let mut item_dict = Dictionary::new();
    item_dict.insert(
        "FilePath".to_string(),
        Value::String(item.file_path.clone()),
    );
    item_dict.insert("Title".to_string(), Value::String(item.title.clone()));
    item_dict.insert(
        "Identifier".to_string(),
        Value::String(item.identifier.clone()),
    );

    if item.is_bookmark {
        item_dict.insert("Bookmark".to_string(), Value::String("YES".to_string()));
        if let Some(first) = item.first_page {
            item_dict.insert("First Page".to_string(), Value::String(first.to_string()));
        }
        if let Some(last) = item.last_page {
            item_dict.insert("Last Page".to_string(), Value::String(last.to_string()));
        }
    }

    items.push(Value::Dictionary(item_dict));

    write_setlist_file(&path, &dict)?;
    Ok(true)
}

/// Remove an item from a setlist .set file by identifier
pub fn remove_item_from_setlist_file(setlist_name: &str, identifier: &str) -> Result<bool> {
    let path = setlist_file_path(setlist_name)?;

    if !path.exists() {
        return Ok(false);
    }

    let mut dict = read_setlist_file(&path)?;

    let items = match dict.get_mut("items") {
        Some(Value::Array(arr)) => arr,
        _ => return Ok(false),
    };

    let original_len = items.len();
    items.retain(|item| {
        if let Value::Dictionary(d) = item {
            match d.get("Identifier") {
                Some(Value::String(id)) => id != identifier,
                _ => true,
            }
        } else {
            true
        }
    });

    if items.len() == original_len {
        return Ok(false); // Nothing removed
    }

    write_setlist_file(&path, &dict)?;
    Ok(true)
}

/// Rebuild a setlist .set file with items in the specified order
pub fn reorder_setlist_file(setlist_name: &str, items: &[SetlistItem]) -> Result<bool> {
    let path = setlist_file_path(setlist_name)?;

    if !path.exists() {
        return Ok(false);
    }

    let mut dict = read_setlist_file(&path)?;

    // Rebuild items array
    let mut new_items = Vec::new();
    for item in items {
        let mut item_dict = Dictionary::new();
        item_dict.insert(
            "FilePath".to_string(),
            Value::String(item.file_path.clone()),
        );
        item_dict.insert("Title".to_string(), Value::String(item.title.clone()));
        item_dict.insert(
            "Identifier".to_string(),
            Value::String(item.identifier.clone()),
        );

        if item.is_bookmark {
            item_dict.insert("Bookmark".to_string(), Value::String("YES".to_string()));
            if let Some(first) = item.first_page {
                item_dict.insert("First Page".to_string(), Value::String(first.to_string()));
            }
            if let Some(last) = item.last_page {
                item_dict.insert("Last Page".to_string(), Value::String(last.to_string()));
            }
        }

        new_items.push(Value::Dictionary(item_dict));
    }

    dict.insert("items".to_string(), Value::Array(new_items));

    write_setlist_file(&path, &dict)?;
    Ok(true)
}

/// Update folder .fld files that reference a renamed setlist
fn update_folders_for_renamed_setlist(old_name: &str, new_name: &str) -> Result<()> {
    let sync_folder = sync_folder_path()?;

    let entries = fs::read_dir(&sync_folder)
        .map_err(|e| ForScoreError::Other(format!("Cannot read sync folder: {}", e)))?;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("fld") {
            continue;
        }

        // Try to read and update the folder file
        if let Ok(mut dict) = read_setlist_file(&path) {
            let mut modified = false;

            if let Some(Value::Array(setlists)) = dict.get_mut("setlists") {
                for setlist in setlists.iter_mut() {
                    if let Value::String(name) = setlist {
                        if name == old_name {
                            *name = new_name.to_string();
                            modified = true;
                        }
                    }
                }
            }

            if modified {
                let _ = write_setlist_file(&path, &dict);
            }
        }
    }

    Ok(())
}
