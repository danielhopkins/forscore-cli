use crate::db::entity;
use crate::error::{ForScoreError, Result};
use crate::models::key::MusicalKey;
use rusqlite::{Connection, Row};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Score {
    pub id: i64,
    pub path: String,
    pub title: String,
    pub sort_title: Option<String>,
    pub uuid: Option<String>,
    pub rating: Option<i32>,
    pub difficulty: Option<i32>,
    pub key: Option<MusicalKey>,
    pub bpm: Option<i32>,
    pub start_page: Option<i32>,
    pub end_page: Option<i32>,
    pub composers: Vec<String>,
    pub genres: Vec<String>,
    pub keywords: Vec<String>,
    pub labels: Vec<String>,
}

impl Score {
    fn from_row(row: &Row) -> rusqlite::Result<Self> {
        let key_code: Option<i32> = row.get("ZKEY")?;
        Ok(Score {
            id: row.get("Z_PK")?,
            path: row.get("ZPATH")?,
            title: row.get::<_, Option<String>>("ZTITLE")?.unwrap_or_default(),
            sort_title: row.get("ZSORTTITLE")?,
            uuid: row.get("ZUUID")?,
            rating: row.get("rating_value")?,
            difficulty: row.get("difficulty_value")?,
            key: key_code.and_then(MusicalKey::from_code),
            bpm: row.get("ZBPM")?,
            start_page: row.get("ZSTARTPAGE")?,
            end_page: row.get("ZENDPAGE")?,
            composers: Vec::new(),
            genres: Vec::new(),
            keywords: Vec::new(),
            labels: Vec::new(),
        })
    }

    pub fn load_metadata(&mut self, conn: &Connection) -> Result<()> {
        // Load composers
        let mut stmt = conn.prepare(
            "SELECT m.ZVALUE FROM ZMETA m
             JOIN Z_4COMPOSERS c ON m.Z_PK = c.Z_10COMPOSERS
             WHERE c.Z_4ITEMS1 = ?",
        )?;
        self.composers = stmt
            .query_map([self.id], |row| row.get(0))?
            .filter_map(|r| r.ok())
            .collect();

        // Load genres (uses ZVALUE2)
        let mut stmt = conn.prepare(
            "SELECT m.ZVALUE2 FROM ZMETA m
             JOIN Z_4GENRES g ON m.Z_PK = g.Z_12GENRES
             WHERE g.Z_4ITEMS4 = ?",
        )?;
        self.genres = stmt
            .query_map([self.id], |row| row.get(0))?
            .filter_map(|r| r.ok())
            .collect();

        // Load keywords
        let mut stmt = conn.prepare(
            "SELECT m.ZVALUE FROM ZMETA m
             JOIN Z_4KEYWORDS k ON m.Z_PK = k.Z_13KEYWORDS
             WHERE k.Z_4ITEMS5 = ?",
        )?;
        self.keywords = stmt
            .query_map([self.id], |row| row.get(0))?
            .filter_map(|r| r.ok())
            .collect();

        // Load labels
        let mut stmt = conn.prepare(
            "SELECT m.ZVALUE FROM ZMETA m
             JOIN Z_4LABELS l ON m.Z_PK = l.Z_14LABELS
             WHERE l.Z_4ITEMS2 = ?",
        )?;
        self.labels = stmt
            .query_map([self.id], |row| row.get(0))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(())
    }
}

/// List all scores with sorting and limit
pub fn list_scores(
    conn: &Connection,
    sort: &str,
    desc: bool,
    limit: usize,
    scores_only: bool,
) -> Result<Vec<Score>> {
    let order_col = match sort {
        "title" => "i.ZSORTTITLE",
        "added" => "i.ZADDED",
        "modified" => "i.ZMODIFIED",
        "played" => "i.ZLASTPLAYED",
        "rating" => "r.ZVALUE5",
        "difficulty" => "d.ZVALUE1",
        "path" => "i.ZPATH",
        _ => "i.ZSORTTITLE",
    };

    let direction = if desc { "DESC" } else { "ASC" };

    let entity_filter = if scores_only {
        "i.Z_ENT = ?".to_string()
    } else {
        "i.Z_ENT IN (?, ?)".to_string()
    };

    let sql = format!(
        "SELECT i.Z_PK, i.ZPATH, i.ZTITLE, i.ZSORTTITLE, i.ZUUID, r.ZVALUE5 as rating_value, d.ZVALUE1 as difficulty_value, i.ZKEY, i.ZBPM, i.ZSTARTPAGE, i.ZENDPAGE
         FROM ZITEM i
         LEFT JOIN ZMETA r ON i.ZRATING = r.Z_PK
         LEFT JOIN ZMETA d ON i.ZDIFFICULTY = d.Z_PK
         WHERE {} ORDER BY {} {} NULLS LAST LIMIT ?",
        entity_filter, order_col, direction
    );

    let mut stmt = conn.prepare(&sql)?;

    let scores: Vec<Score> = if scores_only {
        stmt.query_map(
            rusqlite::params![entity::SCORE, limit as i64],
            Score::from_row,
        )?
        .filter_map(|r| r.ok())
        .collect()
    } else {
        stmt.query_map(
            rusqlite::params![entity::SCORE, entity::BOOKMARK, limit as i64],
            Score::from_row,
        )?
        .filter_map(|r| r.ok())
        .collect()
    };

    Ok(scores)
}

/// List scores with full metadata
pub fn list_scores_with_metadata(conn: &Connection) -> Result<Vec<Score>> {
    let mut scores = list_scores(conn, "title", false, 10000, true)?;
    for score in &mut scores {
        score.load_metadata(conn)?;
    }
    Ok(scores)
}

/// List scores in a setlist (includes both scores and bookmarks)
pub fn list_scores_in_setlist(conn: &Connection, setlist_id: i64) -> Result<Vec<Score>> {
    let mut stmt = conn.prepare(
        "SELECT i.Z_PK, i.ZPATH, i.ZTITLE, i.ZSORTTITLE, i.ZUUID, r.ZVALUE5 as rating_value, d.ZVALUE1 as difficulty_value, i.ZKEY, i.ZBPM, i.ZSTARTPAGE, i.ZENDPAGE
         FROM ZITEM i
         JOIN ZCYLON c ON i.Z_PK = c.ZITEM
         LEFT JOIN ZMETA r ON i.ZRATING = r.Z_PK
         LEFT JOIN ZMETA d ON i.ZDIFFICULTY = d.Z_PK
         WHERE c.ZSETLIST = ? AND i.Z_ENT IN (?, ?)
         ORDER BY c.Z_PK",
    )?;

    let scores: Vec<Score> = stmt
        .query_map(
            [setlist_id, entity::BOOKMARK as i64, entity::SCORE as i64],
            Score::from_row,
        )?
        .filter_map(|r| r.ok())
        .collect();

    Ok(scores)
}

/// List scores in a library
pub fn list_scores_in_library(conn: &Connection, library_id: i64) -> Result<Vec<Score>> {
    let mut stmt = conn.prepare(
        "SELECT i.Z_PK, i.ZPATH, i.ZTITLE, i.ZSORTTITLE, i.ZUUID, r.ZVALUE5 as rating_value, d.ZVALUE1 as difficulty_value, i.ZKEY, i.ZBPM, i.ZSTARTPAGE, i.ZENDPAGE
         FROM ZITEM i
         JOIN Z_4LIBRARIES l ON i.Z_PK = l.Z_4ITEMS3
         LEFT JOIN ZMETA r ON i.ZRATING = r.Z_PK
         LEFT JOIN ZMETA d ON i.ZDIFFICULTY = d.Z_PK
         WHERE l.Z_7LIBRARIES = ? AND i.Z_ENT = ?
         ORDER BY i.ZSORTTITLE, i.ZTITLE",
    )?;

    let scores: Vec<Score> = stmt
        .query_map([library_id, entity::SCORE as i64], Score::from_row)?
        .filter_map(|r| r.ok())
        .collect();

    Ok(scores)
}

/// Get a score by ID
pub fn get_score_by_id(conn: &Connection, id: i64) -> Result<Score> {
    let mut stmt = conn.prepare(
        "SELECT i.Z_PK, i.ZPATH, i.ZTITLE, i.ZSORTTITLE, i.ZUUID, r.ZVALUE5 as rating_value, d.ZVALUE1 as difficulty_value, i.ZKEY, i.ZBPM, i.ZSTARTPAGE, i.ZENDPAGE
         FROM ZITEM i
         LEFT JOIN ZMETA r ON i.ZRATING = r.Z_PK
         LEFT JOIN ZMETA d ON i.ZDIFFICULTY = d.Z_PK
         WHERE i.Z_PK = ? AND i.Z_ENT = ?",
    )?;

    let mut score = stmt
        .query_row([id, entity::SCORE as i64], Score::from_row)
        .map_err(|_| ForScoreError::ScoreNotFound(id.to_string()))?;

    score.load_metadata(conn)?;
    Ok(score)
}

/// Get a score by path
pub fn get_score_by_path(conn: &Connection, path: &str) -> Result<Option<Score>> {
    let mut stmt = conn.prepare(
        "SELECT i.Z_PK, i.ZPATH, i.ZTITLE, i.ZSORTTITLE, i.ZUUID, r.ZVALUE5 as rating_value, d.ZVALUE1 as difficulty_value, i.ZKEY, i.ZBPM, i.ZSTARTPAGE, i.ZENDPAGE
         FROM ZITEM i
         LEFT JOIN ZMETA r ON i.ZRATING = r.Z_PK
         LEFT JOIN ZMETA d ON i.ZDIFFICULTY = d.Z_PK
         WHERE i.ZPATH = ? AND i.Z_ENT = ?",
    )?;

    match stmt.query_row([path, &entity::SCORE.to_string()], Score::from_row) {
        Ok(mut score) => {
            score.load_metadata(conn)?;
            Ok(Some(score))
        }
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// Get a score by title (exact match first, then contains)
pub fn get_score_by_title(conn: &Connection, title: &str) -> Result<Score> {
    // Try exact match first
    let mut stmt = conn.prepare(
        "SELECT i.Z_PK, i.ZPATH, i.ZTITLE, i.ZSORTTITLE, i.ZUUID, r.ZVALUE5 as rating_value, d.ZVALUE1 as difficulty_value, i.ZKEY, i.ZBPM, i.ZSTARTPAGE, i.ZENDPAGE
         FROM ZITEM i
         LEFT JOIN ZMETA r ON i.ZRATING = r.Z_PK
         LEFT JOIN ZMETA d ON i.ZDIFFICULTY = d.Z_PK
         WHERE i.ZTITLE = ? AND i.Z_ENT = ?",
    )?;

    if let Ok(mut score) = stmt.query_row([title, &entity::SCORE.to_string()], Score::from_row) {
        score.load_metadata(conn)?;
        return Ok(score);
    }

    // Try case-insensitive match
    let mut stmt = conn.prepare(
        "SELECT i.Z_PK, i.ZPATH, i.ZTITLE, i.ZSORTTITLE, i.ZUUID, r.ZVALUE5 as rating_value, d.ZVALUE1 as difficulty_value, i.ZKEY, i.ZBPM, i.ZSTARTPAGE, i.ZENDPAGE
         FROM ZITEM i
         LEFT JOIN ZMETA r ON i.ZRATING = r.Z_PK
         LEFT JOIN ZMETA d ON i.ZDIFFICULTY = d.Z_PK
         WHERE LOWER(i.ZTITLE) = LOWER(?) AND i.Z_ENT = ?",
    )?;

    if let Ok(mut score) = stmt.query_row([title, &entity::SCORE.to_string()], Score::from_row) {
        score.load_metadata(conn)?;
        return Ok(score);
    }

    // Try contains match
    let mut stmt = conn.prepare(
        "SELECT i.Z_PK, i.ZPATH, i.ZTITLE, i.ZSORTTITLE, i.ZUUID, r.ZVALUE5 as rating_value, d.ZVALUE1 as difficulty_value, i.ZKEY, i.ZBPM, i.ZSTARTPAGE, i.ZENDPAGE
         FROM ZITEM i
         LEFT JOIN ZMETA r ON i.ZRATING = r.Z_PK
         LEFT JOIN ZMETA d ON i.ZDIFFICULTY = d.Z_PK
         WHERE i.ZTITLE LIKE ? AND i.Z_ENT = ? LIMIT 2",
    )?;

    let pattern = format!("%{}%", title);
    let scores: Vec<Score> = stmt
        .query_map([&pattern, &entity::SCORE.to_string()], Score::from_row)?
        .filter_map(|r| r.ok())
        .collect();

    match scores.len() {
        0 => Err(ForScoreError::ScoreNotFound(title.to_string())),
        1 => {
            let mut score = scores.into_iter().next().unwrap();
            score.load_metadata(conn)?;
            Ok(score)
        }
        _ => Err(ForScoreError::AmbiguousIdentifier(title.to_string())),
    }
}

/// Resolve a score identifier (ID, path, or title)
pub fn resolve_score(conn: &Connection, identifier: &str) -> Result<Score> {
    // Try as numeric ID first
    if let Ok(id) = identifier.parse::<i64>() {
        if let Ok(score) = get_score_by_id(conn, id) {
            return Ok(score);
        }
    }

    // Try as exact path
    if let Some(score) = get_score_by_path(conn, identifier)? {
        return Ok(score);
    }

    // Try as title
    get_score_by_title(conn, identifier)
}

/// Search scores with filters
pub fn search_scores(
    conn: &Connection,
    query: Option<&str>,
    title: Option<&str>,
    composer: Option<&str>,
    genre: Option<&str>,
    key: Option<i32>,
    no_key: bool,
    min_rating: Option<i32>,
    no_rating: bool,
    difficulty: Option<i32>,
    limit: usize,
    scores_only: bool,
) -> Result<Vec<Score>> {
    let mut sql = String::from(
        "SELECT DISTINCT i.Z_PK, i.ZPATH, i.ZTITLE, i.ZSORTTITLE, i.ZUUID, r.ZVALUE5 as rating_value, d.ZVALUE1 as difficulty_value, i.ZKEY, i.ZBPM, i.ZSTARTPAGE, i.ZENDPAGE
         FROM ZITEM i
         LEFT JOIN ZMETA r ON i.ZRATING = r.Z_PK
         LEFT JOIN ZMETA d ON i.ZDIFFICULTY = d.Z_PK",
    );
    let mut joins = Vec::new();
    let mut conditions = if scores_only {
        vec![format!("i.Z_ENT = {}", entity::SCORE)]
    } else {
        vec![format!(
            "i.Z_ENT IN ({}, {})",
            entity::SCORE,
            entity::BOOKMARK
        )]
    };
    let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    // General query searches both title and composer
    let needs_composer_join = query.is_some() || composer.is_some();
    if needs_composer_join {
        joins.push("LEFT JOIN Z_4COMPOSERS c ON i.Z_PK = c.Z_4ITEMS1 LEFT JOIN ZMETA mc ON c.Z_10COMPOSERS = mc.Z_PK");
    }

    if let Some(q) = query {
        conditions.push("(i.ZTITLE LIKE ? OR mc.ZVALUE LIKE ?)".to_string());
        // Split on whitespace and join with % to match "Op 28" -> "Op. 28"
        let words: Vec<&str> = q.split_whitespace().collect();
        let pattern = format!("%{}%", words.join("%"));
        params.push(Box::new(pattern.clone()));
        params.push(Box::new(pattern));
    }

    if let Some(c) = composer {
        conditions.push("mc.ZVALUE LIKE ?".to_string());
        params.push(Box::new(format!("%{}%", c)));
    }

    if genre.is_some() {
        joins.push(
            "JOIN Z_4GENRES g ON i.Z_PK = g.Z_4ITEMS4 JOIN ZMETA mg ON g.Z_12GENRES = mg.Z_PK",
        );
        conditions.push("mg.ZVALUE2 LIKE ?".to_string());
        params.push(Box::new(format!("%{}%", genre.unwrap())));
    }

    if let Some(t) = title {
        conditions.push("i.ZTITLE LIKE ?".to_string());
        params.push(Box::new(format!("%{}%", t)));
    }

    if let Some(k) = key {
        conditions.push("i.ZKEY = ?".to_string());
        params.push(Box::new(k));
    } else if no_key {
        conditions.push("(i.ZKEY IS NULL OR i.ZKEY = 0)".to_string());
    }

    if let Some(rating) = min_rating {
        conditions.push("r.ZVALUE5 >= ?".to_string());
        params.push(Box::new(rating));
    } else if no_rating {
        conditions.push("i.ZRATING IS NULL".to_string());
    }

    if let Some(diff) = difficulty {
        conditions.push("d.ZVALUE1 = ?".to_string());
        params.push(Box::new(diff));
    }

    for join in &joins {
        sql.push(' ');
        sql.push_str(join);
    }

    sql.push_str(" WHERE ");
    sql.push_str(&conditions.join(" AND "));
    sql.push_str(" ORDER BY i.ZSORTTITLE, i.ZTITLE LIMIT ?");
    params.push(Box::new(limit as i64));

    let mut stmt = conn.prepare(&sql)?;
    let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();

    let scores: Vec<Score> = stmt
        .query_map(param_refs.as_slice(), Score::from_row)?
        .filter_map(|r| r.ok())
        .collect();

    Ok(scores)
}

/// List bookmarks in a score
pub fn list_bookmarks(conn: &Connection, score_id: i64) -> Result<Vec<Bookmark>> {
    let mut stmt = conn.prepare(
        "SELECT i.Z_PK, i.ZPATH, i.ZTITLE, i.ZUUID, i.ZSTARTPAGE, i.ZENDPAGE,
                r.ZVALUE5 as rating_value, d.ZVALUE1 as difficulty_value, i.ZKEY
         FROM ZITEM i
         LEFT JOIN ZMETA r ON i.ZRATING = r.Z_PK
         LEFT JOIN ZMETA d ON i.ZDIFFICULTY = d.Z_PK
         WHERE i.ZSCORE = ? AND i.Z_ENT = ?
         ORDER BY i.ZSTARTPAGE",
    )?;

    let bookmarks: Vec<Bookmark> = stmt
        .query_map([score_id, entity::BOOKMARK as i64], |row| {
            let key_code: Option<i32> = row.get("ZKEY")?;
            Ok(Bookmark {
                id: row.get("Z_PK")?,
                path: row.get("ZPATH")?,
                title: row.get("ZTITLE")?,
                uuid: row.get("ZUUID")?,
                start_page: row.get("ZSTARTPAGE")?,
                end_page: row.get("ZENDPAGE")?,
                rating: row.get("rating_value")?,
                difficulty: row.get("difficulty_value")?,
                key: key_code.and_then(MusicalKey::from_code),
                composers: Vec::new(),
                genres: Vec::new(),
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    Ok(bookmarks)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bookmark {
    pub id: i64,
    pub path: String,
    pub title: String,
    pub uuid: Option<String>,
    pub start_page: Option<i32>,
    pub end_page: Option<i32>,
    pub rating: Option<i32>,
    pub difficulty: Option<i32>,
    pub key: Option<MusicalKey>,
    pub composers: Vec<String>,
    pub genres: Vec<String>,
}

impl Bookmark {
    pub fn load_metadata(&mut self, conn: &Connection) -> Result<()> {
        // Load composers
        let mut stmt = conn.prepare(
            "SELECT m.ZVALUE FROM ZMETA m
             JOIN Z_4COMPOSERS c ON m.Z_PK = c.Z_10COMPOSERS
             WHERE c.Z_4ITEMS1 = ?",
        )?;
        self.composers = stmt
            .query_map([self.id], |row| row.get(0))?
            .filter_map(|r| r.ok())
            .collect();

        // Load genres (uses ZVALUE2)
        let mut stmt = conn.prepare(
            "SELECT m.ZVALUE2 FROM ZMETA m
             JOIN Z_4GENRES g ON m.Z_PK = g.Z_12GENRES
             WHERE g.Z_4ITEMS4 = ?",
        )?;
        self.genres = stmt
            .query_map([self.id], |row| row.get(0))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(())
    }
}

/// Get a bookmark by ID
pub fn get_bookmark_by_id(conn: &Connection, id: i64) -> Result<Bookmark> {
    let mut stmt = conn.prepare(
        "SELECT i.Z_PK, i.ZPATH, i.ZTITLE, i.ZUUID, i.ZSTARTPAGE, i.ZENDPAGE,
                r.ZVALUE5 as rating_value, d.ZVALUE1 as difficulty_value, i.ZKEY
         FROM ZITEM i
         LEFT JOIN ZMETA r ON i.ZRATING = r.Z_PK
         LEFT JOIN ZMETA d ON i.ZDIFFICULTY = d.Z_PK
         WHERE i.Z_PK = ? AND i.Z_ENT = ?",
    )?;

    let key_code: Option<i32> =
        stmt.query_row([id, entity::BOOKMARK as i64], |row| row.get("ZKEY"))?;

    let mut bookmark = stmt.query_row([id, entity::BOOKMARK as i64], |row| {
        Ok(Bookmark {
            id: row.get("Z_PK")?,
            path: row.get("ZPATH")?,
            title: row.get("ZTITLE")?,
            uuid: row.get("ZUUID")?,
            start_page: row.get("ZSTARTPAGE")?,
            end_page: row.get("ZENDPAGE")?,
            rating: row.get("rating_value")?,
            difficulty: row.get("difficulty_value")?,
            key: key_code.and_then(MusicalKey::from_code),
            composers: Vec::new(),
            genres: Vec::new(),
        })
    })?;

    bookmark.load_metadata(conn)?;
    Ok(bookmark)
}
