use crate::db::{self, Event};
use chrono::{NaiveDateTime, TimeZone};
use chrono_tz::Tz;
use reqwest;
use sqlx::SqlitePool;

pub async fn sync_ical_events(
    pool: &SqlitePool,
    feed_id: i64,
    url: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Fetch the iCal feed
    let response = reqwest::get(url).await?.text().await?;

    // Parse the iCal feed
    let calendar = ical::IcalParser::new(response.as_bytes())
        .next()
        .ok_or("Failed to parse iCal")??;

    // Process events
    for event in calendar.events {
        let summary = event
            .properties
            .iter()
            .find(|p| p.name == "SUMMARY")
            .and_then(|p| p.value.clone())
            .unwrap_or_default();

        let description = event
            .properties
            .iter()
            .find(|p| p.name == "DESCRIPTION")
            .and_then(|p| p.value.clone())
            .unwrap_or_default();

        let start_time_tz = event
            .properties
            .iter()
            .find(|p| p.name == "DTSTART")
            .and_then(|p| p.params.as_ref())
            .and_then(|p| p.first())
            .and_then(|p| p.1.first())
            .map_or("Etc/UTC", |v| v);

        let start_time_tz = start_time_tz.parse::<Tz>().unwrap_or(Tz::UTC);

        let end_time_tz = event
            .properties
            .iter()
            .find(|p| p.name == "DTEND")
            .and_then(|p| p.params.as_ref())
            .and_then(|p| p.first())
            .and_then(|p| p.1.first())
            .map_or("Etc/UTC", |v| v);

        let end_time_tz = end_time_tz.parse::<Tz>().unwrap_or(Tz::UTC);

        let dtstamp_tz = event
            .properties
            .iter()
            .find(|p| p.name == "DTSTAMP")
            .and_then(|p| p.params.as_ref())
            .and_then(|p| p.first())
            .and_then(|p| p.1.first())
            .map_or("Etc/UTC", |v| v);

        let dtstamp_tz = dtstamp_tz.parse::<Tz>().unwrap_or(Tz::UTC);

        let start_time = event
            .properties
            .iter()
            .find(|p| p.name == "DTSTART")
            .and_then(|p| p.value.as_ref())
            .and_then(|v| NaiveDateTime::parse_from_str(v, "%Y%m%dT%H%M%S").ok())
            .map(|dt| start_time_tz.from_local_datetime(&dt).unwrap().with_timezone(&start_time_tz))
            .ok_or("Invalid start time")?;

        let end_time = event
            .properties
            .iter()
            .find(|p| p.name == "DTEND")
            .and_then(|p| p.value.as_ref())
            .and_then(|v| NaiveDateTime::parse_from_str(v, "%Y%m%dT%H%M%S").ok())
            .map(|dt| end_time_tz.from_local_datetime(&dt).unwrap().with_timezone(&end_time_tz))
            .ok_or("Invalid end time")?;

        let dtstamp = event
            .properties
            .iter()
            .find(|p| p.name == "DTSTAMP")
            .and_then(|p| p.value.as_ref())
            .and_then(|v| NaiveDateTime::parse_from_str(v, "%Y%m%dT%H%M%S").ok())
            .map(|dt| dtstamp_tz.from_local_datetime(&dt).unwrap().with_timezone(&dtstamp_tz))
            .ok_or("Invalid dtstamp")?;

        let location = event
            .properties
            .iter()
            .find(|p| p.name == "LOCATION")
            .and_then(|p| p.value.clone());

        let uid = event
            .properties
            .iter()
            .find(|p| p.name == "UID")
            .and_then(|p| p.value.clone())
            .unwrap_or_default();

        let organizer = event
            .properties
            .iter()
            .find(|p| p.name == "ORGANIZER")
            .and_then(|p| p.value.clone());

        let sequence = event
            .properties
            .iter()
            .find(|p| p.name == "SEQUENCE")
            .and_then(|p| p.value.as_ref())
            .and_then(|v| v.parse::<i64>().ok());

        let status = event
            .properties
            .iter()
            .find(|p| p.name == "STATUS")
            .and_then(|p| p.value.clone());

        let event = Event {
            id: 0,
            feed_id,
            summary,
            description: Some(description),
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
            status,
        };

        db::add_event(pool, &event).await?;
    }

    Ok(())
}
