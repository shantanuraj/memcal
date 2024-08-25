use crate::db::{self, CalendarRow, Event};
use chrono::{NaiveDateTime, TimeZone};
use chrono_tz::Tz;
use ical::parser::{ical::component::IcalTimeZoneTransitionType, Component};
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

    let version = calendar
        .get_property("VERSION")
        .and_then(|p| p.value.clone())
        .unwrap_or("2.0".to_string());
    let prod_id = calendar
        .get_property("PRODID")
        .and_then(|p| p.value.clone())
        .unwrap_or("".to_string());
    let cal_scale = calendar
        .get_property("CALSCALE")
        .and_then(|p| p.value.clone())
        .unwrap_or("GREGORIAN".to_string());
    let name = calendar
        .get_property("X-WR-CALNAME")
        .and_then(|p| p.value.clone());

    let timezone = calendar.timezones.first();
    let tz_id = timezone
        .and_then(|t| t.get_property("TZID").and_then(|p| p.value.clone()))
        .unwrap_or("Etc/UTC".to_string());

    let daylight = {
        if let Some(timezone) = timezone {
            timezone
                .transitions
                .iter()
                .find(|t| matches!(t.transition, IcalTimeZoneTransitionType::DAYLIGHT))
        } else {
            None
        }
    };
    let standard = {
        if let Some(timezone) = timezone {
            timezone
                .transitions
                .iter()
                .find(|t| matches!(t.transition, IcalTimeZoneTransitionType::STANDARD))
        } else {
            None
        }
    };

    let daylight_dtstart =
        daylight.and_then(|t| t.get_property("DTSTART").and_then(|p| p.value.clone()));
    let daylight_tzoffsetfrom =
        daylight.and_then(|t| t.get_property("TZOFFSETFROM").and_then(|p| p.value.clone()));
    let daylight_tzoffsetto =
        daylight.and_then(|t| t.get_property("TZOFFSETTO").and_then(|p| p.value.clone()));
    let daylight_rrule =
        daylight.and_then(|t| t.get_property("RRULE").and_then(|p| p.value.clone()));
    let daylight_tzname =
        daylight.and_then(|t| t.get_property("TZNAME").and_then(|p| p.value.clone()));

    let standard_dtstart =
        standard.and_then(|t| t.get_property("DTSTART").and_then(|p| p.value.clone()));
    let standard_tzoffsetfrom =
        standard.and_then(|t| t.get_property("TZOFFSETFROM").and_then(|p| p.value.clone()));
    let standard_tzoffsetto =
        standard.and_then(|t| t.get_property("TZOFFSETTO").and_then(|p| p.value.clone()));
    let standard_rrule =
        standard.and_then(|t| t.get_property("RRULE").and_then(|p| p.value.clone()));
    let standard_tzname =
        standard.and_then(|t| t.get_property("TZNAME").and_then(|p| p.value.clone()));

    let cal = CalendarRow {
        feed_id,
        version,
        prod_id,
        cal_scale,
        name,
        tz_id,
        daylight_dtstart,
        daylight_tzoffsetfrom,
        daylight_tzoffsetto,
        daylight_rrule,
        daylight_tzname,
        standard_dtstart,
        standard_tzoffsetfrom,
        standard_tzoffsetto,
        standard_rrule,
        standard_tzname,
    };

    db::add_calendar(pool, &cal).await?;

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
            .map(|dt| {
                start_time_tz
                    .from_local_datetime(&dt)
                    .unwrap()
                    .with_timezone(&start_time_tz)
            })
            .ok_or("Invalid start time")?;

        let end_time = event
            .properties
            .iter()
            .find(|p| p.name == "DTEND")
            .and_then(|p| p.value.as_ref())
            .and_then(|v| NaiveDateTime::parse_from_str(v, "%Y%m%dT%H%M%S").ok())
            .map(|dt| {
                end_time_tz
                    .from_local_datetime(&dt)
                    .unwrap()
                    .with_timezone(&end_time_tz)
            })
            .ok_or("Invalid end time")?;

        let dtstamp = event
            .properties
            .iter()
            .find(|p| p.name == "DTSTAMP")
            .and_then(|p| p.value.as_ref())
            .and_then(|v| NaiveDateTime::parse_from_str(v, "%Y%m%dT%H%M%S").ok())
            .map(|dt| {
                dtstamp_tz
                    .from_local_datetime(&dt)
                    .unwrap()
                    .with_timezone(&dtstamp_tz)
            })
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

        let organizer_cn = event
            .properties
            .iter()
            .find(|p| p.name == "ORGANIZER")
            .and_then(|p| p.params.as_ref())
            .and_then(|p| p.iter().find(|(k, _)| k == "CN"))
            .and_then(|(_, v)| v.first())
            .map(|v| v.clone());

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
            organizer_cn,
            sequence,
            status,
        };

        db::add_event(pool, &event).await?;
    }

    Ok(())
}
