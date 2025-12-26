use serde::Serialize;
use tabled::{Table, Tabled};

use crate::models::{Composer, Genre, Keyword, Library, Score, Setlist};
use crate::models::score::Bookmark;

/// Output format helper
pub fn output<T: Serialize + ToTable>(items: &[T], json: bool) {
    if json {
        println!("{}", serde_json::to_string_pretty(items).unwrap());
    } else {
        println!("{}", T::to_table(items));
    }
}

/// Output single score with clean formatting
pub fn output_score(score: &Score, json: bool) {
    if json {
        println!("{}", serde_json::to_string_pretty(score).unwrap());
    } else {
        println!("ID:         {}", score.id);
        println!("Title:      {}", score.title);
        println!("Path:       {}", score.path);
        if let Some(uuid) = &score.uuid {
            println!("UUID:       {}", uuid);
        }
        if let Some(key) = &score.key {
            println!("Key:        {}", key.display());
        }
        if let Some(rating) = score.rating {
            println!("Rating:     {} ({})", "★".repeat(rating as usize), rating);
        }
        if let Some(difficulty) = score.difficulty {
            println!("Difficulty: {}", difficulty);
        }
        if let Some(bpm) = score.bpm {
            if bpm > 0 {
                println!("BPM:        {}", bpm);
            }
        }
        if score.start_page.is_some() || score.end_page.is_some() {
            let pages = match (score.start_page, score.end_page) {
                (Some(s), Some(e)) if s == e => format!("{}", s),
                (Some(s), Some(e)) => format!("{}-{}", s, e),
                (Some(s), None) => format!("{}+", s),
                (None, Some(e)) => format!("-{}", e),
                _ => String::new(),
            };
            if !pages.is_empty() {
                println!("Pages:      {}", pages);
            }
        }
        if !score.composers.is_empty() {
            println!("Composers:  {}", score.composers.join(", "));
        }
        if !score.genres.is_empty() {
            println!("Genres:     {}", score.genres.join(", "));
        }
        if !score.keywords.is_empty() {
            println!("Keywords:   {}", score.keywords.join(", "));
        }
        if !score.labels.is_empty() {
            println!("Labels:     {}", score.labels.join(", "));
        }
    }
}

pub trait ToTable {
    fn to_table(items: &[Self]) -> String
    where
        Self: Sized;
}

#[derive(Tabled)]
struct ScoreRow {
    #[tabled(rename = "ID")]
    id: i64,
    #[tabled(rename = "Title")]
    title: String,
    #[tabled(rename = "Composer")]
    composer: String,
    #[tabled(rename = "Key")]
    key: String,
    #[tabled(rename = "Rating")]
    rating: String,
}

impl ToTable for Score {
    fn to_table(items: &[Self]) -> String {
        let rows: Vec<ScoreRow> = items
            .iter()
            .map(|s| ScoreRow {
                id: s.id,
                title: truncate(&s.title, 40),
                composer: truncate(&s.composers.first().cloned().unwrap_or_default(), 30),
                key: s.key.as_ref().map(|k| k.display()).unwrap_or_default(),
                rating: s.rating.map(|r| "★".repeat(r as usize)).unwrap_or_default(),
            })
            .collect();
        Table::new(rows).to_string()
    }
}

#[derive(Tabled)]
struct SetlistRow {
    #[tabled(rename = "ID")]
    id: i64,
    #[tabled(rename = "Name")]
    title: String,
    #[tabled(rename = "Scores")]
    score_count: i32,
}

impl ToTable for Setlist {
    fn to_table(items: &[Self]) -> String {
        let rows: Vec<SetlistRow> = items
            .iter()
            .map(|s| SetlistRow {
                id: s.id,
                title: s.title.clone(),
                score_count: s.score_count,
            })
            .collect();
        Table::new(rows).to_string()
    }
}

#[derive(Tabled)]
struct LibraryRow {
    #[tabled(rename = "ID")]
    id: i64,
    #[tabled(rename = "Name")]
    title: String,
    #[tabled(rename = "Scores")]
    score_count: i32,
}

impl ToTable for Library {
    fn to_table(items: &[Self]) -> String {
        let rows: Vec<LibraryRow> = items
            .iter()
            .map(|l| LibraryRow {
                id: l.id,
                title: l.title.clone(),
                score_count: l.score_count,
            })
            .collect();
        Table::new(rows).to_string()
    }
}

#[derive(Tabled)]
struct ComposerRow {
    #[tabled(rename = "ID")]
    id: i64,
    #[tabled(rename = "Name")]
    name: String,
    #[tabled(rename = "Scores")]
    score_count: i32,
}

impl ToTable for Composer {
    fn to_table(items: &[Self]) -> String {
        let rows: Vec<ComposerRow> = items
            .iter()
            .map(|c| ComposerRow {
                id: c.id,
                name: c.name.clone(),
                score_count: c.score_count,
            })
            .collect();
        Table::new(rows).to_string()
    }
}

#[derive(Tabled)]
struct GenreRow {
    #[tabled(rename = "ID")]
    id: i64,
    #[tabled(rename = "Name")]
    name: String,
    #[tabled(rename = "Scores")]
    score_count: i32,
}

impl ToTable for Genre {
    fn to_table(items: &[Self]) -> String {
        let rows: Vec<GenreRow> = items
            .iter()
            .map(|g| GenreRow {
                id: g.id,
                name: g.name.clone(),
                score_count: g.score_count,
            })
            .collect();
        Table::new(rows).to_string()
    }
}

#[derive(Tabled)]
struct KeywordRow {
    #[tabled(rename = "ID")]
    id: i64,
    #[tabled(rename = "Name")]
    name: String,
    #[tabled(rename = "Scores")]
    score_count: i32,
}

impl ToTable for Keyword {
    fn to_table(items: &[Self]) -> String {
        let rows: Vec<KeywordRow> = items
            .iter()
            .map(|k| KeywordRow {
                id: k.id,
                name: k.name.clone(),
                score_count: k.score_count,
            })
            .collect();
        Table::new(rows).to_string()
    }
}

#[derive(Tabled)]
struct BookmarkRow {
    #[tabled(rename = "ID")]
    id: i64,
    #[tabled(rename = "Title")]
    title: String,
    #[tabled(rename = "Pages")]
    pages: String,
}

impl ToTable for Bookmark {
    fn to_table(items: &[Self]) -> String {
        let rows: Vec<BookmarkRow> = items
            .iter()
            .map(|b| BookmarkRow {
                id: b.id,
                title: b.title.clone(),
                pages: match (b.start_page, b.end_page) {
                    (Some(s), Some(e)) if s == e => format!("{}", s),
                    (Some(s), Some(e)) => format!("{}-{}", s, e),
                    (Some(s), None) => format!("{}", s),
                    _ => String::new(),
                },
            })
            .collect();
        Table::new(rows).to_string()
    }
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        s.to_string()
    } else {
        format!("{}…", s.chars().take(max_len - 1).collect::<String>())
    }
}
