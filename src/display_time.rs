//! Resolve schedule wall times on the tournament server for remote displays.
use chrono::{DateTime,Local,NaiveDate,NaiveDateTime,NaiveTime,TimeZone};
pub fn scheduled_start_ms(planned: &str, event_date: &str) -> Option<i64> {
    if let Ok(time)=DateTime::parse_from_rfc3339(planned) { return Some(time.timestamp_millis()); }
    let dated=NaiveDateTime::parse_from_str(planned,"%Y-%m-%d %H:%M:%S").ok()
        .or_else(||NaiveDateTime::parse_from_str(planned,"%Y-%m-%dT%H:%M:%S").ok())
        .or_else(|| {
            let date=NaiveDate::parse_from_str(event_date,"%Y-%m-%d").ok()?;
            let time=NaiveTime::parse_from_str(planned,"%H:%M:%S").ok().or_else(||NaiveTime::parse_from_str(planned,"%H:%M").ok())?;
            Some(date.and_time(time))
        })?;
    Local.from_local_datetime(&dated).earliest().map(|time|time.timestamp_millis())
}
