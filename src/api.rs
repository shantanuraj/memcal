use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use axum_extra::TypedHeader;
use headers::{authorization::Bearer, Authorization};
use hyper::HeaderMap;
use ical::{
    generator::{Emitter, IcalCalendarBuilder},
    parser::{
        ical::component::{IcalTimeZone, IcalTimeZoneTransition, IcalTimeZoneTransitionType},
        Component,
    },
    property::Property,
};
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
) -> Result<impl IntoResponse, StatusCode> {
    db::get_feed(&pool, feed_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let calendar = db::get_calendar(&pool, feed_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let _events = db::get_events_for_feed(&pool, feed_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let cal = IcalCalendarBuilder::version(calendar.version)
        .scale(calendar.cal_scale)
        .prodid(calendar.prod_id);

    let mut timezone: IcalTimeZone = IcalTimeZone::new();
    timezone.add_property(Property {
        name: "TZID".to_string(),
        value: Some(calendar.tz_id),
        params: None,
    });

    let dtstart = calendar.daylight_dtstart;
    let tzoffsetfrom = calendar.daylight_tzoffsetfrom;
    let tzoffsetto = calendar.daylight_tzoffsetto;
    let rrule = calendar.daylight_rrule;
    let tzname = calendar.daylight_tzname;

    let daylight = IcalTimeZoneTransition {
        transition: IcalTimeZoneTransitionType::DAYLIGHT,
        properties: vec![
            Property {
                name: "DTSTART".to_string(),
                value: dtstart,
                params: None,
            },
            Property {
                name: "TZOFFSETFROM".to_string(),
                value: tzoffsetfrom,
                params: None,
            },
            Property {
                name: "TZOFFSETTO".to_string(),
                value: tzoffsetto,
                params: None,
            },
            Property {
                name: "RRULE".to_string(),
                value: rrule,
                params: None,
            },
            Property {
                name: "TZNAME".to_string(),
                value: tzname,
                params: None,
            },
        ],
    };

    let dtstart = calendar.standard_dtstart;
    let tzoffsetfrom = calendar.standard_tzoffsetfrom;
    let tzoffsetto = calendar.standard_tzoffsetto;
    let rrule = calendar.standard_rrule;
    let tzname = calendar.standard_tzname;

    let standard = IcalTimeZoneTransition {
        transition: IcalTimeZoneTransitionType::STANDARD,
        properties: vec![
            Property {
                name: "DTSTART".to_string(),
                value: dtstart,
                params: None,
            },
            Property {
                name: "TZOFFSETFROM".to_string(),
                value: tzoffsetfrom,
                params: None,
            },
            Property {
                name: "TZOFFSETTO".to_string(),
                value: tzoffsetto,
                params: None,
            },
            Property {
                name: "RRULE".to_string(),
                value: rrule,
                params: None,
            },
            Property {
                name: "TZNAME".to_string(),
                value: tzname,
                params: None,
            },
        ],
    };

    timezone.transitions.push(daylight);
    timezone.transitions.push(standard);

    let cal = cal
        .add_tz(timezone)
        .set(Property {
            name: "X-WR-CALNAME".to_string(),
            value: calendar.name,
            params: None,
        })
        .build()
        .generate();

    let mut headers = HeaderMap::new();
    headers.insert("content-type", "text/calendar".parse().unwrap());

    Ok((headers, cal))
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
