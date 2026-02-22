use crate::error::{ForScoreError, Result};
use serde::{Deserialize, Serialize};

/// Musical key representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MusicalKey {
    pub code: i32,
    pub note: String,
    pub mode: String,
}

impl TryFrom<&str> for MusicalKey {
    type Error = ForScoreError;

    /// Parse a key string like "C Major", "F# Minor", "Bb Major"
    fn try_from(s: &str) -> Result<Self> {
        let s = s.trim();
        let parts: Vec<&str> = s.split_whitespace().collect();

        if parts.len() != 2 {
            return Err(ForScoreError::InvalidKey(s.to_string()));
        }

        let note_str = parts[0];
        let mode_str = parts[1];

        // Parse note
        let (note_num, accidental) = match note_str.to_uppercase().as_str() {
            "C" => (1, 1),
            "C#" | "C♯" => (1, 2),
            "DB" | "D♭" => (2, 0),
            "D" => (2, 1),
            "D#" | "D♯" => (2, 2),
            "EB" | "E♭" => (3, 0),
            "E" => (3, 1),
            "F" => (4, 1),
            "F#" | "F♯" => (4, 2),
            "GB" | "G♭" => (5, 0),
            "G" => (5, 1),
            "G#" | "G♯" => (5, 2),
            "AB" | "A♭" => (6, 0),
            "A" => (6, 1),
            "A#" | "A♯" => (6, 2),
            "BB" | "B♭" => (7, 0),
            "B" => (7, 1),
            _ => return Err(ForScoreError::InvalidKey(s.to_string())),
        };

        // Parse mode
        let mode_num = match mode_str.to_lowercase().as_str() {
            "major" | "maj" => 0,
            "minor" | "min" => 1,
            _ => return Err(ForScoreError::InvalidKey(s.to_string())),
        };

        let code = note_num * 100 + accidental * 10 + mode_num;
        MusicalKey::try_from(code)
    }
}

impl TryFrom<i32> for MusicalKey {
    type Error = ForScoreError;

    fn try_from(code: i32) -> Result<Self> {
        if code <= 0 {
            return Err(ForScoreError::InvalidKeyCode(code));
        }

        let note_num = code / 100;
        let accidental = (code / 10) % 10;
        let mode_num = code % 10;

        let tonic = match note_num {
            1 => "C",
            2 => "D",
            3 => "E",
            4 => "F",
            5 => "G",
            6 => "A",
            7 => "B",
            _ => return Err(ForScoreError::InvalidKeyCode(code)),
        };

        let note = match accidental {
            2 => format!("{}#", tonic),
            0 => format!("{}b", tonic),
            _ => tonic.to_string(),
        };

        let mode = if mode_num == 0 { "Major" } else { "Minor" };

        Ok(MusicalKey {
            code,
            note,
            mode: mode.to_string(),
        })
    }
}

impl std::fmt::Display for MusicalKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.note, self.mode)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_code() {
        assert_eq!(MusicalKey::try_from(110).unwrap().to_string(), "C Major");
        assert_eq!(MusicalKey::try_from(111).unwrap().to_string(), "C Minor");
        assert_eq!(MusicalKey::try_from(310).unwrap().to_string(), "E Major");
        assert_eq!(MusicalKey::try_from(311).unwrap().to_string(), "E Minor");
        assert_eq!(MusicalKey::try_from(410).unwrap().to_string(), "F Major");
        assert_eq!(MusicalKey::try_from(510).unwrap().to_string(), "G Major");
    }

    #[test]
    fn test_from_string() {
        assert_eq!(MusicalKey::try_from("C Major").unwrap().code, 110);
        assert_eq!(MusicalKey::try_from("F# Minor").unwrap().code, 421);
        assert_eq!(MusicalKey::try_from("Bb Major").unwrap().code, 700);
    }
}
