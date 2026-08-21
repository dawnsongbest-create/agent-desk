use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::domain::{
    delivery::IngestDeliveryResult,
    reader::{validate_selection, ReaderDocument, ReaderValidationError},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReadingDifficulty {
    Normal,
    Technical,
}

impl ReadingDifficulty {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Technical => "technical",
        }
    }

    const fn units_per_minute(self) -> f64 {
        match self {
            Self::Normal => 500.0,
            Self::Technical => 300.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReadingPlanStatus {
    Active,
    Paused,
    Completed,
}

impl ReadingPlanStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Paused => "paused",
            Self::Completed => "completed",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateReadingPlanInput {
    pub title: String,
    pub source_name: Option<String>,
    pub content_markdown: String,
    pub daily_minutes: i64,
    pub schedule_time: String,
    pub difficulty: ReadingDifficulty,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadingPlan {
    pub id: String,
    pub title: String,
    pub source_name: Option<String>,
    pub content_markdown: String,
    pub total_content_length: i64,
    pub daily_minutes: i64,
    pub schedule_time: String,
    pub difficulty: ReadingDifficulty,
    pub status: ReadingPlanStatus,
    pub current_offset: i64,
    pub current_day: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct ReadingPlanDelivery {
    pub plan_id: String,
    pub day: i64,
    pub delivery_id: String,
    pub document_id: String,
    pub content_start: i64,
    pub content_end: i64,
    pub estimated_minutes: i64,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateReadingDeliveryResult {
    pub plan: ReadingPlan,
    pub delivery: IngestDeliveryResult,
    pub generation: ReadingPlanDelivery,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct ReadingSession {
    pub id: String,
    pub source_document_id: String,
    pub reader_document_id: String,
    pub content: String,
    pub estimated_minutes: i64,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadingSessionResult {
    pub session: ReadingSession,
    pub document: ReaderDocument,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadingSection {
    pub content: String,
    pub start: i64,
    pub end: i64,
    pub estimated_minutes: i64,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ReadingValidationError {
    #[error("Reading plan title cannot be empty.")]
    EmptyTitle,
    #[error("Reading plan content cannot be empty.")]
    EmptyContent,
    #[error("Reading plan content exceeds the supported size.")]
    ContentTooLong,
    #[error("Daily reading minutes must be between 1 and 240.")]
    InvalidDailyMinutes,
    #[error("Reading plan time must use a valid HH:MM value.")]
    InvalidScheduleTime,
    #[error("Reading plan has no unread content remaining.")]
    NoContentRemaining,
    #[error(transparent)]
    Reader(#[from] ReaderValidationError),
}

pub fn validate_plan(
    mut input: CreateReadingPlanInput,
) -> Result<CreateReadingPlanInput, ReadingValidationError> {
    input.title = input.title.trim().to_owned();
    if input.title.is_empty() {
        return Err(ReadingValidationError::EmptyTitle);
    }
    if input.content_markdown.trim().is_empty() {
        return Err(ReadingValidationError::EmptyContent);
    }
    if input.content_markdown.chars().count() > 1_000_000 {
        return Err(ReadingValidationError::ContentTooLong);
    }
    if !(1..=240).contains(&input.daily_minutes) {
        return Err(ReadingValidationError::InvalidDailyMinutes);
    }
    let bytes = input.schedule_time.as_bytes();
    let valid_time = bytes.len() == 5
        && bytes[2] == b':'
        && bytes[0].is_ascii_digit()
        && bytes[1].is_ascii_digit()
        && bytes[3].is_ascii_digit()
        && bytes[4].is_ascii_digit()
        && &input.schedule_time[0..2] <= "23"
        && &input.schedule_time[3..5] <= "59";
    if !valid_time {
        return Err(ReadingValidationError::InvalidScheduleTime);
    }
    input.source_name = input.source_name.and_then(|value| {
        let value = value.trim().to_owned();
        (!value.is_empty()).then_some(value)
    });
    Ok(input)
}

fn is_cjk(character: char) -> bool {
    matches!(character as u32, 0x3400..=0x4DBF | 0x4E00..=0x9FFF | 0xF900..=0xFAFF)
}

fn reading_units(content: &str) -> f64 {
    let mut cjk = 0usize;
    let mut latin_words = 0usize;
    let mut in_word = false;
    for character in content.chars() {
        if is_cjk(character) {
            cjk += 1;
            if in_word {
                latin_words += 1;
                in_word = false;
            }
        } else if character.is_alphanumeric() {
            in_word = true;
        } else if in_word {
            latin_words += 1;
            in_word = false;
        }
    }
    if in_word {
        latin_words += 1;
    }

    let structure_pauses = content
        .lines()
        .filter(|line| {
            let line = line.trim_start();
            line.is_empty()
                || line.starts_with('#')
                || line.starts_with("- ")
                || line.starts_with("* ")
                || line.starts_with("> ")
        })
        .count();
    let code_and_formula_markers = content.matches('`').count()
        + content.matches('$').count()
        + content.matches("::").count()
        + content.matches("=>").count();

    cjk as f64
        + latin_words as f64 * 2.5
        + structure_pauses as f64 * 12.0
        + code_and_formula_markers as f64 * 8.0
}

pub fn estimate_reading_minutes(content: &str, difficulty: ReadingDifficulty) -> i64 {
    ((reading_units(content) / difficulty.units_per_minute()).ceil() as i64).max(1)
}

pub fn infer_difficulty(content: &str) -> ReadingDifficulty {
    let technical_markers = content.matches("```").count()
        + content.matches('`').count() / 2
        + content.matches('$').count() / 2
        + content.matches("::").count()
        + content.matches("=>").count();
    if technical_markers >= 2 {
        ReadingDifficulty::Technical
    } else {
        ReadingDifficulty::Normal
    }
}

pub fn next_reading_section(plan: &ReadingPlan) -> Result<ReadingSection, ReadingValidationError> {
    let start = usize::try_from(plan.current_offset).unwrap_or(usize::MAX);
    let all_chars = plan.content_markdown.chars().collect::<Vec<_>>();
    if start >= all_chars.len() {
        return Err(ReadingValidationError::NoContentRemaining);
    }
    let remaining = all_chars[start..].iter().collect::<String>();
    let paragraphs = remaining.split_inclusive("\n\n").collect::<Vec<_>>();
    let mut chosen = String::new();
    for paragraph in paragraphs {
        let candidate = format!("{chosen}{paragraph}");
        let minutes = estimate_reading_minutes(&candidate, plan.difficulty);
        if minutes > plan.daily_minutes {
            break;
        }
        chosen = candidate;
        if minutes >= plan.daily_minutes {
            break;
        }
    }
    if chosen.is_empty() {
        let fallback = usize::try_from(plan.daily_minutes)
            .unwrap_or(1)
            .saturating_mul(match plan.difficulty {
                ReadingDifficulty::Normal => 500,
                ReadingDifficulty::Technical => 300,
            })
            .max(1);
        chosen = all_chars[start..all_chars.len().min(start + fallback)]
            .iter()
            .collect();
    }
    let length = chosen.chars().count();
    let end = (start + length) as i64;
    Ok(ReadingSection {
        estimated_minutes: estimate_reading_minutes(&chosen, plan.difficulty),
        content: chosen,
        start: start as i64,
        end,
    })
}

pub fn validate_session_content(content: String) -> Result<String, ReadingValidationError> {
    Ok(validate_selection(content)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plan(content: String, difficulty: ReadingDifficulty) -> ReadingPlan {
        ReadingPlan {
            id: "plan_1".to_owned(),
            title: "测试阅读".to_owned(),
            source_name: None,
            total_content_length: content.chars().count() as i64,
            content_markdown: content,
            daily_minutes: 2,
            schedule_time: "08:30".to_owned(),
            difficulty,
            status: ReadingPlanStatus::Active,
            current_offset: 0,
            current_day: 0,
            created_at: "now".to_owned(),
            updated_at: "now".to_owned(),
        }
    }

    #[test]
    fn estimates_chinese_english_and_technical_reading() {
        assert_eq!(
            estimate_reading_minutes(&"中".repeat(1_000), ReadingDifficulty::Normal),
            2
        );
        let english = (0..400).map(|_| "word").collect::<Vec<_>>().join(" ");
        assert_eq!(
            estimate_reading_minutes(&english, ReadingDifficulty::Normal),
            2
        );
        let technical = "```rust\nfn main() { Result::<(), Error>::Ok(())?; }\n```".repeat(10);
        assert!(
            estimate_reading_minutes(&technical, ReadingDifficulty::Technical)
                > estimate_reading_minutes(&technical, ReadingDifficulty::Normal)
        );
        assert_eq!(infer_difficulty(&technical), ReadingDifficulty::Technical);
    }

    #[test]
    fn creates_a_daily_section_on_markdown_boundaries_without_losing_content() {
        let content = format!(
            "# 第一节\n\n{}\n\n# 第二节\n\n{}",
            "甲".repeat(700),
            "乙".repeat(700)
        );
        let plan = plan(content.clone(), ReadingDifficulty::Normal);
        let first = next_reading_section(&plan).unwrap();
        assert!(first.content.starts_with("# 第一节"));
        assert!(first.end > first.start);
        assert_eq!(
            first.content,
            content.chars().take(first.end as usize).collect::<String>()
        );
    }

    #[test]
    fn validates_plan_time_and_preserves_full_markdown() {
        let content = "# 书\n\n正文".repeat(100);
        let validated = validate_plan(CreateReadingPlanInput {
            title: "  一份计划  ".to_owned(),
            source_name: Some("  本地文档  ".to_owned()),
            content_markdown: content.clone(),
            daily_minutes: 12,
            schedule_time: "07:30".to_owned(),
            difficulty: ReadingDifficulty::Normal,
        })
        .unwrap();
        assert_eq!(validated.title, "一份计划");
        assert_eq!(validated.source_name.as_deref(), Some("本地文档"));
        assert_eq!(validated.content_markdown, content);
    }
}
