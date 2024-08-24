use chrono::DateTime;
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};

#[derive(Debug, Serialize, Deserialize)]
pub struct Feed {
    pub id: i64,
    pub url: String,
    pub manage_token: String,
}

#[derive(Debug)]
pub struct Event {
    pub id: i64,
    pub feed_id: i64,
    pub summary: String,
    pub description: Option<String>,
    pub start_time: DateTime<Tz>,
    pub start_time_tz: Tz,
    pub end_time: DateTime<Tz>,
    pub end_time_tz: Tz,
    pub location: Option<String>,
    pub uid: String,
    pub dtstamp: DateTime<Tz>,
    pub dtstamp_tz: Tz,
    pub organizer: Option<String>,
    pub sequence: Option<i64>,
    pub status: Option<String>,
}

#[derive(FromRow)]
struct EventRow {
    id: i64,
    feed_id: i64,
    summary: String,
    description: Option<String>,
    start_time: String,
    start_time_tz: String,
    end_time: String,
    end_time_tz: String,
    location: Option<String>,
    uid: String,
    dtstamp: String,
    dtstamp_tz: String,
    organizer: Option<String>,
    sequence: Option<i64>,
    status: Option<String>,
}

impl TryFrom<EventRow> for Event {
    type Error = chrono::ParseError;

    fn try_from(row: EventRow) -> Result<Self, Self::Error> {
        let start_time_tz = row.start_time_tz.parse::<Tz>().unwrap_or(Tz::UTC);
        let end_time_tz = row.end_time_tz.parse::<Tz>().unwrap_or(Tz::UTC);
        let dtstamp_tz = row.dtstamp_tz.parse::<Tz>().unwrap_or(Tz::UTC);

        Ok(Event {
            id: row.id,
            feed_id: row.feed_id,
            summary: row.summary,
            description: row.description,
            start_time: DateTime::parse_from_rfc3339(&row.start_time)?.with_timezone(&start_time_tz),
            start_time_tz,
            end_time: DateTime::parse_from_rfc3339(&row.end_time)?.with_timezone(&end_time_tz),
            end_time_tz,
            location: row.location,
            uid: row.uid,
            dtstamp: DateTime::parse_from_rfc3339(&row.dtstamp)?.with_timezone(&dtstamp_tz),
            dtstamp_tz,
            organizer: row.organizer,
            sequence: row.sequence,
            status: row.status,
        })
    }
}

pub async fn init_db(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS feeds (
            id INTEGER PRIMARY KEY,
            url TEXT NOT NULL,
            manage_token TEXT NOT NULL
        )",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS events (
            id INTEGER PRIMARY KEY,
            feed_id INTEGER NOT NULL
                constraint events_feeds_id_fk
                    references feeds,
            summary TEXT NOT NULL,
            description TEXT,
            start_time TEXT NOT NULL,
            start_time_tz TEXT NOT NULL,
            end_time TEXT NOT NULL,
            end_time_tz TEXT NOT NULL,
            location TEXT,
            uid TEXT NOT NULL,
            dtstamp TEXT NOT NULL,
            dtstamp_tz TEXT NOT NULL,
            organizer TEXT,
            sequence INTEGER,
            status TEXT,
            constraint events_pk
                unique (feed_id, start_time, end_time, start_time_tz, end_time_tz)
        )",
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn get_all_feeds(pool: &SqlitePool) -> Result<Vec<Feed>, sqlx::Error> {
    sqlx::query_as!(Feed, "SELECT id, url, manage_token FROM feeds")
        .fetch_all(pool)
        .await
}

pub async fn add_feed(
    pool: &SqlitePool,
    id: i64,
    url: &str,
    manage_token: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "INSERT INTO feeds (id, url, manage_token) VALUES (?, ?, ?)",
        id,
        url,
        manage_token
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn get_feed(pool: &SqlitePool, id: i64) -> Result<Option<Feed>, sqlx::Error> {
    sqlx::query_as!(
        Feed,
        "SELECT id, url, manage_token FROM feeds WHERE id = ?",
        id
    )
    .fetch_optional(pool)
    .await
}

pub async fn delete_feed(pool: &SqlitePool, id: i64) -> Result<(), sqlx::Error> {
    sqlx::query!("DELETE FROM feeds WHERE id = ?", id)
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn add_event(pool: &SqlitePool, event: &Event) -> Result<(), sqlx::Error> {
    let start_time = event.start_time.to_rfc3339();
    let start_time_tz = event.start_time_tz.to_string();
    let end_time = event.end_time.to_rfc3339();
    let end_time_tz = event.end_time_tz.to_string();
    let dtstamp = event.dtstamp.to_rfc3339();
    let dtstamp_tz = event.dtstamp_tz.to_string();

    sqlx::query!(
        "INSERT INTO events (
            feed_id,
            summary,
            description,
            start_time,
            start_time_tz,
            end_time,
            end_time_tz,
            location,
            uid,
            dtstamp,
            dtstamp_tz,
            organizer,
            sequence,
            status
        ) VALUES (
            ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?
        )",
        event.feed_id,
        event.summary,
        event.description,
        start_time,
        start_time_tz,
        end_time,
        end_time_tz,
        event.location,
        event.uid,
        dtstamp,
        dtstamp_tz,
        event.organizer,
        event.sequence,
        event.status,
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn get_events_for_feed(
    pool: &SqlitePool,
    feed_id: i64,
) -> Result<Vec<Event>, sqlx::Error> {
    let rows = sqlx::query_as!(
        EventRow,
        "SELECT
            id, feed_id, summary, description, start_time, start_time_tz,
            end_time, end_time_tz, location, uid, dtstamp, dtstamp_tz,
            organizer, sequence, status
        FROM events WHERE feed_id = ?",
        feed_id
    )
    .fetch_all(pool)
    .await?;

    let events: Result<Vec<Event>, _> = rows.into_iter().map(Event::try_from).collect();
    events.map_err(|e| sqlx::Error::Decode(Box::new(e)))
}

pub async fn delete_events_for_feed(pool: &SqlitePool, feed_id: i64) -> Result<(), sqlx::Error> {
    sqlx::query!("DELETE FROM events WHERE feed_id = ?", feed_id)
        .execute(pool)
        .await?;

    Ok(())
}
