use crate::error::{ForScoreError, Result};
use serde::{Deserialize, Serialize};

/// Musical key representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MusicalKey {
    pub code: i32,
    pub note: String,
    pub mode: String,
}

impl MusicalKey {
    /// Parse a key code (e.g., 110 = C Major, 311 = E Minor)
    /// Format: first digit = note (1-7 = C-B), second = sharp (0/1), third = mode (0=major, 1=minor)
    pub fn from_code(code: i32) -> Option<Self> {
        if code <= 0 {
            return None;
        }

        let note_num = code / 100;
        let sharp = (code / 10) % 10;
        let mode_num = code % 10;

        let note_base = match note_num {
            1 => "C",
            2 => "D",
            3 => "E",
            4 => "F",
            5 => "G",
            6 => "A",
            7 => "B",
            _ => return None,
        };

        let note = if sharp == 1 {
            format!("{}#", note_base)
        } else {
            note_base.to_string()
        };

        let mode = if mode_num == 0 { "Major" } else { "Minor" };

        Some(Self {
            code,
            note,
            mode: mode.to_string(),
        })
    }

    /// Parse a key string like "C Major", "F# Minor", "Bb Major"
    pub fn from_string(s: &str) -> Result<Self> {
        let s = s.trim();
        let parts: Vec<&str> = s.split_whitespace().collect();

        if parts.len() != 2 {
            return Err(ForScoreError::InvalidKey(s.to_string()));
        }

        let note_str = parts[0];
        let mode_str = parts[1];

        // Parse note
        let (note_num, sharp) = match note_str.to_uppercase().as_str() {
            "C" => (1, 0),
            "C#" | "C♯" | "DB" | "D♭" => (1, 1),
            "D" => (2, 0),
            "D#" | "D♯" | "EB" | "E♭" => (2, 1),
            "E" => (3, 0),
            "F" => (4, 0),
            "F#" | "F♯" | "GB" | "G♭" => (4, 1),
            "G" => (5, 0),
            "G#" | "G♯" | "AB" | "A♭" => (5, 1),
            "A" => (6, 0),
            "A#" | "A♯" | "BB" | "B♭" => (6, 1),
            "B" => (7, 0),
            _ => return Err(ForScoreError::InvalidKey(s.to_string())),
        };

        // Parse mode
        let mode_num = match mode_str.to_lowercase().as_str() {
            "major" | "maj" => 0,
            "minor" | "min" => 1,
            _ => return Err(ForScoreError::InvalidKey(s.to_string())),
        };

        let code = note_num * 100 + sharp * 10 + mode_num;
        Ok(Self::from_code(code).unwrap())
    }

    /// Get display string
    pub fn display(&self) -> String {
        format!("{} {}", self.note, self.mode)
    }
}

impl std::fmt::Display for MusicalKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_code() {
        assert_eq!(MusicalKey::from_code(110).unwrap().display(), "C Major");
        assert_eq!(MusicalKey::from_code(111).unwrap().display(), "C Minor");
        assert_eq!(MusicalKey::from_code(310).unwrap().display(), "E Major");
        assert_eq!(MusicalKey::from_code(311).unwrap().display(), "E Minor");
        assert_eq!(MusicalKey::from_code(410).unwrap().display(), "F Major");
        assert_eq!(MusicalKey::from_code(510).unwrap().display(), "G Major");
    }

    #[test]
    fn test_from_string() {
        assert_eq!(MusicalKey::from_string("C Major").unwrap().code, 110);
        assert_eq!(MusicalKey::from_string("F# Minor").unwrap().code, 411);
        assert_eq!(MusicalKey::from_string("Bb Major").unwrap().code, 610);
    }
}
