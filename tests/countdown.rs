use std::time::Duration;
use tournament_manager_rs::schedule::{Phase, Timer, advance_countdown};
#[test]
fn countdown_keeps_match_idle_until_three_seconds() {
    let mut t = Timer::new(15, 105);
    let mut c = Some(Duration::from_secs(3));
    for expected in [2, 1] {
        assert!(!advance_countdown(&mut t, &mut c, Duration::from_secs(1)));
        assert_eq!(c.unwrap().as_secs(), expected);
        assert_eq!(t.phase, Phase::Idle);
        assert_eq!(t.remaining.as_secs(), 120);
    }
    assert!(advance_countdown(&mut t, &mut c, Duration::from_secs(1)));
    assert_eq!(c, None);
    assert_eq!(t.phase, Phase::Autonomous);
    assert_eq!(t.remaining.as_secs(), 15);
}
#[test]
fn countdown_overshoot_only_consumes_post_start_time() {
    let mut t = Timer::new(0, 60);
    let mut c = Some(Duration::from_secs(3));
    assert!(advance_countdown(
        &mut t,
        &mut c,
        Duration::from_millis(3500)
    ));
    assert_eq!(t.phase, Phase::Driver);
    assert_eq!(t.remaining, Duration::from_millis(59500));
}
