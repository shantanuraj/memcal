use crate::{db, ical::sync_ical_events};
use axum::extract::{Path, State};
use maud::{html, PreEscaped, DOCTYPE};
use sqlx::SqlitePool;

pub async fn index() -> maud::Markup {
    html! {
        (DOCTYPE)
        head {
            meta charset="utf-8";
            title { "memcal" }
            meta name="viewport" content="width=device-width, initial-scale=1";
            meta name="description" content="An iCal compatible server with memory.";
            meta name="author" content="Shantanu Raj";
            link rel="author" href="https://sraj.me";
            style type="text/css" {
                (PreEscaped(include_str!("./global.css")))
            }
        }
        body {
            .app-container {
                .sidebar {
                    .logo { "memcal" }
                    nav {
                        a href="#" { "Home" }
                        a href="#" { "Feed" }
                    }
                }
                .main-content {
                    header {
                        h1 { "Welcome to memcal" }
                        p { "An iCal compatible server with memory." }
                    }
                    .card {
                        h2 { "Add New Feed" }
                        form action="/feed" method="POST" {
                            input placeholder="iCal feed URL" type="url" id="url" name="url" required;
                            button type="submit" { "Add Feed" }
                        }
                    }
                }
            }
        }
    }
}

pub async fn feed_page(
    State(pool): State<SqlitePool>,
    Path((feed_id, _manage_token)): Path<(i64, String)>,
) -> Result<maud::Markup, axum::http::StatusCode> {
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
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;

    let feed_name = calendar.name.unwrap_or("Feed".to_string());
    let title = format!("{} | memcal", feed_name);

    Ok(html! {
        (DOCTYPE)
        head {
            meta charset="utf-8";
            title { (title) }
            meta name="viewport" content="width=device-width, initial-scale=1";
            meta name="description" content="Feed details and events";
            style type="text/css" {
                (PreEscaped(include_str!("./global.css")))
            }
        }
        body {
            .app-container {
                .sidebar {
                    .logo { "memcal" }
                    nav {
                        a href="/" { "Home" }
                        a href="#" class="active" { "Feed" }
                    }
                }
                .main-content {
                    header {
                        h1 { (feed_name) }
                        p.feed-url {
                            "URL: "
                            a target="_blank" rel="noopener noreferrer" href=(feed.url.clone()) { (feed.url) }
                        }
                    }
                    .card {
                        h2 { "Events" }
                        @if events.is_empty() {
                            p.no-events { "No events found for this feed." }
                        } @else {
                            ul.event-list {
                                @for event in events {
                                    li.event-item {
                                        h3 { (event.summary) }
                                        p.event-time {
                                            span.label { "Start: " }
                                            (event.start_time.format("%Y-%m-%d %H:%M"))
                                        }
                                        p.event-time {
                                            span.label { "End: " }
                                            (event.end_time.format("%Y-%m-%d %H:%M"))
                                        }
                                        @if let Some(description) = &event.description {
                                            @if !description.is_empty() {
                                                p.event-description {
                                                    span.label { "Description: " }
                                                    (description)
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    form.delete-form action={ "/feed/{}/{}" (feed_id) (feed.manage_token) } method="POST" {
                        input type="hidden" name="_method" value="DELETE";
                        button type="submit" class="delete-btn" { "Delete Feed" }
                    }
                }
            }
        }
    })
}
