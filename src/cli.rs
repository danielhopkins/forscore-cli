use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "forscore")]
#[command(version)]
#[command(about = "CLI tool for managing forScore metadata", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Manage scores
    Scores {
        #[command(subcommand)]
        command: ScoresCommand,
    },
    /// Manage setlists
    Setlists {
        #[command(subcommand)]
        command: SetlistsCommand,
    },
    /// Manage libraries
    Libraries {
        #[command(subcommand)]
        command: LibrariesCommand,
    },
    /// Manage composers
    Composers {
        #[command(subcommand)]
        command: ComposersCommand,
    },
    /// List genres
    Genres {
        #[command(subcommand)]
        command: GenresCommand,
    },
    /// List tags (keywords)
    Tags {
        #[command(subcommand)]
        command: TagsCommand,
    },
    /// Export data
    Export {
        #[command(subcommand)]
        command: ExportCommand,
    },
    /// Import data
    Import {
        #[command(subcommand)]
        command: ImportCommand,
    },
    /// List bookmarks in a score
    Bookmarks {
        #[command(subcommand)]
        command: BookmarksCommand,
    },
    /// Show library statistics
    Info,
    /// Backup the database
    Backup {
        /// Output file path
        #[arg(short, long)]
        output: Option<String>,
    },
    /// iCloud sync status and logs
    Sync {
        #[command(subcommand)]
        command: Option<SyncCommand>,
    },
    /// Fix common issues in the library
    Fixes {
        #[command(subcommand)]
        command: FixesCommand,
    },
}

#[derive(Subcommand)]
pub enum SyncCommand {
    /// Show recent sync activity log
    Log {
        /// Number of entries to show
        #[arg(short, long, default_value = "20")]
        limit: usize,
    },
    /// Trigger a sync (requires accessibility permissions)
    Trigger,
}

#[derive(Subcommand)]
pub enum ScoresCommand {
    /// List all scores
    Ls {
        /// Filter by library name or ID
        #[arg(long)]
        library: Option<String>,
        /// Filter by setlist name or ID
        #[arg(long)]
        setlist: Option<String>,
        /// Limit number of results
        #[arg(long, default_value = "25")]
        limit: usize,
        /// Sort by field: title, added, modified, played, rating, difficulty, path
        #[arg(long, default_value = "title")]
        sort: String,
        /// Sort descending
        #[arg(long)]
        desc: bool,
        /// Only show scores (exclude bookmarks)
        #[arg(long)]
        scores_only: bool,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Search scores
    Search {
        /// Search query (matches title or composer)
        query: Option<String>,
        /// Search by title only
        #[arg(long)]
        title: Option<String>,
        /// Search by composer
        #[arg(long)]
        composer: Option<String>,
        /// Search by genre
        #[arg(long)]
        genre: Option<String>,
        /// Search by key (e.g., "C Major", "F# Minor")
        #[arg(long)]
        key: Option<String>,
        /// Find items with no key set
        #[arg(long)]
        no_key: bool,
        /// Filter by minimum rating (1-6)
        #[arg(long)]
        rating: Option<i32>,
        /// Find items with no rating set
        #[arg(long)]
        no_rating: bool,
        /// Filter by difficulty (1-5)
        #[arg(long)]
        difficulty: Option<i32>,
        /// Limit number of results
        #[arg(long, default_value = "25")]
        limit: usize,
        /// Only show scores (exclude bookmarks)
        #[arg(long)]
        scores_only: bool,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Show detailed info for a score
    Show {
        /// Score ID, path, or title
        identifier: String,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Open a score in forScore
    Open {
        /// Score ID, path, or title
        identifier: String,
    },
    /// Edit score metadata
    Edit {
        /// Score ID, path, or title
        identifier: String,
        /// Set title
        #[arg(long)]
        title: Option<String>,
        /// Set composer
        #[arg(long)]
        composer: Option<String>,
        /// Set genre
        #[arg(long)]
        genre: Option<String>,
        /// Set key (e.g., "C Major", "F# Minor")
        #[arg(long)]
        key: Option<String>,
        /// Set rating (1-6)
        #[arg(long)]
        rating: Option<i32>,
        /// Set difficulty (1-5)
        #[arg(long)]
        difficulty: Option<i32>,
        /// Set tags (comma-separated)
        #[arg(long)]
        tags: Option<String>,
        /// Preview changes without applying
        #[arg(long)]
        dry_run: bool,
    },
}

#[derive(Subcommand)]
pub enum SetlistsCommand {
    /// List all setlists
    Ls {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Show scores in a setlist
    Show {
        /// Setlist ID or name
        identifier: String,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Create a new setlist
    Create {
        /// Setlist name
        name: String,
    },
    /// Rename a setlist
    Rename {
        /// Setlist ID or name
        identifier: String,
        /// New name
        new_name: String,
    },
    /// Delete a setlist
    Delete {
        /// Setlist ID or name
        identifier: String,
    },
    /// Add a score to a setlist
    AddScore {
        /// Setlist ID or name
        setlist: String,
        /// Score ID, path, or title
        score: String,
    },
    /// Remove a score from a setlist
    RemoveScore {
        /// Setlist ID or name
        setlist: String,
        /// Score ID, path, or title
        score: String,
    },
    /// Reorder a score within a setlist
    Reorder {
        /// Setlist ID or name
        setlist: String,
        /// Score ID, path, or title
        score: String,
        /// New position (1-based)
        #[arg(long)]
        position: usize,
    },
}

#[derive(Subcommand)]
pub enum LibrariesCommand {
    /// List all libraries
    Ls {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Show scores in a library
    Show {
        /// Library ID or name
        identifier: String,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Add a score to a library
    AddScore {
        /// Library ID or name
        library: String,
        /// Score ID, path, or title
        score: String,
    },
    /// Remove a score from a library
    RemoveScore {
        /// Library ID or name
        library: String,
        /// Score ID, path, or title
        score: String,
    },
}

#[derive(Subcommand)]
pub enum ComposersCommand {
    /// List all composers
    Ls {
        /// Show only unused composers
        #[arg(long)]
        unused: bool,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Rename a composer
    Rename {
        /// Current composer name
        old_name: String,
        /// New composer name
        new_name: String,
    },
    /// Merge two composers (move all scores from source to target)
    Merge {
        /// Source composer name
        source: String,
        /// Target composer name
        target: String,
    },
}

#[derive(Subcommand)]
pub enum GenresCommand {
    /// List all genres
    Ls {
        /// Show only unused genres
        #[arg(long)]
        unused: bool,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand)]
pub enum TagsCommand {
    /// List all tags (keywords)
    Ls {
        /// Show only unused tags
        #[arg(long)]
        unused: bool,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand)]
pub enum ExportCommand {
    /// Export all scores to CSV
    Csv {
        /// Output file path
        #[arg(short, long, default_value = "scores.csv")]
        output: String,
    },
}

#[derive(Subcommand)]
pub enum ImportCommand {
    /// Import scores from CSV
    Csv {
        /// Input CSV file
        file: String,
        /// Preview changes without applying
        #[arg(long)]
        dry_run: bool,
    },
}

#[derive(Subcommand)]
pub enum BookmarksCommand {
    /// List bookmarks in a score
    Ls {
        /// Score ID, path, or title
        score: String,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Show detailed info for a bookmark
    Show {
        /// Bookmark ID
        id: i64,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Edit bookmark metadata
    Edit {
        /// Bookmark ID
        id: i64,
        /// Set title
        #[arg(long)]
        title: Option<String>,
        /// Set composer
        #[arg(long)]
        composer: Option<String>,
        /// Set genre
        #[arg(long)]
        genre: Option<String>,
        /// Set key (e.g., "C Major", "F# Minor")
        #[arg(long)]
        key: Option<String>,
        /// Set rating (1-6)
        #[arg(long)]
        rating: Option<i32>,
        /// Set difficulty (1-5)
        #[arg(long)]
        difficulty: Option<i32>,
        /// Preview changes without applying
        #[arg(long)]
        dry_run: bool,
    },
    /// Delete a bookmark
    Delete {
        /// Bookmark ID
        id: i64,
    },
}

#[derive(Subcommand)]
pub enum FixesCommand {
    /// Find and remove duplicate bookmarks (keeps older, removes newer)
    DuplicateBookmarks {
        /// Preview changes without applying
        #[arg(long)]
        dry_run: bool,
    },
}
