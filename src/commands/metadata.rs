use crate::cli::{ComposersCommand, GenresCommand, TagsCommand};
use crate::db::{open_readonly, open_readwrite, warn_if_running};
use crate::error::Result;
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
            println!("Renamed '{}' to '{}'", old_name, new_name);
        }

        ComposersCommand::Merge { source, target } => {
            warn_if_running();
            let conn = open_readwrite()?;
            merge_composers(&conn, &source, &target)?;
            println!("Merged '{}' into '{}'", source, target);
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
