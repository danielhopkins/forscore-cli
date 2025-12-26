use thiserror::Error;

#[derive(Error, Debug)]
pub enum ForScoreError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("Score not found: {0}")]
    ScoreNotFound(String),

    #[error("Setlist not found: {0}")]
    SetlistNotFound(String),

    #[error("Library not found: {0}")]
    LibraryNotFound(String),

    #[error("Composer not found: {0}")]
    ComposerNotFound(String),

    #[error("Ambiguous identifier '{0}': matches multiple items")]
    AmbiguousIdentifier(String),

    #[error("Invalid key format: {0}. Use format like 'C Major', 'F# Minor', 'Bb Major'")]
    InvalidKey(String),

    #[error("Invalid rating: {0}. Must be 1-6")]
    InvalidRating(i32),

    #[error("Invalid difficulty: {0}. Must be 1-5")]
    InvalidDifficulty(i32),

    #[error("forScore database not found at expected location")]
    DatabaseNotFound,

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("CSV error: {0}")]
    Csv(#[from] csv::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, ForScoreError>;
