use tournament_manager_rs::{auth, model::*, scoring::Game, setup, store};
fn draft(id: &str) -> Event {
    let mut e = Event {
        game: id.into(),
        name: "Wizard Test".into(),
        ..Default::default()
    };
    e.config.competition = if id.starts_with("VIQC") {
        "VEX IQ".into()
    } else {
        "VEX V5".into()
    };
    for i in 1..=16 {
        e.teams.push(Team {
            id: i,
            number: format!("{i}A"),
            ..Default::default()
        })
    }
    e
}
#[test]
fn wizard_steps_match_the_requested_order() {
    assert_eq!(
        setup::STEPS,
        [
            "Competition type",
            "Tournament type",
            "General setup",
            "Choose game",
            "Team list",
            "Eliminations setup",
            "Field set",
            "Skills challenges",
            "Pit displays",
            "Schedule setup",
            "Create matches",
            "Awards"
        ]
    );
}
#[test]
fn wizard_builds_both_match_types_with_fields_and_times() {
    for id in ["VIQC_2025_MixAndMatch", "VRC_2025_PushBack"] {
        let mut e = draft(id);
        setup::preset(&mut e.config, 1);
        let g = Game::all().into_iter().find(|g| g.id() == id).unwrap();
        setup::create_matches(&mut e, &g).unwrap();
        assert!(e.matches.iter().any(|m| m.round == 1));
        assert!(e.matches.iter().any(|m| m.round == 2));
        let fields = e.config.fields();
        assert!(
            e.matches
                .iter()
                .all(|m| fields.contains(&m.field) && m.planned.len() == 8)
        );
        let ids = e
            .matches
            .iter()
            .map(|m| m.id)
            .collect::<std::collections::HashSet<_>>();
        assert_eq!(ids.len(), e.matches.len());
        assert!(e.teams.iter().all(|t| !t.checked_in));
    }
}
#[test]
fn wizard_validation_rejects_invalid_drafts() {
    let mut e = draft("VRC_2025_PushBack");
    e.name.clear();
    assert!(setup::validate_step(&e, 2, "", "").is_err());
    e.name = "Test".into();
    assert!(setup::validate_step(&e, 2, "one", "two").is_err());
    e.teams[1].number = e.teams[0].number.clone();
    assert!(setup::validate_step(&e, 4, "", "").is_err());
    e.config.field_sets[0].fields.push("Field 1".into());
    assert!(setup::validate_step(&e, 6, "", "").is_err());
    assert!(setup::parse_time("25:61").is_err());
}
#[test]
fn password_hash_is_salted_and_checks_credentials() {
    let a = auth::hash("test-only-password").unwrap();
    let b = auth::hash("test-only-password").unwrap();
    assert_ne!(a, b);
    assert!(!a.contains("test-only-password"));
    assert!(auth::verify("test-only-password", &a));
    assert!(!auth::verify("wrong", &a));
    assert!(auth::verify("", ""));
}
#[test]
fn wizard_configuration_round_trips_and_drives_displays() {
    let mut e = draft("VRC_2025_PushBack");
    setup::preset(&mut e.config, 2);
    setup::configure_displays(&mut e.config, 3);
    assert_eq!(
        e.config.displays.iter().filter(|d| d.kind == "Pit").count(),
        3
    );
    assert_eq!(
        e.config
            .displays
            .iter()
            .filter(|d| d.kind == "Field")
            .count(),
        2
    );
    assert_eq!(
        e.config
            .displays
            .iter()
            .filter(|d| d.kind == "Audience")
            .count(),
        1
    );
    e.config.password_hash = auth::hash("fixture-password").unwrap();
    let p = std::env::temp_dir().join(format!("tmrs-wizard-{}.tmrs", std::process::id()));
    store::save(&p, &e).unwrap();
    let got = store::load(&p).unwrap();
    assert_eq!(got.config.displays.len(), 6);
    assert!(auth::verify("fixture-password", &got.config.password_hash));
    std::fs::remove_file(p).unwrap();
}
#[test]
fn setup_changes_invalidate_schedules_and_overlaps_are_rejected() {
    let mut e = draft("VRC_2025_PushBack");
    let before = setup::schedule_fingerprint(&e);
    e.config.schedule.qualification_rounds += 1;
    assert_ne!(before, setup::schedule_fingerprint(&e));
    e.config.schedule.qualification_start = "09:00".into();
    let g = Game::all().into_iter().find(|g| g.id() == e.game).unwrap();
    assert!(setup::create_matches(&mut e, &g).is_err());
}
#[test]
fn tournament_password_applies_only_to_remote_connections() {
    let hash = auth::hash("network-only").unwrap();
    assert!(auth::remote_allowed(true, None, &hash));
    assert!(!auth::remote_allowed(false, None, &hash));
    assert!(!auth::remote_allowed(false, Some("wrong"), &hash));
    assert!(auth::remote_allowed(false, Some("network-only"), &hash));
    let mut e = draft("VRC_2025_PushBack");
    e.config.password_hash = hash;
    let p = std::env::temp_dir().join(format!("tmrs-local-{}.tmrs", std::process::id()));
    store::save(&p, &e).unwrap();
    assert!(store::load(&p).is_ok());
    std::fs::remove_file(p).unwrap();
}
#[test]
fn dated_schedule_blocks_create_matches_inside_their_windows() {
    let mut e = draft("VRC_2025_PushBack");
    e.config.schedule.blocks = vec![
        ScheduleBlock {
            date: "2026-11-20".into(),
            start: "09:00".into(),
            end: "09:30".into(),
            cycle_seconds: 180,
            round: 1,
        },
        ScheduleBlock {
            date: "2026-11-20".into(),
            start: "10:00".into(),
            end: "11:00".into(),
            cycle_seconds: 180,
            round: 2,
        },
    ];
    let g = Game::all().into_iter().find(|g| g.id() == e.game).unwrap();
    setup::create_round(&mut e, &g, 1).unwrap();
    assert_eq!(e.matches.iter().filter(|m| m.round == 1).count(), 10);
    assert!(!e.matches.iter().any(|m| m.round == 2));
    setup::create_round(&mut e, &g, 2).unwrap();
    assert_eq!(e.matches.iter().filter(|m| m.round == 1).count(), 10);
    assert_eq!(e.matches.iter().filter(|m| m.round == 2).count(), 20);
    assert!(
        e.matches
            .iter()
            .all(|m| m.planned.starts_with("2026-11-20"))
    );
    let ids = e
        .matches
        .iter()
        .map(|m| m.id)
        .collect::<std::collections::HashSet<_>>();
    assert_eq!(ids.len(), 30);
    e.config.schedule.blocks[1].start = "09:10".into();
    assert!(setup::validate_blocks(&e.config.schedule.blocks).is_err());
}
