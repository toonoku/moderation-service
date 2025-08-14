use axum::{
    extract::{Path, State},
    response::Json,
    routing::{delete, get, post},
    Router,
};
use garde::Validate;
use regex::Regex;
use sqlx::PgPool;

use crate::{cache::ModerationCache, errors::Error, models::*};

#[derive(Clone)]
pub struct AppContext {
    pub pool: PgPool,
    pub cache: ModerationCache,
}

pub fn app_routes() -> Router<AppContext> {
    Router::new()
        // Check comments
        .route("/moderate", post(api_moderate))
        // Bad words
        .route("/rules/badwords", get(list_badwords).post(add_badword))
        .route("/rules/badwords/{word}", delete(delete_badword))
        // Regex rules
        .route("/rules/regex", get(list_regex).post(add_regex))
        .route("/rules/regex/{id}", delete(delete_regex))
        // Settings
        .route("/rules/settings", get(list_settings).post(insert_setting))
}

async fn api_moderate(
    State(state): State<AppContext>,
    Json(payload): Json<CommentRequest>,
) -> Result<Json<ApiResponse<ModerationResponse>>, Error> {
    payload
        .validate()
        .map_err(|e| Error::Validation(e.to_string()))?;

    let moderation_result = moderate_comment(&state.cache, &payload);

    Ok(Json(ApiResponse {
        success: true,
        message: "Comment moderated successfully".to_string(),
        data: moderation_result,
    }))
}

async fn list_badwords(
    State(state): State<AppContext>,
) -> Result<Json<ApiResponse<Vec<(String, String)>>>, Error> {
    let items = state
        .cache
        .bad_words
        .iter()
        .map(|(k, value)| (k.to_string(), value.to_string()))
        .collect();

    Ok(Json(ApiResponse {
        success: true,
        message: "Bad words retrieved successfully".to_string(),
        data: items,
    }))
}

async fn add_badword(
    State(state): State<AppContext>,
    Json(body): Json<BadWordCreate>,
) -> Result<Json<ApiResponse<Option<String>>>, Error> {
    body.validate()
        .map_err(|e| Error::Validation(e.to_string()))?;

    sqlx::query(
        "INSERT INTO bad_words (word, moderation_action) VALUES ($1, $2) ON CONFLICT DO NOTHING",
    )
    .bind(&body.word)
    .bind(&body.action)
    .execute(&state.pool)
    .await?;

    let rows: Vec<BadWordRow> = sqlx::query_as("SELECT * FROM bad_words ORDER BY id")
        .fetch_all(&state.pool)
        .await?;

    state
        .cache
        .load_bad_words(
            rows.into_iter()
                .map(|r| (r.word, r.moderation_action.to_string()))
                .collect(),
        )
        .await;

    Ok(Json(ApiResponse {
        success: true,
        message: "Bad word added successfully".to_string(),
        data: None,
    }))
}

async fn delete_badword(
    State(state): State<AppContext>,
    Path(word): Path<String>,
) -> Result<Json<ApiResponse<Option<String>>>, Error> {
    let res = sqlx::query!("DELETE FROM bad_words WHERE word = $1", word)
        .execute(&state.pool)
        .await?;

    if res.rows_affected() == 0 {
        return Err(Error::NotFound);
    }

    let rows: Vec<BadWordRow> = sqlx::query_as("SELECT * FROM bad_words ORDER BY id")
        .fetch_all(&state.pool)
        .await?;

    state
        .cache
        .load_bad_words(
            rows.into_iter()
                .map(|r| (r.word, r.moderation_action.to_string()))
                .collect(),
        )
        .await;

    Ok(Json(ApiResponse {
        success: true,
        message: "Bad word deleted successfully".to_string(),
        data: None,
    }))
}

async fn list_regex(
    State(state): State<AppContext>,
) -> Result<Json<ApiResponse<Vec<RegexRuleRow>>>, Error> {
    let rows: Vec<RegexRuleRow> = sqlx::query_as("SELECT * FROM regex_rules ORDER BY id")
        .fetch_all(&state.pool)
        .await?;

    Ok(Json(ApiResponse {
        success: true,
        message: "Regex rules retrieved successfully".to_string(),
        data: rows,
    }))
}

async fn add_regex(
    State(state): State<AppContext>,
    Json(body): Json<RegexRuleCreate>,
) -> Result<Json<ApiResponse<Option<String>>>, Error> {
    body.validate()
        .map_err(|e| Error::Validation(e.to_string()))?;

    let _ = Regex::new(&body.pattern).map_err(|e| Error::Regex(e.to_string()))?;

    let _: RegexRuleRow = sqlx::query_as(
        "INSERT INTO regex_rules (pattern, description, moderation_action) VALUES ($1, $2, $3) RETURNING id, pattern, description, moderation_action"
    )
        .bind(&body.pattern)
        .bind(&body.description)
        .bind(&body.action)
        .fetch_one(&state.pool)
        .await?;

    let rows: Vec<RegexRuleRow> = sqlx::query_as("SELECT * FROM regex_rules ORDER BY id")
        .fetch_all(&state.pool)
        .await?;

    let compiled_all = rows
        .into_iter()
        .map(|r| {
            let re = Regex::new(&r.pattern).unwrap();
            (
                r.id,
                re,
                r.description.unwrap_or_else(|| "Regex kuralı".into()),
                r.moderation_action.to_string(),
            )
        })
        .collect();

    state.cache.load_regex_rules(compiled_all).await;

    Ok(Json(ApiResponse {
        success: true,
        message: "Regex rule added successfully".to_string(),
        data: None,
    }))
}

async fn delete_regex(
    State(state): State<AppContext>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<Option<String>>>, Error> {
    let res = sqlx::query!("DELETE FROM regex_rules WHERE id = $1", id)
        .execute(&state.pool)
        .await?;

    if res.rows_affected() == 0 {
        return Err(Error::NotFound);
    }

    let rows: Vec<RegexRuleRow> = sqlx::query_as("SELECT * FROM regex_rules ORDER BY id")
        .fetch_all(&state.pool)
        .await?;

    let compiled_all = rows
        .into_iter()
        .map(|r| {
            let re = regex::Regex::new(&r.pattern).unwrap();
            (
                r.id,
                re,
                r.description.unwrap_or_else(|| "Regex kuralı".into()),
                r.moderation_action.to_string(),
            )
        })
        .collect();

    state.cache.load_regex_rules(compiled_all).await;

    Ok(Json(ApiResponse {
        success: true,
        message: "Regex rule deleted successfully".to_string(),
        data: None,
    }))
}

async fn list_settings(
    State(state): State<AppContext>,
) -> Result<Json<ApiResponse<Vec<SettingRow>>>, Error> {
    let rows: Vec<SettingRow> = sqlx::query_as("SELECT * FROM settings ORDER BY key")
        .fetch_all(&state.pool)
        .await?;

    Ok(Json(ApiResponse {
        success: true,
        message: "Settings retrieved successfully".to_string(),
        data: rows,
    }))
}

async fn insert_setting(
    State(state): State<AppContext>,
    Json(body): Json<SettingInsert>,
) -> Result<Json<ApiResponse<Option<String>>>, Error> {
    body.validate()
        .map_err(|e| Error::Validation(e.to_string()))?;

    sqlx::query!(
        "INSERT INTO settings (key, value) VALUES ($1, $2)
         ON CONFLICT (key) DO UPDATE SET value = EXCLUDED.value",
        body.key,
        body.value
    )
    .execute(&state.pool)
    .await?;

    let rows: Vec<SettingRow> = sqlx::query_as("SELECT * FROM settings ORDER BY key")
        .fetch_all(&state.pool)
        .await?;

    state
        .cache
        .load_settings(rows.into_iter().map(|r| (r.key, r.value)).collect())
        .await;

    Ok(Json(ApiResponse {
        success: true,
        message: "Setting updated successfully".to_string(),
        data: None,
    }))
}

// Check comment here
pub fn moderate_comment(cache: &ModerationCache, req: &CommentRequest) -> ModerationResponse {
    let text = req.content.to_lowercase();

    if let Some(bundle) = cache.bad_words_matcher.read().unwrap().as_ref() {
        if let Some(mat) = bundle.ac.find(&text) {
            let pat_index = mat.pattern();
            let word = &bundle.words[pat_index];
            let action = &bundle.actions[pat_index];

            return ModerationResponse {
                status: action.clone(),
                reason: Some(format!("Küfür tespit edildi: {word}")),
            };
        }
    }

    if let Some(bundle) = cache.regex_set_bundle.read().unwrap().as_ref() {
        let matches = bundle.set.matches(&text);
        if let Some(idx) = matches.into_iter().next() {
            let action = &bundle.actions[idx];
            let desc = &bundle.descriptions[idx];

            return ModerationResponse {
                status: action.clone(),
                reason: Some(desc.clone()),
            };
        }
    }

    ModerationResponse {
        status: "APPROVED".into(),
        reason: None,
    }
}
