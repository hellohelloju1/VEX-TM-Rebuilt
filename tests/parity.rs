use serde_json::{Value as J, json};
use tournament_manager_rs::{
    rules::{Machine, V},
    scoring::Game,
};
fn normalize(v: J) -> J {
    match v {
        J::Bool(b) => json!(b as u8),
        J::Array(a) => J::Array(a.into_iter().map(normalize).collect()),
        J::Object(o) => J::Object(o.into_iter().map(|(k, v)| (k, normalize(v))).collect()),
        J::Number(n) => json!(n.as_f64().unwrap()),
        x => x,
    }
}
#[test]
fn recovered_game_scoring_parity() {
    let cases: Vec<J> = serde_json::from_str(include_str!("scoring-fixtures.json")).unwrap();
    let games = Game::all();
    let mut failures = vec![];
    for (i, c) in cases.iter().enumerate() {
        let g = games
            .iter()
            .find(|g| g.id() == c["game"].as_str().unwrap())
            .unwrap();
        let mut vm = Machine::new();
        vm.load(&g.data["rules"]).unwrap();
        let mut args = vec![];
        if !c["skill"].is_null() {
            args.push(V::from_json(&c["skill"]));
        }
        args.push(V::from_json(&c["input"]));
        let result = vm.call(c["name"].as_str().unwrap(), args);
        match result {
            Err(e) => failures.push(format!("{i} {} {}: {e:#}", c["game"], c["name"])),
            Ok(v) => {
                let mut actual = normalize(v.json());
                let mut expected = normalize(c["output"].clone());
                if c["name"].as_str().unwrap().contains("constraints") {
                    for a in [&mut actual, &mut expected] {
                        if let Some(xs) = a.as_array_mut() {
                            xs.sort_by_key(|x| x.to_string());
                        }
                    }
                }
                if actual != expected {
                    failures.push(format!(
                        "{i} {} {} mismatch: actual {actual} expected {expected}",
                        c["game"], c["name"]
                    ))
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} / {} failures:\n{}",
        failures.len(),
        cases.len(),
        failures
            .iter()
            .take(35)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}
#[test]
fn all_games_default_match_scores_and_constraints() {
    for g in Game::all() {
        let teams = (1..=(g.team_size() * 2) as u32).collect::<Vec<_>>();
        let m = g.new_match(1, 2, &teams);
        g.score(&m).unwrap_or_else(|e| panic!("{} {e:#}", g.id()));
        g.constraints(&m)
            .unwrap_or_else(|e| panic!("{} constraints {e:#}", g.id()));
    }
}
#[test]
fn current_games_and_saved_event_compatibility_are_included() {
    let ids = Game::all()
        .iter()
        .map(|g| g.id().to_owned())
        .collect::<Vec<_>>();
    assert_eq!(ids, vec!["VIQC_2025_MixAndMatch", "VRC_2025_PushBack", "VIQC_2026_LevelUp", "VRC_2026_Override"]);
}
