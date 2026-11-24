//! Schedule diagnostics shared by the Tauri wizard and native commands.
use crate::{model::*, scoring::Game, setup};
use anyhow::{Result, bail};
use serde::Serialize;
use std::collections::BTreeMap;
#[derive(Serialize)]
pub struct BlockEstimate {
    pub index: usize,
    pub capacity: usize,
    pub appearances_per_team: f64,
    pub minutes: u64,
}
#[derive(Serialize)]
pub struct Plan {
    pub blocks: Vec<BlockEstimate>,
    pub practice: usize,
    pub qualifications: usize,
    pub warnings: Vec<String>,
    pub appearances: BTreeMap<u32, [usize; 2]>,
}
pub fn estimate(e: &Event, g: &Game) -> Result<Plan> {
    setup::validate_blocks(&e.config.schedule.blocks)?;
    let fields = e.config.fields().len();
    if fields == 0 {
        bail!("Add at least one field before scheduling")
    }
    let mut p = Plan {
        blocks: vec![],
        practice: 0,
        qualifications: 0,
        warnings: vec![],
        appearances: BTreeMap::new(),
    };
    for (index, b) in e.config.schedule.blocks.iter().enumerate() {
        let duration = setup::parse_time(&b.end)? - setup::parse_time(&b.start)?;
        let capacity = (duration / b.cycle_seconds) as usize * fields;
        if capacity == 0 {
            bail!("Block {} is too short for one match cycle", index + 1)
        }
        if b.round == 1 {
            p.practice += capacity
        } else {
            p.qualifications += capacity
        }
        p.blocks.push(BlockEstimate {
            index,
            capacity,
            appearances_per_team: (capacity * g.team_size() * 2) as f64
                / e.teams.len().max(1) as f64,
            minutes: duration / 60,
        });
        let match_seconds: u64 = g.data["defaultMatchTimes"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|x| x["seconds"].as_u64())
            .sum();
        if b.cycle_seconds < match_seconds + 30 {
            p.warnings.push(format!(
                "Block {} leaves less than 30 seconds for field reset after a match.",
                index + 1
            ));
        }
    }
    if fields > 1 {
        p.warnings.push(format!("Capacity assumes {fields} fields run simultaneously. Use one field if your field set alternates matches."));
    }
    if p.practice == 0 {
        p.warnings
            .push("No practice block is scheduled. Practice matches are optional.".into());
    }
    Ok(p)
}
pub fn preview(e: &Event, g: &Game) -> Result<Plan> {
    let mut p = estimate(e, g)?;
    let mut draft = e.clone();
    setup::create_matches(&mut draft, g)?;
    for t in &e.teams {
        p.appearances.insert(t.id, [0, 0]);
    }
    for m in &draft.matches {
        for s in m.alliances.iter().flat_map(|a| &a.teams) {
            if !s.surrogate {
                if let Some(v) = p.appearances.get_mut(&s.team) {
                    v[(m.round - 1) as usize] += 1;
                }
            }
        }
    }
    for (i, label) in ["practice", "qualification"].iter().enumerate() {
        let low = p.appearances.values().map(|a| a[i]).min().unwrap_or(0);
        let high = p.appearances.values().map(|a| a[i]).max().unwrap_or(0);
        if low != high {
            p.warnings.push(format!("{label}: teams receive {low}–{high} counted matches. Adjust block duration for equal appearances."));
        }
    }
    Ok(p)
}
