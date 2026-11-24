use std::time::Duration;
use tournament_manager_rs::{
    model::*,
    schedule::{self, Phase, Timer},
    scoring::{Game, rankings},
    store,
};
fn event() -> Event {
    let mut e = Event::default();
    e.game = "VRC_2025_PushBack".into();
    for i in 1..=10 {
        e.teams.push(Team {
            id: i,
            number: format!("{i}A"),
            checked_in: true,
            ..Default::default()
        })
    }
    e
}
#[test]
fn schedule_has_no_duplicate_teams_and_equal_counted_appearances() {
    let e = event();
    let g = Game::all().into_iter().find(|g| g.id() == e.game).unwrap();
    let ms = schedule::generate(&e, &g, 5, 42).unwrap();
    let mut appearances = std::collections::BTreeMap::<u32, u32>::new();
    for m in ms {
        let ids = m
            .alliances
            .iter()
            .flat_map(|a| &a.teams)
            .map(|t| t.team)
            .collect::<std::collections::HashSet<_>>();
        assert_eq!(ids.len(), 4);
        for t in m
            .alliances
            .iter()
            .flat_map(|a| &a.teams)
            .filter(|t| !t.surrogate)
        {
            *appearances.entry(t.team).or_default() += 1;
        }
    }
    assert_eq!(appearances.len(), 10);
    assert!(appearances.values().all(|x| *x == 5));
}
#[test]
fn timer_pauses_and_requires_driver_resume() {
    let mut t = Timer::new(15, 105);
    t.start();
    t.advance(Duration::from_secs(5));
    t.pause();
    t.advance(Duration::from_secs(20));
    assert_eq!(t.remaining.as_secs(), 10);
    t.resume();
    t.advance(Duration::from_secs(10));
    assert_eq!(t.phase, Phase::AutoPause);
    t.advance(Duration::from_secs(500));
    assert_eq!(t.phase, Phase::AutoPause);
    t.resume();
    assert_eq!(t.remaining.as_secs(), 105);
    t.advance(Duration::from_secs(105));
    assert_eq!(t.phase, Phase::Finished);
}
#[test]
fn save_load_and_original_database_protection() {
    let mut p = std::env::temp_dir();
    p.push(format!("tmrs-test-{}.tmrs", std::process::id()));
    let e = event();
    store::save(&p, &e).unwrap();
    assert_eq!(store::load(&p).unwrap().teams.len(), 10);
    store::save(&p, &e).unwrap();
    std::fs::remove_file(&p).unwrap();
    let c = rusqlite::Connection::open(&p).unwrap();
    c.execute("CREATE TABLE teams(id INTEGER)", []).unwrap();
    drop(c);
    assert!(store::save(&p, &e).is_err());
    std::fs::remove_file(p).unwrap();
}
#[test]
fn unsubmitted_matches_do_not_affect_rankings() {
    let mut e = event();
    let g = Game::all().into_iter().find(|g| g.id() == e.game).unwrap();
    e.matches.push(g.new_match(1, 2, &[1, 2, 3, 4]));
    assert!(rankings(&e, &g).unwrap().iter().all(|r| r.played == 0));
    e.matches[0].state = MatchState::Submitted;
    assert_eq!(
        rankings(&e, &g)
            .unwrap()
            .iter()
            .filter(|r| r.played == 1)
            .count(),
        4
    );
}
#[test]
fn elimination_series_advance_and_lock_qualification() {
    let mut e = event();
    let g = Game::all().into_iter().find(|g| g.id() == e.game).unwrap();
    e.matches.push(g.new_match(1, 2, &[1, 2, 3, 4]));
    e.alliances = vec![vec![1, 2], vec![3, 4], vec![5, 6], vec![7, 8]];
    tournament_manager_rs::bracket::start(&mut e, &g, 2).unwrap();
    assert_eq!(e.bracket.len(), 2);
    assert!(tournament_manager_rs::bracket::submit(&mut e, &g, 0).is_err());
    let first = e.matches.iter().position(|m| m.series == Some(1)).unwrap();
    assert!(tournament_manager_rs::bracket::submit(&mut e, &g, first).is_err());
    while e
        .bracket
        .iter()
        .find(|s| s.round == 5)
        .and_then(|s| s.winner)
        .is_none()
    {
        let idx = e
            .matches
            .iter()
            .position(|m| m.series.is_some() && m.state != MatchState::Submitted)
            .unwrap();
        e.matches[idx]
            .values
            .insert("red_blocks_long_goal_1".into(), 1);
        tournament_manager_rs::bracket::submit(&mut e, &g, idx).unwrap();
    }
    assert_eq!(e.bracket.len(), 3);
    assert_eq!(e.matches.len(), 7);
    tournament_manager_rs::bracket::unlock(&mut e);
    assert!(e.bracket.is_empty());
    assert_eq!(e.matches.len(), 1);
}
#[test]
fn iq_finals_pair_adjacent_ranks_and_lock_qualifications() {
    let mut e = event();
    e.game = "VIQC_2025_MixAndMatch".into();
    let g = Game::all().into_iter().find(|g| g.id() == e.game).unwrap();
    e.matches = schedule::generate(&e, &g, 1, 42).unwrap();
    for m in &mut e.matches {
        m.state = MatchState::Submitted;
    }
    let ranked = rankings(&e, &g).unwrap();
    tournament_manager_rs::bracket::iq_finals(&mut e, &g, 10).unwrap();
    let fs = e
        .matches
        .iter()
        .filter(|m| m.round == 15)
        .collect::<Vec<_>>();
    assert_eq!(fs.len(), 5);
    assert_eq!(fs[0].alliances[0].teams[0].team, ranked[8].team);
    assert_eq!(fs[4].alliances[0].teams[0].team, ranked[0].team);
    assert!(tournament_manager_rs::bracket::submit(&mut e, &g, 0).is_err());
}
#[test]
fn editing_scores_and_dq_updates_display_results() {
    let g = Game::all()
        .into_iter()
        .find(|g| g.id() == "VRC_2025_PushBack")
        .unwrap();
    let mut m = g.new_match(1, 2, &[1, 2, 3, 4]);
    assert_eq!(
        g.score(&m).unwrap()["alliances"][0]["score"],
        serde_json::json!(0.)
    );
    m.values.insert("red_blocks_long_goal_1".into(), 2);
    assert_eq!(
        g.score(&m).unwrap()["alliances"][0]["score"],
        serde_json::json!(6.)
    );
    m.round = 3;
    m.alliances[0].teams[0].dq = true;
    assert_eq!(
        g.score(&m).unwrap()["alliances"][0]["score"],
        serde_json::json!(0.)
    );
}
#[test]
fn non_power_of_two_alliances_advance_through_byes() {
    let mut e = event();
    e.alliances = vec![vec![1, 2], vec![3, 4], vec![5, 6]];
    let g = Game::all().into_iter().find(|g| g.id() == e.game).unwrap();
    tournament_manager_rs::bracket::start(&mut e, &g, 1).unwrap();
    assert!(
        e.bracket
            .iter()
            .any(|s| s.winner.is_some() && s.matches.is_empty())
    );
    while e
        .bracket
        .iter()
        .find(|s| s.round == 5)
        .and_then(|s| s.winner)
        .is_none()
    {
        let i = e
            .matches
            .iter()
            .position(|m| m.state != MatchState::Submitted)
            .unwrap();
        e.matches[i]
            .values
            .insert("red_blocks_long_goal_1".into(), 1);
        tournament_manager_rs::bracket::submit(&mut e, &g, i).unwrap();
    }
    assert_eq!(e.matches.len(), 2);
}
#[test]
fn twelve_alliance_bracket_enforces_strictly_two_team_rosters() {
    let mut e = event();
    e.teams = (1..=24)
        .map(|id| Team {
            id,
            number: id.to_string(),
            ..Default::default()
        })
        .collect();
    e.alliances = (0..12).map(|i| vec![i * 2 + 1, i * 2 + 2]).collect();
    let g = Game::all().into_iter().find(|g| g.id() == e.game).unwrap();
    tournament_manager_rs::bracket::start(&mut e, &g, 1).unwrap();
    while e
        .bracket
        .iter()
        .find(|s| s.round == 5)
        .and_then(|s| s.winner)
        .is_none()
    {
        let i = e
            .matches
            .iter()
            .position(|m| m.state != MatchState::Submitted)
            .unwrap();
        e.matches[i]
            .values
            .insert("red_blocks_long_goal_1".into(), 1);
        tournament_manager_rs::bracket::submit(&mut e, &g, i).unwrap();
    }
    assert_eq!(e.matches.len(), 11);
    let mut e = event();
    e.config.elimination_team_count = 3;
    e.alliances = vec![vec![1, 2, 3], vec![4, 5, 6]];
    assert!(tournament_manager_rs::bracket::start(&mut e, &g, 2).is_err());
}

#[test]
fn unlocked_score_keeps_values_and_persists_as_draft() {
    let mut e = event();
    let g = Game::all().into_iter().find(|g| g.id() == e.game).unwrap();
    e.matches = schedule::generate(&e, &g, 1, 42).unwrap();
    let id = e.matches[0].id;
    e.matches[0].state = MatchState::Submitted;
    let values = e.matches[0].alliances.clone();
    tournament_manager_rs::bracket::reopen(&mut e, id).unwrap();
    assert_eq!(e.matches[0].state, MatchState::Scored);
    assert_eq!(serde_json::to_value(&values).unwrap(), serde_json::to_value(&e.matches[0].alliances).unwrap());
    let path = std::env::temp_dir().join(format!("tmrs-unlocked-{}.tmrs", std::process::id()));
    store::save(&path, &e).unwrap();
    let loaded = store::load(&path).unwrap();
    assert_eq!(loaded.matches[0].state, MatchState::Scored);
    assert!(loaded.matches[0].audit.iter().any(|a|a.contains("unlocked")));
    std::fs::remove_file(path).unwrap();
}

#[test]
fn alliance_captains_skip_teams_already_picked() {
    let mut e = event();
    let g = Game::all().into_iter().find(|g|g.id()==e.game).unwrap();
    let ranks = rankings(&e, &g).unwrap();
    e.selection_started = true;
    tournament_manager_rs::bracket::pick(&mut e, &g, ranks[1].team).unwrap();
    assert_eq!(e.alliances[0], vec![ranks[0].team, ranks[1].team]);
    tournament_manager_rs::bracket::pick(&mut e, &g, ranks[4].team).unwrap();
    assert_eq!(e.alliances[1], vec![ranks[2].team, ranks[4].team]);
    assert!(tournament_manager_rs::bracket::pick(&mut e, &g, ranks[0].team).is_err());
}

#[test]
fn undo_alliance_selection_restores_captain_and_partner() {
    let mut e = event();
    let g = Game::all().into_iter().find(|g|g.id()==e.game).unwrap();
    let ranks = rankings(&e, &g).unwrap();
    e.selection_started = true;
    tournament_manager_rs::bracket::pick(&mut e, &g, ranks[1].team).unwrap();
    tournament_manager_rs::bracket::undo_selection(&mut e).unwrap();
    assert!(e.alliances.is_empty());
    tournament_manager_rs::bracket::pick(&mut e, &g, ranks[2].team).unwrap();
    assert_eq!(e.alliances[0],vec![ranks[0].team,ranks[2].team]);
    tournament_manager_rs::bracket::start(&mut e, &g, 1).unwrap_err();
    tournament_manager_rs::bracket::undo_selection(&mut e).unwrap();
    assert!(tournament_manager_rs::bracket::undo_selection(&mut e).is_err());
}

#[test]
fn new_tournament_file_exists_before_setup_and_never_overwrites() {
    let path=std::env::temp_dir().join(format!("tmrs-before-setup-{}.tmrs",std::process::id()));
    let mut e=Event::default();
    store::create(&path,&e).unwrap();
    assert!(!store::load(&path).unwrap().config.configured);
    let before=std::fs::read(&path).unwrap();
    assert!(store::create(&path,&e).is_err());assert_eq!(std::fs::read(&path).unwrap(),before);
    e.name="Setup completed".into();e.config.configured=true;store::save(&path,&e).unwrap();assert_eq!(store::load(&path).unwrap().name,"Setup completed");
    std::fs::remove_file(path).unwrap();
}

#[test]
fn score_preview_setting_defaults_for_old_files_and_persists() {
    let mut encoded=serde_json::to_value(Event::default()).unwrap();encoded["config"].as_object_mut().unwrap().remove("show_score_preview");let mut e:Event=serde_json::from_value(encoded).unwrap();assert!(e.config.show_score_preview);e.config.show_score_preview=false;
    let path=std::env::temp_dir().join(format!("tmrs-preview-{}.tmrs",std::process::id()));store::save(&path,&e).unwrap();assert!(!store::load(&path).unwrap().config.show_score_preview);std::fs::remove_file(path).unwrap();
}
