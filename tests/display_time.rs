use chrono::{Local,TimeZone};
use tournament_manager_rs::display_time::scheduled_start_ms;
#[test]
fn dated_and_legacy_schedules_resolve_in_server_timezone() {
    let expected=Local.with_ymd_and_hms(2026,11,24,9,30,0).earliest().unwrap().timestamp_millis();
    for (planned,date) in [("2026-11-24 09:30:00",""),("2026-11-24T09:30:00",""),("09:30:00","2026-11-24"),("09:30","2026-11-24")] {
        assert_eq!(scheduled_start_ms(planned,date),Some(expected));
    }
    assert_eq!(scheduled_start_ms("2026-11-24T09:30:00+08:00",""),Some(1795483800000));
}
#[test]
fn unscheduled_and_invalid_dates_have_no_countdown() {
    for (planned,date) in [("",""),("not a time","2026-11-24"),("09:30",""),("2026-02-30 09:30:00","")] {assert_eq!(scheduled_start_ms(planned,date),None);}
}
