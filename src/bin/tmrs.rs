use anyhow::{Context, Result, bail};
use std::path::Path;
use tournament_manager_rs::{
    model::*,
    schedule,
    scoring::{Game, rankings},
    store,
};
fn main() -> Result<()> {
    let args = std::env::args().collect::<Vec<_>>();
    match args.get(1).map(String::as_str) {
        Some("games") => {
            for g in Game::all() {
                println!("{}\t{}", g.id(), g.title())
            }
        }
        Some("demo") => {
            let path = args.get(2).context("Usage: tmrs demo output.tmrs")?;
            if Path::new(path).exists() {
                bail!("Destination already exists")
            };
            let mut e = Event {
                name: "Rust Robotics Invitational".into(),
                game: args
                    .get(3)
                    .cloned()
                    .unwrap_or_else(|| "VRC_2025_PushBack".into()),
                venue: "Demonstration event".into(),
                ..Default::default()
            };
            for i in 1..=16 {
                e.teams.push(Team {
                    id: i,
                    number: format!("{i:03}A"),
                    name: format!("Robotics Team {i}"),
                    organization: "Demo School".into(),
                    city: "Shanghai".into(),
                    checked_in: true,
                    inspected: i % 3 != 0,
                    ..Default::default()
                });
            }
            let g = Game::all()
                .into_iter()
                .find(|g| g.id() == e.game)
                .context("Unsupported game")?;
            e.matches = schedule::generate(&e, &g, 4, 202526)?;
            for (i, m) in e.matches.iter_mut().take(4).enumerate() {
                if g.cooperative() {
                    m.values.insert("connected_pins".into(), 10 + i as i64);
                    m.values.insert("color_2_stacks".into(), 2);
                } else {
                    m.values
                        .insert("red_blocks_long_goal_1".into(), (i + 1) as i64 * 2);
                    m.values
                        .insert("blue_blocks_long_goal_2".into(), i as i64 + 1);
                }
                m.state = MatchState::Submitted;
            }
            e.announcements = "Welcome to the Rust Robotics Invitational".into();
            store::save(Path::new(path), &e)?;
            println!(
                "Created {} teams and {} matches",
                e.teams.len(),
                e.matches.len()
            );
        }
        Some("inspect") => {
            let e = store::load(Path::new(
                args.get(2).context("Usage: tmrs inspect file.tmrs")?,
            ))?;
            let g = Game::all()
                .into_iter()
                .find(|g| g.id() == e.game)
                .context("Unknown game")?;
            println!(
                "{}: {} teams, {} matches, {} skills attempts",
                e.name,
                e.teams.len(),
                e.matches.len(),
                e.skills.len()
            );
            for (i, r) in rankings(&e, &g)?.iter().enumerate() {
                println!(
                    "{}\t{}\tplayed={}\twp={}",
                    i + 1,
                    e.team_number(r.team),
                    r.played,
                    r.wp
                );
            }
        }
        Some("score") => {
            let file = args
                .get(2)
                .context("Usage: tmrs score fixture-request.json")?;
            let v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(file)?)?;
            let g = Game::all()
                .into_iter()
                .find(|g| Some(g.id()) == v["game"].as_str())
                .context("Unknown game")?;
            let mut vm = tournament_manager_rs::rules::Machine::new();
            vm.load(&g.data["rules"])?;
            let input = tournament_manager_rs::rules::V::from_json(&v["input"]);
            println!("{}", vm.call("calc_match_score", vec![input])?.json());
        }
        _ => println!("tmrs games | demo output.tmrs | inspect file.tmrs | score request.json"),
    };
    Ok(())
}
