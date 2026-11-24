mod displays;
mod wizard;
use anyhow::{Context, Result};
use eframe::egui::{self, Color32, RichText};
use std::{
    io::{Read, Write},
    net::TcpListener,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tournament_manager_rs::{
    model::*,
    schedule::{self, Phase, Timer},
    scoring::{Game, rankings},
    store,
};
const BLUE: Color32 = Color32::from_rgb(0, 112, 192);
const NAVY: Color32 = Color32::from_rgb(27, 29, 82);
struct App {
    event: Event,
    games: Vec<Game>,
    path: Option<PathBuf>,
    startup: bool,
    tab: usize,
    selected: usize,
    status: String,
    dirty: bool,
    undo: Vec<Event>,
    rounds: usize,
    finalists: usize,
    seed: u64,
    team_filter: String,
    new_team: Team,
    skill_team: u32,
    skill_prog: bool,
    skill_input: Option<Match>,
    award_name: String,
    award_team: u32,
    display: bool,
    open_displays: std::collections::BTreeSet<u32>,
    wizard: Option<wizard::Wizard>,

    timer: Timer,
    last: Instant,
    server: Option<Arc<Mutex<(String, String)>>>,
    confirm_schedule: bool,
    remote_dialog: bool,
    hardware: Option<tournament_manager_rs::hardware::V5>,
    hardware_path: String,
    hardware_index: usize,
    hardware_status: serde_json::Value,
    hardware_last: Instant,
    hardware_mode: String,
    hardware_seconds: u64,
    options_prompt: bool,
    options_unlocked: bool,
    options_password: String,
    options_error: String,
}
impl Default for App {
    fn default() -> Self {
        Self {
            event: Event::default(),
            games: Game::all(),
            path: None,
            startup: true,
            tab: 0,
            selected: 0,
            status: "Ready".into(),
            dirty: false,
            undo: vec![],
            rounds: 4,
            finalists: 10,
            seed: 202526,
            team_filter: String::new(),
            new_team: Team::default(),
            skill_team: 0,
            skill_prog: false,
            skill_input: None,
            award_name: String::new(),
            award_team: 0,
            display: false,
            open_displays: Default::default(),
            wizard: None,

            timer: Timer::new(15, 105),
            last: Instant::now(),
            server: None,
            confirm_schedule: false,
            remote_dialog: false,
            hardware: None,
            hardware_path: tournament_manager_rs::hardware::DEFAULT_DRIVER.into(),
            hardware_index: 0,
            hardware_status: serde_json::Value::Null,
            hardware_last: Instant::now(),
            hardware_mode: String::new(),
            hardware_seconds: u64::MAX,
            options_prompt: false,
            options_unlocked: false,
            options_password: String::new(),
            options_error: String::new(),
        }
    }
}
impl App {
    fn game(&self) -> Game {
        self.games
            .iter()
            .find(|g| g.id() == self.event.game)
            .unwrap_or(&self.games[0])
            .clone()
    }
    fn checkpoint(&mut self) {
        self.undo.push(self.event.clone());
        if self.undo.len() > 50 {
            self.undo.remove(0);
        }
        self.dirty = true;
    }
    fn result(&mut self, r: Result<()>) {
        self.status = match r {
            Ok(()) => "Done".into(),
            Err(e) => format!("{e:#}"),
        }
    }
    fn save(&mut self, save_as: bool) {
        let path = if save_as || self.path.is_none() {
            rfd::FileDialog::new()
                .add_filter(
                    "Rust Tournament",
                    "tmrs".split_whitespace().collect::<Vec<_>>().as_slice(),
                )
                .set_file_name("tournament.tmrs")
                .save_file()
        } else {
            self.path.clone()
        };
        if let Some(p) = path {
            match store::save(&p, &self.event) {
                Ok(()) => {
                    self.path = Some(p);
                    self.dirty = false;
                    self.status = "Tournament saved".into()
                }
                Err(e) => self.status = format!("Save failed: {e:#}"),
            }
        }
    }
    fn open(&mut self) {
        if let Some(p) = rfd::FileDialog::new()
            .add_filter("Rust Tournament", &["tmrs"])
            .pick_file()
        {
            match store::load(&p) {
                Ok(e) => self.activate_event(e, p),
                Err(e) => self.status = format!("{e:#}"),
            }
        }
    }
    fn reset_timer(&mut self) {
        let g = self.game();
        let mut a = 0;
        let mut d = 0;
        for t in g.data["defaultMatchTimes"].as_array().unwrap_or(&vec![]) {
            match t["type"].as_str() {
                Some("AUTONOMOUS") => a = t["seconds"].as_u64().unwrap_or(0),
                Some("DRIVER_CTRL") => d = t["seconds"].as_u64().unwrap_or(60),
                _ => (),
            }
        }
        if let Some(hw) = &mut self.hardware {
            if hw.connected {
                let _ = hw.control("idle", 0);
            }
        }
        self.hardware_mode.clear();
        self.timer = Timer::new(a, d);
        self.last = Instant::now();
    }
    fn add_teams(&mut self, ts: Vec<Team>) {
        let mut existing = self
            .event
            .teams
            .iter()
            .map(|t| t.number.to_uppercase())
            .collect::<std::collections::HashSet<_>>();
        self.checkpoint();
        let mut n = 0;
        for mut t in ts {
            if existing.insert(t.number.to_uppercase()) {
                t.id = self.event.next_team_id();
                self.event.teams.push(t);
                n += 1
            }
        }
        self.status = format!("Imported {n} teams");
    }
    fn new_event(&mut self) {
        self.wizard = Some(wizard::Wizard::default());
    }
    fn activate_event(&mut self, e: Event, p: PathBuf) {
        self.event = e;
        self.event.config.elimination_team_count = 2;
        self.event.config.allow_unbalanced = false;
        self.path = Some(p);
        self.startup = false;
        self.dirty = false;
        self.selected = 0;
        self.undo.clear();
        self.open_displays.clear();
        self.options_unlocked = false;
        self.finalists = self.event.config.iq_finalists;
        self.rounds = self.event.config.schedule.qualification_rounds;
        self.reset_timer();
    }
    fn wizard_view(&mut self, ui: &mut egui::Ui) {
        if let Some(mut w) = self.wizard.take() {
            w.ui(ui);
            if w.cancelled {
                return;
            }
            if w.finished {
                w.finished = false;
                match tournament_manager_rs::auth::hash(&w.password) {
                    Err(e) => w.error = e.to_string(),
                    Ok(hash) => {
                        w.event.config.password_hash = hash;
                        w.event.config.judge_password_hash =
                            tournament_manager_rs::auth::hash(&w.judge_password)
                                .unwrap_or_default();
                        w.event.config.inspector_password_hash =
                            tournament_manager_rs::auth::hash(&w.inspector_password)
                                .unwrap_or_default();
                        w.event.config.scorekeeper_password_hash =
                            tournament_manager_rs::auth::hash(&w.scorekeeper_password)
                                .unwrap_or_default();
                        w.event.config.configured = true;
                        if let Some(p) = rfd::FileDialog::new()
                            .add_filter("Rust Tournament", &["tmrs"])
                            .set_file_name("tournament.tmrs")
                            .save_file()
                        {
                            match store::save(&p, &w.event) {
                                Ok(()) => {
                                    self.activate_event(w.event, p);
                                    self.status="Tournament created. Practice and qualification matches are ready.".into();
                                    return;
                                }
                                Err(e) => w.error = e.to_string(),
                            }
                        }
                    }
                }
            }
            self.wizard = Some(w);
        }
    }
    fn load_demo(&mut self, game: &str) {
        self.event = Event {
            name: if game.starts_with("VIQC") {
                "VEX IQ · Mix & Match Demo".into()
            } else {
                "VEX V5 · Push Back Demo".into()
            },
            game: game.into(),
            venue: "Demonstration event".into(),
            ..Default::default()
        };
        for i in 1..=16 {
            self.event.teams.push(Team {
                id: i,
                number: format!("{i:03}A"),
                name: format!("Robotics Team {i}"),
                organization: "Demo School".into(),
                checked_in: true,
                inspected: i % 3 != 0,
                ..Default::default()
            });
        }
        let g = self.game();
        self.event.matches = schedule::generate(&self.event, &g, 4, 202526).expect("valid demo");
        for (i, m) in self.event.matches.iter_mut().take(4).enumerate() {
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
        self.event.config.competition = if self.game().cooperative() {
            "VEX IQ".into()
        } else {
            "VEX V5".into()
        };
        for (i, m) in self.event.matches.iter_mut().enumerate() {
            let seconds = 9 * 3600 + i * 180;
            m.planned = format!("{:02}:{:02}", seconds / 3600, seconds / 60 % 60);
        }
        self.event.announcements = "Welcome! This demonstration uses fictional teams.".into();
        self.path = None;
        self.startup = false;
        self.selected = 0;
        self.dirty = true;
        self.undo.clear();
        self.reset_timer();
        self.status = "Demo loaded. Save as a new .tmrs file to keep your changes.".into();
    }
    fn startup(&mut self, ui: &mut egui::Ui) {
        ui.add_space(30.);
        ui.heading(
            RichText::new("VEX Tournament Manager · Rust")
                .size(28.)
                .color(NAVY),
        );
        ui.add_space(25.);
        ui.label(
            RichText::new("Select an option to begin using the Tournament Manager.")
                .size(23.)
                .color(BLUE),
        );
        ui.add_space(30.);
        for (i, title, detail) in [
            (
                0,
                "Create a new tournament",
                "Select a new tournament filename to hold your tournament data",
            ),
            (
                1,
                "Open an existing tournament",
                "Open a previously-created Rust tournament data file",
            ),
            (
                2,
                "Connect to a remote server",
                "View connection and compatibility information",
            ),
        ] {
            ui.horizontal(|ui| {
                ui.add_space(40.);
                let b = ui.add_sized(
                    [620., 65.],
                    egui::Button::new(
                        RichText::new(format!("➜  {title}\n{detail}"))
                            .size(20.)
                            .color(NAVY),
                    )
                    .frame(false),
                );
                if b.clicked() {
                    match i {
                        0 => self.new_event(),
                        1 => self.open(),
                        _ => self.remote_dialog = true,
                    }
                }
            });
            ui.add_space(15.);
        }
        ui.add_space(20.);
        ui.horizontal(|ui| {
            if ui.button("Try VEX V5 demo").clicked() {
                self.load_demo("VRC_2025_PushBack")
            }
            if ui.button("Try VEX IQ demo").clicked() {
                self.load_demo("VIQC_2025_MixAndMatch")
            }
        });
        ui.label("Reconstruction of v2025_26.2.2 · Rust edition 0.1");
        ui.label(
            "Original .db files: teams can be imported read-only after creating a tournament.",
        );
    }
    fn teams(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading("Team List");
            ui.text_edit_singleline(&mut self.team_filter);
            if ui.button("Import CSV…").clicked() {
                if let Some(p) = rfd::FileDialog::new()
                    .add_filter("CSV", &["csv"])
                    .pick_file()
                {
                    match store::import_csv(&p, self.event.next_team_id()) {
                        Ok(ts) => self.add_teams(ts),
                        Err(e) => self.status = format!("{e:#}"),
                    }
                }
            }
            if ui.button("Import original TM teams…").clicked() {
                if let Some(p) = rfd::FileDialog::new().pick_file() {
                    match store::original_teams(&p, self.event.next_team_id()) {
                        Ok(ts) => self.add_teams(ts),
                        Err(e) => self.status = format!("{e:#}"),
                    }
                }
            }
        });
        ui.horizontal(|ui| {
            ui.label("Number");
            ui.add(egui::TextEdit::singleline(&mut self.new_team.number).desired_width(90.));
            ui.label("Name");
            ui.text_edit_singleline(&mut self.new_team.name);
            if ui.button("Add team").clicked() {
                let mut t = self.new_team.clone();
                t.number = t.number.trim().to_uppercase();
                if t.number.is_empty() {
                    self.status = "Enter a team number".into()
                } else if self
                    .event
                    .teams
                    .iter()
                    .any(|x| x.number.eq_ignore_ascii_case(&t.number))
                {
                    self.status = "Team number already exists".into()
                } else {
                    self.checkpoint();
                    t.id = self.event.next_team_id();
                    self.event.teams.push(t);
                    self.new_team = Team::default();
                }
            }
        });
        ui.separator();
        let filter = self.team_filter.to_lowercase();
        let mut changed = false;
        egui::ScrollArea::both().show(ui, |ui| {
            egui::Grid::new("teams")
                .striped(true)
                .min_col_width(90.)
                .show(ui, |ui| {
                    for s in [
                        "Number",
                        "Name",
                        "Organization",
                        "City",
                        "Checked in",
                        "Inspected",
                        "Notes",
                    ] {
                        ui.strong(s);
                    }
                    ui.end_row();
                    for t in &mut self.event.teams {
                        if !filter.is_empty()
                            && !format!("{} {} {}", t.number, t.name, t.organization)
                                .to_lowercase()
                                .contains(&filter)
                        {
                            continue;
                        }
                        ui.label(&t.number);
                        changed |= ui
                            .add(egui::TextEdit::singleline(&mut t.name).desired_width(150.))
                            .changed();
                        changed |= ui
                            .add(
                                egui::TextEdit::singleline(&mut t.organization).desired_width(150.),
                            )
                            .changed();
                        changed |= ui
                            .add(egui::TextEdit::singleline(&mut t.city).desired_width(100.))
                            .changed();
                        changed |= ui.checkbox(&mut t.checked_in, "").changed();
                        changed |= ui.checkbox(&mut t.inspected, "").changed();
                        changed |= ui
                            .add(egui::TextEdit::singleline(&mut t.notes).desired_width(150.))
                            .changed();
                        ui.end_row();
                    }
                });
        });
        self.dirty |= changed;
    }
    fn scoring(&mut self, ui: &mut egui::Ui) {
        let game = self.game();
        ui.horizontal(|ui| {
            ui.heading("Match Scoring");
            ui.label(game.title());
        });
        ui.separator();
        if self.event.matches.is_empty() {
            ui.label("No matches. Check in teams and generate matches in Match Schedule.");
            return;
        }
        self.selected = self.selected.min(self.event.matches.len() - 1);
        ui.columns(2, |cols| {
            cols[0].set_max_width(280.);
            egui::ScrollArea::vertical()
                .id_salt("matches")
                .max_height(370.)
                .show(&mut cols[0], |ui| {
                    for (i, m) in self.event.matches.iter().enumerate() {
                        let teams = m
                            .alliances
                            .iter()
                            .map(|a| {
                                a.teams
                                    .iter()
                                    .map(|t| self.event.team_number(t.team))
                                    .collect::<Vec<_>>()
                                    .join(" / ")
                            })
                            .collect::<Vec<_>>()
                            .join("   vs   ");
                        let label = format!(
                            "{}{}  {:?}\n{}",
                            match m.round {
                                1 => "P",
                                2 => "Q",
                                15 => "F",
                                _ => "E",
                            },
                            m.number,
                            m.state,
                            teams
                        );
                        if ui.selectable_label(i == self.selected, label).clicked() {
                            self.selected = i;
                        }
                    }
                });
            let ui = &mut cols[1];
            let names = self
                .event
                .teams
                .iter()
                .map(|t| (t.id, t.number.clone()))
                .collect::<std::collections::BTreeMap<_, _>>();
            let locked = (self.event.matches[self.selected].round == 2
                && (!self.event.bracket.is_empty()
                    || self.event.matches.iter().any(|m| m.round == 15)))
                || (self.event.matches[self.selected].round != 2
                    && self.event.matches[self.selected].state == MatchState::Submitted);
            let mut submit = false;
            let m = &mut self.event.matches[self.selected];
            if locked {
                ui.label("Score locked. Use Alliance Selection to unlock qualification.");
            }
            ui.label(format!("Match {} · {} · {}", m.number, m.division, m.field));
            let mut changed = false;
            ui.add_enabled_ui(!locked, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("scoreform")
                    .max_height(430.)
                    .show(ui, |ui| {
                        for (ai, a) in m.alliances.iter_mut().enumerate() {
                            ui.colored_label(
                                if ai == 0 { Color32::DARK_RED } else { BLUE },
                                if ai == 0 {
                                    "Red alliance / Team 1"
                                } else {
                                    "Blue alliance / Team 2"
                                },
                            );
                            for t in &mut a.teams {
                                ui.horizontal(|ui| {
                                    ui.label(names.get(&t.team).cloned().unwrap_or_default());
                                    changed |= ui.checkbox(&mut t.dq, "DQ").changed();
                                    changed |= ui.checkbox(&mut t.no_show, "No show").changed();
                                    changed |= ui.checkbox(&mut t.surrogate, "Surrogate").changed();
                                });
                            }
                        }
                        changed |= score_controls(ui, &game, m, false);
                    });
            });
            if changed {
                if m.state == MatchState::Submitted {
                    m.state = MatchState::Scored
                }
                self.dirty = true;
            }
            match game.score(m) {
                Ok(s) => {
                    ui.horizontal(|ui| {
                        for (i, c) in [Color32::DARK_RED, BLUE].iter().enumerate() {
                            ui.label(
                                RichText::new(format!(
                                    "{:.0}",
                                    s["alliances"][i]["score"].as_f64().unwrap_or(0.)
                                ))
                                .size(42.)
                                .color(*c),
                            );
                        }
                    });
                }
                Err(e) => {
                    ui.colored_label(Color32::DARK_RED, format!("Scoring unavailable: {e:#}"));
                }
            }
            let valid = game.constraints(m);
            match &valid {
                Ok(errors) if !errors.is_empty() => {
                    ui.colored_label(
                        Color32::DARK_RED,
                        format!("Invalid inputs: {}", errors.join(", ")),
                    );
                }
                Err(e) => {
                    ui.colored_label(Color32::DARK_RED, format!("Validation error: {e:#}"));
                }
                _ => (),
            };
            let can_submit =
                !locked && valid.as_ref().is_ok_and(|x| x.is_empty()) && game.score(m).is_ok();
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(!locked, egui::Button::new("Save draft"))
                    .clicked()
                {
                    m.state = MatchState::Scored;
                    self.dirty = true;
                }
                if ui
                    .add_enabled(can_submit, egui::Button::new("Submit final score"))
                    .clicked()
                {
                    submit = true;
                }
                if ui.button("Audience display").clicked() {
                    self.display = true;
                }
            });
            if submit {
                self.checkpoint();
                let r =
                    tournament_manager_rs::bracket::submit(&mut self.event, &game, self.selected);
                self.result(r);
            }
        });
        ui.separator();
        self.field_timer(ui);
    }
    fn field_timer(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.strong("Field Control");
            let secs = self.timer.remaining.as_secs_f64().ceil() as u64;
            ui.label(RichText::new(format!("{:02}:{:02}", secs / 60, secs % 60)).size(36.));
            ui.label(format!("{:?}", self.timer.phase));
            if ui
                .add_enabled(
                    self.timer.phase == Phase::Idle,
                    egui::Button::new("Start match"),
                )
                .clicked()
            {
                let prepared = if let Some(hw) = &mut self.hardware {
                    if hw.connected {
                        let m = self.event.matches.get(self.selected);
                        m.map(|m| {
                            hw.prepare(
                                m.round,
                                m.number,
                                self.event
                                    .config
                                    .fields()
                                    .iter()
                                    .position(|f| *f == m.field)
                                    .map(|i| i as u32 + 1)
                                    .unwrap_or(1),
                            )
                        })
                        .unwrap_or(Ok(()))
                    } else {
                        Ok(())
                    }
                } else {
                    Ok(())
                };
                match prepared {
                    Ok(()) => self.timer.start(),
                    Err(e) => self.status = e.to_string(),
                }
            }
            if ui.button("Pause").clicked() {
                self.timer.pause()
            }
            if ui.button("Resume / Driver").clicked() {
                self.timer.resume()
            }
            if ui.button("Abort").clicked() {
                self.timer.abort()
            }
            if ui.button("Reset").clicked() {
                self.reset_timer()
            }
        });
        if self.hardware.as_ref().is_some_and(|h| h.connected) {
            ui.label("V5 controller connected");
        } else {
            ui.label("No field controller connected. Connect one in Setup → Field Hardware.");
        }
    }
    fn ranks(&mut self, ui: &mut egui::Ui) {
        let game = self.game();
        ui.heading("Qualification Rankings");
        ui.label("Only submitted qualification scores count. Surrogate appearances are excluded.");
        match rankings(&self.event, &game) {
            Err(e) => {
                ui.colored_label(Color32::DARK_RED, format!("{e:#}"));
            }
            Ok(rows) => {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    egui::Grid::new("ranks")
                        .striped(true)
                        .min_col_width(78.)
                        .show(ui, |ui| {
                            for s in [
                                "Rank",
                                "Team",
                                "Played",
                                "W–L–T",
                                "WP",
                                "AP",
                                "SP",
                                "Avg score",
                            ] {
                                ui.strong(s);
                            }
                            ui.end_row();
                            for (i, r) in rows.iter().enumerate() {
                                for v in [
                                    (i + 1).to_string(),
                                    self.event.team_number(r.team),
                                    r.played.to_string(),
                                    format!("{}–{}–{}", r.wins, r.losses, r.ties),
                                    format!("{:.2}", r.wp / r.played.max(1) as f64),
                                    format!("{:.2}", r.ap / r.played.max(1) as f64),
                                    format!("{:.2}", r.sp / r.played.max(1) as f64),
                                    format!("{:.2}", r.average_score),
                                ] {
                                    ui.label(v);
                                }
                                ui.end_row();
                            }
                        });
                });
            }
        }
    }
    fn schedule(&mut self, ui: &mut egui::Ui) {
        ui.heading("Match Schedule");
        ui.horizontal(|ui| {
            ui.label("Qualification rounds");
            ui.add(egui::DragValue::new(&mut self.rounds).range(1..=100));
            ui.label("Seed");
            ui.add(egui::DragValue::new(&mut self.seed));
            if ui.button("Generate schedule").clicked() {
                if self.event.matches.is_empty() {
                    self.generate()
                } else {
                    self.confirm_schedule = true
                }
            }
        });
        ui.label("Uses checked-in teams; balances appearances and reduces repeated partners/opponents. Extra slots are marked surrogate.");
        egui::ScrollArea::both().show(ui, |ui| {
            egui::Grid::new("schedule").striped(true).show(ui, |ui| {
                for h in [
                    "Round",
                    "Match",
                    "Red / Team 1",
                    "Blue / Team 2",
                    "Field",
                    "Time",
                    "State",
                ] {
                    ui.strong(h);
                }
                ui.end_row();
                let names = self
                    .event
                    .teams
                    .iter()
                    .map(|t| (t.id, t.number.clone()))
                    .collect::<std::collections::BTreeMap<_, _>>();
                for m in &mut self.event.matches {
                    ui.label(m.round.to_string());
                    ui.label(m.number.to_string());
                    for a in &m.alliances {
                        ui.label(
                            a.teams
                                .iter()
                                .map(|t| {
                                    format!(
                                        "{}{}",
                                        names.get(&t.team).cloned().unwrap_or_default(),
                                        if t.surrogate { "*" } else { "" }
                                    )
                                })
                                .collect::<Vec<_>>()
                                .join(", "),
                        );
                    }
                    self.dirty |= ui.text_edit_singleline(&mut m.field).changed();
                    self.dirty |= ui
                        .add(egui::TextEdit::singleline(&mut m.planned).desired_width(80.))
                        .changed();
                    ui.label(format!("{:?}", m.state));
                    ui.end_row();
                }
            });
        });
    }
    fn generate(&mut self) {
        if self.event.config.configured {
            self.checkpoint();
            self.event.config.schedule.qualification_rounds = self.rounds;
            let g = self.game();
            let r = tournament_manager_rs::setup::create_matches(&mut self.event, &g);
            self.selected = 0;
            self.result(r);
            return;
        }
        match schedule::generate(&self.event, &self.game(), self.rounds, self.seed) {
            Ok(ms) => {
                self.checkpoint();
                self.event.matches = ms;
                self.selected = 0;
                self.status = format!("Generated {} matches", self.event.matches.len())
            }
            Err(e) => self.status = format!("{e:#}"),
        }
    }
    fn skills(&mut self, ui: &mut egui::Ui) {
        ui.heading("Robot Skills Challenge");
        if !self.event.config.skills_driver && !self.event.config.skills_autonomous {
            ui.label("Skills challenges are disabled for this tournament.");
            return;
        }
        if !self.event.config.skills_driver {
            self.skill_prog = true
        } else if !self.event.config.skills_autonomous {
            self.skill_prog = false
        }
        let game = self.game();
        ui.horizontal(|ui| {
            team_picker(ui, "skills_team", &mut self.skill_team, &self.event.teams);
            ui.add_enabled(
                self.event.config.skills_driver && self.event.config.skills_autonomous,
                egui::Checkbox::new(&mut self.skill_prog, "Autonomous Coding"),
            );
            if ui.button("New attempt").clicked() {
                let (shared, vals) = game.defaults(true);
                let mut m = game.new_match(0, 30, &[self.skill_team]);
                m.values = shared;
                m.alliances = vec![AllianceInput {
                    teams: vec![Slot {
                        team: self.skill_team,
                        ..Default::default()
                    }],
                    values: vals[0].clone(),
                }];
                self.skill_input = Some(m);
            }
        });
        if let Some(m) = &mut self.skill_input {
            egui::ScrollArea::vertical()
                .max_height(300.)
                .show(ui, |ui| {
                    score_controls(ui, &game, m, true);
                });
            let idx = if self.skill_prog { 1 } else { 0 };
            let n = game.data["skills"].as_array().map(|a| a.len()).unwrap_or(0);
            let idx = idx.min(n.saturating_sub(1));
            let valid = game.run("check_skills_constraints", m, Some(idx as u32));
            let score = game.run("calc_skills_score", m, Some(idx as u32));
            match &score {
                Ok(s) => {
                    ui.label(
                        RichText::new(format!(
                            "Score: {:.0}",
                            s["alliances"][0]["score"].as_f64().unwrap_or(0.)
                        ))
                        .size(30.),
                    );
                }
                Err(e) => {
                    ui.colored_label(Color32::DARK_RED, format!("{e:#}"));
                }
            }
            if let Ok(v) = &valid {
                if !v.is_null() && v.as_array().is_some_and(|a| !a.is_empty()) {
                    ui.colored_label(Color32::DARK_RED, v.to_string());
                }
            }
            let attempts = self
                .event
                .skills
                .iter()
                .filter(|s| s.team == self.skill_team && s.programming == self.skill_prog)
                .count();
            let max = if self.event.config.configured {
                self.event.config.skills_max_attempts
            } else {
                game.data["skills"][idx]["maxAttempts"]
                    .as_u64()
                    .unwrap_or(3) as usize
            };
            let enabled = valid
                .is_ok_and(|v| v.is_null() || v.as_array().is_some_and(|a| a.is_empty()))
                && score.is_ok()
                && self.skill_team != 0
                && attempts < max;
            if ui
                .add_enabled(enabled, egui::Button::new("Record attempt"))
                .clicked()
            {
                let score = score.unwrap()["alliances"][0]["score"]
                    .as_f64()
                    .unwrap_or(0.) as i64;
                let stop = *m.alliances[0].values.get("skills_stop_time").unwrap_or(&0);
                self.checkpoint();
                self.event.skills.push(Skill {
                    team: self.skill_team,
                    programming: self.skill_prog,
                    score,
                    stop_time: stop,
                    attempt: attempts as u32 + 1,
                });
                self.skill_input = None;
            }
        }
        ui.separator();
        ui.strong("Skills rankings · best driving + best autonomous score");
        let mut rows = self
            .event
            .teams
            .iter()
            .map(|t| {
                let best = |p| {
                    self.event
                        .skills
                        .iter()
                        .filter(|s| s.team == t.id && s.programming == p)
                        .map(|s| s.score)
                        .max()
                        .unwrap_or(0)
                };
                (t.number.clone(), best(false), best(true))
            })
            .collect::<Vec<_>>();
        rows.sort_by_key(|x| std::cmp::Reverse(x.1 + x.2));
        egui::Grid::new("skillsranks").striped(true).show(ui, |ui| {
            for s in ["Rank", "Team", "Driving", "Autonomous", "Combined"] {
                ui.strong(s);
            }
            ui.end_row();
            for (i, (t, d, p)) in rows.iter().enumerate() {
                for x in [
                    (i + 1).to_string(),
                    t.clone(),
                    d.to_string(),
                    p.to_string(),
                    (d + p).to_string(),
                ] {
                    ui.label(x);
                }
                ui.end_row();
            }
        });
    }
    fn alliance(&mut self, ui: &mut egui::Ui) {
        if self.game().cooperative() {
            ui.heading("VEX IQ Finals");
            ui.label(
                "Adjacent qualification ranks form teamwork pairs. Lower-ranked pairs run first.",
            );
            ui.horizontal(|ui| {
                ui.label("Finalist teams");
                ui.add(egui::DragValue::new(&mut self.finalists).range(2..=100));
                if ui.button("Create finals matches").clicked() {
                    self.checkpoint();
                    let g = self.game();
                    let r = tournament_manager_rs::bracket::iq_finals(
                        &mut self.event,
                        &g,
                        self.finalists,
                    );
                    self.result(r)
                }
                if ui.button("Unlock qualification / remove finals").clicked() {
                    self.checkpoint();
                    tournament_manager_rs::bracket::unlock(&mut self.event);
                    self.selected = 0;
                }
            });
            let g = self.game();
            let mut rows = self
                .event
                .matches
                .iter()
                .filter(|m| m.round == 15)
                .filter_map(|m| {
                    if m.state != MatchState::Submitted {
                        return None;
                    }
                    g.score(m).ok().map(|s| {
                        (
                            m,
                            s["alliances"][0]["score"].as_f64().unwrap_or(0.),
                            m.values.get("stoptime").copied().unwrap_or(0),
                        )
                    })
                })
                .collect::<Vec<_>>();
            rows.sort_by(|a, b| b.1.total_cmp(&a.1).then(b.2.cmp(&a.2)));
            for (i, (m, score, stop)) in rows.iter().enumerate() {
                ui.label(format!(
                    "{}. {} · score {:.0} · stop {}",
                    i + 1,
                    m.alliances
                        .iter()
                        .flat_map(|a| &a.teams)
                        .map(|t| self.event.team_number(t.team))
                        .collect::<Vec<_>>()
                        .join(" + "),
                    score,
                    stop
                ));
            }
            ui.small("Tied finals are displayed in score/stop-time order; automatic tie-break rounds are outside this proof of concept.");
            return;
        }

        ui.heading("Alliance Selection");
        ui.label("Seed captains from qualification ranks, then select partners. Each team may belong to one alliance.");
        if ui.button("Seed alliance captains").clicked() {
            match rankings(&self.event, &self.game()) {
                Ok(rows) => {
                    self.checkpoint();
                    self.event.alliances = rows
                        .iter()
                        .filter(|r| r.played > 0)
                        .take(self.event.config.elimination_alliances)
                        .map(|r| vec![r.team])
                        .collect();
                }
                Err(e) => self.status = format!("{e:#}"),
            }
        }
        let mut used = self
            .event
            .alliances
            .iter()
            .flatten()
            .copied()
            .collect::<std::collections::HashSet<_>>();
        for (i, a) in self.event.alliances.iter_mut().enumerate() {
            ui.horizontal(|ui| {
                ui.strong(format!("Alliance {}", i + 1));
                for t in a.iter() {
                    ui.label(
                        self.event
                            .teams
                            .iter()
                            .find(|x| x.id == *t)
                            .map(|x| x.number.clone())
                            .unwrap_or_default(),
                    );
                }
                let mut pick = 0;
                let options = self
                    .event
                    .teams
                    .iter()
                    .filter(|t| !used.contains(&t.id))
                    .cloned()
                    .collect::<Vec<_>>();
                if a.len() < 2 {
                    team_picker(ui, format!("alliance{i}"), &mut pick, &options);
                } else {
                    ui.label("2 teams selected");
                }
                if pick != 0 && a.len() < 2 {
                    a.push(pick);
                    used.insert(pick);
                    self.dirty = true;
                }
            });
        }
        ui.horizontal(|ui| {
            ui.label("Wins required per series");
            ui.add(egui::DragValue::new(&mut self.event.elimination_wins).range(1..=2));
            if ui.button("Create elimination bracket").clicked() {
                self.checkpoint();
                let g = self.game();
                let wins = self.event.elimination_wins;
                let r = tournament_manager_rs::bracket::start(&mut self.event, &g, wins);
                self.result(r)
            }
            if ui.button("Unlock qualification / remove bracket").clicked() {
                self.checkpoint();
                tournament_manager_rs::bracket::unlock(&mut self.event);
                self.selected = 0;
            }
        });
        for series in &self.event.bracket {
            ui.label(format!(
                "Series {} · round {} · {} vs {} · {}–{}{}",
                series.id,
                series.round,
                series.teams[0]
                    .iter()
                    .map(|t| self.event.team_number(*t))
                    .collect::<Vec<_>>()
                    .join(" / "),
                series.teams[1]
                    .iter()
                    .map(|t| self.event.team_number(*t))
                    .collect::<Vec<_>>()
                    .join(" / "),
                series.wins[0],
                series.wins[1],
                series
                    .winner
                    .map(|w| format!(" · winner {}", w + 1))
                    .unwrap_or_default()
            ));
        }
    }

    fn awards(&mut self, ui: &mut egui::Ui) {
        ui.heading("Awards");
        ui.horizontal(|ui| {
            ui.text_edit_singleline(&mut self.award_name);
            team_picker(ui, "awardteam", &mut self.award_team, &self.event.teams);
            if ui.button("Add award").clicked() && !self.award_name.trim().is_empty() {
                self.checkpoint();
                self.event.awards.push(Award {
                    name: self.award_name.trim().into(),
                    team: self.award_team,
                    ..Default::default()
                });
                self.award_name.clear();
            }
        });
        let names = self
            .event
            .teams
            .iter()
            .map(|t| (t.id, t.number.clone()))
            .collect::<std::collections::BTreeMap<_, _>>();
        egui::Grid::new("awards").striped(true).show(ui, |ui| {
            for a in &mut self.event.awards {
                self.dirty |= ui.text_edit_singleline(&mut a.name).changed();
                ui.label(names.get(&a.team).cloned().unwrap_or_default());
                self.dirty |= ui.text_edit_singleline(&mut a.recipient).changed();
                self.dirty |= ui.text_edit_singleline(&mut a.description).changed();
                ui.end_row();
            }
        });
    }
    fn event_options(&mut self, ui: &mut egui::Ui) {
        ui.heading("Tournament Setup");
        egui::Grid::new("setup").show(ui, |ui| {
            ui.label("Event name");
            self.dirty |= ui.text_edit_singleline(&mut self.event.name).changed();
            ui.end_row();
            ui.label("Venue");
            self.dirty |= ui.text_edit_singleline(&mut self.event.venue).changed();
            ui.end_row();
            ui.label("Date");
            self.dirty |= ui.text_edit_singleline(&mut self.event.date).changed();
            ui.end_row();
            ui.label("Game");
            let old = self.event.game.clone();
            egui::ComboBox::from_id_salt("game")
                .width(450.)
                .selected_text(self.game().title())
                .show_ui(ui, |ui| {
                    for g in &self.games {
                        ui.selectable_value(
                            &mut self.event.game,
                            g.id().into(),
                            format!("{} [{}]", g.title(), g.id()),
                        );
                    }
                });
            if old != self.event.game {
                if self.event.matches.is_empty() {
                    self.dirty = true;
                    self.skill_input = None;
                    self.reset_timer()
                } else {
                    self.event.game = old;
                    self.status =
                        "Start a new tournament to change game after generating matches".into();
                }
            }
            ui.end_row();
        });
        ui.separator();
        ui.heading("Pit announcements");
        self.dirty |= ui
            .text_edit_multiline(&mut self.event.announcements)
            .changed();
        ui.separator();
        ui.heading("Local viewer");
        ui.label("A read-only browser viewer for event teams and match schedules. Original TM network protocol is not implemented.");
        if self.server.is_none() {
            if ui
                .button("Start tournament LAN viewer (port 8790)")
                .clicked()
            {
                match start_viewer(self.event.config.password_hash.clone()) {
                    Ok(s) => {
                        self.server = Some(s);
                        self.status = "LAN viewer active on port 8790; remote computers require the tournament password".into()
                    }
                    Err(e) => self.status = format!("{e:#}"),
                }
            }
        } else {
            ui.hyperlink("http://127.0.0.1:8790");
        }
        ui.separator();
        self.hardware_options(ui);
        ui.label("The V5 adapter uses the original app’s installed driver. Original TM remote-client protocol is separate from this app’s LAN viewer.");
    }
    fn hardware_options(&mut self, ui: &mut egui::Ui) {
        ui.heading("Field Hardware");
        ui.label("VEX V5 field controller — original driver adapter");
        ui.horizontal_wrapped(|ui| {
            ui.add(egui::TextEdit::singleline(&mut self.hardware_path).desired_width(650.));
            if ui.button("Browse driver…").clicked() {
                if let Some(p) = rfd::FileDialog::new().pick_file() {
                    self.hardware_path = p.to_string_lossy().into();
                }
            }
        });
        if ui.button("Discover V5 controllers").clicked() {
            match tournament_manager_rs::hardware::V5::discover(std::path::Path::new(
                &self.hardware_path,
            )) {
                Ok(hw) => {
                    self.status = format!("Found {} V5 controller(s)", hw.count);
                    self.hardware = Some(hw)
                }
                Err(e) => self.status = format!("{e:#}"),
            }
        }
        if let Some(hw) = &mut self.hardware {
            ui.label(format!("{} controller(s) detected", hw.count));
            ui.add(
                egui::DragValue::new(&mut self.hardware_index)
                    .range(0..=hw.count.saturating_sub(1)),
            );
            if !hw.connected {
                if ui
                    .add_enabled(
                        hw.count > 0,
                        egui::Button::new("Connect selected controller"),
                    )
                    .clicked()
                {
                    match hw.connect(self.hardware_index) {
                        Ok(()) => {
                            self.status = "V5 controller connected and disabled".into();
                            self.hardware_mode.clear()
                        }
                        Err(e) => self.status = e.to_string(),
                    }
                }
            } else {
                if ui.button("Disable & disconnect").clicked() {
                    hw.disconnect();
                    self.hardware_mode.clear();
                }
                ui.label(
                    "Start, pause, resume, abort and timer controls now command this controller.",
                );
            }
            ui.collapsing("Live controller status", |ui| {
                ui.monospace(
                    serde_json::to_string_pretty(&self.hardware_status).unwrap_or_default(),
                );
            });
        }
        ui.small("The hardware adapter is supported by the recovered driver API; controller behavior still needs an on-device acceptance test.");
    }
    fn sync_hardware(&mut self) {
        let Some(hw) = &mut self.hardware else { return };
        if !hw.connected {
            return;
        }
        let mode = match self.timer.phase {
            Phase::Autonomous => "auto",
            Phase::Driver => "driver",
            Phase::Idle => "idle",
            _ => "disable",
        };
        let seconds = self.timer.remaining.as_secs_f64().ceil() as u64;
        if self.hardware_last.elapsed() < Duration::from_millis(250)
            && self.hardware_mode == mode
            && self.hardware_seconds == seconds
        {
            return;
        }
        self.hardware_last = Instant::now();
        let result = (|| -> Result<()> {
            if self.hardware_mode != mode {
                hw.control(mode, seconds)?;
                self.hardware_mode = mode.into();
            } else if self.hardware_seconds != seconds {
                hw.request(&format!("timer {}", seconds.saturating_mul(1000)))?;
            }
            self.hardware_seconds = seconds;
            self.hardware_status = hw.status()?;
            Ok(())
        })();
        if let Err(e) = result {
            self.timer.abort();
            let _ = hw.control("disable", 0);
            hw.connected = false;
            self.status = format!("Field hardware error: {e:#}");
        }
    }
    fn display_options(&mut self, ui: &mut egui::Ui) {
        ui.heading("Displays");
        ui.collapsing("Sponsor logos for pit displays", |ui| {
            if ui.button("Add sponsor logos…").clicked() {
                if let Some(paths) = rfd::FileDialog::new()
                    .add_filter("Images", &["png", "jpg", "jpeg"])
                    .pick_files()
                {
                    for path in paths {
                        match std::fs::read(&path) {
                            Ok(bytes)
                                if bytes.len() <= 8 * 1024 * 1024
                                    && image::load_from_memory(&bytes).is_ok() =>
                            {
                                self.event.config.sponsors.push(Sponsor {
                                    name: path
                                        .file_stem()
                                        .unwrap_or_default()
                                        .to_string_lossy()
                                        .into(),
                                    image: bytes,
                                });
                                self.dirty = true
                            }
                            _ => self.status = "Choose a valid PNG or JPEG under 8 MB".into(),
                        }
                    }
                }
            }
            let mut remove = None;
            for (i, sponsor) in self.event.config.sponsors.iter().enumerate() {
                ui.horizontal(|ui| {
                    ui.label(&sponsor.name);
                    if ui.button("Remove").clicked() {
                        remove = Some(i)
                    }
                });
            }
            if let Some(i) = remove {
                self.event.config.sponsors.remove(i);
                self.dirty = true;
            }
        });
        ui.label("Open each display as a separate window, move it to the desired monitor, then enable fullscreen.");
        ui.horizontal(|ui| {
            for kind in ["Pit", "Field", "Audience"] {
                if ui.button(format!("Add {kind} display")).clicked() {
                    let id = self
                        .event
                        .config
                        .displays
                        .iter()
                        .map(|d| d.id)
                        .max()
                        .unwrap_or(0)
                        + 1;
                    self.event.config.displays.push(DisplayConfig {
                        screen: "Rankings".into(),
                        id,
                        name: format!("{kind} Display {id}"),
                        kind: kind.into(),
                        field: self
                            .event
                            .config
                            .fields()
                            .first()
                            .cloned()
                            .unwrap_or_default(),
                        fullscreen: false,
                    });
                    self.dirty = true;
                }
            }
        });
        let fields = self.event.config.fields();
        let mut remove = None;
        egui::Grid::new("displaysettings")
            .striped(true)
            .show(ui, |ui| {
                for h in ["Name", "Type", "Field", "Fullscreen", "Window", ""] {
                    ui.strong(h);
                }
                ui.end_row();
                for d in &mut self.event.config.displays {
                    self.dirty |= ui.text_edit_singleline(&mut d.name).changed();
                    ui.vertical(|ui| {
                        ui.label(&d.kind);
                        if d.kind == "Pit" {
                            egui::ComboBox::from_id_salt(("pitscreen", d.id))
                                .selected_text(&d.screen)
                                .show_ui(ui, |ui| {
                                    for name in ["Rankings", "Skills"] {
                                        self.dirty |= ui
                                            .selectable_value(&mut d.screen, name.into(), name)
                                            .changed();
                                    }
                                });
                        }
                    });
                    if d.kind == "Field" {
                        egui::ComboBox::from_id_salt(("displayfield", d.id))
                            .selected_text(&d.field)
                            .show_ui(ui, |ui| {
                                for f in &fields {
                                    self.dirty |=
                                        ui.selectable_value(&mut d.field, f.clone(), f).changed();
                                }
                            });
                    } else {
                        ui.label("All / live match");
                    }
                    self.dirty |= ui.checkbox(&mut d.fullscreen, "").changed();
                    let open = self.open_displays.contains(&d.id);
                    if ui.button(if open { "Close" } else { "Open" }).clicked() {
                        if open {
                            self.open_displays.remove(&d.id);
                        } else {
                            self.open_displays.insert(d.id);
                        }
                    }
                    if ui.button("Remove").clicked() {
                        remove = Some(d.id)
                    }
                    ui.end_row();
                }
            });
        if let Some(id) = remove {
            self.event.config.displays.retain(|d| d.id != id);
            self.open_displays.remove(&id);
            self.dirty = true;
        }
        ui.separator();
        ui.label("Pit: upcoming matches, qualification rankings, skills and announcements.");
        ui.label(
            "Field: the selected field’s active or queued match, timer, teams and next matches.",
        );
        ui.label(
            "Audience: the live selected match, large scores, teams, timer and announcements.",
        );
    }
    fn reports(&mut self, ui: &mut egui::Ui) {
        ui.heading("Reports & Export");
        if self.hardware.as_ref().is_some_and(|h| h.connected)
            && matches!(self.timer.phase, Phase::Autonomous | Phase::Driver)
        {
            ui.label("Finish or pause the active field before opening print or save dialogs.");
            return;
        }
        for (by_team, label) in [
            (false, "Print schedule by time…"),
            (true, "Print schedule by team…"),
        ] {
            if ui.button(label).clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .set_file_name(if by_team {
                        "schedule-by-team.html"
                    } else {
                        "schedule-by-time.html"
                    })
                    .save_file()
                {
                    match std::fs::write(
                        &path,
                        tournament_manager_rs::reports::schedule(&self.event, by_team),
                    ) {
                        Ok(()) => {
                            let _ = std::process::Command::new("/usr/bin/open")
                                .arg(&path)
                                .spawn();
                            self.status =
                                "Schedule opened. Use its Print schedule button or Cmd+P.".into()
                        }
                        Err(e) => self.status = e.to_string(),
                    }
                }
            }
        }

        if ui.button("Export team list CSV…").clicked() {
            if let Some(p) = rfd::FileDialog::new()
                .set_file_name("teams.csv")
                .save_file()
            {
                let r = store::export_teams(&p, &self.event);
                self.result(r)
            }
        }
        if ui.button("Export tournament report HTML…").clicked() {
            if let Some(p) = rfd::FileDialog::new()
                .set_file_name("tournament-report.html")
                .save_file()
            {
                let r = std::fs::write(p, report(&self.event, &self.game())).map_err(Into::into);
                self.result(r)
            }
        }
        if ui.button("Export event JSON…").clicked() {
            if let Some(p) = rfd::FileDialog::new()
                .set_file_name("tournament.json")
                .save_file()
            {
                let r = serde_json::to_string_pretty(&self.event)
                    .map_err(Into::into)
                    .and_then(|s| std::fs::write(p, s).map_err(Into::into));
                self.result(r)
            }
        }
        ui.label("Open the HTML report in a browser to print team lists, match schedules, results, rankings, skills and awards.");
    }
}
fn team_picker(ui: &mut egui::Ui, id: impl std::hash::Hash, pick: &mut u32, teams: &[Team]) {
    egui::ComboBox::from_id_salt(id)
        .selected_text(
            teams
                .iter()
                .find(|t| t.id == *pick)
                .map(|t| t.number.as_str())
                .unwrap_or("Select team"),
        )
        .show_ui(ui, |ui| {
            for t in teams {
                ui.selectable_value(pick, t.id, &t.number);
            }
        });
}
fn score_controls(ui: &mut egui::Ui, g: &Game, m: &mut Match, skills: bool) -> bool {
    let mut changed = false;
    egui::Grid::new(if skills {
        "skillscontrols"
    } else {
        "matchcontrols"
    })
    .striped(true)
    .show(ui, |ui| {
        for c in g.controls(skills) {
            let raw = c["name"].as_str().unwrap();
            let name = raw.strip_prefix("skills_").unwrap_or(raw);
            let (label, map, key) = if let Some(key) = name.strip_prefix("a1_") {
                ("Red / Team 1", &mut m.alliances[0].values, key)
            } else if let Some(key) = name.strip_prefix("a2_") {
                if m.alliances.len() < 2 {
                    continue;
                }
                ("Blue / Team 2", &mut m.alliances[1].values, key)
            } else {
                ("Match", &mut m.values, name)
            };
            let val = map.entry(key.into()).or_insert(0);
            ui.label(format!("{label}: {}", key.replace('_', " ")));
            if c["kind"] == "wxCheckBox" || c["kind"] == "wxRadioButton" {
                let mut yes = *val != 0;
                if ui.checkbox(&mut yes, "").changed() {
                    *val = yes as i64;
                    changed = true
                }
            } else if c["kind"] == "wxChoice" {
                let choices = c["choices"].as_array().cloned().unwrap_or_default();
                let before = *val;
                egui::ComboBox::from_id_salt(raw)
                    .selected_text(
                        choices
                            .get(*val as usize)
                            .and_then(|v| v.as_str())
                            .unwrap_or("None"),
                    )
                    .show_ui(ui, |ui| {
                        for (i, v) in choices.iter().enumerate() {
                            ui.selectable_value(val, i as i64, v.as_str().unwrap_or(""));
                        }
                    });
                changed |= before != *val;
            } else {
                changed |= ui
                    .add(
                        egui::DragValue::new(val).range(
                            c["min"].as_i64().unwrap_or(0)..=c["max"].as_i64().unwrap_or(1000),
                        ),
                    )
                    .changed();
            }
            ui.end_row();
        }
    });
    changed
}
impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let now = Instant::now();
        self.timer.advance(now.duration_since(self.last));
        self.last = now;
        self.sync_hardware();
        ctx.request_repaint_after(Duration::from_millis(100));
        if self.server.is_some() {
            let mut public = self.event.clone();
            public.config.password_hash.clear();
            public.config.judge_password_hash.clear();
            public.config.inspector_password_hash.clear();
            public.config.scorekeeper_password_hash.clear();
            public.config.sponsors.clear();
            if let Ok(data) = serde_json::to_string(&public) {
                *self.server.as_ref().unwrap().lock().unwrap() =
                    (data, self.event.config.password_hash.clone());
            }
        }
        egui::TopBottomPanel::top("menu").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("New tournament…").clicked() {
                        self.new_event();
                        ui.close()
                    }
                    if ui.button("Open…").clicked() {
                        self.open();
                        ui.close()
                    }
                    if ui.button("Save").clicked() {
                        self.save(false);
                        ui.close()
                    }
                    if ui.button("Save as…").clicked() {
                        self.save(true);
                        ui.close()
                    }
                });
                if ui
                    .add_enabled(!self.undo.is_empty(), egui::Button::new("Undo"))
                    .clicked()
                {
                    self.event = self.undo.pop().unwrap();
                    self.dirty = true;
                }
                ui.separator();
                ui.strong(if self.startup {
                    "Tournament Manager"
                } else {
                    &self.event.name
                });
                if self.dirty {
                    ui.label("• unsaved");
                }
            });
            if !self.startup && self.wizard.is_none() {
                ui.separator();
                ui.horizontal_wrapped(|ui| {
                    for (i, t) in [
                        "Match Scoring",
                        "Rankings",
                        "Team List / Check In",
                        "Alliance Selection",
                        "Skills",
                        "Match Schedule",
                        "Awards",
                        "Reports",
                        "Setup",
                        "Displays",
                    ]
                    .iter()
                    .enumerate()
                    {
                        if ui.selectable_label(self.tab == i, *t).clicked() {
                            if i == 8 && self.event.config.protect_options && !self.options_unlocked
                            {
                                self.options_prompt = true
                            } else {
                                self.tab = i
                            }
                        }
                    }
                });
            }
        });
        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(&self.status);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        self.path
                            .as_ref()
                            .and_then(|p| p.file_name())
                            .map(|s| s.to_string_lossy().to_string())
                            .unwrap_or_default(),
                    );
                });
            });
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            if self.wizard.is_some() {
                self.wizard_view(ui)
            } else if self.startup {
                self.startup(ui)
            } else {
                match self.tab {
                    0 => self.scoring(ui),
                    1 => self.ranks(ui),
                    2 => self.teams(ui),
                    3 => self.alliance(ui),
                    4 => self.skills(ui),
                    5 => self.schedule(ui),
                    6 => self.awards(ui),
                    7 => self.reports(ui),
                    8 => self.event_options(ui),
                    _ => self.display_options(ui),
                }
            }
        });
        if self.options_prompt {
            let mut unlock = false;
            let mut cancel = false;
            egui::Window::new("Protected Options")
                .collapsible(false)
                .show(ctx, |ui| {
                    ui.label("Enter the tournament password to open Options.");
                    ui.add(egui::TextEdit::singleline(&mut self.options_password).password(true));
                    ui.colored_label(Color32::DARK_RED, &self.options_error);
                    ui.horizontal(|ui| {
                        unlock = ui.button("Open Options").clicked();
                        cancel = ui.button("Cancel").clicked();
                    });
                });
            if cancel {
                self.options_prompt = false;
                self.options_password.clear()
            }
            if unlock {
                if tournament_manager_rs::auth::verify(
                    &self.options_password,
                    &self.event.config.password_hash,
                ) {
                    self.options_unlocked = true;
                    self.options_prompt = false;
                    self.options_password.clear();
                    self.options_error.clear();
                    self.tab = 8;
                } else {
                    self.options_error = "Incorrect tournament password".into();
                }
            }
        }
        if self.remote_dialog {
            egui::Window::new("Remote server compatibility").open(&mut self.remote_dialog).show(ctx,|ui|{ui.label("Original TM remote-server authentication and protobuf RPC are recovered but have not been reimplemented. Use a local Rust tournament file in this build.");});
        }
        if self.confirm_schedule {
            egui::Window::new("Replace match schedule?")
                .collapsible(false)
                .show(ctx, |ui| {
                    ui.label(
                        "This replaces all matches and scores. An in-memory undo snapshot is kept.",
                    );
                    ui.horizontal(|ui| {
                        if ui.button("Replace schedule").clicked() {
                            self.generate();
                            self.confirm_schedule = false
                        }
                        if ui.button("Cancel").clicked() {
                            self.confirm_schedule = false
                        }
                    });
                });
        }
        if self.display {
            if let Some(d) = self
                .event
                .config
                .displays
                .iter()
                .find(|d| d.kind == "Audience")
            {
                self.open_displays.insert(d.id);
            }
            self.display = false;
        }
        for d in self.event.config.displays.clone() {
            if !self.open_displays.contains(&d.id) {
                continue;
            }
            let mut close = false;
            ctx.show_viewport_immediate(
                egui::ViewportId::from_hash_of(("display", d.id)),
                egui::ViewportBuilder::default()
                    .with_title(&d.name)
                    .with_inner_size([1100., 740.])
                    .with_fullscreen(d.fullscreen),
                |ctx, _| {
                    ctx.request_repaint_after(Duration::from_millis(200));
                    egui::CentralPanel::default()
                        .frame(egui::Frame::new().fill(Color32::from_rgb(12, 20, 37)))
                        .show(ctx, |ui| {
                            displays::render(
                                ui,
                                &self.event,
                                &self.game(),
                                &d,
                                self.selected,
                                &self.timer,
                            )
                        });
                    close = ctx.input(|i| i.viewport().close_requested());
                },
            );
            if close {
                self.open_displays.remove(&d.id);
            }
        }
    }
}
fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
fn report(e: &Event, g: &Game) -> String {
    let mut s = format!(
        "<!doctype html><meta charset=utf-8><title>{}</title><style>body{{font:15px system-ui;margin:40px;color:#17213b}}table{{border-collapse:collapse;width:100%;margin-bottom:32px}}td,th{{padding:8px;border-bottom:1px solid #ddd;text-align:left}}h1{{color:#0070c0}}@media print{{h2{{break-before:auto}}}}</style><h1>{}</h1><p>{} · {} · {}</p><h2>Teams</h2><table><tr><th>Number<th>Name<th>Organization<th>Check in<th>Inspection</tr>",
        esc(&e.name),
        esc(&e.name),
        esc(&g.title()),
        esc(&e.venue),
        esc(&e.date)
    );
    for t in &e.teams {
        s += &format!(
            "<tr><td>{}<td>{}<td>{}<td>{}<td>{}</tr>",
            esc(&t.number),
            esc(&t.name),
            esc(&t.organization),
            t.checked_in,
            t.inspected
        )
    }
    s += "</table><h2>Schedule & Results</h2><table><tr><th>Round<th>Match<th>Teams<th>Field / Time<th>Score<th>State</tr>";
    for m in &e.matches {
        let teams = m
            .alliances
            .iter()
            .map(|a| {
                a.teams
                    .iter()
                    .map(|t| esc(&e.team_number(t.team)))
                    .collect::<Vec<_>>()
                    .join(" / ")
            })
            .collect::<Vec<_>>()
            .join(" vs ");
        let score = g
            .score(m)
            .map(|v| {
                format!(
                    "{} – {}",
                    v["alliances"][0]["score"], v["alliances"][1]["score"]
                )
            })
            .unwrap_or_else(|_| "Unavailable".into());
        s += &format!(
            "<tr><td>{}<td>{}<td>{}<td>{} {}<td>{}<td>{:?}</tr>",
            m.round,
            m.number,
            teams,
            esc(&m.field),
            esc(&m.planned),
            score,
            m.state
        )
    }
    s += "</table><h2>Rankings</h2><table><tr><th>Rank<th>Team<th>Played<th>WP<th>AP<th>SP</tr>";
    if let Ok(rows) = rankings(e, g) {
        for (i, r) in rows.iter().enumerate() {
            s += &format!(
                "<tr><td>{}<td>{}<td>{}<td>{}<td>{}<td>{}</tr>",
                i + 1,
                esc(&e.team_number(r.team)),
                r.played,
                r.wp,
                r.ap,
                r.sp
            )
        }
    }
    s += "</table><h2>Skills Attempts</h2><table>";
    for r in &e.skills {
        s += &format!(
            "<tr><td>{}<td>{}<td>{}<td>{}</tr>",
            esc(&e.team_number(r.team)),
            if r.programming {
                "Autonomous"
            } else {
                "Driving"
            },
            r.attempt,
            r.score
        )
    }
    s += "</table><h2>Awards</h2><table>";
    for a in &e.awards {
        s += &format!(
            "<tr><td>{}<td>{}<td>{}<td>{}</tr>",
            esc(&a.name),
            esc(&e.team_number(a.team)),
            esc(&a.recipient),
            esc(&a.description)
        )
    }
    s += "</table>";
    s
}
fn start_viewer(password_hash: String) -> Result<Arc<Mutex<(String, String)>>> {
    let listener = TcpListener::bind("0.0.0.0:8790").context("Viewer port unavailable")?;
    let data = Arc::new(Mutex::new(("{}".to_string(), password_hash)));
    let shared = data.clone();
    std::thread::spawn(move || {
        for mut stream in listener.incoming().flatten() {
            let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
            let mut req = [0; 2048];
            let snapshot = shared.lock().unwrap().clone();
            let n = stream.read(&mut req).unwrap_or(0);
            let local = stream.peer_addr().is_ok_and(|a| a.ip().is_loopback());
            let request = String::from_utf8_lossy(&req[..n]);
            use base64::Engine;
            let credentials = request
                .lines()
                .find_map(|l| {
                    l.strip_prefix("Authorization: Basic ")
                        .or_else(|| l.strip_prefix("authorization: Basic "))
                })
                .and_then(|s| {
                    base64::engine::general_purpose::STANDARD
                        .decode(s.trim())
                        .ok()
                })
                .and_then(|b| String::from_utf8(b).ok());
            let pass = credentials
                .as_deref()
                .and_then(|s| s.split_once(':'))
                .map(|(_, p)| p);
            if !tournament_manager_rs::auth::remote_allowed(local, pass, &snapshot.1) {
                let _=stream.write_all(b"HTTP/1.1 401 Unauthorized\r\nWWW-Authenticate: Basic realm=\"Tournament Server\"\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
                continue;
            }

            let api = String::from_utf8_lossy(&req).starts_with("GET /api/event ");
            let (kind, body) = if api {
                ("application/json", snapshot.0.clone())
            } else {
                ("text/html", "<!doctype html><meta charset=utf-8><title>Tournament Viewer</title><style>body{font:16px system-ui;max-width:1000px;margin:40px auto;color:#17213b}td,th{padding:9px;text-align:left;border-bottom:1px solid #ddd}table{width:100%}h1{color:#0070c0}</style><h1 id=title>Tournament Viewer</h1><p id=info></p><h2>Match Schedule</h2><table id=matches></table><h2>Teams</h2><table id=teams></table><p id=announcement></p><script>async function refresh(){let e=await(await fetch('/api/event')).json();title.textContent=e.name;info.textContent=e.venue+' · '+e.date;announcement.textContent=e.announcements;let names=new Map(e.teams.map(t=>[t.id,t.number]));function table(el,headers,rows){el.replaceChildren();for(let row of [headers,...rows]){let tr=document.createElement('tr');for(let v of row){let td=document.createElement('td');td.textContent=v;tr.append(td)}el.append(tr)}}table(teams,['Number','Name','Check in','Inspection'],e.teams.map(t=>[t.number,t.name,t.checked_in,t.inspected]));table(matches,['Round','Match','Teams','Field','Time','State'],e.matches.map(m=>[m.round,m.number,m.alliances.map(a=>a.teams.map(t=>names.get(t.team)).join(' / ')).join(' vs '),m.field,m.planned,m.state]));}refresh();setInterval(refresh,2000)</script>".into())
            };
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: {kind}; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = stream.write_all(response.as_bytes());
        }
    });
    Ok(data)
}
fn configure_style(ctx: &egui::Context) {
    ctx.set_visuals(egui::Visuals::light());
    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = egui::vec2(14., 12.);
    style.spacing.interact_size = egui::vec2(120., 40.);
    style.spacing.button_padding = egui::vec2(16., 10.);
    for (kind, size) in [
        (egui::TextStyle::Heading, 32.),
        (egui::TextStyle::Body, 20.),
        (egui::TextStyle::Button, 20.),
        (egui::TextStyle::Small, 17.),
        (egui::TextStyle::Monospace, 19.),
    ] {
        style
            .text_styles
            .insert(kind, egui::FontId::proportional(size));
    }
    ctx.set_style(style);
}
fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1440., 1000.])
            .with_min_inner_size([900., 700.]),
        ..Default::default()
    };
    eframe::run_native(
        "Tournament Manager · Rust",
        options,
        Box::new(|cc| {
            cc.egui_ctx.set_visuals(egui::Visuals::light());
            configure_style(&cc.egui_ctx);
            let mut app = App::default();
            let args = std::env::args().collect::<Vec<_>>();
            if let Some(i) = args.iter().position(|a| a == "--demo") {
                app.load_demo(if args.get(i + 1).is_some_and(|s| s == "iq") {
                    "VIQC_2025_MixAndMatch"
                } else {
                    "VRC_2025_PushBack"
                });
            }
            Ok(Box::new(app))
        }),
    )
}

#[cfg(test)]
mod ui_tests {
    use super::*;
    #[test]
    fn startup_and_all_tabs_render_for_both_games() {
        let ctx = egui::Context::default();
        configure_style(&ctx);
        let mut app = App::default();
        let input = || egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1180., 820.),
            )),
            ..Default::default()
        };
        let out = ctx.run(input(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| app.startup(ui));
        });
        assert!(!out.shapes.is_empty());
        for id in ["VIQC_2025_MixAndMatch", "VRC_2025_PushBack"] {
            app.load_demo(id);
            for tab in 0..10 {
                let out = ctx.run(input(), |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| match tab {
                        0 => app.scoring(ui),
                        1 => app.ranks(ui),
                        2 => app.teams(ui),
                        3 => app.alliance(ui),
                        4 => app.skills(ui),
                        5 => app.schedule(ui),
                        6 => app.awards(ui),
                        7 => app.reports(ui),
                        8 => app.event_options(ui),
                        _ => app.display_options(ui),
                    });
                });
                assert!(!out.shapes.is_empty(), "{id} tab {tab}");
            }
        }
    }
}

#[cfg(test)]
mod walkthrough_tests {
    use super::*;
    #[test]
    fn every_walkthrough_step_and_display_layout_renders() {
        let ctx = egui::Context::default();
        configure_style(&ctx);
        let raw = || egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1180., 820.),
            )),
            ..Default::default()
        };
        for id in ["VIQC_2025_MixAndMatch", "VRC_2025_PushBack"] {
            let mut app = App::default();
            app.load_demo(id);
            let mut w = wizard::Wizard::default();
            w.event = app.event.clone();
            for step in 0..12 {
                w.step = step;
                let out = ctx.run(raw(), |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| w.ui(ui));
                });
                assert!(!out.shapes.is_empty(), "step {step}");
            }
            for d in &app.event.config.displays {
                let out = ctx.run(raw(), |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        displays::render(ui, &app.event, &app.game(), d, 0, &app.timer)
                    });
                });
                assert!(!out.shapes.is_empty(), "{}", d.kind);
            }
        }
    }
}

#[cfg(test)]
mod ui_render;

#[cfg(test)]
mod scrolling_tests {
    use super::*;
    #[test]
    fn pit_repeats_short_lists_without_a_blank_reset() {
        for time in [0., 30., 3000.] {
            let ctx = egui::Context::default();
            configure_style(&ctx);
            let mut app = App::default();
            app.load_demo("VRC_2025_PushBack");
            app.event.teams.truncate(4);
            app.event.matches.clear();
            let out = ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(1600., 1000.),
                    )),
                    time: Some(time),
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        displays::render(
                            ui,
                            &app.event,
                            &app.game(),
                            &app.event.config.displays[0],
                            0,
                            &app.timer,
                        )
                    });
                },
            );
            let repeated=out.shapes.iter().filter(|s|matches!(&s.shape,egui::epaint::Shape::Text(t)if t.galley.text()=="Robotics Team 1"&&s.clip_rect.contains(t.pos))).count();
            assert!(repeated >= 2, "list did not repeat at {time}: {repeated}");
        }
    }
}
