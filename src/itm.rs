//! ITM file handling for forScore sync
//!
//! forScore syncs metadata via .itm sidecar files (gzipped plists).
//! When we edit the database, we also need to update these files
//! for changes to sync to other devices.

use crate::error::{ForScoreError, Result};
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use plist::Value;
use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;

/// Get the path to the forScore sync folder
pub fn sync_folder_path() -> Result<PathBuf> {
    let home = dirs::home_dir()
        .ok_or_else(|| ForScoreError::Other("Cannot find home directory".into()))?;
    let path = home.join("Library/Containers/com.mgsdevelopment.forscore/Data/Library/Preferences/Sync");

    if path.exists() {
        Ok(path)
    } else {
        Err(ForScoreError::Other("Sync folder not found".into()))
    }
}

/// Get the ITM file path for a score's PDF path
pub fn itm_path_for_score(pdf_path: &str) -> Result<PathBuf> {
    let sync_folder = sync_folder_path()?;
    let itm_filename = format!("{}.itm", pdf_path);
    Ok(sync_folder.join(itm_filename))
}

/// Read and decompress an ITM file, returning the plist Value
pub fn read_itm(path: &PathBuf) -> Result<Value> {
    if !path.exists() {
        return Err(ForScoreError::Other(format!(
            "ITM file not found: {}",
            path.display()
        )));
    }

    let file = File::open(path)?;
    let mut decoder = GzDecoder::new(file);
    let mut decompressed = Vec::new();
    decoder.read_to_end(&mut decompressed)?;

    let value: Value = plist::from_bytes(&decompressed)
        .map_err(|e| ForScoreError::Other(format!("Failed to parse ITM plist: {}", e)))?;

    Ok(value)
}

/// Write a plist Value to a gzipped ITM file
pub fn write_itm(path: &PathBuf, value: &Value) -> Result<()> {
    // Serialize to binary plist
    let mut plist_data = Vec::new();
    plist::to_writer_binary(&mut plist_data, value)
        .map_err(|e| ForScoreError::Other(format!("Failed to serialize ITM plist: {}", e)))?;

    // Gzip compress
    let file = File::create(path)?;
    let mut encoder = GzEncoder::new(file, Compression::default());
    encoder.write_all(&plist_data)?;
    encoder.finish()?;

    Ok(())
}

/// Update fields in an ITM file for a score
pub struct ItmUpdate {
    pub title: Option<String>,
    pub composer: Option<String>,
    pub genre: Option<String>,
    pub key: Option<i64>,
    pub rating: Option<i64>,
    pub difficulty: Option<i64>,
}

impl ItmUpdate {
    pub fn new() -> Self {
        Self {
            title: None,
            composer: None,
            genre: None,
            key: None,
            rating: None,
            difficulty: None,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.title.is_none()
            && self.composer.is_none()
            && self.genre.is_none()
            && self.key.is_none()
            && self.rating.is_none()
            && self.difficulty.is_none()
    }
}

/// Update an ITM file with the given changes
pub fn update_itm(pdf_path: &str, update: &ItmUpdate) -> Result<bool> {
    if update.is_empty() {
        return Ok(false);
    }

    let itm_path = itm_path_for_score(pdf_path)?;

    if !itm_path.exists() {
        // ITM file doesn't exist - that's okay, forScore will create it
        // This can happen for newly added scores
        return Ok(false);
    }

    let value = read_itm(&itm_path)?;

    // Convert to dictionary for modification
    let mut dict = match value {
        Value::Dictionary(d) => d,
        _ => return Err(ForScoreError::Other("ITM file is not a dictionary".into())),
    };

    // Apply updates
    if let Some(title) = &update.title {
        dict.insert("title".to_string(), Value::String(title.clone()));
    }

    if let Some(composer) = &update.composer {
        dict.insert("composer".to_string(), Value::String(composer.clone()));
    }

    if let Some(genre) = &update.genre {
        dict.insert("genre".to_string(), Value::String(genre.clone()));
    }

    if let Some(key) = update.key {
        dict.insert("key".to_string(), Value::Integer(key.into()));
    }

    if let Some(rating) = update.rating {
        dict.insert("rating".to_string(), Value::Integer(rating.into()));
    }

    if let Some(difficulty) = update.difficulty {
        dict.insert("difficulty".to_string(), Value::Integer(difficulty.into()));
    }

    // Write back
    write_itm(&itm_path, &Value::Dictionary(dict))?;

    Ok(true)
}

/// Update fields for a bookmark in an ITM file
pub struct ItmBookmarkUpdate {
    pub title: Option<String>,
    pub composer: Option<String>,
    pub genre: Option<String>,
    pub key: Option<i64>,
    pub rating: Option<i64>,
    pub difficulty: Option<i64>,
}

impl ItmBookmarkUpdate {
    pub fn new() -> Self {
        Self {
            title: None,
            composer: None,
            genre: None,
            key: None,
            rating: None,
            difficulty: None,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.title.is_none()
            && self.composer.is_none()
            && self.genre.is_none()
            && self.key.is_none()
            && self.rating.is_none()
            && self.difficulty.is_none()
    }
}

/// Delete a bookmark from an ITM file by UUID
pub fn delete_bookmark_from_itm(pdf_path: &str, bookmark_uuid: Option<&str>) -> Result<bool> {
    let uuid = match bookmark_uuid {
        Some(u) => u,
        None => return Ok(false), // Can't match without UUID
    };

    let itm_path = itm_path_for_score(pdf_path)?;

    if !itm_path.exists() {
        return Ok(false);
    }

    let value = read_itm(&itm_path)?;

    // Convert to dictionary for modification
    let mut dict = match value {
        Value::Dictionary(d) => d,
        _ => return Err(ForScoreError::Other("ITM file is not a dictionary".into())),
    };

    // Get bookmarks array
    let bookmarks = match dict.get_mut("bookmarks") {
        Some(Value::Array(arr)) => arr,
        _ => return Ok(false), // No bookmarks array
    };

    // Find and remove the bookmark by UUID
    let original_len = bookmarks.len();
    bookmarks.retain(|bookmark| {
        if let Value::Dictionary(bm_dict) = bookmark {
            match bm_dict.get("Identifier") {
                Some(Value::String(id)) => id != uuid,
                _ => true, // Keep bookmarks without Identifier
            }
        } else {
            true // Keep non-dictionary items
        }
    });

    if bookmarks.len() == original_len {
        return Ok(false); // No bookmark was removed
    }

    // Write back
    write_itm(&itm_path, &Value::Dictionary(dict))?;

    Ok(true)
}

/// Update a bookmark within an ITM file
pub fn update_bookmark_in_itm(pdf_path: &str, bookmark_uuid: Option<&str>, update: &ItmBookmarkUpdate) -> Result<bool> {
    if update.is_empty() {
        return Ok(false);
    }

    let uuid = match bookmark_uuid {
        Some(u) => u,
        None => return Ok(false), // Can't match without UUID
    };

    let itm_path = itm_path_for_score(pdf_path)?;

    if !itm_path.exists() {
        return Ok(false);
    }

    let value = read_itm(&itm_path)?;

    // Convert to dictionary for modification
    let mut dict = match value {
        Value::Dictionary(d) => d,
        _ => return Err(ForScoreError::Other("ITM file is not a dictionary".into())),
    };

    // Get bookmarks array
    let bookmarks = match dict.get_mut("bookmarks") {
        Some(Value::Array(arr)) => arr,
        _ => return Ok(false), // No bookmarks array
    };

    // Find the bookmark by UUID
    let mut found = false;
    for bookmark in bookmarks.iter_mut() {
        if let Value::Dictionary(ref mut bm_dict) = bookmark {
            // Check if this bookmark matches the UUID
            let matches = match bm_dict.get("Identifier") {
                Some(Value::String(id)) => id == uuid,
                _ => false,
            };

            if matches {
                found = true;

                // Apply updates
                if let Some(title) = &update.title {
                    bm_dict.insert("Title".to_string(), Value::String(title.clone()));
                }

                if let Some(composer) = &update.composer {
                    bm_dict.insert("Composer".to_string(), Value::String(composer.clone()));
                }

                if let Some(genre) = &update.genre {
                    bm_dict.insert("Genre".to_string(), Value::String(genre.clone()));
                }

                if let Some(key) = update.key {
                    bm_dict.insert("Key".to_string(), Value::Integer(key.into()));
                }

                if let Some(rating) = update.rating {
                    bm_dict.insert("Rating".to_string(), Value::Integer(rating.into()));
                }

                if let Some(difficulty) = update.difficulty {
                    bm_dict.insert("Difficulty".to_string(), Value::Integer(difficulty.into()));
                }

                break;
            }
        }
    }

    if !found {
        return Ok(false);
    }

    // Write back
    write_itm(&itm_path, &Value::Dictionary(dict))?;

    Ok(true)
}
