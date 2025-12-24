use crate::cli::ExportCommand;
use crate::db::open_readonly;
use crate::error::Result;
use crate::models::score::list_scores_with_metadata;
use csv::Writer;
use std::fs::File;

pub fn handle(cmd: ExportCommand) -> Result<()> {
    match cmd {
        ExportCommand::Csv { output } => {
            let conn = open_readonly()?;
            let scores = list_scores_with_metadata(&conn)?;

            let file = File::create(&output)?;
            let mut wtr = Writer::from_writer(file);

            // Write header
            wtr.write_record([
                "id",
                "path",
                "title",
                "composer",
                "genre",
                "key",
                "rating",
                "difficulty",
                "bpm",
                "keywords",
                "labels",
            ])?;

            // Write rows
            for score in &scores {
                wtr.write_record([
                    &score.id.to_string(),
                    &score.path,
                    &score.title,
                    &score.composers.join("; "),
                    &score.genres.join("; "),
                    &score.key.as_ref().map(|k| k.display()).unwrap_or_default(),
                    &score.rating.map(|r| r.to_string()).unwrap_or_default(),
                    &score.difficulty.map(|d| d.to_string()).unwrap_or_default(),
                    &score.bpm.map(|b| b.to_string()).unwrap_or_default(),
                    &score.keywords.join("; "),
                    &score.labels.join("; "),
                ])?;
            }

            wtr.flush()?;
            println!("Exported {} scores to {}", scores.len(), output);
        }
    }

    Ok(())
}
