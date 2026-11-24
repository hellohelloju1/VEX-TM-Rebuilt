use crate::{model::*, schedule, scoring::Game};
use anyhow::{Context, Result, bail};
pub const STEPS: [&str; 12] = [
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
    "Awards",
];
pub const PRESETS: [&str; 4] = [
    "School scrimmage",
    "Small tournament",
    "Medium tournament",
    "Large event / Expert",
];
pub const AWARDS: [&str; 12] = [
    "Excellence Award",
    "Tournament Champions",
    "Tournament Finalists",
    "Teamwork Champions",
    "Teamwork Finalists",
    "Robot Skills Champion",
    "Design Award",
    "Innovate Award",
    "Think Award",
    "Amaze Award",
    "Judges' Award",
    "Sportsmanship Award",
];
pub fn preset(c: &mut EventConfig, index: usize) {
    c.tournament_type = PRESETS[index.min(3)].into();
    c.schedule.practice_rounds = if index >= 2 { 2 } else { 1 };
    c.schedule.qualification_rounds = [3, 4, 6, 8][index.min(3)];
    c.elimination_alliances = if index == 0 {
        4
    } else if index == 3 {
        16
    } else {
        8
    };
    c.field_sets = vec![FieldSet {
        name: "Field Set 1".into(),
        fields: (1..=[1, 1, 2, 4][index.min(3)])
            .map(|n| format!("Field {n}"))
            .collect(),
    }];
    configure_displays(c, [1, 1, 2, 3][index.min(3)]);
}
pub fn configure_displays(c: &mut EventConfig, pits: usize) {
    let mut displays = vec![];
    let mut id = 1;
    for n in 1..=pits {
        displays.push(DisplayConfig {
            screen: "Rankings".into(),
            id,
            name: format!("Pit Display {n}"),
            kind: "Pit".into(),
            field: String::new(),
            fullscreen: false,
        });
        id += 1
    }
    for field in c.fields() {
        displays.push(DisplayConfig {
            screen: "Rankings".into(),
            id,
            name: format!("{field} Display"),
            kind: "Field".into(),
            field,
            fullscreen: false,
        });
        id += 1
    }
    displays.push(DisplayConfig {
        screen: "Rankings".into(),
        id,
        name: "Audience Display".into(),
        kind: "Audience".into(),
        field: String::new(),
        fullscreen: false,
    });
    c.displays = displays;
}
pub fn parse_time(s: &str) -> Result<u64> {
    let (h, m) = s.split_once(':').context("Use HH:MM time")?;
    let h: u64 = h.parse().context("Invalid hour")?;
    let m: u64 = m.parse().context("Invalid minute")?;
    if h > 23 || m > 59 {
        bail!("Time must be between 00:00 and 23:59")
    };
    Ok(h * 3600 + m * 60)
}
pub fn create_matches(e: &mut Event, g: &Game) -> Result<()> {
    if !e.config.schedule.blocks.is_empty() {
        return create_block_matches(e, g);
    }
    let c = &e.config;
    let fields = c.fields();
    if fields.is_empty() {
        bail!("Add at least one field")
    };
    if c.schedule.qualification_rounds == 0 {
        bail!("Create at least one qualification round")
    };
    if c.schedule.cycle_seconds < 30 {
        bail!("Match cycle time must be at least 30 seconds")
    };
    let practice = parse_time(&c.schedule.practice_start)?;
    let qual = parse_time(&c.schedule.qualification_start)?;
    let mut roster = e.clone();
    for t in &mut roster.teams {
        t.checked_in = true
    }
    let mut matches = vec![];
    for (round, count, start) in [
        (1, c.schedule.practice_rounds, practice),
        (2, c.schedule.qualification_rounds, qual),
    ] {
        if count == 0 {
            continue;
        }
        let generated = schedule::generate(&roster, g, count, c.schedule.seed + round as u64)?;
        for (i, mut m) in generated.into_iter().enumerate() {
            m.id = matches.len() as u32 + 1;
            m.number = i as u32 + 1;
            m.round = round;
            m.field = fields[i % fields.len()].clone();
            let seconds = start + (i / fields.len()) as u64 * c.schedule.cycle_seconds;
            m.planned = format!(
                "{:02}:{:02}:{:02}",
                seconds / 3600,
                seconds / 60 % 60,
                seconds % 60
            );
            matches.push(m)
        }
    }
    let practice_count = matches.iter().filter(|m| m.round == 1).count();
    if practice_count > 0
        && qual
            < practice + (practice_count.div_ceil(fields.len()) as u64) * c.schedule.cycle_seconds
    {
        bail!(
            "Qualification start overlaps the practice schedule; choose a later qualification start"
        )
    }
    e.matches = matches;
    e.bracket.clear();
    Ok(())
}
pub fn validate_step(e: &Event, step: usize, password: &str, confirmation: &str) -> Result<()> {
    match step + 1 {
        0 | 3 => {
            if e.name.trim().is_empty() {
                bail!("Enter an event name")
            };
            if step == 2 && password.trim().is_empty() {
                bail!("Enter a tournament password for connections from other computers")
            };
            if step == 2 && password != confirmation {
                bail!("The passwords do not match")
            }
        }
        5 => {
            let g = Game::all()
                .into_iter()
                .find(|g| g.id() == e.game)
                .context("Choose a game")?;
            if e.teams.len() < g.team_size() * 2 {
                bail!("Add at least {} teams", g.team_size() * 2)
            }
            let mut set = std::collections::HashSet::new();
            for t in &e.teams {
                if t.number.trim().is_empty() || !set.insert(t.number.to_uppercase()) {
                    bail!("Team numbers must be nonempty and unique")
                }
            }
        }
        6 => {
            if e.config.competition == "VEX IQ" {
                if e.config.iq_finalists < 2
                    || e.config.iq_finalists % 2 != 0
                    || e.config.iq_finalists > e.teams.len()
                {
                    bail!("Choose an even finalist count no larger than the team list")
                }
            } else {
                let n = e.config.elimination_alliances;
                if n < 2 || n > 32 || n * 2 > e.teams.len() {
                    bail!("Choose 2–32 alliances with enough available teams")
                }
            }
        }
        7 => {
            let fields = e.config.fields();
            if fields.is_empty() || fields.iter().any(|f| f.trim().is_empty()) {
                bail!("Name at least one field")
            };
            let set = fields.iter().collect::<std::collections::HashSet<_>>();
            if set.len() != fields.len() {
                bail!("Field names must be unique")
            }
        }
        10 => {
            if !e.config.schedule.blocks.is_empty() {
                validate_blocks(&e.config.schedule.blocks)?;
                return Ok(());
            }
            parse_time(&e.config.schedule.practice_start)?;
            parse_time(&e.config.schedule.qualification_start)?;
        }
        11 => {
            if !e.matches.iter().any(|m| m.round == 2) {
                bail!("Create practice and qualification matches before continuing")
            }
        }
        _ => (),
    }
    Ok(())
}

pub fn schedule_fingerprint(e: &Event) -> String {
    serde_json::to_string(&(
        &e.game,
        e.teams
            .iter()
            .map(|t| (t.id, &t.number))
            .collect::<Vec<_>>(),
        &e.config.field_sets,
        &e.config.schedule,
    ))
    .expect("setup data")
}

pub fn validate_blocks(blocks: &[ScheduleBlock]) -> Result<()> {
    if !blocks.iter().any(|b| b.round == 2) {
        bail!("Add a qualification schedule block")
    };
    for (i, b) in blocks.iter().enumerate() {
        if ![1, 2].contains(&b.round) {
            bail!("Choose Practice or Qualification for every block")
        }
        validate_date(&b.date)?;
        if (parse_time(&b.end)?).saturating_sub(parse_time(&b.start)?) < b.cycle_seconds {
            bail!("Each schedule block must fit at least one match cycle")
        }
        if b.cycle_seconds < 30 || parse_time(&b.end)? <= parse_time(&b.start)? {
            bail!("Schedule blocks need an end after the start and a cycle of at least 30 seconds")
        };
        for old in &blocks[..i] {
            if old.date == b.date
                && parse_time(&b.start)? < parse_time(&old.end)?
                && parse_time(&old.start)? < parse_time(&b.end)?
            {
                bail!("Schedule blocks overlap")
            }
        }
    }
    Ok(())
}
fn create_block_matches(e: &mut Event, g: &Game) -> Result<()> {
    validate_blocks(&e.config.schedule.blocks)?;
    let fields = e.config.fields();
    if fields.is_empty() {
        bail!("Configure a field")
    };
    let mut roster = e.clone();
    for t in &mut roster.teams {
        t.checked_in = true
    }
    let mut blocks = e.config.schedule.blocks.clone();
    blocks.sort_by_key(|b| (b.date.clone(), b.start.clone()));
    let mut matches = vec![];
    let mut numbers = [0u32; 3];
    for (i, b) in blocks.iter().enumerate() {
        if ![1, 2].contains(&b.round) {
            bail!("Choose Practice or Qualification for every block")
        }
        validate_date(&b.date)?;
        if (parse_time(&b.end)?).saturating_sub(parse_time(&b.start)?) < b.cycle_seconds {
            bail!("Each schedule block must fit at least one match cycle")
        }
        let start = parse_time(&b.start)?;
        let capacity = ((parse_time(&b.end)? - start) / b.cycle_seconds) as usize * fields.len();
        if capacity == 0 {
            continue;
        }
        let rounds = (capacity * g.team_size() * 2)
            .div_ceil(e.teams.len().max(1))
            .clamp(1, 100);
        let generated = schedule::generate(&roster, g, rounds, e.config.schedule.seed + i as u64)?;
        for (j, mut m) in generated.into_iter().take(capacity).enumerate() {
            m.id = matches.len() as u32 + 1;
            m.round = b.round;
            numbers[b.round as usize] += 1;
            m.number = numbers[b.round as usize];
            m.field = fields[j % fields.len()].clone();
            let seconds = start + (j / fields.len()) as u64 * b.cycle_seconds;
            m.planned = format!(
                "{} {:02}:{:02}:{:02}",
                b.date,
                seconds / 3600,
                seconds / 60 % 60,
                seconds % 60
            );
            matches.push(m)
        }
    }
    if !matches.iter().any(|m| m.round == 2) {
        bail!("No qualification matches fit the selected blocks")
    };
    e.matches = matches;
    e.bracket.clear();
    Ok(())
}
pub fn create_round(e: &mut Event, g: &Game, round: u32) -> Result<()> {
    if ![1, 2].contains(&round) {
        bail!("Choose practice or qualifications")
    }
    if !e.bracket.is_empty() {
        bail!("Unlock eliminations before regenerating the schedule")
    }
    let mut draft = e.clone();
    create_matches(&mut draft, g)?;
    let mut retained = e
        .matches
        .iter()
        .filter(|m| m.round != round)
        .cloned()
        .collect::<Vec<_>>();
    let mut id = retained.iter().map(|m| m.id).max().unwrap_or(0) + 1;
    for mut m in draft.matches.into_iter().filter(|m| m.round == round) {
        m.id = id;
        id += 1;
        retained.push(m)
    }
    retained.sort_by_key(|m| (m.round, m.number));
    e.matches = retained;
    Ok(())
}

pub fn validate_date(date: &str) -> Result<()> {
    let parts = date.split('-').collect::<Vec<_>>();
    if parts.len() != 3 || parts[0].len() != 4 || parts[1].len() != 2 || parts[2].len() != 2 {
        bail!("Use a YYYY-MM-DD event date")
    }
    let y: u32 = parts[0].parse()?;
    let m: u32 = parts[1].parse()?;
    let d: u32 = parts[2].parse()?;
    let leap = y % 4 == 0 && (y % 100 != 0 || y % 400 == 0);
    let days = match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if leap {
                29
            } else {
                28
            }
        }
        _ => 0,
    };
    if y < 2000 || d == 0 || d > days {
        bail!("Choose a valid calendar date")
    };
    Ok(())
}
