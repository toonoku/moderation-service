use garde::Validate;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::fmt;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "moderation_action_enum")]
#[sqlx(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ModerationAction {
    Approved,
    Rejected,
    NeedsReview,
}

impl fmt::Display for ModerationAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ModerationAction::Approved => write!(f, "APPROVED"),
            ModerationAction::Rejected => write!(f, "REJECTED"),
            ModerationAction::NeedsReview => write!(f, "NEEDS_REVIEW"),
        }
    }
}

#[derive(Deserialize, Validate)]
pub struct CommentRequest {
    #[garde(length(min = 1, max = 5000))]
    pub content: String,
}

#[derive(Serialize)]
pub struct ModerationResponse {
    pub status: String, // APPROVED | REJECTED | NEEDS_REVIEW
    pub reason: Option<String>,
}

#[derive(FromRow, Debug, Serialize)]
pub struct BadWordRow {
    pub id: i32,
    pub word: String,
    pub moderation_action: ModerationAction,
}

#[derive(Deserialize, Validate)]
pub struct BadWordCreate {
    #[garde(length(min = 2, max = 64))]
    pub word: String,
    #[garde(skip)]
    pub action: ModerationAction,
}

#[derive(FromRow, Debug, Serialize)]
pub struct RegexRuleRow {
    pub id: i32,
    pub pattern: String,
    pub description: Option<String>,
    pub moderation_action: ModerationAction,
}

#[derive(Deserialize, Validate)]
pub struct RegexRuleCreate {
    #[garde(length(min = 1, max = 512))]
    pub pattern: String,
    #[garde(length(min = 0, max = 256))]
    pub description: Option<String>,
    #[garde(skip)]
    pub action: ModerationAction,
}

#[derive(FromRow, Debug, Serialize)]
pub struct SettingRow {
    pub key: String,
    pub value: String,
}

#[derive(Deserialize, Validate)]
pub struct SettingInsert {
    #[garde(pattern(r"^[a-z0-9_]{2,64}$"))]
    pub key: String,
    #[garde(length(min = 1, max = 128))]
    pub value: String,
}

#[derive(Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub message: String,
    pub data: T,
}
