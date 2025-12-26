use crate::cli::{ComposersCommand, GenresCommand, TagsCommand};
use crate::db::{open_readonly, open_readwrite, warn_if_running};
use crate::error::Result;
use crate::itm::rename_composer_in_all_itm;
use crate::models::meta::{
    list_composers, list_genres, list_keywords, merge_composers, rename_composer,
};
use crate::output::output;

pub fn handle_composers(cmd: ComposersCommand) -> Result<()> {
    match cmd {
        ComposersCommand::Ls { unused, json } => {
            let conn = open_readonly()?;
            let composers = list_composers(&conn, unused)?;
            output(&composers, json);
        }

        ComposersCommand::Rename { old_name, new_name } => {
            warn_if_running();
            let conn = open_readwrite()?;
            rename_composer(&conn, &old_name, &new_name)?;

            // Also update ITM files (both score-level and bookmark-level)
            match rename_composer_in_all_itm(&old_name, &new_name) {
                Ok((files, scores, bookmarks)) => {
                    println!("Renamed '{}' to '{}'", old_name, new_name);
                    if files > 0 {
                        println!(
                            "Updated {} ITM files ({} scores, {} bookmarks)",
                            files, scores, bookmarks
                        );
                    }
                }
                Err(e) => {
                    println!("Renamed '{}' to '{}' (database only)", old_name, new_name);
                    eprintln!("Warning: Failed to update ITM files: {}", e);
                }
            }
        }

        ComposersCommand::Merge { source, target } => {
            warn_if_running();
            let conn = open_readwrite()?;
            merge_composers(&conn, &source, &target)?;

            // Also update ITM files (rename source to target)
            match rename_composer_in_all_itm(&source, &target) {
                Ok((files, scores, bookmarks)) => {
                    println!("Merged '{}' into '{}'", source, target);
                    if files > 0 {
                        println!(
                            "Updated {} ITM files ({} scores, {} bookmarks)",
                            files, scores, bookmarks
                        );
                    }
                }
                Err(e) => {
                    println!("Merged '{}' into '{}' (database only)", source, target);
                    eprintln!("Warning: Failed to update ITM files: {}", e);
                }
            }
        }
    }

    Ok(())
}

pub fn handle_genres(cmd: GenresCommand) -> Result<()> {
    match cmd {
        GenresCommand::Ls { unused, json } => {
            let conn = open_readonly()?;
            let genres = list_genres(&conn, unused)?;
            output(&genres, json);
        }
    }

    Ok(())
}

pub fn handle_tags(cmd: TagsCommand) -> Result<()> {
    match cmd {
        TagsCommand::Ls { unused, json } => {
            let conn = open_readonly()?;
            let keywords = list_keywords(&conn, unused)?;
            output(&keywords, json);
        }
    }

    Ok(())
}
