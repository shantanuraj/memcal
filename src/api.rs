use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use uuid::Uuid;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use sonyflake::Sonyflake;

#[derive(Deserialize)]
pub struct AddFeedRequest {
    url: String,
}

#[derive(Serialize)]
pub struct AddFeedResponse {
    url: String,
    manage_token: String,
    manage_url: String,
}

pub async fn add_feed(
    State(pool): State<SqlitePool>,
    Json(payload): Json<AddFeedRequest>,
) -> Result<Json<AddFeedResponse>, StatusCode> {

    let sf = Sonyflake::new().unwrap();
    let feed_id = sf.next_id().unwrap();
    let manage_token = Uuid::new_v4().to_string();

    // TODO: Implement database insertion

    Ok(Json(AddFeedResponse {
        url: format!("/feed/{}", feed_id),
        manage_token: manage_token.clone(),
        manage_url: format!("/feed/{}/{}", feed_id, manage_token),
    }))
}

pub async fn get_feed(
    State(pool): State<SqlitePool>,
    Path(feed_id): Path<String>,
) -> Result<String, StatusCode> {
    // TODO: Implement fetching and returning iCal data
    Ok("iCal data placeholder".to_string())
}

pub async fn delete_feed(
    State(pool): State<SqlitePool>,
    Path(feed_id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    // TODO: Implement feed deletion
    Ok(StatusCode::NO_CONTENT)
}
