use crate::{model::*, scoring::Game};
use anyhow::{Result, bail};
use rand::{SeedableRng, rngs::StdRng, seq::SliceRandom};
use std::{
    collections::{HashMap, HashSet},
    time::Duration,
};
pub fn generate(event: &Event, game: &Game, rounds: usize, seed: u64) -> Result<Vec<Match>> {
    let slots = game.team_size() * 2;
    let ids = event
        .teams
        .iter()
        .filter(|t| t.checked_in)
        .map(|t| t.id)
        .collect::<Vec<_>>();
    if ids.len() < slots {
        bail!("Check in at least {slots} teams")
    };
    if rounds == 0 || rounds > 100 {
        bail!("Choose 1–100 rounds")
    };
    let mut rng = StdRng::seed_from_u64(seed);
    let mut partner = HashMap::<(u32, u32), u32>::new();
    let mut opponent = partner.clone();
    let mut last = HashSet::new();
    let mut out = vec![];
    let mut appearances = HashMap::<u32, u32>::new();
    for _ in 0..rounds {
        let mut pending = ids.clone();
        pending.shuffle(&mut rng);
        while !pending.is_empty() {
            let take = pending.len().min(slots);
            let mut best = vec![];
            let mut best_cost = u64::MAX;
            for _ in 0..160 {
                let mut options = pending.clone();
                options.shuffle(&mut rng);
                let mut chosen = options[..take].to_vec();
                if chosen.len() < slots {
                    let mut extras = ids
                        .iter()
                        .filter(|x| !chosen.contains(x))
                        .copied()
                        .collect::<Vec<_>>();
                    extras.shuffle(&mut rng);
                    extras.sort_by_key(|id| appearances.get(id).copied().unwrap_or(0));
                    chosen.extend(extras.into_iter().take(slots - take));
                }
                let mut cost = chosen.iter().filter(|id| last.contains(*id)).count() as u64 * 1000;
                for i in 0..slots {
                    for j in i + 1..slots {
                        let p = if chosen[i] < chosen[j] {
                            (chosen[i], chosen[j])
                        } else {
                            (chosen[j], chosen[i])
                        };
                        cost += if i / game.team_size() == j / game.team_size() {
                            *partner.get(&p).unwrap_or(&0) as u64 * 50
                        } else {
                            *opponent.get(&p).unwrap_or(&0) as u64 * 8
                        };
                    }
                }
                if cost < best_cost {
                    best_cost = cost;
                    best = chosen;
                }
            }
            let primary = best[..take].iter().copied().collect::<HashSet<_>>();
            pending.retain(|x| !primary.contains(x));
            let mut m = game.new_match(out.len() as u32 + 1, 2, &best);
            for (aidx, a) in m.alliances.iter_mut().enumerate() {
                for (tidx, t) in a.teams.iter_mut().enumerate() {
                    t.surrogate = aidx * game.team_size() + tidx >= take;
                    *appearances.entry(t.team).or_default() += 1;
                }
            }
            for i in 0..slots {
                for j in i + 1..slots {
                    let p = if best[i] < best[j] {
                        (best[i], best[j])
                    } else {
                        (best[j], best[i])
                    };
                    let map = if i / game.team_size() == j / game.team_size() {
                        &mut partner
                    } else {
                        &mut opponent
                    };
                    *map.entry(p).or_default() += 1;
                }
            }
            last = best.into_iter().collect();
            out.push(m);
        }
    }
    Ok(out)
}
#[derive(Clone, Debug, PartialEq)]
pub enum Phase {
    Idle,
    Autonomous,
    AutoPause,
    Driver,
    Paused,
    Finished,
    Aborted,
}
pub struct Timer {
    pub phase: Phase,
    pub remaining: Duration,
    pub auto: u64,
    pub driver: u64,
    resume: Phase,
}
impl Timer {
    pub fn new(auto: u64, driver: u64) -> Self {
        Self {
            phase: Phase::Idle,
            remaining: Duration::from_secs(auto + driver),
            auto,
            driver,
            resume: Phase::Idle,
        }
    }
    pub fn start(&mut self) {
        if self.phase != Phase::Idle {
            return;
        }
        self.phase = if self.auto > 0 {
            Phase::Autonomous
        } else {
            Phase::Driver
        };
        self.remaining = Duration::from_secs(if self.auto > 0 {
            self.auto
        } else {
            self.driver
        });
    }
    pub fn pause(&mut self) {
        if matches!(self.phase, Phase::Autonomous | Phase::Driver) {
            self.resume = self.phase.clone();
            self.phase = Phase::Paused;
        }
    }
    pub fn resume(&mut self) {
        if self.phase == Phase::Paused {
            self.phase = self.resume.clone()
        } else if self.phase == Phase::AutoPause {
            self.phase = Phase::Driver;
            self.remaining = Duration::from_secs(self.driver)
        }
    }
    pub fn abort(&mut self) {
        self.phase = Phase::Aborted;
        self.remaining = Duration::ZERO
    }
    pub fn advance(&mut self, dt: Duration) {
        if !matches!(self.phase, Phase::Autonomous | Phase::Driver) {
            return;
        }
        self.remaining = self.remaining.saturating_sub(dt);
        if self.remaining.is_zero() {
            self.phase = if self.phase == Phase::Autonomous {
                Phase::AutoPause
            } else {
                Phase::Finished
            };
        }
    }
}

/// Returns true only when the three-second pre-start countdown finishes.
pub fn advance_countdown(
    timer: &mut Timer,
    countdown: &mut Option<Duration>,
    dt: Duration,
) -> bool {
    if let Some(left) = *countdown {
        if dt < left {
            *countdown = Some(left - dt);
            false
        } else {
            *countdown = None;
            timer.start();
            timer.advance(dt.saturating_sub(left));
            true
        }
    } else {
        timer.advance(dt);
        false
    }
}
