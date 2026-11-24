use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Team {
    pub id: u32,
    #[serde(default)]
    pub age_group: String,
    pub number: String,
    pub name: String,
    pub organization: String,
    pub city: String,
    pub checked_in: bool,
    pub inspected: bool,
    pub notes: String,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Slot {
    pub team: u32,
    pub dq: bool,
    pub no_show: bool,
    pub surrogate: bool,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AllianceInput {
    pub teams: Vec<Slot>,
    pub values: BTreeMap<String, i64>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
pub enum MatchState {
    #[default]
    Unplayed,
    Scored,
    Submitted,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Match {
    #[serde(default)]
    pub series: Option<u32>,
    pub id: u32,
    pub round: u32,
    pub number: u32,
    pub division: String,
    pub field: String,
    pub planned: String,
    pub state: MatchState,
    pub alliances: Vec<AllianceInput>,
    pub values: BTreeMap<String, i64>,
    pub audit: Vec<String>,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Skill {
    pub team: u32,
    pub programming: bool,
    pub score: i64,
    pub stop_time: i64,
    pub attempt: u32,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Award {
    pub name: String,
    pub team: u32,
    pub recipient: String,
    pub description: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Event {
    #[serde(default)]
    pub config: EventConfig,
    #[serde(default)]
    pub bracket: Vec<crate::bracket::Series>,
    #[serde(default = "default_wins")]
    pub elimination_wins: u32,
    pub format: u32,
    pub name: String,
    pub game: String,
    pub venue: String,
    pub date: String,
    pub teams: Vec<Team>,
    pub matches: Vec<Match>,
    pub skills: Vec<Skill>,
    pub awards: Vec<Award>,
    pub announcements: String,
    #[serde(default)]
    pub selection_started: bool,
    pub alliances: Vec<Vec<u32>>,
}
impl Default for Event {
    fn default() -> Self {
        Self {
            config: EventConfig::default(),
            bracket: vec![],
            elimination_wins: 1,
            format: 1,
            name: "New Tournament".into(),
            game: "VRC_2026_Override".into(),
            venue: String::new(),
            date: String::new(),
            teams: vec![],
            matches: vec![],
            skills: vec![],
            awards: vec![],
            announcements: String::new(),
            selection_started: false,
            alliances: vec![],
        }
    }
}
impl Event {
    pub fn team_number(&self, id: u32) -> String {
        self.teams
            .iter()
            .find(|t| t.id == id)
            .map(|t| t.number.clone())
            .unwrap_or_else(|| "—".into())
    }
    pub fn next_team_id(&self) -> u32 {
        self.teams.iter().map(|x| x.id).max().unwrap_or(0) + 1
    }
}

fn default_wins() -> u32 {
    1
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FieldSet {
    pub name: String,
    pub fields: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DisplayConfig {
    #[serde(default = "default_screen")]
    pub screen: String,
    pub id: u32,
    pub name: String,
    pub kind: String,
    pub field: String,
    pub fullscreen: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScheduleConfig {
    #[serde(default)]
    pub blocks: Vec<ScheduleBlock>,
    pub practice_rounds: usize,
    pub qualification_rounds: usize,
    pub practice_start: String,
    pub qualification_start: String,
    pub cycle_seconds: u64,
    pub seed: u64,
}
impl Default for ScheduleConfig {
    fn default() -> Self {
        Self {
            blocks: vec![],
            practice_rounds: 1,
            qualification_rounds: 4,
            practice_start: "09:00".into(),
            qualification_start: "10:00".into(),
            cycle_seconds: 180,
            seed: 202526,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct EventConfig {
    pub competition: String,
    pub tournament_type: String,
    pub password_hash: String,
    pub protect_options: bool,
    pub judge_password_hash: String,
    pub inspector_password_hash: String,
    pub scorekeeper_password_hash: String,
    pub elimination_team_count: usize,
    pub allow_unbalanced: bool,
    pub selection_order: String,
    pub sponsors: Vec<Sponsor>,
    pub elimination_alliances: usize,
    pub iq_finalists: usize,
    pub field_sets: Vec<FieldSet>,
    pub skills_driver: bool,
    pub skills_autonomous: bool,
    pub skills_fields: usize,
    pub skills_max_attempts: usize,
    pub displays: Vec<DisplayConfig>,
    pub schedule: ScheduleConfig,
    pub configured: bool,
    pub show_score_preview: bool,
}
impl Default for EventConfig {
    fn default() -> Self {
        Self {
            competition: "VEX V5".into(),
            show_score_preview: true,
            tournament_type: "Small tournament".into(),
            password_hash: String::new(),
            protect_options: false,
            judge_password_hash: String::new(),
            inspector_password_hash: String::new(),
            scorekeeper_password_hash: String::new(),
            elimination_team_count: 2,
            allow_unbalanced: false,
            selection_order: "Ranking".into(),
            sponsors: vec![],
            elimination_alliances: 8,
            iq_finalists: 10,
            field_sets: vec![FieldSet {
                name: "Field Set 1".into(),
                fields: vec!["Field 1".into()],
            }],
            skills_driver: true,
            skills_autonomous: true,
            skills_fields: 1,
            skills_max_attempts: 3,
            displays: vec![
                DisplayConfig {
                    screen: "Rankings".into(),
                    id: 1,
                    name: "Pit Display 1".into(),
                    kind: "Pit".into(),
                    field: String::new(),
                    fullscreen: false,
                },
                DisplayConfig {
                    screen: "Rankings".into(),
                    id: 2,
                    name: "Field Display 1".into(),
                    kind: "Field".into(),
                    field: "Field 1".into(),
                    fullscreen: false,
                },
                DisplayConfig {
                    screen: "Rankings".into(),
                    id: 3,
                    name: "Audience Display".into(),
                    kind: "Audience".into(),
                    field: String::new(),
                    fullscreen: false,
                },
            ],
            schedule: ScheduleConfig::default(),
            configured: false,
        }
    }
}
impl EventConfig {
    pub fn fields(&self) -> Vec<String> {
        self.field_sets
            .iter()
            .flat_map(|s| s.fields.clone())
            .collect()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Sponsor {
    pub name: String,
    pub image: Vec<u8>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScheduleBlock {
    pub date: String,
    pub start: String,
    pub end: String,
    pub cycle_seconds: u64,
    pub round: u32,
}
impl Default for ScheduleBlock {
    fn default() -> Self {
        Self {
            date: "2026-11-20".into(),
            start: "09:00".into(),
            end: "10:00".into(),
            cycle_seconds: 360,
            round: 1,
        }
    }
}

fn default_screen() -> String {
    "Rankings".into()
}
