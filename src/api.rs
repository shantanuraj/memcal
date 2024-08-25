use axum::{
    async_trait,
    extract::{FromRequest, Path, Request, State},
    http::Method,
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
    Form, Json, RequestExt,
};
use axum_extra::TypedHeader;
use headers::ContentType;
use hyper::{header::CONTENT_TYPE, HeaderMap};
use ical::{
    generator::{Emitter, IcalCalendarBuilder, IcalEventBuilder},
    ical_param, ical_property,
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

use crate::{db, ical::sync_ical_events};

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
    TypedHeader(content_type): TypedHeader<ContentType>,
    JsonOrForm(payload): JsonOrForm<AddFeedRequest>,
) -> Result<Response, StatusCode> {
    let sf = Sonyflake::new().unwrap();
    let feed_id = sf.next_id().unwrap() as i64;
    let manage_token = Uuid::new_v4().to_string();

    db::add_feed(&pool, feed_id, &payload.url, &manage_token)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if content_type == ContentType::form_url_encoded() {
        let redirect_url = format!("/feed/{}/{}", feed_id, manage_token);
        Ok(Redirect::to(&redirect_url).into_response())
    } else {
        let response = AddFeedResponse {
            url: format!("/feed/{}", feed_id),
            manage_token: manage_token.clone(),
            manage_url: format!("/feed/{}/{}", feed_id, manage_token),
        };
        Ok(Json(response).into_response())
    }
}

pub async fn get_feed(
    State(pool): State<SqlitePool>,
    Path(feed_id): Path<i64>,
) -> Result<impl IntoResponse, StatusCode> {
    let feed = db::get_feed(&pool, feed_id)
        .await
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(axum::http::StatusCode::NOT_FOUND)?;

    let mut calendar = db::get_calendar(&pool, feed_id)
        .await
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;

    if calendar.is_none() {
        if let Err(e) = sync_ical_events(&pool, feed_id, &feed.url).await {
            eprintln!("Error syncing feed {}: {}", feed_id, e);
            return Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR);
        }
        calendar = db::get_calendar(&pool, feed_id)
            .await
            .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    let calendar = calendar.unwrap();

    let events = db::get_events_for_feed(&pool, feed_id)
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

    let mut cal = cal.add_tz(timezone).set(Property {
        name: "X-WR-CALNAME".to_string(),
        value: calendar.name,
        params: None,
    });

    let events = events
        .iter()
        .map(|event| {
            let mut ev = IcalEventBuilder::tzid(event.start_time_tz.to_string())
                .uid(event.uid.clone())
                .changed(event.dtstamp.format("%Y%m%dT%H%M%S").to_string())
                .start(event.start_time.format("%Y%m%dT%H%M%S").to_string())
                .end(event.end_time.format("%Y%m%dT%H%M%S").to_string())
                .set(ical_property!(
                    "DESCRIPTION",
                    &event.description.clone().unwrap_or("".to_string())
                ))
                .set(ical_property!("SUMMARY", &event.summary));

            if let Some(location) = &event.location {
                ev = ev.set(ical_property!("LOCATION", location));
            }
            if let Some(organizer) = &event.organizer {
                ev = ev.set(ical_property!(
                    "ORGANIZER",
                    organizer,
                    ical_param!("CN", event.organizer_cn.clone().unwrap_or("".to_string()))
                ));
            }
            if let Some(seq) = event.sequence {
                ev = ev.set(ical_property!("SEQUENCE", seq.to_string()));
            }
            if let Some(status) = &event.status {
                ev = ev.set(ical_property!("STATUS", status));
            }

            ev.build()
        })
        .collect::<Vec<_>>();

    for event in events {
        cal = cal.add_event(event);
    }

    let ics = cal.build().generate();

    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, "text/calendar".parse().unwrap());

    Ok((headers, ics))
}


#[derive(Deserialize)]
pub struct DeleteFeedRequest {
    #[serde(rename = "_method")]
    method: Option<String>,
}

pub async fn delete_feed(
    method: Method,
    State(pool): State<SqlitePool>,
    Path((feed_id, manage_token)): Path<(i64, String)>,
    JsonOrForm(payload): JsonOrForm<DeleteFeedRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    let is_form_request = payload.method.is_some();
    let method = payload.method.unwrap_or(method.to_string());
    if method != "DELETE" {
        return Err(StatusCode::METHOD_NOT_ALLOWED);
    }

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

    db::delete_calendar(&pool, feed_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    db::delete_feed(&pool, feed_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if is_form_request {
        Ok(Redirect::to("/").into_response())
    } else {
        Ok(StatusCode::NO_CONTENT.into_response())
    }
}

pub struct JsonOrForm<T>(T);

#[async_trait]
impl<S, T> FromRequest<S> for JsonOrForm<T>
where
    S: Send + Sync,
    Json<T>: FromRequest<()>,
    Form<T>: FromRequest<()>,
    T: 'static,
{
    type Rejection = Response;

    async fn from_request(req: Request, _state: &S) -> Result<Self, Self::Rejection> {
        let content_type_header = req.headers().get(CONTENT_TYPE);
        let content_type = content_type_header.and_then(|value| value.to_str().ok());

        if let Some(content_type) = content_type {
            if content_type.starts_with("application/json") {
                let Json(payload) = req.extract().await.map_err(IntoResponse::into_response)?;
                return Ok(Self(payload));
            }

            if content_type.starts_with("application/x-www-form-urlencoded") {
                let Form(payload) = req.extract().await.map_err(IntoResponse::into_response)?;
                return Ok(Self(payload));
            }
        }

        Err(StatusCode::UNSUPPORTED_MEDIA_TYPE.into_response())
    }
}
