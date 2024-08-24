use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use axum_extra::TypedHeader;
use headers::{authorization::Bearer, Authorization};
use serde::{Deserialize, Serialize};
use sonyflake::Sonyflake;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::db;

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
    let feed_id = sf.next_id().unwrap() as i64;
    let manage_token = Uuid::new_v4().to_string();

    db::add_feed(&pool, feed_id, &payload.url, &manage_token)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(AddFeedResponse {
        url: format!("/feed/{}", feed_id),
        manage_token: manage_token.clone(),
        manage_url: format!("/feed/{}/{}", feed_id, manage_token),
    }))
}

pub async fn get_feed(
    State(pool): State<SqlitePool>,
    Path(feed_id): Path<i64>,
) -> Result<String, StatusCode> {
    let feed = db::get_feed(&pool, feed_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let _events = db::get_events_for_feed(&pool, feed_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // TODO: Convert events to iCal format
    Ok(format!("iCal data for feed: {}", feed.url))
}

pub async fn delete_feed(
    State(pool): State<SqlitePool>,
    Path(feed_id): Path<i64>,
    TypedHeader(Authorization(auth)): TypedHeader<Authorization<Bearer>>,
) -> Result<StatusCode, StatusCode> {
    let manage_token = auth.token().to_string();
    let feed = db::get_feed(&pool, feed_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    if feed.manage_token != manage_token {
        return Err(StatusCode::UNAUTHORIZED);
    }

    db::delete_events_for_feed(&pool, feed_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    db::delete_feed(&pool, feed_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::NO_CONTENT)
}
