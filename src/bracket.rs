use crate::{model::*, scoring::Game};
use anyhow::{Context, Result, bail};
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Series {
    pub id: u32,
    pub round: u32,
    pub teams: [Vec<u32>; 2],
    pub wins: [u32; 2],
    pub winner: Option<usize>,
    pub matches: Vec<u32>,
}
pub fn start(event: &mut Event, game: &Game, wins: u32) -> Result<()> {
    let n = event.alliances.len();
    if !(2..=128).contains(&n) {
        bail!("Select 2–128 alliances")
    };
    if wins == 0 || wins > 2 {
        bail!("Series must require one or two wins")
    };
    if event.alliances.iter().any(|a| a.len() != 2) {
        bail!("Elimination matches require exactly two teams per alliance (2v2)")
    };
    if !event.bracket.is_empty() {
        bail!("Eliminations already exist; unlock qualification first")
    };
    let mut seen = std::collections::HashSet::new();
    for id in event.alliances.iter().flatten() {
        if !seen.insert(*id) {
            bail!("Team selected twice")
        }
    }
    event.elimination_wins = wins;
    let mut seeds = vec![0usize, 1];
    while seeds.len() < n.next_power_of_two() {
        let size = seeds.len() * 2;
        seeds = seeds.into_iter().flat_map(|s| [s, size - 1 - s]).collect();
    }
    let round = match n.next_power_of_two() {
        2 => 5,
        4 => 4,
        8 => 3,
        16 => 6,
        32 => 7,
        64 => 8,
        _ => 9,
    };
    for pair in seeds.chunks(2) {
        new_series(
            event,
            game,
            round,
            [
                event.alliances.get(pair[0]).cloned().unwrap_or_default(),
                event.alliances.get(pair[1]).cloned().unwrap_or_default(),
            ],
        );
    }
    Ok(())
}
fn new_series(e: &mut Event, g: &Game, round: u32, teams: [Vec<u32>; 2]) {
    let id = e.bracket.len() as u32 + 1;
    let mut s = Series {
        id,
        round,
        teams,
        ..Default::default()
    };
    if s.teams[0].is_empty() {
        s.winner = Some(1)
    } else if s.teams[1].is_empty() {
        s.winner = Some(0)
    } else {
        new_match(e, g, &mut s);
    }
    e.bracket.push(s);
}
fn new_match(e: &mut Event, g: &Game, s: &mut Series) {
    let id = e.matches.iter().map(|m| m.id).max().unwrap_or(0) + 1;
    let mut m = g.new_match(
        id,
        s.round,
        &s.teams.iter().flatten().copied().collect::<Vec<_>>(),
    );
    for (i, side) in s.teams.iter().enumerate() {
        let selected = side
            .iter()
            .take(2)
            .map(|id| crate::model::Slot {
                team: *id,
                ..Default::default()
            })
            .collect();
        m.alliances[i].teams = selected;
    }
    m.number = s.matches.len() as u32 + 1;
    m.series = Some(s.id);
    e.matches.push(m);
    s.matches.push(id);
}
pub fn submit(event: &mut Event, game: &Game, index: usize) -> Result<()> {
    let m = event.matches.get(index).context("No selected match")?;
    if m.round == 2 && (!event.bracket.is_empty() || event.matches.iter().any(|m| m.round == 15)) {
        bail!("Qualification scores locked after eliminations begin")
    };
    if m.state == MatchState::Submitted && m.round != 2 {
        bail!("Elimination score already submitted")
    };
    let invalid = game.constraints(m)?;
    if !invalid.is_empty() {
        bail!("Invalid inputs: {}", invalid.join(", "))
    };
    let score = game.score(m)?;
    let scores = [
        score["alliances"][0]["score"]
            .as_f64()
            .context("Red score")?,
        score["alliances"][1]["score"]
            .as_f64()
            .context("Blue score")?,
    ];
    if m.series.is_some() && scores[0] == scores[1] {
        bail!("Tied elimination match: replay before submitting")
    };
    let series = m.series;
    event.matches[index].state = MatchState::Submitted;
    event.matches[index]
        .audit
        .push(format!("Submitted at {:?}", std::time::SystemTime::now()));
    if let Some(id) = series {
        let idx = event
            .bracket
            .iter()
            .position(|s| s.id == id)
            .context("Missing elimination series")?;
        let mut s = event.bracket[idx].clone();
        let winner = if scores[0] > scores[1] { 0 } else { 1 };
        s.wins[winner] += 1;
        if s.wins[winner] >= event.elimination_wins {
            s.winner = Some(winner)
        } else {
            new_match(event, game, &mut s);
        }
        let round = s.round;
        event.bracket[idx] = s;
        let all = event
            .bracket
            .iter()
            .filter(|s| s.round == round)
            .collect::<Vec<_>>();
        if all.iter().all(|s| s.winner.is_some()) && round != 5 {
            let next = match round {
                9 => 8,
                8 => 7,
                7 => 6,
                6 => 3,
                3 => 4,
                _ => 5,
            };
            if !event.bracket.iter().any(|s| s.round == next) {
                let winners = all
                    .iter()
                    .map(|s| s.teams[s.winner.unwrap()].clone())
                    .collect::<Vec<_>>();
                for pair in winners.chunks(2) {
                    new_series(event, game, next, [pair[0].clone(), pair[1].clone()])
                }
            }
        }
    }
    Ok(())
}
pub fn unlock(event: &mut Event) {
    event
        .matches
        .retain(|m| m.series.is_none() && m.round != 15);
    event.bracket.clear();
    event.alliances.clear();
    event.selection_started = false;
}

/// IQ pairs adjacent qualification ranks, running the lowest seeds first.
pub fn iq_finals(event: &mut Event, game: &Game, count: usize) -> Result<()> {
    if !game.cooperative() {
        bail!("IQ finals require a VEX IQ game")
    }
    if event.matches.iter().any(|m| m.round == 15) {
        bail!("Finals already exist")
    }
    let ranked = crate::scoring::rankings(event, game)?
        .into_iter()
        .filter(|r| r.played > 0)
        .take(count)
        .map(|r| r.team)
        .collect::<Vec<_>>();
    if ranked.len() < 2 || ranked.len() % 2 != 0 {
        bail!("Choose an even number of ranked teams")
    }
    for (i, pair) in ranked.chunks(2).rev().enumerate() {
        let id = event.matches.iter().map(|m| m.id).max().unwrap_or(0) + 1;
        let mut m = game.new_match(id, 15, pair);
        m.number = i as u32 + 1;
        event.matches.push(m)
    }
    Ok(())
}

/// Captains are always the highest ranked team still available.
pub fn pick(event: &mut Event, game: &Game, partner: u32) -> Result<()> {
    if !event.selection_started || !event.bracket.is_empty() { bail!("Alliance selection is not open"); }
    let limit = event.config.elimination_alliances.min(event.teams.len()/2);
    if event.alliances.len() >= limit { bail!("All alliances selected"); }
    let available: Vec<u32> = crate::scoring::rankings(event, game)?.into_iter().map(|r|r.team).filter(|id| !event.alliances.iter().flatten().any(|t|t==id)).collect();
    let captain = *available.first().context("No available captain")?;
    if partner == captain || !available.contains(&partner) { bail!("Choose an available partner"); }
    event.alliances.push(vec![captain, partner]);
    Ok(())
}
pub fn reopen(event: &mut Event, id: u32) -> Result<()> {
    let index = event.matches.iter().position(|m|m.id==id).context("Unknown match")?;
    if event.matches[index].state != MatchState::Submitted { bail!("Match is not locked"); }
    let m = event.matches[index].clone();
    if m.round == 2 && (!event.bracket.is_empty() || event.matches.iter().any(|m|m.round==15)) { bail!("Remove eliminations before changing qualification scores"); }
    if let Some(series) = m.series {
        let i = event.bracket.iter().position(|s|s.id==series).context("Missing series")?;
        let round = event.bracket[i].round;
        let same_round: Vec<u32> = event.bracket.iter().filter(|s|s.round==round).map(|s|s.id).collect();
        let first = event.bracket.iter().position(|s|s.round==round).unwrap();
        let first_id = event.bracket[first].id;
        event.bracket.retain(|s|s.id <= first_id || same_round.contains(&s.id));
        let keep: Vec<u32> = event.bracket.iter().map(|s|s.id).collect();
        event.matches.retain(|x|x.series.is_none() || keep.contains(&x.series.unwrap()));
        let g = crate::scoring::Game::all().into_iter().find(|g|g.id()==event.game).context("Unknown game")?;
        let ids = event.bracket[i].matches.clone();
        let mut wins = [0,0];
        for x in &event.matches { if x.id != id && ids.contains(&x.id) && x.state == MatchState::Submitted { let score=g.score(x)?; let a=score["alliances"][0]["score"].as_f64().unwrap_or(0.);let b=score["alliances"][1]["score"].as_f64().unwrap_or(0.);wins[if a>b{0}else{1}]+=1; } }
        event.bracket[i].wins=wins;event.bracket[i].winner=None;
    }
    let m=event.matches.iter_mut().find(|m|m.id==id).unwrap();
    m.state=MatchState::Scored;m.audit.push("Score unlocked for correction".into());
    Ok(())
}

/// Undo the latest partner pick before bracket generation.
pub fn undo_selection(event: &mut Event) -> Result<()> {
    if !event.selection_started || !event.bracket.is_empty() { bail!("Alliance selection is not open"); }
    event.alliances.pop().context("No alliance selection to undo")?;
    Ok(())
}
