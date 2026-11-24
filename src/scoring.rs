use crate::{
    model::*,
    rules::{Machine, V},
};
use anyhow::{Context, Result, bail};
use serde_json::{Value as J, json};
use std::collections::BTreeMap;
thread_local! {static SCORE_CACHE:std::cell::RefCell<std::collections::HashMap<String,J>>=std::cell::RefCell::new(std::collections::HashMap::new());}
#[derive(Clone, Debug)]
pub struct Game {
    pub data: J,
}
impl Game {
    pub fn all() -> Vec<Game> {
        serde_json::from_str::<Vec<J>>(include_str!("../analysis/games.json"))
            .expect("embedded rules")
            .into_iter()
            .map(|data| Game { data })
            .collect()
    }
    pub fn id(&self) -> &str {
        self.data["id"].as_str().unwrap()
    }
    pub fn title(&self) -> String {
        format!(
            "{} · {} · {}",
            self.data["name"].as_str().unwrap_or("").replace("&&", "&"),
            self.data["season"].as_str().unwrap_or(""),
            if self.cooperative() {
                "VEX IQ"
            } else {
                "VEX V5"
            }
        )
    }
    pub fn team_size(&self) -> usize {
        self.data["rounds"][1]["teamsPerAlliance"]
            .as_u64()
            .unwrap_or(2) as usize
    }
    pub fn cooperative(&self) -> bool {
        self.data["program"].as_str() == Some("VEXIQ")
    }
    pub fn controls(&self, skills: bool) -> Vec<J> {
        let a = self.data["controls"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        a.into_iter()
            .filter(|c| c["name"].as_str().unwrap_or("").starts_with("skills_") == skills)
            .collect()
    }
    pub fn defaults(&self, skills: bool) -> (BTreeMap<String, i64>, Vec<BTreeMap<String, i64>>) {
        let mut shared = BTreeMap::new();
        let mut aa = vec![BTreeMap::new(), BTreeMap::new()];
        for c in self.controls(skills) {
            let raw = c["name"].as_str().unwrap_or("");
            let name = raw.strip_prefix("skills_").unwrap_or(raw);
            let val = c["default"].as_i64().unwrap_or(0);
            if let Some(n) = name.strip_prefix("a1_") {
                aa[0].insert(n.into(), val);
            } else if let Some(n) = name.strip_prefix("a2_") {
                aa[1].insert(n.into(), val);
            } else {
                shared.insert(name.into(), val);
            }
        }
        (shared, aa)
    }
    pub fn new_match(&self, id: u32, round: u32, teams: &[u32]) -> Match {
        let (shared, values) = self.defaults(false);
        Match {
            id,
            number: id,
            round,
            division: "Division 1".into(),
            field: "Field 1".into(),
            values: shared,
            alliances: (0..2)
                .map(|a| AllianceInput {
                    teams: teams
                        .iter()
                        .skip(a * self.team_size())
                        .take(self.team_size())
                        .map(|id| Slot {
                            team: *id,
                            ..Default::default()
                        })
                        .collect(),
                    values: values[a].clone(),
                })
                .collect(),
            ..Default::default()
        }
    }
    pub fn input(m: &Match) -> J {
        json!({"matchTuple":{"round":m.round},"scoreTypes":m.values,"alliances":m.alliances.iter().map(|a|json!({"scoreTypes":a.values,"teams":a.teams.iter().map(|t|json!({"dq":t.dq,"noShow":t.no_show})).collect::<Vec<_>>()})).collect::<Vec<_>>()})
    }
    pub fn score(&self, m: &Match) -> Result<J> {
        self.cached("calc_match_score", m)
    }
    pub fn constraints(&self, m: &Match) -> Result<Vec<String>> {
        let v = self.cached("check_constraints", m)?;
        Ok(v.as_array()
            .map(|a| {
                a.iter()
                    .map(|x| x.as_str().unwrap_or("constraint").to_owned())
                    .collect()
            })
            .unwrap_or_default())
    }
    fn cached(&self, name: &str, m: &Match) -> Result<J> {
        let key = format!(
            "{}|{}|{}|{}",
            self.id(),
            self.data["version"],
            name,
            Self::input(m)
        );
        if let Some(v) = SCORE_CACHE.with(|c| c.borrow().get(&key).cloned()) {
            return Ok(v);
        }
        let v = self.run(name, m, None)?;
        SCORE_CACHE.with(|c| {
            let mut c = c.borrow_mut();
            if c.len() >= 4096 {
                c.clear()
            }
            c.insert(key, v.clone());
        });
        Ok(v)
    }
    pub fn run(&self, name: &str, m: &Match, skill: Option<u32>) -> Result<J> {
        let mut input = Self::input(m);
        if skill.is_some() && input["alliances"].as_array().is_some_and(|a| a.len() == 1) {
            let zeros = input["alliances"][0]["scoreTypes"]
                .as_object()
                .map(|m| {
                    m.keys()
                        .map(|k| (k.clone(), json!(0)))
                        .collect::<serde_json::Map<String, J>>()
                })
                .unwrap_or_default();
            input["alliances"]
                .as_array_mut()
                .unwrap()
                .push(json!({"scoreTypes":zeros,"teams":[{"dq":false,"noShow":false}]}));
        }
        if !self.data["rules"].is_null() {
            let mut vm = Machine::new();
            vm.load(&self.data["rules"])?;
            let mut args = vec![];
            if let Some(i) = skill {
                args.push(V::from_json(&json!(
                    if self.data["skills"][i as usize]["skillsType"] == "PROGRAMMING" {
                        1
                    } else {
                        2
                    }
                )))
            }
            args.push(V::from_json(&input));
            vm.call(name, args)
                .map(|v| v.json())
                .with_context(|| format!("{}: {name}", self.id()))
        } else {
            self.formula(name, m, skill)
        }
    }
    fn formula(&self, name: &str, m: &Match, skill: Option<u32>) -> Result<J> {
        let cfg = if let Some(i) = skill {
            self.data["skills"][i as usize]
                .get("scoring")
                .unwrap_or(&self.data["defaultScoring"])
        } else {
            &self.data["defaultScoring"]
        };
        let input = Self::input(m);
        let mut vars = json!({"match":m.values,"alliance":m.alliances.iter().map(|a|json!(a.values)).collect::<Vec<_>>()});
        for (i, a) in m.alliances.iter().enumerate() {
            for (k, v) in &a.values {
                vars[format!("a{}_{}", i + 1, k)] = json!(v)
            }
        }
        if name.contains("constraints") {
            let mut failed = vec![];
            for (i, c) in cfg["constraints"]
                .as_array()
                .unwrap_or(&vec![])
                .iter()
                .enumerate()
            {
                let v = Machine::new().expression(c, &vars)?;
                if v.as_f64() == Some(0.) {
                    failed.push(format!("constraint {}", i + 1))
                }
            }
            return Ok(json!(failed));
        }
        let mut scores = vec![];
        for (i, a) in m.alliances.iter().enumerate() {
            let mut local = vars.clone();
            for (k, v) in &a.values {
                local[k] = json!(v)
            }
            local["allianceIdx"] = json!(i);
            let ast = &cfg["expressions"]["allianceScoreFormula"];
            let raw = if ast.is_null() {
                *a.values.get("score").unwrap_or(&0) as f64
            } else {
                Machine::new()
                    .expression(ast, &local)?
                    .as_f64()
                    .unwrap_or(0.)
            };
            let dq = if m.round == 3 || m.round == 16 {
                a.teams.iter().any(|t| t.dq || t.no_show)
            } else {
                a.teams.iter().all(|t| t.dq || t.no_show)
            };
            scores.push((if dq { 0. } else { raw }, dq));
        }
        let mut out = vec![];
        for (i, a) in m.alliances.iter().enumerate() {
            let (our, dq) = scores[i];
            let (their, tdq) = scores.get(1 - i).copied().unwrap_or((0., false));
            let mut ts = vec![];
            for t in &a.teams {
                let mut v = vars.clone();
                v["ourScore"] = json!(our);
                v["theirScore"] = json!(their);
                v["dq"] = json!(t.dq as u8);
                v["noShow"] = json!(t.no_show as u8);
                v["theirAllianceDq"] = json!(tdq as u8);
                v["ourDqCount"] = json!(a.teams.iter().filter(|t| t.dq).count());
                v["theirDqCount"] = json!(
                    m.alliances
                        .get(1 - i)
                        .map(|a| a.teams.iter().filter(|t| t.dq).count())
                        .unwrap_or(0)
                );
                let bad = t.dq || t.no_show;
                let default_wp = if bad {
                    0.
                } else if tdq || our > their {
                    2.
                } else if our == their {
                    1.
                } else {
                    0.
                };
                let mut wp = default_wp;
                let mut awp = 0.;
                let mut ap = 0.;
                let mut sp = if bad {
                    0.
                } else if tdq {
                    our
                } else {
                    our.min(their)
                };
                for (k, val) in [
                    ("wpFormula", &mut wp),
                    ("awpFormula", &mut awp),
                    ("apFormula", &mut ap),
                    ("spFormula", &mut sp),
                ] {
                    let expr = &cfg["expressions"][k];
                    if !expr.is_null() {
                        *val = Machine::new().expression(expr, &v)?.as_f64().unwrap_or(0.)
                    }
                }
                ts.push(json!({"score":if bad{0.}else{our},"wp":wp,"awp":awp,"ap":ap,"sp":sp,"win":(wp==2.)as u8,"loss":(wp==0.)as u8,"tie":(wp==1.)as u8,"no_show":t.no_show}));
            }
            out.push(json!({"score":our,"dq":dq as u8,"teams":ts}));
        }
        let _ = input;
        Ok(json!({"alliances":out}))
    }
    pub fn validate_shape(m: &Match) -> Result<()> {
        if m.alliances.len() != 2 || m.alliances.iter().any(|a| a.teams.is_empty()) {
            bail!("match requires two nonempty alliances")
        };
        Ok(())
    }
}
#[derive(Clone, Debug, Default)]
pub struct Rank {
    pub team: u32,
    pub played: usize,
    pub wp: f64,
    pub ap: f64,
    pub sp: f64,
    pub scores: Vec<(f64, bool)>,
    pub wins: u32,
    pub losses: u32,
    pub ties: u32,
    pub key: Vec<f64>,
    pub score_key: Vec<f64>,
    pub average_score: f64,
}
pub fn rankings(event: &Event, game: &Game) -> Result<Vec<Rank>> {
    let mut rows: BTreeMap<u32, Rank> = event
        .teams
        .iter()
        .map(|t| {
            (
                t.id,
                Rank {
                    team: t.id,
                    ..Default::default()
                },
            )
        })
        .collect();
    for m in &event.matches {
        if m.round != 2 || m.state != MatchState::Submitted {
            continue;
        }
        let score = game.score(m)?;
        for (ai, a) in m.alliances.iter().enumerate() {
            for (ti, t) in a.teams.iter().enumerate() {
                if t.surrogate {
                    continue;
                }
                let v = &score["alliances"][ai]["teams"][ti];
                let r = rows.get_mut(&t.team).context("unknown scheduled team")?;
                let n = |k: &str| v[k].as_f64().unwrap_or(0.);
                r.played += 1;
                r.wp += n("wp") + n("awp");
                r.ap += n("ap");
                r.sp += n("sp");
                r.wins += n("win") as u32;
                r.losses += n("loss") as u32;
                r.ties += n("tie") as u32;
                r.scores.push((n("score"), t.no_show));
            }
        }
    }
    let method = game.data["rounds"][1]["rankings"]
        .as_str()
        .unwrap_or("WP_SP");
    let mut rows = rows.into_values().collect::<Vec<_>>();
    for r in &mut rows {
        r.scores.sort_by(|a, b| {
            if method == "AVG_PTS_3_OF_4_KEEP_NO_SHOW" {
                b.1.cmp(&a.1).then(b.0.total_cmp(&a.0))
            } else {
                b.0.total_cmp(&a.0)
            }
        });
        let drop = if method.contains("3_OF_4") {
            (r.scores.len() / 4).min(4)
        } else if method == "AVG_PTS_DROP_1_OF_6" && r.scores.len() >= 6 {
            1
        } else {
            0
        };
        let mut kept = r.scores[..r.scores.len() - drop]
            .iter()
            .map(|x| x.0)
            .collect::<Vec<_>>();
        kept.sort_by(|a, b| b.total_cmp(a));
        let n = r.played.max(1) as f64;
        let avg = kept.iter().sum::<f64>() / kept.len().max(1) as f64;
        r.average_score = avg;
        r.key = match method {
            "AVG_WP_AP_SP" => vec![r.wp / n, r.ap / n, r.sp / n],
            "WP_AP_SP" => vec![r.wp, r.ap, r.sp],
            "AVG_WP_AVG_SCORE_HIGH_SCORES" => vec![r.wp / n, avg],
            "WP_SP" => vec![r.wp, r.sp],
            _ => vec![avg],
        };
        if method.contains("AVG_PTS") {
            for len in (1..kept.len()).rev() {
                r.score_key
                    .push(kept[..len].iter().sum::<f64>() / len as f64)
            }
        } else {
            r.score_key
                .extend(kept.iter().take(if method == "AVG_WP_AP_SP" {
                    2
                } else {
                    kept.len()
                }))
        }
    }
    rows.sort_by(|a, b| {
        for (x, y) in a.key.iter().zip(&b.key) {
            let c = y.total_cmp(x);
            if !c.is_eq() {
                return c;
            }
        }
        for (x, y) in a.score_key.iter().zip(&b.score_key) {
            let c = y.total_cmp(x);
            if !c.is_eq() {
                return c;
            }
        }
        a.score_key
            .len()
            .cmp(&b.score_key.len())
            .then(b.team.cmp(&a.team))
    });
    Ok(rows)
}
