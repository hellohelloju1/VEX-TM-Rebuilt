use eframe::egui::{self, Color32, RichText};
use tournament_manager_rs::{model::*, scoring::Game, setup, store};
pub struct Wizard {
    pub event: Event,
    pub step: usize,
    pub password: String,
    pub confirmation: String,
    show_passwords: bool,
    pub error: String,
    pub finished: bool,
    pub cancelled: bool,
    generated_for: Option<String>,
    team: Team,
    pub judge_password: String,
    pub inspector_password: String,
    pub scorekeeper_password: String,
    generated_team_count: usize,
    team_backup: Vec<Team>,
    block: ScheduleBlock,
    editing_block: Option<usize>,
}
impl Default for Wizard {
    fn default() -> Self {
        let mut event = Event::default();
        event.name = String::new();
        setup::preset(&mut event.config, 1);
        Self {
            event,
            step: 0,
            password: String::new(),
            confirmation: String::new(),
            show_passwords: false,
            error: String::new(),
            finished: false,
            cancelled: false,
            generated_for: None,
            team: Team::default(),
            judge_password: String::new(),
            inspector_password: String::new(),
            scorekeeper_password: String::new(),
            generated_team_count: 16,
            team_backup: vec![],
            block: ScheduleBlock::default(),
            editing_block: None,
        }
    }
}
impl Wizard {
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.heading("Tournament Setup");
        ui.label(format!("Step {} of {}", self.step + 1, setup::STEPS.len()));
        ui.add(egui::ProgressBar::new(
            (self.step + 1) as f32 / setup::STEPS.len() as f32,
        ));
        ui.separator();
        let body_height = (ui.available_height() - 140.).max(250.);
        ui.horizontal_top(|ui| {
            ui.allocate_ui_with_layout(
                egui::vec2(255., body_height),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    ui.heading("VEX");
                    ui.label("TOURNAMENT SETUP");
                    ui.separator();
                    for (i, title) in setup::STEPS.iter().enumerate() {
                        ui.label(
                            RichText::new(format!(
                                "{}  {title}",
                                if i < self.step {
                                    "✓"
                                } else if i == self.step {
                                    "➜"
                                } else {
                                    "○"
                                }
                            ))
                            .color(if i == self.step {
                                Color32::from_rgb(120, 30, 170)
                            } else {
                                Color32::DARK_GRAY
                            }),
                        );
                    }
                },
            );
            ui.add_space(15.);
            ui.vertical(|ui| {
                ui.heading(setup::STEPS[self.step]);
                egui::ScrollArea::vertical()
                    .id_salt("wizardbody")
                    .max_height((body_height - 55.).max(180.))
                    .show(ui, |ui| self.page(ui));
            });
        });
        ui.separator();
        if !self.error.is_empty() {
            ui.colored_label(Color32::DARK_RED, &self.error);
        }
        ui.horizontal(|ui| {
            if ui.button("Cancel setup").clicked() {
                self.cancelled = true
            }
            if ui
                .add_enabled(self.step > 0, egui::Button::new("Back"))
                .clicked()
            {
                self.step -= 1;
                self.error.clear();
            }
            if ui
                .button(if self.step == 11 {
                    "Finish & save tournament…"
                } else {
                    "Next →"
                })
                .clicked()
            {
                if self.step==9&&self.event.config.schedule.blocks.is_empty(){self.error="Add at least one qualification schedule block.".into();return;}
                if self.step==10 && self.generated_for.as_deref()!=Some(setup::schedule_fingerprint(&self.event).as_str()){self.error="Create or regenerate matches for the current team list and schedule settings.".into();return;}
                match setup::validate_step(
                    &self.event,
                    self.step,
                    &self.password,
                    &self.confirmation,
                ) {
                    Ok(()) => {
                        self.error.clear();
                        if self.step==4{let size=if self.event.config.competition=="VEX IQ"{1}else{2};let max=self.event.teams.len()/size;while self.event.config.elimination_alliances>max&&self.event.config.elimination_alliances>2{self.event.config.elimination_alliances/=2;}self.event.config.iq_finalists=self.event.config.iq_finalists.min(self.event.teams.len()/2*2);}
                        if self.step==6{let pits=self.event.config.displays.iter().filter(|d|d.kind=="Pit").count();setup::configure_displays(&mut self.event.config,pits);}
                        if self.step == 11 {
                            self.finished = true
                        } else {
                            self.step += 1
                        }
                    }
                    Err(e) => self.error = e.to_string(),
                }
            }
        });
    }
    fn page(&mut self, ui: &mut egui::Ui) {
        match self.step + 1 {
            0 => {
                ui.label("Name the event you are creating.");
                ui.add(
                    egui::TextEdit::singleline(&mut self.event.name)
                        .hint_text("Event name")
                        .desired_width(450.),
                );
                ui.label("Venue");
                ui.text_edit_singleline(&mut self.event.venue);
                ui.label("Date");
                ui.text_edit_singleline(&mut self.event.date);
            }
            1 => {
                ui.label("Choose your competition program.");
                ui.horizontal_wrapped(|ui| {
                    for (label, id, bytes) in [
                        (
                            "VEX IQ",
                            "VIQC_2025_MixAndMatch",
                            include_bytes!("../assets/iq.png").as_slice(),
                        ),
                        (
                            "VEX V5",
                            "VRC_2025_PushBack",
                            include_bytes!("../assets/v5.png").as_slice(),
                        ),
                    ] {
                        ui.vertical(|ui| {
                            let texture = crate::displays::texture(ui.ctx(), bytes, label);
                            if ui
                                .add(egui::Button::image(egui::Image::new((
                                    texture.id(),
                                    egui::vec2(250., 115.),
                                ))))
                                .clicked()
                            {
                                self.event.config.competition = label.into();
                                self.event.game = id.into();
                                self.event.matches.clear();
                            }
                            ui.radio_value(&mut self.event.config.competition, label.into(), label);
                        });
                    }
                });
                self.event.game = if self.event.config.competition == "VEX IQ" {
                    "VIQC_2025_MixAndMatch".into()
                } else {
                    "VRC_2025_PushBack".into()
                };
            }
            2 => {
                ui.label("Select the type that most closely matches your tournament setup.");
                for (i, name) in setup::PRESETS.iter().enumerate() {
                    ui.group(|ui|{if ui.radio(self.event.config.tournament_type==*name,*name).clicked(){setup::preset(&mut self.event.config,i)}ui.label(["Small, unofficial scrimmage or in-school tournament. Generate teams and choose how many matches each team plays.","A small tournament using a single competition field. Standard options are provided automatically.","A tournament with two or more fields. Two-field defaults can be reviewed and changed.","A large event, or complete control over the available tournament configuration."][i]);});
                    ui.add_space(12.);
                }
            }
            3 => {
                ui.group(|ui| {
                    ui.heading("Event name");
                    ui.label("Used on printed reports, audience displays and pit displays.");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.event.name)
                            .desired_width(f32::INFINITY),
                    );
                });
                ui.group(|ui|{ui.heading("Tournament Manager password — required for network connections");ui.checkbox(&mut self.show_passwords,"Show passwords");ui.label("Other computers use this password when connecting to this server. Reopening this tournament locally does not require it.");ui.horizontal(|ui|{ui.add(egui::TextEdit::singleline(&mut self.password).password(!self.show_passwords).desired_width(260.));if ui.button("Create random password").clicked(){self.password=format!("{:012x}",rand::random::<u64>()&0xffffffffffff);self.confirmation=self.password.clone();self.show_passwords=true;}});ui.label("Confirm password");ui.add(egui::TextEdit::singleline(&mut self.confirmation).password(!self.show_passwords));ui.checkbox(&mut self.event.config.protect_options,"Protect Options / Wizard with this password");});
                for (title, value, description) in [
                    (
                        "Judge password — optional",
                        &mut self.judge_password,
                        "Reserved for judge connections.",
                    ),
                    (
                        "Inspector password — optional",
                        &mut self.inspector_password,
                        "Reserved for inspection connections.",
                    ),
                    (
                        "Scorekeeper password — optional",
                        &mut self.scorekeeper_password,
                        "Reserved for scorekeeper connections.",
                    ),
                ] {
                    ui.group(|ui| {
                        ui.heading(title);
                        ui.label(description);
                        ui.horizontal(|ui| {
                            ui.add(
                                egui::TextEdit::singleline(value)
                                    .password(!self.show_passwords)
                                    .desired_width(260.),
                            );
                            if ui.button("Create random password").clicked() {
                                *value = format!("{:012x}", rand::random::<u64>() & 0xffffffffffff);
                            }
                        });
                    });
                }
            }
            4 => {
                ui.label("2025/2026 games");
                for g in Game::all() {
                    if g.cooperative() == (self.event.config.competition == "VEX IQ") {
                        ui.radio_value(&mut self.event.game, g.id().into(), g.title());
                    }
                }
            }
            5 => {
                ui.label("Configure the teams competing at this event.");
                ui.group(|ui| {
                    ui.heading("Current Team List");
                    egui::ScrollArea::both()
                        .id_salt("teamroster")
                        .max_height(350.)
                        .show(ui, |ui| {
                            egui::Grid::new("wizardteams").striped(true).show(ui, |ui| {
                                for h in ["Number", "Age Group", "Team Name", "Organization"] {
                                    ui.strong(h);
                                }
                                ui.end_row();
                                for t in &mut self.event.teams {
                                    ui.add(
                                        egui::TextEdit::singleline(&mut t.number)
                                            .desired_width(85.),
                                    );
                                    ui.add(
                                        egui::TextEdit::singleline(&mut t.age_group)
                                            .desired_width(120.),
                                    );
                                    ui.add(
                                        egui::TextEdit::singleline(&mut t.name).desired_width(200.),
                                    );
                                    ui.add(
                                        egui::TextEdit::singleline(&mut t.organization)
                                            .desired_width(180.),
                                    );
                                    ui.end_row();
                                }
                            });
                        });
                    ui.label(format!("{} teams", self.event.teams.len()));
                });
                ui.group(|ui| {
                    ui.heading("Modify Team List");
                    ui.horizontal_wrapped(|ui| {
                        if ui.button("Import from CSV…").clicked() {
                            if let Some(p) = rfd::FileDialog::new()
                                .add_filter("CSV", &["csv"])
                                .pick_file()
                            {
                                match store::import_csv(&p, self.event.next_team_id()) {
                                    Ok(ts) => {
                                        self.team_backup = self.event.teams.clone();
                                        for mut t in ts {
                                            if !self
                                                .event
                                                .teams
                                                .iter()
                                                .any(|x| x.number.eq_ignore_ascii_case(&t.number))
                                            {
                                                t.id = self.event.next_team_id();
                                                self.event.teams.push(t)
                                            }
                                        }
                                    }
                                    Err(e) => self.error = e.to_string(),
                                }
                            }
                        }
                        ui.add(egui::DragValue::new(&mut self.generated_team_count).range(2..=500));
                        if ui.button("Generate Teams").clicked() {
                            self.team_backup = self.event.teams.clone();
                            self.event.teams = (1..=self.generated_team_count as u32)
                                .map(|id| Team {
                                    id,
                                    number: id.to_string(),
                                    name: format!("Team {id}"),
                                    ..Default::default()
                                })
                                .collect();
                        }
                        if ui.button("Revert").clicked() {
                            std::mem::swap(&mut self.event.teams, &mut self.team_backup)
                        }
                        if ui.button("Clear").clicked() {
                            self.team_backup = self.event.teams.clone();
                            self.event.teams.clear();
                        }
                    });
                    ui.horizontal_wrapped(|ui| {
                        ui.add(
                            egui::TextEdit::singleline(&mut self.team.number)
                                .hint_text("Number")
                                .desired_width(90.),
                        );
                        ui.add(
                            egui::TextEdit::singleline(&mut self.team.name)
                                .hint_text("Team name")
                                .desired_width(190.),
                        );
                        if ui.button("Add team").clicked() {
                            self.add_team(self.team.number.clone(), self.team.name.clone());
                            self.team = Team::default();
                        }
                    });
                });
            }
            6 => {
                if self.event.config.competition == "VEX IQ" {
                    ui.heading("Teamwork finals");
                    ui.label("Number of finalist teams");
                    ui.add(
                        egui::DragValue::new(&mut self.event.config.iq_finalists).range(2..=100),
                    );
                } else {
                    ui.group(|ui| {
                        ui.heading("Elimination alliances");
                        ui.label("Number of alliances competing in the elimination tournament");
                        ui.add(
                            egui::DragValue::new(&mut self.event.config.elimination_alliances)
                                .range(2..=32),
                        );
                    });
                    ui.group(|ui| {
                        ui.heading("Teams per elimination alliance");
                        ui.label("Each elimination match is strictly two teams versus two teams.");
                        ui.add(
                            egui::DragValue::new(&mut self.event.config.elimination_team_count)
                                .range(2..=2),
                        );
                    });
                    self.event.config.allow_unbalanced = false;
                    ui.label("Every elimination alliance must contain exactly two teams.");
                    ui.group(|ui| {
                        ui.heading("Alliance selection audience display");
                        ui.radio_value(
                            &mut self.event.config.selection_order,
                            "Team number".into(),
                            "Order by team number",
                        );
                        ui.radio_value(
                            &mut self.event.config.selection_order,
                            "Ranking".into(),
                            "Order by ranking",
                        );
                    });
                    ui.label("Wins needed to advance");
                    ui.add(egui::DragValue::new(&mut self.event.elimination_wins).range(1..=2));
                }
            }
            7 => {
                ui.label("Configure the field sets and the fields in each set.");
                if ui.button("Add field set").clicked() {
                    let n = self.event.config.field_sets.len() + 1;
                    let id = self.event.config.fields().len() + 1;
                    self.event.config.field_sets.push(FieldSet {
                        name: format!("Field Set {n}"),
                        fields: vec![format!("Field {id}")],
                    });
                }
                let mut remove = None;
                let total = self.event.config.fields().len();
                for (i, set) in self.event.config.field_sets.iter_mut().enumerate() {
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            ui.text_edit_singleline(&mut set.name);
                            if ui.button("Remove set").clicked() {
                                remove = Some(i)
                            }
                        });
                        for field in &mut set.fields {
                            ui.text_edit_singleline(field);
                        }
                        if ui.button(format!("Add field to {}", set.name)).clicked() {
                            set.fields.push(format!("Field {}", total + 1));
                        }
                    });
                }
                if let Some(i) = remove {
                    self.event.config.field_sets.remove(i);
                }
            }
            8 => {
                ui.checkbox(
                    &mut self.event.config.skills_driver,
                    "Enable Driving Skills",
                );
                ui.checkbox(
                    &mut self.event.config.skills_autonomous,
                    "Enable Autonomous Coding Skills",
                );
                ui.label("Skills fields");
                ui.add(egui::DragValue::new(&mut self.event.config.skills_fields).range(1..=20));
                ui.label("Maximum attempts per team per challenge");
                ui.add(
                    egui::DragValue::new(&mut self.event.config.skills_max_attempts).range(1..=20),
                );
            }
            9 => {
                let mut count = self
                    .event
                    .config
                    .displays
                    .iter()
                    .filter(|d| d.kind == "Pit")
                    .count();
                ui.label("Number of pit displays");
                if ui
                    .add(egui::DragValue::new(&mut count).range(0..=20))
                    .changed()
                {
                    setup::configure_displays(&mut self.event.config, count)
                }
                ui.label("Pit displays show upcoming matches, rankings, skills and announcements.");
                ui.small("A field display is also created for each field, plus an audience display. Display windows can be moved to other monitors.");
                for d in self
                    .event
                    .config
                    .displays
                    .iter_mut()
                    .filter(|d| d.kind == "Pit")
                {
                    ui.text_edit_singleline(&mut d.name);
                    egui::ComboBox::from_id_salt(("pitscreen", d.id))
                        .selected_text(&d.screen)
                        .show_ui(ui, |ui| {
                            for name in ["Rankings", "Skills"] {
                                ui.selectable_value(&mut d.screen, name.into(), name);
                            }
                        });
                    ui.checkbox(&mut d.fullscreen, "Open fullscreen");
                }
            }
            10 => {
                ui.group(|ui| {
                    ui.heading("New Schedule Block");
                    ui.horizontal_wrapped(|ui| {
                        ui.label("Date");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.block.date).desired_width(150.),
                        );
                        ui.label("Start");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.block.start).desired_width(90.),
                        );
                        ui.label("End");
                        ui.add(egui::TextEdit::singleline(&mut self.block.end).desired_width(90.));
                    });
                    ui.horizontal_wrapped(|ui| {
                        ui.label("Cycle time (seconds)");
                        ui.add(
                            egui::DragValue::new(&mut self.block.cycle_seconds).range(30..=3600),
                        );
                        egui::ComboBox::from_id_salt("blocktype")
                            .selected_text(if self.block.round == 1 {
                                "Practice"
                            } else {
                                "Qualification"
                            })
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut self.block.round, 1, "Practice");
                                ui.selectable_value(&mut self.block.round, 2, "Qualification");
                            });
                    });
                    ui.horizontal(|ui| {
                        if ui
                            .button(if self.editing_block.is_some() {
                                "Save block"
                            } else {
                                "Add block"
                            })
                            .clicked()
                        {
                            let valid = setup::parse_time(&self.block.start).and_then(|start| {
                                setup::parse_time(&self.block.end).map(|end| end > start)
                            });
                            if valid.is_ok_and(|v| v) {
                                if let Some(i) = self.editing_block.take() {
                                    self.event.config.schedule.blocks[i] = self.block.clone()
                                } else {
                                    self.event.config.schedule.blocks.push(self.block.clone())
                                }
                                self.error.clear();
                            } else {
                                self.error = "Enter valid start and end times (HH:MM).".into();
                            }
                        }
                        if ui.button("Cancel edit").clicked() {
                            self.editing_block = None;
                        }
                    });
                });
                ui.label("Double-click a block to edit it.");
                let mut remove = None;
                egui::Grid::new("blocks").striped(true).show(ui, |ui| {
                    for h in ["Type", "Start", "End", "Cycle", "Matches", ""] {
                        ui.strong(h);
                    }
                    ui.end_row();
                    for (i, b) in self.event.config.schedule.blocks.iter().enumerate() {
                        if ui
                            .selectable_label(
                                self.editing_block == Some(i),
                                if b.round == 1 {
                                    "Practice"
                                } else {
                                    "Qualification"
                                },
                            )
                            .double_clicked()
                        {
                            self.block = b.clone();
                            self.editing_block = Some(i);
                        }
                        ui.label(format!("{} {}", b.date, b.start));
                        ui.label(&b.end);
                        ui.label(format!("{} sec", b.cycle_seconds));
                        let count = setup::parse_time(&b.end)
                            .unwrap_or(0)
                            .saturating_sub(setup::parse_time(&b.start).unwrap_or(0))
                            / b.cycle_seconds.max(1)
                            * self.event.config.fields().len() as u64;
                        ui.label(count.to_string());
                        if ui.button("Remove").clicked() {
                            remove = Some(i)
                        }
                        ui.end_row();
                    }
                });
                if let Some(i) = remove {
                    self.event.config.schedule.blocks.remove(i);
                    self.editing_block = None;
                }
                if ui.button("Clear schedule blocks").clicked() {
                    self.event.config.schedule.blocks.clear();
                }
                ui.small("Block capacity is the maximum match count; match generation balances appearances within those periods.");
            }
            11 => {
                ui.label("Create practice and qualification matches separately. Existing matches of that type are replaced.");
                ui.horizontal_wrapped(|ui| {
                    for (round, label) in [
                        (1, "Create Practice Matches"),
                        (2, "Create Qualification Matches"),
                    ] {
                        ui.group(|ui| {
                            if ui.button(label).clicked() {
                                let g = Game::all()
                                    .into_iter()
                                    .find(|g| g.id() == self.event.game)
                                    .unwrap();
                                match setup::create_round(&mut self.event, &g, round) {
                                    Ok(()) => {
                                        self.error.clear();
                                        self.generated_for =
                                            Some(setup::schedule_fingerprint(&self.event));
                                    }
                                    Err(e) => self.error = e.to_string(),
                                }
                            }
                            let count = self
                                .event
                                .matches
                                .iter()
                                .filter(|m| m.round == round)
                                .count();
                            ui.label(format!(
                                "{count} matches ({:.2} per team)",
                                count as f64
                                    * if self.event.config.competition == "VEX IQ" {
                                        2.
                                    } else {
                                        4.
                                    }
                                    / self.event.teams.len().max(1) as f64
                            ));
                        });
                    }
                });
                egui::Grid::new("wizardmatches")
                    .striped(true)
                    .show(ui, |ui| {
                        for m in self.event.matches.iter().take(12) {
                            ui.label(if m.round == 1 {
                                "Practice"
                            } else {
                                "Qualification"
                            });
                            ui.label(m.number.to_string());
                            ui.label(&m.field);
                            ui.label(&m.planned);
                            ui.end_row();
                        }
                    });
            }
            12 => {
                ui.label(
                    "Choose the awards you plan to present. Recipients can be assigned later.",
                );
                for name in setup::AWARDS {
                    if self.event.config.competition == "VEX IQ" && name.starts_with("Tournament ")
                    {
                        continue;
                    }
                    if self.event.config.competition == "VEX V5" && name.starts_with("Teamwork ") {
                        continue;
                    }
                    let mut selected = self.event.awards.iter().any(|a| a.name == name);
                    if ui.checkbox(&mut selected, name).changed() {
                        if selected {
                            self.event.awards.push(Award {
                                name: name.into(),
                                ..Default::default()
                            })
                        } else {
                            self.event.awards.retain(|a| a.name != name)
                        }
                    }
                }
                ui.separator();
                ui.label(format!(
                    "{} · {} · {} teams · {} matches",
                    self.event.name,
                    self.event.config.competition,
                    self.event.teams.len(),
                    self.event.matches.len()
                ));
            }
            _ => (),
        }
    }
    fn add_team(&mut self, number: String, name: String) {
        let number = number.trim().to_uppercase();
        if number.is_empty() {
            return;
        }
        if self
            .event
            .teams
            .iter()
            .any(|t| t.number.eq_ignore_ascii_case(&number))
        {
            self.error = format!("Team {number} already exists");
            return;
        }
        self.event.teams.push(Team {
            id: self.event.next_team_id(),
            number,
            name: name.trim().into(),
            ..Default::default()
        });
    }
}
