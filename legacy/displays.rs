use eframe::egui::{self, Color32, RichText};
use tournament_manager_rs::{
    model::*,
    schedule::Timer,
    scoring::{Game, rankings},
};
pub fn render(
    ui: &mut egui::Ui,
    e: &Event,
    g: &Game,
    d: &DisplayConfig,
    selected: usize,
    timer: &Timer,
) {
    *ui.visuals_mut() = egui::Visuals::dark();
    ui.visuals_mut().override_text_color = Some(Color32::WHITE);
    if d.kind == "Pit" {
        pit(ui, e, g, d);
        return;
    }
    ui.vertical_centered(|ui| {
        ui.heading(&e.name);
        ui.label(format!("{} · {}", d.name, g.title()));
    });
    ui.separator();
    let active = e.matches.get(selected);
    let chosen = if d.kind == "Field" {
        active.filter(|m| m.field == d.field).or_else(|| {
            e.matches
                .iter()
                .find(|m| m.field == d.field && m.state != MatchState::Submitted)
        })
    } else {
        active
    };
    if let Some(m) = chosen {
        let running = active.is_some_and(|a| a.id == m.id);
        let secs = if running {
            timer.remaining.as_secs_f64().ceil() as u64
        } else {
            0
        };
        ui.vertical_centered(|ui| {
            ui.label(
                RichText::new(format!(
                    "{} {} · {}",
                    round_name(m.round),
                    m.number,
                    m.field
                ))
                .size(30.),
            );
            ui.label(
                RichText::new(if running {
                    format!("{:02}:{:02}", secs / 60, secs % 60)
                } else {
                    "QUEUED".into()
                })
                .size(if d.kind == "Audience" { 100. } else { 76. }),
            );
            ui.label(if running {
                format!("{:?}", timer.phase)
            } else {
                "Waiting for field operator".into()
            });
            ui.label(if m.state == MatchState::Submitted {
                "Final score"
            } else {
                "Live / provisional score"
            });
        });
        match g.score(m) {
            Ok(score) => {
                ui.columns(2, |cols| {
                    for (i, col) in cols.iter_mut().enumerate() {
                        col.vertical_centered(|ui| {
                            let color = if i == 0 {
                                Color32::LIGHT_RED
                            } else {
                                Color32::LIGHT_BLUE
                            };
                            ui.colored_label(
                                color,
                                RichText::new(format!(
                                    "{:.0}",
                                    score["alliances"][i]["score"].as_f64().unwrap_or(0.)
                                ))
                                .size(if d.kind == "Audience" {
                                    100.
                                } else {
                                    72.
                                }),
                            );
                            ui.label(
                                RichText::new(
                                    m.alliances[i]
                                        .teams
                                        .iter()
                                        .map(|t| e.team_number(t.team))
                                        .collect::<Vec<_>>()
                                        .join(" / "),
                                )
                                .size(30.),
                            );
                            for t in &m.alliances[i].teams {
                                if t.dq || t.no_show {
                                    ui.colored_label(
                                        color,
                                        format!(
                                            "{} {} {}",
                                            e.team_number(t.team),
                                            if t.dq { "DQ" } else { "" },
                                            if t.no_show { "NO SHOW" } else { "" }
                                        ),
                                    );
                                }
                            }
                        });
                    }
                });
            }
            Err(err) => {
                ui.label(format!("Score unavailable: {err}"));
            }
        }
        if d.kind == "Field" {
            ui.separator();
            ui.heading("Next on this field");
            for next in e
                .matches
                .iter()
                .filter(|x| x.field == d.field && x.id != m.id && x.state != MatchState::Submitted)
                .take(3)
            {
                ui.label(match_line(e, next));
            }
        }
    } else {
        ui.vertical_centered(|ui| {
            ui.add_space(70.);
            ui.label(RichText::new("Waiting for a match").size(38.));
            if !d.field.is_empty() {
                ui.heading(&d.field);
            }
        });
    }
    ui.separator();
    ui.vertical_centered(|ui| {
        ui.label(RichText::new(&e.announcements).size(22.));
    });
}
fn pit(ui: &mut egui::Ui, e: &Event, g: &Game, d: &DisplayConfig) {
    let rect = ui.available_rect_before_wrap();
    ui.allocate_rect(rect, egui::Sense::hover());
    let p = ui.painter_at(rect);
    let w = rect.width();
    let h = rect.height();
    let scale = (h / 900.).clamp(0.7, 1.6);
    let at = |x: f32, y: f32| egui::pos2(rect.left() + x, rect.top() + y);
    for i in 0..32 {
        let a = i as f32 / 32.;
        let shade = (a * 36.) as u8;
        p.rect_filled(
            egui::Rect::from_min_max(at(0., a * h), at(w, (i + 1) as f32 / 32. * h)),
            0.,
            Color32::from_gray(shade),
        );
    }
    let font = |size: f32| egui::FontId::proportional(size * scale);
    p.text(
        at(22., 22. * scale),
        egui::Align2::LEFT_TOP,
        &e.name,
        font(23.),
        Color32::LIGHT_GRAY,
    );
    p.text(
        at(22., 75. * scale),
        egui::Align2::LEFT_TOP,
        if d.screen == "Skills" {
            "ROBOT SKILLS"
        } else {
            "RANKINGS"
        },
        font(47.),
        Color32::WHITE,
    );
    let logo = texture(
        ui.ctx(),
        if g.cooperative() {
            include_bytes!("../assets/iq-display.png")
        } else {
            include_bytes!("../assets/v5-display.png")
        },
        "Display program",
    );
    let lh = 90. * scale;
    let lw = lh * logo.size_vec2().x / logo.size_vec2().y;
    let lr = egui::Rect::from_min_size(at(w - lw - 28., 12. * scale), egui::vec2(lw, lh));
    p.image(
        logo.id(),
        lr,
        egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1., 1.)),
        Color32::WHITE,
    );
    let top = 205. * scale;
    let left = egui::Rect::from_min_max(at(16., top), at(w * 0.635, h - 18.));
    let right = egui::Rect::from_min_max(at(w * 0.647, top), at(w - 16., h - 18.));
    let cols = [0.015, 0.075, 0.165, 0.685, 0.775, 0.855, 0.935];
    for (i, name) in [
        "RANK",
        "TEAM",
        "",
        if d.screen == "Skills" {
            "SKILLS"
        } else if g.cooperative() {
            "AVG"
        } else {
            "WP"
        },
        "AP",
        "SP",
        "WLT",
    ]
    .iter()
    .enumerate()
    {
        p.text(
            egui::pos2(
                left.left() + left.width() * cols[i],
                left.top() - 25. * scale,
            ),
            egui::Align2::LEFT_TOP,
            *name,
            font(16.),
            Color32::WHITE,
        );
    }
    p.text(
        egui::pos2(right.left(), right.top() - 25. * scale),
        egui::Align2::LEFT_TOP,
        "MATCH SCHEDULE & RESULTS",
        font(17.),
        Color32::WHITE,
    );
    let mut rows = rankings(e, g).unwrap_or_default();
    if d.screen == "Skills" {
        for r in &mut rows {
            r.average_score = [false, true]
                .iter()
                .map(|v| {
                    e.skills
                        .iter()
                        .filter(|s| s.team == r.team && s.programming == *v)
                        .map(|s| s.score)
                        .max()
                        .unwrap_or(0)
                })
                .sum::<i64>() as f64;
        }
        rows.sort_by(|a, b| b.average_score.total_cmp(&a.average_score));
    }
    let row_h = 34. * scale;
    let logo_h = 180. * scale;
    let total = rows.len() as f32 * row_h + e.config.sponsors.len() as f32 * (logo_h + 12. * scale);
    let time = ui.input(|i| i.time) as f32;
    let offset = if total > 0. {
        (time * 27.).rem_euclid(total)
    } else {
        0.
    };
    let copies = if total > 0. {
        (left.height() / total).ceil() as usize + 2
    } else {
        1
    };
    let lp = p.with_clip_rect(left);
    let mut y = left.top() - offset;
    for _cycle in 0..copies {
        let mut sponsor = 0;
        for (index, r) in rows.iter().enumerate() {
            if !e.config.sponsors.is_empty() && index % 12 == 0 && sponsor < e.config.sponsors.len()
            {
                let s = &e.config.sponsors[sponsor];
                sponsor += 1;
                let dest = egui::Rect::from_min_size(
                    egui::pos2(left.left(), y),
                    egui::vec2(left.width(), logo_h),
                );
                lp.rect_filled(dest, 0., Color32::WHITE);
                let t = texture(ui.ctx(), &s.image, &s.name);
                let size = t.size_vec2();
                let factor = (dest.width() / size.x).min(dest.height() / size.y);
                let image = egui::Rect::from_center_size(dest.center(), size * factor);
                lp.image(
                    t.id(),
                    image,
                    egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1., 1.)),
                    Color32::WHITE,
                );
                y += logo_h + 12. * scale;
            }
            let rr = egui::Rect::from_min_size(
                egui::pos2(left.left(), y),
                egui::vec2(left.width(), row_h),
            );
            lp.rect_filled(
                rr,
                0.,
                if index % 2 == 0 {
                    Color32::from_gray(224)
                } else {
                    Color32::WHITE
                },
            );
            let team = e.teams.iter().find(|t| t.id == r.team);
            let name = team
                .map(|t| t.name.chars().take(28).collect::<String>())
                .unwrap_or_default();
            let n = r.played.max(1) as f64;
            let values = [
                (index + 1).to_string(),
                e.team_number(r.team),
                name,
                format!(
                    "{:.2}",
                    if g.cooperative() || d.screen == "Skills" {
                        r.average_score
                    } else {
                        r.wp / n
                    }
                ),
                format!("{:.2}", r.ap / n),
                format!("{:.2}", r.sp / n),
                format!("{}-{}-{}", r.wins, r.losses, r.ties),
            ];
            for (i, text) in values.iter().enumerate() {
                lp.text(
                    egui::pos2(left.left() + left.width() * cols[i], y + row_h * 0.5),
                    egui::Align2::LEFT_CENTER,
                    text,
                    font(if i == 2 { 19. } else { 18. }),
                    Color32::BLACK,
                );
            }
            y += row_h;
        }
        for s in e.config.sponsors.iter().skip(sponsor) {
            let dest = egui::Rect::from_min_size(
                egui::pos2(left.left(), y),
                egui::vec2(left.width(), logo_h),
            );
            lp.rect_filled(dest, 0., Color32::WHITE);
            let t = texture(ui.ctx(), &s.image, &s.name);
            let size = t.size_vec2();
            let factor = (dest.width() / size.x).min(dest.height() / size.y);
            lp.image(
                t.id(),
                egui::Rect::from_center_size(dest.center(), size * factor),
                egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1., 1.)),
                Color32::WHITE,
            );
            y += logo_h + 12. * scale;
        }
    }
    let matches = e
        .matches
        .iter()
        .filter(|m| m.state != MatchState::Submitted)
        .take(6)
        .collect::<Vec<_>>();
    let card_h = 80. * scale;
    for (i, m) in matches.iter().enumerate() {
        let y = right.top() + i as f32 * card_h;
        let card = egui::Rect::from_min_size(
            egui::pos2(right.left(), y),
            egui::vec2(right.width(), card_h),
        );
        p.rect_filled(
            card,
            0.,
            if i % 2 == 0 {
                Color32::from_gray(224)
            } else {
                Color32::WHITE
            },
        );
        let prefix = match m.round {
            1 => "P",
            2 => "Q",
            15 => "F",
            _ => "E",
        };
        p.text(
            egui::pos2(right.left() + 20. * scale, y + card_h * 0.5),
            egui::Align2::LEFT_CENTER,
            format!("{prefix}{}", m.number),
            font(26.),
            Color32::BLACK,
        );
        for (ai, a) in m.alliances.iter().enumerate() {
            for (ti, t) in a.teams.iter().enumerate() {
                p.text(
                    egui::pos2(
                        right.left() + right.width() * (0.34 + ti as f32 * 0.23),
                        y + card_h * (if ai == 0 { 0.30 } else { 0.72 }),
                    ),
                    egui::Align2::CENTER_CENTER,
                    e.team_number(t.team),
                    font(22.),
                    if ai == 0 {
                        Color32::from_rgb(205, 55, 55)
                    } else {
                        Color32::from_rgb(32, 140, 193)
                    },
                );
            }
        }
        p.text(
            egui::pos2(right.right() - 12., y + card_h * 0.28),
            egui::Align2::RIGHT_CENTER,
            &m.field,
            font(22.),
            Color32::BLACK,
        );
        p.text(
            egui::pos2(right.right() - 12., y + card_h * 0.72),
            egui::Align2::RIGHT_CENTER,
            display_time(&m.planned),
            font(20.),
            Color32::BLACK,
        );
    }
}
fn display_time(value: &str) -> String {
    let raw = value.split_whitespace().last().unwrap_or(value);
    let mut parts = raw.split(':');
    if let (Some(hour), Some(min)) = (
        parts.next().and_then(|s| s.parse::<u32>().ok()),
        parts.next(),
    ) {
        format!(
            "{}:{} {}",
            if hour % 12 == 0 { 12 } else { hour % 12 },
            min,
            if hour < 12 { "AM" } else { "PM" }
        )
    } else {
        value.into()
    }
}

fn match_line(e: &Event, m: &Match) -> String {
    format!(
        "{} {}  ·  {}  ·  {}\n{}",
        round_name(m.round),
        m.number,
        m.field,
        m.planned,
        m.alliances
            .iter()
            .map(|a| a
                .teams
                .iter()
                .map(|t| e.team_number(t.team))
                .collect::<Vec<_>>()
                .join(" / "))
            .collect::<Vec<_>>()
            .join("  vs  ")
    )
}
fn round_name(round: u32) -> &'static str {
    match round {
        1 => "Practice",
        2 => "Qualification",
        15 => "Finals",
        _ => "Elimination",
    }
}

pub fn texture(ctx: &egui::Context, bytes: &[u8], name: &str) -> egui::TextureHandle {
    use std::hash::{Hash, Hasher};
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    bytes.hash(&mut hash);
    let id = egui::Id::new(("image", hash.finish()));
    if let Some(t) = ctx.data_mut(|d| d.get_temp::<egui::TextureHandle>(id)) {
        return t;
    }
    let image = image::load_from_memory(bytes)
        .map(|i| i.thumbnail(1600, 1200).to_rgba8())
        .unwrap_or_else(|_| image::RgbaImage::new(1, 1));
    let size = [image.width() as usize, image.height() as usize];
    let t = ctx.load_texture(
        name,
        egui::ColorImage::from_rgba_unmultiplied(size, image.as_raw()),
        egui::TextureOptions::LINEAR,
    );
    ctx.data_mut(|d| d.insert_temp(id, t.clone()));
    t
}
