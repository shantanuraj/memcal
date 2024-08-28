use crate::{db, ical::sync_ical_events};
use axum::extract::{Path, State};
use maud::{html, PreEscaped, DOCTYPE};
use sqlx::SqlitePool;
use axum::response::IntoResponse;

pub async fn index() -> maud::Markup {
    html! {
        (DOCTYPE)
        head {
            meta charset="utf-8";
            title { "memcal" }
            link rel="icon" type="image/svg+xml" href="/public/favicon.svg";
            link rel="apple-touch-icon" sizes="180x180" href="/public/apple-touch-icon.png";
            link rel="icon" type="image/png" sizes="16x16" href="/public/favicon-16x16.png";
            link rel="icon" type="image/png" sizes="32x32" href="/public/favicon-32x32.png";
            link rel="icon" type="image/png" sizes="96x96" href="/public/favicon-96x96.png";
            link rel="icon" type="image/png" sizes="128x128" href="/public/favicon-128x128.png";
            link rel="icon" type="image/png" sizes="256x256" href="/public/favicon-256x256.png";
            link rel="icon" type="image/png" sizes="512x512" href="/public/favicon-512x512.png";
            meta name="theme-color" content="#1e1e2d";
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
    Path((feed_id, manage_token)): Path<(i64, String)>,
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
    let delete_url = format!("/feed/{}/{}", feed_id, manage_token);

    Ok(html! {
        (DOCTYPE)
        head {
            meta charset="utf-8";
            title { (title) }
            link rel="icon" type="image/svg+xml" href="/public/favicon.svg";
            link rel="apple-touch-icon" sizes="180x180" href="/public/apple-touch-icon.png";
            link rel="icon" type="image/png" sizes="16x16" href="/public/favicon-16x16.png";
            link rel="icon" type="image/png" sizes="32x32" href="/public/favicon-32x32.png";
            link rel="icon" type="image/png" sizes="96x96" href="/public/favicon-96x96.png";
            link rel="icon" type="image/png" sizes="128x128" href="/public/favicon-128x128.png";
            link rel="icon" type="image/png" sizes="256x256" href="/public/favicon-256x256.png";
            link rel="icon" type="image/png" sizes="512x512" href="/public/favicon-512x512.png";
            meta name="theme-color" content="#1e1e2d";
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
                                        @if let Some(location) = &event.location {
                                            @if !location.is_empty() {
                                                p.event-location {
                                                    span.label { "Location: " }
                                                    (location.replace(r"\,", ","))
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    form.delete-form action={ (delete_url) } method="POST" {
                        input type="hidden" name="_method" value="DELETE";
                        button type="submit" class="delete-btn" { "Delete Feed" }
                    }
                }
            }
        }
    })
}

pub async fn robots_txt() -> impl IntoResponse {
    include_str!("robots.txt")
}
