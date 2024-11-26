use anyhow::{anyhow, Error};
use chrono::{DateTime, Duration, Local, NaiveDate, NaiveDateTime, Utc};
use ical::parser::ical::component::IcalEvent;
use regex::Regex;
use rrule::RRuleSet;
use std::str::FromStr;

trait OptionVecPush<T> {
    fn push(&mut self, t: T);
}

impl<T> OptionVecPush<T> for Option<Vec<T>> {
    fn push(&mut self, element: T) {
        if self.is_none() {
            let _ = self.insert(vec![element]);
        } else {
            self.as_mut().unwrap().push(element);
        }
    }
}

#[derive(Debug)]
pub enum DateMaybeTime {
    DateTime(DateTime<Utc>),
    Date(NaiveDate), // without time zone
}

impl From<NaiveDate> for DateMaybeTime {
    fn from(value: NaiveDate) -> Self {
        Self::Date(value)
    }
}

impl From<DateTime<Utc>> for DateMaybeTime {
    fn from(value: DateTime<Utc>) -> Self {
        Self::DateTime(value)
    }
}

#[derive(Debug)]
pub enum EventStatus {
    Tentative,
    Confirmed,
    Cancelled,
}

impl FromStr for EventStatus {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "TENTATIVE" => Ok(EventStatus::Tentative),
            "CONFIRMED" => Ok(EventStatus::Confirmed),
            "CANCELLED" => Ok(EventStatus::Cancelled),
            _ => Err(()),
        }
    }
}

#[derive(Debug)]
pub enum EventTransparency {
    Opaque,
    Transparent,
}

impl FromStr for EventTransparency {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "OPAQUE" => Ok(EventTransparency::Opaque),
            "TRANSPARENT" => Ok(EventTransparency::Transparent),
            _ => Err(()),
        }
    }
}

fn parse_duration(s: &str) -> Result<Duration, Error> {
    let re = Regex::new(
                r"^P(?:(?P<days>\d+)D)?(?:T(?:(?P<hours>\d+)H)?(?:(?P<minutes>\d+)M)?(?:(?P<seconds>\d+)S)?)?$",
            ).unwrap();

    if let Some(captures) = re.captures(s) {
        let days = captures
            .name("days")
            .map(|m| m.as_str().parse::<i64>().unwrap_or(0))
            .unwrap_or(0);
        let hours = captures
            .name("hours")
            .map(|m| m.as_str().parse::<i64>().unwrap_or(0))
            .unwrap_or(0);
        let minutes = captures
            .name("minutes")
            .map(|m| m.as_str().parse::<i64>().unwrap_or(0))
            .unwrap_or(0);
        let seconds = captures
            .name("seconds")
            .map(|m| m.as_str().parse::<i64>().unwrap_or(0))
            .unwrap_or(0);

        Ok(Duration::days(days)
            + Duration::hours(hours)
            + Duration::minutes(minutes)
            + Duration::seconds(seconds))
    } else {
        Err(anyhow!("Invalid duration format"))
    }
}

fn parse_datetime(s: &str) -> Result<DateMaybeTime, Error> {
    if let Ok(d) = NaiveDate::parse_from_str(s, "%Y%m%d") {
        return Ok(d.into());
    }
    let naive_datetime_res = NaiveDateTime::parse_from_str(s, "%Y%m%dT%H%M%SZ");
    if let Ok(dt) = naive_datetime_res {
        return Ok(dt.and_utc().into());
    }
    // No DateTime given, assume local
    let naive_datetime_res = NaiveDateTime::parse_from_str(s, "%Y%m%dT%H%M%S");
    if let Ok(dt) = naive_datetime_res {
        // TODO: does this work?
        let dt = dt.and_local_timezone(Local).unwrap();
        return Ok(dt.to_utc().into());
    }

    dateparser::parse(s).map(Into::into)
}

#[derive(Debug, Default)]
pub struct Event {
    pub uid: Option<String>,
    pub created: Option<DateMaybeTime>,
    pub summary: Option<String>,
    pub start: Option<DateMaybeTime>,
    pub end: Option<DateMaybeTime>,
    pub duration: Option<Duration>,
    pub location: Option<String>,
    pub description: Option<String>,
    pub status: Option<EventStatus>,
    pub transparency: Option<EventTransparency>,
    pub categories: Option<Vec<String>>,
    pub attendees: Option<Vec<String>>,
    pub organizer: Option<String>,
    pub priority: Option<u8>,
    pub sequence: Option<i32>,
    pub dtstamp: Option<DateMaybeTime>,
    pub recurrence_id: Option<DateMaybeTime>,
    pub rrule: Option<RRuleSet>,
    pub comment: Option<String>,
    pub attach: Option<Vec<String>>,
    pub alarms: Option<Vec<String>>,
    pub last_modified: Option<DateMaybeTime>,
}

impl TryFrom<&IcalEvent> for Event {
    type Error = Error;

    fn try_from(value: &IcalEvent) -> Result<Self, Self::Error> {
        map_ical_event(value)
    }
}

fn map_ical_event(input: &IcalEvent) -> Result<Event, Error> {
    let mut event = Event::default();
    let mut rrule_lines: Option<Vec<_>> = None;
    let mut has_rrules = false;
    for prop in input.properties.iter() {
        if prop.value.is_none() {
            continue;
        }
        let value = prop.value.as_ref().unwrap();
        let key: &str = &prop.name;
        if ["RDATE", "RRULE", "EXDATE", "EXRULE", "DTSTART"].contains(&key) {
            rrule_lines.push(format!("{}:{}", key, value));
        }
        match key {
            "UID" => event.uid = Some(value.to_string()),
            "SUMMARY" => event.summary = Some(value.to_string()),
            "DTSTART" => event.start = Some(parse_datetime(value.as_str())?),
            "DTEND" => event.end = Some(parse_datetime(value.as_str())?),
            "CREATED" => event.created = Some(parse_datetime(value.as_str())?),
            "DURATION" => event.duration = Some(parse_duration(value)?),
            "LOCATION" => event.location = Some(value.to_string()),
            "DESCRIPTION" => event.description = Some(value.to_string()),
            "STATUS" => event.status = Some(value.parse().map_err(|_| anyhow!("Invalid status"))?),
            "LAST-MODIFIED" => event.last_modified = Some(parse_datetime(value)?),
            "TRANSPARENCY" => {
                event.transparency =
                    Some(value.parse().map_err(|_| anyhow!("Invalid transparency"))?)
            }
            "CATEGORIES" => event.categories.push(value.to_string()), // Push to OptionVector
            "ATTENDEE" => event.attendees.push(value.to_string()),    // Push to OptionVector
            "ORGANIZER" => event.organizer = Some(value.to_string()),
            "PRIORITY" => {
                event.priority = Some(value.parse().map_err(|_| anyhow!("Invalid priority"))?)
            }
            "SEQUENCE" => {
                event.sequence = Some(value.parse().map_err(|_| anyhow!("Invalid sequence"))?)
            }
            "DTSTAMP" => event.dtstamp = Some(parse_datetime(value.as_str())?),
            "RECURRENCE-ID" => event.recurrence_id = Some(parse_datetime(value.as_str())?),
            "RRULE" => has_rrules = true,
            "RDATE" | "EXRULE" | "EXDATE" => (),
            "COMMENT" => event.comment = Some(value.to_string()),
            "ATTACH" => event.attach.push(value.to_string()),
            "ALARM" => event.alarms.push(value.to_string()),
            x if x.starts_with("X-") => (),
            "TRANSP" | "CLASS" => (),
            x => return Err(anyhow!("Unknown property key: {}", x)),
        }
    }
    if has_rrules {
        let rrule: RRuleSet = rrule_lines.unwrap().join("\n").parse()?;
        event.rrule = Some(rrule);
    }
    Ok(event)
}

#[cfg(test)]
mod tests {
    use std::{fs::File, io::BufReader};

    use super::*;

    #[test]
    fn it_works() {
        let buf = BufReader::new(File::open("resources/test1.ical").unwrap());

        let reader = ical::IcalParser::new(buf);

        for calendar in reader {
            let cal = calendar.unwrap();
            for event in cal.events {
                let res = map_ical_event(&event);
                let res = res.unwrap();
                if res.summary == Some("Jeden Montag bis Freitag ganztägig".into()) {
                    println!("{:#?}", res);
                    for event in res.rrule.unwrap().into_iter().take(100) {
                        println!("Occurance: {}", event)
                    }
                }
            }
        }
    }
}
