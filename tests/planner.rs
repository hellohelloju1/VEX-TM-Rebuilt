use tournament_manager_rs::{model::*, planner, scoring::Game, setup};
fn draft() -> Event {
    let mut e = Event::default();
    for id in 1..=24 {
        e.teams.push(Team {
            id,
            number: id.to_string(),
            ..Default::default()
        });
    }
    e.config.schedule.blocks = vec![
        ScheduleBlock {
            date: "2026-11-23".into(),
            start: "09:00".into(),
            end: "09:36".into(),
            round: 1,
            cycle_seconds: 360,
        },
        ScheduleBlock {
            date: "2026-11-23".into(),
            start: "10:00".into(),
            end: "12:24".into(),
            round: 2,
            cycle_seconds: 360,
        },
    ];
    e
}
#[test]
fn preview_counts_actual_appearances_without_mutating_tournament() {
    let e = draft();
    let g = Game::all().into_iter().find(|g| g.id() == e.game).unwrap();
    let p = planner::preview(&e, &g).unwrap();
    assert_eq!(p.practice, 6);
    assert_eq!(p.qualifications, 24);
    assert!(p.appearances.values().all(|x| x == &[1, 4]));
    assert!(e.matches.is_empty());
}
#[test]
fn partial_rounds_are_reported() {
    let mut e = draft();
    e.config.schedule.blocks[1].end = "10:42".into();
    let g = Game::all().into_iter().find(|g| g.id() == e.game).unwrap();
    let p = planner::preview(&e, &g).unwrap();
    assert!(p.warnings.iter().any(|s| s.contains("counted matches")));
}
#[test]
fn invalid_dates_types_overlaps_and_short_blocks_are_rejected() {
    let mut e = draft();
    e.config.schedule.blocks[0].date = "2026-02-30".into();
    assert!(setup::validate_blocks(&e.config.schedule.blocks).is_err());
    assert!(setup::validate_date("2028-02-29").is_ok());
    e = draft();
    e.config.schedule.blocks[0].round = 255;
    assert!(setup::validate_blocks(&e.config.schedule.blocks).is_err());
    e = draft();
    e.config.schedule.blocks[0].end = "09:01".into();
    assert!(setup::validate_blocks(&e.config.schedule.blocks).is_err());
    e = draft();
    e.config.schedule.blocks[1].start = "09:30".into();
    assert!(setup::validate_blocks(&e.config.schedule.blocks).is_err());
}
#[test]
fn simultaneous_field_capacity_is_explicit() {
    let mut e = draft();
    e.config.field_sets[0].fields.push("Field 2".into());
    let g = Game::all().into_iter().find(|g| g.id() == e.game).unwrap();
    let p = planner::preview(&e, &g).unwrap();
    assert_eq!(p.practice, 12);
    assert_eq!(p.qualifications, 48);
    assert!(p.warnings.iter().any(|s| s.contains("simultaneously")));
    assert!(p.appearances.values().all(|x| x == &[2, 8]));
}
