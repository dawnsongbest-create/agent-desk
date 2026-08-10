use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StickyCardKind {
    Note,
    Task,
}

impl StickyCardKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Note => "note",
            Self::Task => "task",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StickyCard {
    pub id: String,
    pub kind: StickyCardKind,
    pub text: String,
    pub completed: bool,
    pub due_date: Option<String>,
    pub position: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewStickyCard {
    pub kind: StickyCardKind,
    pub text: String,
    pub due_date: Option<String>,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum StickyValidationError {
    #[error("Text cannot be empty.")]
    EmptyText,
    #[error("Text must be 4000 characters or fewer.")]
    TextTooLong,
    #[error("Only tasks can have a due date.")]
    NoteDueDate,
    #[error("Due date must be a real calendar date in YYYY-MM-DD format.")]
    InvalidDueDate,
    #[error("A reorder request cannot contain duplicate card IDs.")]
    DuplicatePlacement,
}

pub fn normalize_text(text: String) -> Result<String, StickyValidationError> {
    let normalized = text.trim().to_string();
    if normalized.is_empty() {
        return Err(StickyValidationError::EmptyText);
    }
    if normalized.chars().count() > 4000 {
        return Err(StickyValidationError::TextTooLong);
    }
    Ok(normalized)
}

pub fn validate_due_date(
    due_date: Option<String>,
) -> Result<Option<String>, StickyValidationError> {
    let Some(value) = due_date else {
        return Ok(None);
    };
    if !is_valid_iso_date(&value) {
        return Err(StickyValidationError::InvalidDueDate);
    }
    Ok(Some(value))
}

fn is_valid_iso_date(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-' {
        return false;
    }

    let parse = |range: std::ops::Range<usize>| value[range].parse::<u32>().ok();
    let (Some(year), Some(month), Some(day)) = (parse(0..4), parse(5..7), parse(8..10)) else {
        return false;
    };
    if year == 0 || !(1..=12).contains(&month) {
        return false;
    }
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let max_day = match month {
        2 if leap => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    };
    (1..=max_day).contains(&day)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_real_iso_dates() {
        assert!(is_valid_iso_date("2028-02-29"));
        assert!(is_valid_iso_date("2026-08-10"));
        assert!(!is_valid_iso_date("2026-02-29"));
        assert!(!is_valid_iso_date("2026-13-01"));
        assert!(!is_valid_iso_date("Aug 10"));
    }

    #[test]
    fn normalizes_lightweight_text_without_flattening_lines() {
        assert_eq!(
            normalize_text("  first line\nsecond line  ".to_string()).unwrap(),
            "first line\nsecond line"
        );
    }
}
