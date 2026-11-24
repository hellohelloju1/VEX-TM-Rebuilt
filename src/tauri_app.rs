use anyhow::{Context, Result, bail};
use serde_json::{Value, json};
use std::{
    io::Write,
    net::TcpListener,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tauri::{Manager, WebviewWindow};
use tournament_manager_rs::{
    auth, bracket, display_settings,
    field_control::AUDIENCE_MODES,
    hardware,
    model::*,
    planner, reports,
    schedule::{Phase, Timer},
    scoring::{Game, rankings},
    setup, store,
};
struct Session {
    event: Event,
    path: Option<PathBuf>,
    new_path: Option<PathBuf>,
    dirty: bool,
    revision: u64,
    active: Option<u32>,
    saved: Option<Match>,
    saved_score: Value,
    bonus: Option<String>,
    usb: Value,
    audience: String,
    selected_field: String,
    auto_results: bool,
    timer: Timer,
    countdown: Option<Duration>,
    sounds: bool,
    last: Instant,
    hardware: Option<hardware::V5>,
    hw_mode: String,
    hw_seconds: u64,
    hw_status: Value,
    error: String,
    unlocked: bool,
    server: bool,
    ranks: Vec<Value>,
    active_score: Value,
}
impl Default for Session {
    fn default() -> Self {
        Self {
            event: Event::default(),
            path: None,
            new_path: None,
            dirty: false,
            revision: 0,
            active: None,
            saved: None,
            saved_score: Value::Null,
            bonus: None,
            usb: Value::Null,
            audience: "None".into(),
            selected_field: String::new(),
            auto_results: false,
            timer: Timer::new(15, 105),
            countdown: None,
            sounds: true,
            last: Instant::now(),
            hardware: None,
            hw_mode: String::new(),
            hw_seconds: u64::MAX,
            hw_status: Value::Null,
            error: String::new(),
            unlocked: false,
            server: false,
            ranks: vec![],
            active_score: Value::Null,
        }
    }
}
#[derive(Clone)]
struct Shared(Arc<Mutex<Session>>);
fn game(e: &Event) -> Result<Game> {
    Game::all()
        .into_iter()
        .find(|g| g.id() == e.game)
        .context("Choose a supported game")
}
fn public_event(e: &Event) -> Event {
    let mut e = e.clone();
    e.config.password_hash.clear();
    e.config.judge_password_hash.clear();
    e.config.inspector_password_hash.clear();
    e.config.scorekeeper_password_hash.clear();
    e
}
fn snapshot(s: &Session) -> Value {
    let scheduled_start_ms = s.event.matches.iter().find(|m|Some(m.id)==s.active)
        .and_then(|m|tournament_manager_rs::display_time::scheduled_start_ms(&m.planned,&s.event.date));
    let server_time_ms = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis() as u64;
    json!({"event":public_event(&s.event),"scheduled_start_ms":scheduled_start_ms,"server_time_ms":server_time_ms,"revision":s.revision,"dirty":s.dirty,"path":s.path.as_ref().and_then(|p|p.file_name()).map(|p|p.to_string_lossy()),"rankings":s.ranks,"recent_quals":display_settings::recent_qual_results(&s.event),"active":s.active,"scores":s.active_score,"field_control":{"saved":s.saved,"saved_score":s.saved_score,"audience":s.audience,"field":s.selected_field,"auto_results":s.auto_results,"sounds":s.sounds,"show_score_preview":s.event.config.show_score_preview},"timer":{"phase":if s.countdown.is_some(){"Countdown".to_string()}else{format!("{:?}",s.timer.phase)},"seconds":s.countdown.unwrap_or(s.timer.remaining).as_secs_f64().ceil() as u64},"hardware":{"driver":hardware::default_driver().to_string_lossy(),"connected":s.hardware.as_ref().is_some_and(|h|h.connected),"count":s.hardware.as_ref().map(|h|h.count).unwrap_or(0),"status":s.hw_status,"usb":s.usb},"error":s.error,"server":s.server,"platform":std::env::consts::OS,"password_set":!s.event.config.password_hash.is_empty(),"protected":s.event.config.protect_options&&!s.unlocked})
}
fn refresh_scores(s: &mut Session) {
    s.ranks=game(&s.event).and_then(|g|rankings(&s.event,&g)).map(|rs|rs.into_iter().map(|r| json!({"team":r.team,"played":r.played,"wp":r.wp,"ap":r.ap,"sp":r.sp,"wins":r.wins,"losses":r.losses,"ties":r.ties,"average_score":r.average_score})).collect()).unwrap_or_default();
    refresh_active_score(s);
}
fn refresh_active_score(s: &mut Session) {
    s.active_score = game(&s.event)
        .ok()
        .and_then(|g| {
            s.event
                .matches
                .iter()
                .find(|m| Some(m.id) == s.active)
                .and_then(|m| g.score(m).ok())
        })
        .unwrap_or(Value::Null);
}
fn changed(s: &mut Session) {
    s.dirty = true;
    s.revision += 1;
    if !running(s) { refresh_scores(s); } else { refresh_active_score(s); }
    autosave(s);
}
fn autosave(s: &mut Session) {
    if !s.dirty || !s.event.config.configured { return; }
    if s.path.is_none() {
        let dir = std::env::var_os("HOME").map(std::path::PathBuf::from).unwrap_or_else(std::env::temp_dir).join("Documents/Tournament Manager");
        if let Err(e) = std::fs::create_dir_all(&dir) { s.error = format!("Autosave failed: {e}"); return; }
        s.path = Some(dir.join(format!("Tournament-{}.tmrs", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos())));
    }
    match store::save(s.path.as_ref().unwrap(), &s.event) {
        Ok(()) => { s.dirty = false; },
        Err(e) => { s.error = format!("Autosave failed: {e}"); }
    }
}
fn remember_score(s: &mut Session,g: &Game,index: usize,draft: bool)->Result<()> {
    s.saved=Some(s.event.matches[index].clone());
    s.saved_score=g.score(s.saved.as_ref().unwrap())?;
    if s.auto_results && !draft { s.audience="Saved Match Results".into(); }
    Ok(())
}
fn running(s: &Session) -> bool {
    s.countdown.is_some()
        || matches!(
            s.timer.phase,
            Phase::Autonomous | Phase::Driver | Phase::Paused | Phase::AutoPause
        )
}
fn stop(s: &mut Session) {
    s.countdown = None;
    if let Some(h) = &mut s.hardware {
        let _ = h.control("disable", 0);
    }
    s.timer.abort();
    s.hw_mode.clear();
}
fn reset(s: &mut Session) {
    s.countdown = None;
    let mut a = 0;
    let mut d = 60;
    if let Ok(g) = game(&s.event) {
        for t in g.data["defaultMatchTimes"].as_array().into_iter().flatten() {
            match t["type"].as_str() {
                Some("AUTONOMOUS") => a = t["seconds"].as_u64().unwrap_or(0),
                Some("DRIVER_CTRL") => d = t["seconds"].as_u64().unwrap_or(60),
                _ => (),
            }
        }
    }
    if let Some(h) = &mut s.hardware {
        let _ = h.control("idle", 0);
    }
    s.timer = Timer::new(a, d);
    s.last = Instant::now();
    s.hw_mode.clear();
}
fn text<'a>(v: &'a Value, k: &str) -> &'a str {
    v[k].as_str().unwrap_or("")
}
fn num(v: &Value, k: &str) -> u32 {
    v[k].as_u64().unwrap_or(0) as u32
}
fn ensure_options(s: &Session) -> Result<()> {
    if s.event.config.protect_options && !s.unlocked {
        bail!("Unlock tournament options with the network password first")
    };
    Ok(())
}
fn ensure_edit(s: &Session) -> Result<()> {
    if running(s) {
        bail!("Finish or abort the current match before changing tournament setup")
    };
    if s.event.config.protect_options && !s.unlocked {
        bail!("Unlock tournament options with the network password first")
    };
    Ok(())
}
fn execute(action: &str, p: Value, shared: &Shared, app: &tauri::AppHandle) -> Result<Value> {
    // Native dialogs must not hold the session lock: the field-control worker keeps running.
    match action {
        "new_file" => {
            { let s=shared.0.lock().unwrap();if running(&s){bail!("Finish or abort the current match before creating a tournament");} }
            let Some(path)=rfd::FileDialog::new().add_filter("Tournament", &["tmrs"]).set_file_name("tournament.tmrs").save_file() else{return Ok(json!({"created":false}));};
            store::create(&path,&Event::default())?;
            shared.0.lock().unwrap().new_path=Some(path);
            return Ok(json!({"created":true}));
        }
        "save" => {
            let (existing, e) = {
                let s = shared.0.lock().unwrap();
                if running(&s) {
                    bail!("Finish or abort the current match before opening a save dialog")
                };
                (s.path.clone(), s.event.clone())
            };
            let path = if p["as_new"] == true || existing.is_none() {
                rfd::FileDialog::new()
                    .add_filter("Tournament", &["tmrs"])
                    .set_file_name("tournament.tmrs")
                    .save_file()
            } else {
                existing
            };
            if let Some(path) = path {
                store::save(&path, &e)?;
                let mut s = shared.0.lock().unwrap();
                s.path = Some(path);
                s.dirty = false;
            }
            return Ok(snapshot(&shared.0.lock().unwrap()));
        }
        "open" => {
            {
                let s = shared.0.lock().unwrap();
                if running(&s) {
                    bail!("Finish or abort the current match before opening another tournament")
                }
            }
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("Tournament", &["tmrs"])
                .pick_file()
            {
                let mut e = store::load(&path)?;
                e.config.elimination_team_count = 2;
                let mut s = shared.0.lock().unwrap();
                stop(&mut s);
                s.event = e;
                s.path = Some(path);
                s.new_path = None;
                s.active = None;
                s.saved = None;
                s.audience = "None".into();
                s.selected_field.clear();
                s.unlocked = false;
                s.dirty = false;
                s.revision += 1;
                refresh_scores(&mut s);
                reset(&mut s);
            };
            return Ok(snapshot(&shared.0.lock().unwrap()));
        }
        "import_teams" => {
            let start = {
                let s = shared.0.lock().unwrap();
                ensure_edit(&s)?;
                s.event.next_team_id()
            };
            let original = p["original"] == true;
            if let Some(path) = rfd::FileDialog::new().pick_file() {
                let ts = if original {
                    store::original_teams(&path, start)?
                } else {
                    store::import_csv(&path, start)?
                };
                return Ok(serde_json::to_value(ts)?);
            }
            return Ok(json!([]));
        }
        "sponsor" => {
            {
                let s = shared.0.lock().unwrap();
                ensure_options(&s)?;
            }
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("Sponsor image", &["png", "jpg", "jpeg"])
                .pick_file()
            {
                let bytes = std::fs::read(&path)?;
                if bytes.len() > 10_000_000 {
                    bail!("Use a sponsor image smaller than 10 MB")
                };
                image::load_from_memory(&bytes)?;
                return Ok(
                    json!({"name":path.file_stem().unwrap_or_default().to_string_lossy(),"image":bytes}),
                );
            }
            return Ok(Value::Null);
        }
        "export" => {
            let e = {
                let s = shared.0.lock().unwrap();
                if running(&s) {
                    bail!("Finish or abort the match before exporting")
                };
                s.event.clone()
            };
            let kind = text(&p, "kind");
            let ext = if kind == "teams" { "csv" } else { "json" };
            if let Some(path) = rfd::FileDialog::new()
                .set_file_name(format!("tournament.{ext}"))
                .save_file()
            {
                if kind == "teams" {
                    store::export_teams(&path, &e)?;
                } else {
                    std::fs::write(path, serde_json::to_string_pretty(&public_event(&e))?)?;
                }
            }
            return Ok(Value::Null);
        }
        "announcements" => {
            let mut s = shared.0.lock().unwrap();
            s.event.announcements = text(&p, "text").into();
            s.dirty = true;
            s.revision += 1;
            return Ok(snapshot(&s));
        }
        "report_pdf" => {
            let e=shared.0.lock().unwrap().event.clone();let kind=text(&p,"kind");
            reports::pdf_data(&e,kind)?;
            if let Some(path)=rfd::FileDialog::new().add_filter("PDF", &["pdf"]).set_file_name(format!("{kind}.pdf")).save_file(){reports::export_pdf(&path,&e,kind)?;return Ok(json!(true));}
            return Ok(json!(false));
        }
        "report" => {
            let e = {
                let s = shared.0.lock().unwrap();
                if running(&s) {
                    bail!("Finish or abort the current match before printing")
                };
                s.event.clone()
            };
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("Printable HTML", &["html"])
                .set_file_name("schedule.html")
                .save_file()
            {
                std::fs::write(&path, reports::schedule(&e, p["by_team"] == true))?;
                tournament_manager_rs::platform::open(&path)?;
            }
            return Ok(Value::Null);
        }
        "display_settings" => {
            let displays: Vec<DisplayConfig> = serde_json::from_value(p["displays"].clone())?;
            let sponsors: Vec<Sponsor> = serde_json::from_value(p["sponsors"].clone())?;
            let (old, result) = {
                let mut s = shared.0.lock().unwrap();
                ensure_options(&s)?;
                display_settings::validate(&s.event, &displays, &sponsors)?;
                let old = s.event.config.displays.clone();
                s.event.config.displays = displays.clone();
                s.event.config.sponsors = sponsors;
                changed(&mut s);
                (old, snapshot(&s))
            };
            for d in &old {
                if !displays.iter().any(|x| x.id == d.id) {
                    if let Some(w) = app.get_webview_window(&format!("display-{}", d.id)) {
                        let _ = w.close();
                    }
                }
            }
            for d in &displays {
                if let Some(w) = app.get_webview_window(&format!("display-{}", d.id)) {
                    w.set_title(if d.kind == "Field" { &d.field } else { &d.name })?;
                    w.set_fullscreen(d.fullscreen)?;
                }
            }
            return Ok(result);
        }
        "hardware_diagnostics" => {
            let usb = hardware::usb_diagnostics();
            let mut s = shared.0.lock().unwrap();
            s.usb = usb;
            return Ok(snapshot(&s));
        }
        "install_driver" => {
            tournament_manager_rs::platform::install_driver(p["name"].as_str().unwrap_or(""))?;
            return Ok(Value::Null);
        }
        "remote_connect" => {
            let address=p["address"].as_str().unwrap_or("").trim();
            if address.is_empty(){bail!("Enter the host computer's IP address or hostname")}
            let address=if address.contains("://"){address.to_owned()}else{format!("http://{address}")};
            let mut url=tauri::Url::parse(&address).context("Enter a valid server address")?;
            if !matches!(url.scheme(),"http"|"https") || url.host_str().is_none() || !url.username().is_empty() || url.password().is_some(){bail!("Use an HTTP or HTTPS server address without a password in the URL")}
            if url.port().is_none() && url.scheme()=="http"{url.set_port(Some(8790)).map_err(|_|anyhow::anyhow!("Invalid server port"))?;}
            url.set_path(if p["displays"]==true{"/"}else{"/TM/"});url.set_query(Some("client=1"));url.set_fragment(None);
            // External windows receive no local Tauri capability or dispatch access.
            if let Some(w)=app.get_webview_window("remote-tournament"){w.navigate(url)?;w.set_focus()?;}else{
                tauri::WebviewWindowBuilder::new(app,"remote-tournament",tauri::WebviewUrl::External(url))
                    .title("Remote Tournament").inner_size(1100.,780.).min_inner_size(390.,580.).build()?;
            }
            return Ok(Value::Null);
        }
        "field_control" => {
            if let Some(w) = app.get_webview_window("field-control") {
                w.set_focus()?;
            } else {
                tauri::WebviewWindowBuilder::new(
                    app,
                    "field-control",
                    tauri::WebviewUrl::App("index.html?control=1".into()),
                )
                .title("Match Field Set #1")
                .inner_size(1000., 720.)
                .min_inner_size(720., 580.)
                .build()?;
            }
            return Ok(Value::Null);
        }
        "display" => {
            let id = num(&p, "id");
            let d = {
                let s = shared.0.lock().unwrap();
                s.event
                    .config
                    .displays
                    .iter()
                    .find(|d| d.id == id)
                    .cloned()
                    .context("Unknown display")?
            };
            let label = format!("display-{id}");
            if let Some(w) = app.get_webview_window(&label) {
                w.set_focus()?;
            } else {
                tauri::WebviewWindowBuilder::new(
                    app,
                    &label,
                    tauri::WebviewUrl::App(format!("index.html?display={id}").into()),
                )
                .title(if d.kind == "Field" { &d.field } else { &d.name })
                .inner_size(1440., 810.)
                .fullscreen(d.fullscreen)
                .build()?;
            }
            return Ok(Value::Null);
        }
        "server" => {
            let mut s = shared.0.lock().unwrap();
            if s.server {
                return Ok(json!(true));
            };
            if s.event.config.password_hash.is_empty() {
                bail!("Set a network password before starting remote displays")
            };
            let listener = TcpListener::bind("0.0.0.0:8790")?;
            s.server = true;
            drop(s);
            let sh = shared.clone();
            let app = app.clone();
            std::thread::spawn(move || serve(listener, sh, app));
            return Ok(json!(true));
        }
        _ => (),
    }
    match action {
        "score" => {
            let m: Match = serde_json::from_value(p["match"].clone())?;
            let g = {
                let s = shared.0.lock().unwrap();
                game(&s.event)?
            };
            return Ok(json!({"score":g.score(&m)?,"errors":g.constraints(&m)?}));
        }
        "validate" => {
            let e: Event = serde_json::from_value(p["event"].clone())?;
            setup::validate_step(
                &e,
                num(&p, "step") as usize,
                text(&p, "password"),
                text(&p, "confirmation"),
            )?;
            return Ok(Value::Null);
        }
        "preset" => {
            let mut e: Event = serde_json::from_value(p["event"].clone())?;
            setup::preset(&mut e.config, num(&p, "index") as usize);
            return Ok(serde_json::to_value(e)?);
        }
        "plan" => {
            let e: Event = serde_json::from_value(p["event"].clone())?;
            let g = game(&e)?;
            return Ok(serde_json::to_value(planner::preview(&e, &g)?)?);
        }
        "generate" => {
            let mut e: Event = serde_json::from_value(p["event"].clone())?;
            let g = game(&e)?;
            setup::create_round(&mut e, &g, num(&p, "round"))?;
            return Ok(serde_json::to_value(e)?);
        }
        _ => (),
    }
    let mut s = shared.0.lock().unwrap();
    match action {
        "bootstrap" => {
            return Ok(
                json!({"state":snapshot(&s),"default_event":public_event(&Event::default()),"games":Game::all().into_iter().map(|g|json!({"id":g.id(),"title":g.title(),"season":g.data["season"],"cooperative":g.cooperative(),"team_size":g.team_size(),"controls":g.controls(false),"skills_controls":g.controls(true)})).collect::<Vec<_>>(),"steps":setup::STEPS,"presets":setup::PRESETS,"awards":setup::AWARDS}),
            );
        }
        "snapshot" => return Ok(snapshot(&s)),
        "unlock" => {
            if !auth::verify(text(&p, "password"), &s.event.config.password_hash) {
                bail!("Incorrect tournament password")
            };
            s.unlocked = true;
        }
        "commit" => {
            ensure_edit(&s)?;
            let mut e: Event = serde_json::from_value(p["event"].clone())?;
            game(&e)?;
            e.config.elimination_team_count = 2;
            for step in [4, 5, 6, 9, 10] {
                setup::validate_step(&e, step, "", "")?;
            }
            if e.name.trim().is_empty() {
                bail!("Enter an event name")
            }
            if p["new"] == true && text(&p, "password").is_empty() {
                bail!("Set a network password for remote computers")
            }
            e.config.password_hash = if !text(&p, "password").is_empty() {
                auth::hash(text(&p, "password"))?
            } else {
                s.event.config.password_hash.clone()
            };
            if p["new"] != true {
                e.config.judge_password_hash = s.event.config.judge_password_hash.clone();
                e.config.inspector_password_hash = s.event.config.inspector_password_hash.clone();
                e.config.scorekeeper_password_hash =
                    s.event.config.scorekeeper_password_hash.clone();
            }
            for (key, target) in [
                ("judge_password", &mut e.config.judge_password_hash),
                ("inspector_password", &mut e.config.inspector_password_hash),
                (
                    "scorekeeper_password",
                    &mut e.config.scorekeeper_password_hash,
                ),
            ] {
                if let Some(password) = p[key].as_str() {
                    *target = auth::hash(password)?
                }
            }
            if p["new"] == true {
                s.path = Some(s.new_path.take().or_else(||if !s.event.config.configured{s.path.clone()}else{None}).context("Create a tournament file before setup")?);
            }
            s.event = e;
            s.event.config.configured = true;
            s.active = None;
            s.saved = None;
            s.audience = "None".into();
            s.selected_field.clear();
            s.unlocked = true;
            changed(&mut s);
            reset(&mut s);
        }
        "teams" => {
            ensure_edit(&s)?;
            let teams: Vec<Team> = serde_json::from_value(p["teams"].clone())?;
            let mut seen = std::collections::HashSet::new();
            if teams.iter().any(|t| {
                t.id == 0 || t.number.trim().is_empty() || !seen.insert(t.number.to_uppercase())
            }) {
                bail!("Team numbers must be unique and nonempty")
            };
            if teams.len() != s.event.teams.len()
                || teams.iter().zip(&s.event.teams).any(|(a, b)| a.id != b.id)
            {
                if !s.event.matches.is_empty() {
                    bail!(
                        "Change the roster through Tournament setup so matches can be regenerated"
                    )
                }
            }
            s.event.teams = teams;
            changed(&mut s);
        }
        "pit_screen" => {
            display_settings::pit_screen(&mut s.event,num(&p,"id"),text(&p,"screen"))?;
            changed(&mut s);
        }
        "field_setting" => {
            if let Some(value) = p["show_score_preview"].as_bool() {
                s.event.config.show_score_preview=value;
                changed(&mut s);
            }
            if let Some(value) = p["sounds"].as_bool() {
                s.sounds = value;
            }
            if let Some(mode) = p["audience"].as_str() {
                if !AUDIENCE_MODES.contains(&mode) {
                    bail!("Unknown audience screen")
                };
                s.audience = mode.into();
            }
            if let Some(field) = p["field"].as_str() {
                if running(&s) {
                    bail!("Finish or abort the match before changing fields")
                };
                if !s.event.config.fields().contains(&field.to_string()) {
                    bail!("Unknown field")
                };
                if let Some(id) = s.active {
                    if let Some(mut m) = s
                        .event
                        .matches
                        .iter()
                        .find(|m| m.id == id && m.state != MatchState::Submitted)
                        .cloned()
                    {
                        let index = (s
                            .event
                            .config
                            .fields()
                            .iter()
                            .position(|f| f == field)
                            .unwrap_or(0)
                            + 1) as u32;
                        if let Some(hw) = &mut s.hardware {
                            if hw.connected {
                                hw.prepare(m.round, m.number, index)?;
                            }
                        }
                        m.field = field.into();
                        if let Some(target) = s.event.matches.iter_mut().find(|m| m.id == id) {
                            *target = m;
                        }
                        s.dirty = true;
                        s.revision += 1;
                    }
                }
                s.selected_field = field.into();
            }
            if let Some(auto) = p["auto_results"].as_bool() {
                s.auto_results = auto;
            }
        }
        "field_bonus" => {
            if game(&s.event)?.cooperative() {
                bail!("Autonomous bonus applies to V5 only")
            }
            let id = s.active.context("Load a match first")?;
            let m = s
                .event
                .matches
                .iter_mut()
                .find(|m| m.id == id && m.state != MatchState::Submitted)
                .context("Match is locked")?;
            let bonus = text(&p, "bonus");
            if !["None", "Tie", "Red", "Blue"].contains(&bonus) {
                bail!("Unknown autonomous result")
            }
            for (i, a) in m.alliances.iter_mut().enumerate() {
                a.values.insert(
                    "auto".into(),
                    i64::from((i == 0 && bonus == "Red") || (i == 1 && bonus == "Blue")),
                );
                a.values
                    .insert("auto_tie".into(), i64::from(bonus == "Tie"));
            }
            s.bonus = Some(bonus.into());
            changed(&mut s);
        }
        "send_to_fc" => {
            if running(&s) {
                bail!("Finish or abort the current match first")
            };
            let id = num(&p, "id");
            let mut m = s
                .event
                .matches
                .iter()
                .find(|m| m.id == id)
                .cloned()
                .context("Unknown match")?;
            if m.state == MatchState::Submitted {
                bail!("This match has already been submitted")
            };
            if !s.selected_field.is_empty() {
                m.field = s.selected_field.clone();
            }
            let field = (s
                .event
                .config
                .fields()
                .iter()
                .position(|f| *f == m.field)
                .unwrap_or(0)
                + 1) as u32;
            if let Some(h) = &mut s.hardware {
                if h.connected {
                    h.prepare(m.round, m.number, field)?;
                }
            }
            if let Some(old) = s.event.matches.iter_mut().find(|m| m.id == id) {
                old.field = m.field.clone();
            }

            s.active = Some(id);
            s.bonus = None;
            s.dirty = true;
            s.revision += 1;
            if s.selected_field.is_empty() {
                s.selected_field = m.field.clone();
            }
            s.audience = "Up Next".into();
            refresh_scores(&mut s);
            reset(&mut s);
        }
        "reopen_match" => {
            if running(&s) { bail!("Finish or abort the match before unlocking scores"); }
            bracket::reopen(&mut s.event, num(&p, "id"))?;
            s.saved = None;
            s.saved_score = Value::Null;
            changed(&mut s);
        }
        "submit" => {
            if running(&s) && p["draft"] != true {
                bail!("Finish or abort the match before submitting scores")
            };
            let mut m: Match = serde_json::from_value(p["match"].clone())?;
            let index = s
                .event
                .matches
                .iter()
                .position(|old| old.id == m.id)
                .context("Unknown match")?;
            let old = &s.event.matches[index];
            if old.state == MatchState::Submitted
                || serde_json::to_value(
                    &old.alliances
                        .iter()
                        .map(|a| a.teams.iter().map(|t| t.team).collect::<Vec<_>>())
                        .collect::<Vec<_>>(),
                )? != serde_json::to_value(
                    &m.alliances
                        .iter()
                        .map(|a| a.teams.iter().map(|t| t.team).collect::<Vec<_>>())
                        .collect::<Vec<_>>(),
                )?
            {
                bail!("Submitted matches or team assignments cannot be changed here")
            };
            if s.active == Some(m.id) && s.bonus.is_some() {
                for (a, old) in m
                    .alliances
                    .iter_mut()
                    .zip(&s.event.matches[index].alliances)
                {
                    for key in ["auto", "auto_tie"] {
                        if let Some(v) = old.values.get(key) {
                            a.values.insert(key.into(), *v);
                        }
                    }
                }
            }
            let g = game(&s.event)?;
            if p["draft"] != true && !g.constraints(&m)?.is_empty() {
                bail!("Correct the scoring constraint errors first")
            };
            g.score(&m)?;
            s.event.matches[index] = m;
            if p["draft"] == true {
                s.event.matches[index].state = MatchState::Scored;
            } else {
                bracket::submit(&mut s.event, &g, index)?;
            }
            remember_score(&mut s,&g,index,p["draft"] == true)?;
            changed(&mut s);
        }
        "timer" => {
            let before = s.timer.phase.clone();
            let was_counting = s.countdown.is_some();
            let cmd = text(&p, "command");
            match cmd {
                "start" => {
                    if s.active.is_none() {
                        bail!("Select a match first")
                    };
                    if s.event
                        .matches
                        .iter()
                        .find(|m| Some(m.id) == s.active)
                        .is_none_or(|m| m.state == MatchState::Submitted)
                    {
                        bail!("Select an unplayed match")
                    };
                    if s.timer.phase != Phase::Idle || s.countdown.is_some() {
                        bail!("Reset the timer before starting another match")
                    };
                    s.countdown = Some(Duration::from_secs(3));
                    s.audience = "In-Match".into();
                }
                "pause" => s.timer.pause(),
                "resume" => s.timer.resume(),
                "abort" => stop(&mut s),
                "reset" => reset(&mut s),
                _ => bail!("Unknown timer command"),
            };
            if s.sounds {
                let sound = match cmd {
                    "pause" if before != s.timer.phase => Some("Pause.wav"),
                    "resume" if before != s.timer.phase => Some("Start.wav"),
                    "abort" if was_counting || before != Phase::Idle => Some("Abort.wav"),
                    _ => None,
                };
                if let Some(name) = sound {
                    play_sound(name);
                }
            }
            s.last = Instant::now();
            if let Err(e) = sync_hardware(&mut s) {
                stop(&mut s);
                s.error = format!("Field disabled: {e:#}");
                return Err(e);
            }
        }
        "hardware" => {
            if running(&s) && !["rescan", "disconnect"].contains(&text(&p, "command")) {
                bail!("Finish or abort the match before changing controllers")
            };
            match text(&p, "command") {
                "rescan" => {
                    s.hardware
                        .as_mut()
                        .context("Discover controllers first")?
                        .rescan()?;
                }
                "discover" => {
                    s.hardware = None;
                    s.hardware = Some(hardware::V5::discover(&if text(&p,"path").is_empty(){hardware::default_driver()}else{PathBuf::from(text(&p,"path"))})?);
                }
                "connect" => {
                    s.hardware
                        .as_mut()
                        .context("Discover controllers first")?
                        .connect(num(&p, "index") as usize)?;
                }
                "disconnect" => {
                    stop(&mut s);
                    s.hardware = None;
                }
                _ => bail!("Unknown hardware command"),
            };
        }
        "match_unlock" => {
            ensure_edit(&s)?;
            bracket::reopen(&mut s.event, num(&p, "id"))?;
            changed(&mut s);
        }
        "selection_start" => {
            ensure_edit(&s)?;
            if !s.event.bracket.is_empty() { bail!("Eliminations already started"); }
            s.event.selection_started = true;
            s.audience = "Alliance Selection".into();
            changed(&mut s);
        }
        "selection_undo" => {
            ensure_edit(&s)?;
            bracket::undo_selection(&mut s.event)?;
            changed(&mut s);
        }
        "selection_pick" => {
            ensure_edit(&s)?;
            let g = game(&s.event)?;
            bracket::pick(&mut s.event, &g, num(&p, "team"))?;
            changed(&mut s);
        }
        "eliminations" => {
            ensure_edit(&s)?;
            let g = game(&s.event)?;
            let mut draft = s.event.clone();
            if g.cooperative() {
                let count = draft.config.iq_finalists;
                bracket::iq_finals(&mut draft, &g, count)?;
            } else {
                if !draft.selection_started { bail!("Start alliance selection first"); }
                let alliances = draft.alliances.clone();
                if alliances
                    .iter()
                    .flatten()
                    .any(|id| !draft.teams.iter().any(|t| t.id == *id))
                {
                    bail!("Unknown alliance team")
                };
                draft.alliances = alliances;
                let wins = draft.elimination_wins;
                bracket::start(&mut draft, &g, wins)?;
            }
            s.event = draft;
            changed(&mut s);
        }
        "unlock_eliminations" => {
            ensure_edit(&s)?;
            bracket::unlock(&mut s.event);
            changed(&mut s);
        }
        "skill_score" | "skill_record" => {
            let m: Match = serde_json::from_value(p["match"].clone())?;
            let g = game(&s.event)?;
            let programming = p["programming"] == true;
            let idx = if programming { 1 } else { 0 };
            let errors = g.run("check_skills_constraints", &m, Some(idx))?;
            let score = g.run("calc_skills_score", &m, Some(idx))?;
            if action == "skill_score" {
                return Ok(json!({"score":score,"errors":errors}));
            };
            if errors.as_array().is_some_and(|a| !a.is_empty()) {
                bail!("Correct skills scoring errors")
            };
            let team = num(&p, "team");
            if !s.event.teams.iter().any(|t| t.id == team) {
                bail!("Choose a team")
            };
            if (programming && !s.event.config.skills_autonomous)
                || (!programming && !s.event.config.skills_driver)
            {
                bail!("This skills challenge is disabled")
            };
            let attempt = s
                .event
                .skills
                .iter()
                .filter(|x| x.team == team && x.programming == programming)
                .count()
                + 1;
            if attempt > s.event.config.skills_max_attempts {
                bail!("Maximum attempts reached")
            };
            s.event.skills.push(Skill {
                team,
                programming,
                score: score["alliances"][0]["score"].as_f64().unwrap_or(0.) as i64,
                stop_time: *m.alliances[0].values.get("skills_stop_time").unwrap_or(&0),
                attempt: attempt as u32,
            });
            changed(&mut s);
        }
        "skill_template" => {
            let g = game(&s.event)?;
            let team = num(&p, "team");
            let (shared, vals) = g.defaults(true);
            let mut m = g.new_match(0, 30, &[team]);
            m.values = shared;
            m.alliances = vec![AllianceInput {
                teams: vec![Slot {
                    team,
                    ..Default::default()
                }],
                values: vals[0].clone(),
            }];
            return Ok(serde_json::to_value(m)?);
        }
        "awards" => {
            s.event.awards = serde_json::from_value(p["awards"].clone())?;
            changed(&mut s);
        }
        _ => bail!("Unknown action"),
    }
    autosave(&mut s);
    Ok(snapshot(&s))
}
fn sync_hardware(s: &mut Session) -> Result<()> {
    if !s.hardware.as_ref().is_some_and(|h| h.connected) {
        return Ok(());
    }
    let mode = match s.timer.phase {
        Phase::Autonomous => "auto",
        Phase::Driver => "driver",
        Phase::Idle => "idle",
        _ => "disable",
    };
    let seconds = s.timer.remaining.as_secs_f64().ceil() as u64;
    let h = s.hardware.as_mut().unwrap();
    if s.hw_mode != mode {
        h.control(mode, seconds)?;
        s.hw_mode = mode.into();
        s.hw_seconds = seconds;
    } else if s.hw_seconds != seconds {
        h.request(&format!("timer {}", seconds * 1000))?;
        s.hw_seconds = seconds;
    }
    s.hw_status = h.status()?;
    Ok(())
}
#[tauri::command]
async fn dispatch(
    window: WebviewWindow,
    app: tauri::AppHandle,
    state: tauri::State<'_, Shared>,
    action: String,
    payload: Option<Value>,
) -> std::result::Result<Value, String> {
    let field_allowed = window.label() == "field-control"
        && [
            "snapshot",
            "field_setting",
            "field_bonus",
            "timer",
        ]
        .contains(&action.as_str());
    if window.label() != "main" && !field_allowed && action != "snapshot" {
        return Err("Displays are read-only".into());
    }
    let shared = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        execute(&action, payload.unwrap_or(Value::Null), &shared, &app)
            .map_err(|e| format!("{e:#}"))
    })
    .await
    .map_err(|e| e.to_string())?
}
fn serve(listener: TcpListener, shared: Shared, app: tauri::AppHandle) {
    let mut sessions = tournament_manager_rs::web_sessions::Sessions::default();
    for stream in listener.incoming() {
        let Ok(mut stream) = stream else { continue };
        let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
        let _ = stream.set_write_timeout(Some(Duration::from_secs(2)));
        let request = match tournament_manager_rs::web_control::read(&mut stream) {
            Ok(r) => r,
            Err(_) => {
                let _ = write!(
                    stream,
                    "HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                );
                continue;
            }
        };
        let s = shared.0.lock().unwrap();
        let path = request.path.as_str();
        let operator = path == "/TM" || path.starts_with("/TM/") || path.starts_with("/TM?");
        let token = request
            .headers
            .get("cookie")
            .and_then(|c| tournament_manager_rs::web_sessions::cookie(c))
            .unwrap_or("");
        let allowed = sessions.valid(token, &s.event.config.password_hash);
        if path == "/TM/login" || path == "/login" {
            let valid = tournament_manager_rs::web_control::valid_post(&request)
                && serde_json::from_slice::<Value>(&request.body)
                    .ok()
                    .is_some_and(|v| {
                        tournament_manager_rs::web_control::authorized(
                            v["password"].as_str(),
                            &s.event.config.password_hash,
                        )
                    });
            let (status, cookie) = if valid {
                let token = sessions.create(&s.event.config.password_hash);
                (
                    "200 OK",
                    format!(
                        "Set-Cookie: tm_session={token}; HttpOnly; SameSite=Strict; Path=/; Max-Age=28800\r\nSet-Cookie: tm_session=; HttpOnly; SameSite=Strict; Path=/TM; Max-Age=0\r\n"
                    ),
                )
            } else {
                ("401 Unauthorized", String::new())
            };
            drop(s);
            let _ = write!(
                stream,
                "HTTP/1.1 {status}\r\n{cookie}Cache-Control: no-store\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            );
            continue;
        }
        if operator
            && path == "/TM/logout"
            && allowed
            && tournament_manager_rs::web_control::valid_post(&request)
        {
            sessions.remove(token);
            drop(s);
            let _ = write!(
                stream,
                "HTTP/1.1 200 OK\r\nSet-Cookie: tm_session=; HttpOnly; SameSite=Strict; Path=/; Max-Age=0\r\nSet-Cookie: tm_session=; HttpOnly; SameSite=Strict; Path=/TM; Max-Age=0\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            );
            continue;
        }
        if !operator && !allowed {
            let (status,mime,bytes):(&str,&str,&[u8])=match path.split('?').next().unwrap_or("/") {
                "/login.js" => ("200 OK","text/javascript",include_bytes!("../frontend/login.js")),
                "/style.css" => ("200 OK","text/css",include_bytes!("../frontend/style.css")),
                "/tm.css" => ("200 OK","text/css",include_bytes!("../frontend/tm.css")),
                "/fonts/ReadexPro.ttf" => ("200 OK","font/ttf",include_bytes!("../frontend/fonts/ReadexPro.ttf")),
                "/state" => ("401 Unauthorized","application/json",b"{\"error\":\"Sign in required\"}"),
                _ => ("200 OK","text/html",include_bytes!("../frontend/display-login.html")),
            };
            drop(s);
            let _=write!(stream,"HTTP/1.1 {status}\r\nContent-Type: {mime}\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nContent-Security-Policy: default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; frame-ancestors 'none'; base-uri 'self'\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",bytes.len());
            let _=stream.write_all(bytes);
            continue;
        }
        if operator && !allowed {
            let resource = path.split('?').next().unwrap_or("");
            let (status, mime, bytes): (&str, &str, &[u8]) = match resource {
                "/TM/login.js" => (
                    "200 OK",
                    "text/javascript",
                    include_bytes!("../frontend/login.js"),
                ),
                "/TM/style.css" => (
                    "200 OK",
                    "text/css",
                    include_bytes!("../frontend/style.css"),
                ),
                "/TM/tm.css" => ("200 OK", "text/css", include_bytes!("../frontend/tm.css")),
                "/TM/fonts/ReadexPro.ttf" => (
                    "200 OK",
                    "font/ttf",
                    include_bytes!("../frontend/fonts/ReadexPro.ttf"),
                ),
                "/TM/api" => (
                    "401 Unauthorized",
                    "application/json",
                    b"{\"error\":\"Sign in required\"}",
                ),
                _ => (
                    "200 OK",
                    "text/html",
                    include_bytes!("../frontend/login.html"),
                ),
            };
            drop(s);
            let _ = write!(
                stream,
                "HTTP/1.1 {status}\r\nContent-Type: {mime}\r\nCache-Control: no-store\r\nContent-Security-Policy: default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; frame-ancestors 'none'; base-uri 'self'\r\nX-Content-Type-Options: nosniff\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                bytes.len()
            );
            let _ = stream.write_all(bytes);
            continue;
        }
        if operator && allowed {
            drop(s);
            if path == "/TM/api" {
                let result = (|| -> Result<Value> {
                    if !tournament_manager_rs::web_control::valid_post(&request) {
                        bail!("Use a same-origin JSON POST")
                    };
                    let data: Value = serde_json::from_slice(&request.body)?;
                    let action = data["action"].as_str().context("Action")?;
                    if !tournament_manager_rs::web_control::allowed_action(action) {
                        bail!("This action is unavailable in browser match control")
                    };
                    execute(action, data["payload"].clone(), &shared, &app)
                })();
                let (status, body) = match result {
                    Ok(v) => ("200 OK", v.to_string()),
                    Err(e) => (
                        "400 Bad Request",
                        json!({"error":format!("{e:#}")}).to_string(),
                    ),
                };
                let _ = write!(
                    stream,
                    "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                continue;
            }
            let resource = path
                .split('?')
                .next()
                .unwrap_or("")
                .trim_start_matches("/TM")
                .trim_start_matches('/');
            let (mime, bytes): (&str, &[u8]) = match resource {
                "" => ("text/html", include_bytes!("../frontend/tm.html")),
                "tm-bridge.js" => (
                    "text/javascript",
                    include_bytes!("../frontend/tm-bridge.js"),
                ),
                "tm.css" => ("text/css", include_bytes!("../frontend/tm.css")),
                "app.js" => ("text/javascript", include_bytes!("../frontend/app.js")),
                "control.js" => ("text/javascript", include_bytes!("../frontend/control.js")),
                "displays.js" => ("text/javascript", include_bytes!("../frontend/displays.js")),
                "planner.js" => ("text/javascript", include_bytes!("../frontend/planner.js")),
                "levelup-field.js" => ("text/javascript", include_bytes!("../frontend/levelup-field.js")),
                "levelup-field.png" => ("image/png", include_bytes!("../frontend/levelup-field.png")),
                "override-field.js" => ("text/javascript", include_bytes!("../frontend/override-field.js")),
                "override-quadrants.png" => ("image/png", include_bytes!("../frontend/override-quadrants.png")),
                "override-field.png" => ("image/png", include_bytes!("../frontend/override-field.png")),
                "style.css" => ("text/css", include_bytes!("../frontend/style.css")),
                "fonts/ReadexPro.ttf" => (
                    "font/ttf",
                    include_bytes!("../frontend/fonts/ReadexPro.ttf"),
                ),
                _ => {
                    let _ = write!(
                        stream,
                        "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                    );
                    continue;
                }
            };
            let _ = write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: {mime}\r\nContent-Security-Policy: default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; frame-ancestors 'none'; base-uri 'self'\r\nX-Content-Type-Options: nosniff\r\nCache-Control: no-store\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                bytes.len()
            );
            let _ = stream.write_all(bytes);
            continue;
        }
        if allowed && ["/v5-display.png", "/iq-display.png", "/fonts/ReadexPro.ttf"].contains(&path)
        {
            let bytes: &[u8] = if path == "/fonts/ReadexPro.ttf" {
                include_bytes!("../frontend/fonts/ReadexPro.ttf")
            } else if path == "/v5-display.png" {
                include_bytes!("../assets/v5-display.png")
            } else {
                include_bytes!("../assets/iq-display.png")
            };
            drop(s);
            let _ = write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                if path.ends_with(".ttf") {
                    "font/ttf"
                } else {
                    "image/png"
                },
                bytes.len()
            );
            let _ = stream.write_all(bytes);
            continue;
        }
        let (status, mime, body): (&str, &str, String) = if !allowed {
            (
                "401 Unauthorized",
                "text/plain",
                "Authentication required".into(),
            )
        } else if path == "/state" {
            ("200 OK", "application/json", snapshot(&s).to_string())
        } else {
            match path.split('?').next().unwrap_or("/") {
                "/style.css" => (
                    "200 OK",
                    "text/css",
                    include_str!("../frontend/style.css").into(),
                ),
                "/remote.js" => (
                    "200 OK",
                    "text/javascript",
                    include_str!("../frontend/remote.js").into(),
                ),
                "/displays.js" => (
                    "200 OK",
                    "text/javascript",
                    include_str!("../frontend/displays.js").into(),
                ),
                "/planner.js" => (
                    "200 OK",
                    "text/javascript",
                    include_str!("../frontend/planner.js").into(),
                ),
                _ => (
                    "200 OK",
                    "text/html",
                    include_str!("../frontend/remote.html").into(),
                ),
            }
        };
        drop(s);
        let _ = write!(
            stream,
            "HTTP/1.1 {status}\r\nContent-Type: {mime}; charset=utf-8\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
    }
}
fn play_sound(name: &str) { tournament_manager_rs::platform::play_sound(name); }

pub fn run() {
    let shared = Shared(Arc::new(Mutex::new(Session::default())));
    let worker = shared.clone();
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(Duration::from_millis(200));
            let mut s = worker.0.lock().unwrap();
            let now = Instant::now();
            let dt = now - s.last;
            s.last = now;
            let before = s.timer.phase.clone();
            let previous = s.timer.remaining.as_secs_f64().ceil() as u64;
            let mut countdown = s.countdown.take();
            let started = tournament_manager_rs::schedule::advance_countdown(
                &mut s.timer,
                &mut countdown,
                dt,
            );
            s.countdown = countdown;
            if s.sounds {
                if started {
                    play_sound("Start.wav");
                } else if before != s.timer.phase
                    && matches!(s.timer.phase, Phase::AutoPause | Phase::Finished)
                {
                    play_sound("Stop.wav");
                } else if s.timer.phase == Phase::Driver {
                    let left = s.timer.remaining.as_secs_f64().ceil() as u64;
                    if previous != left {
                        if let Ok(g) = game(&s.event) {
                            for sound in g.data["defaultSounds"].as_array().into_iter().flatten() {
                                if sound["state"] == "DRIVER" {
                                    if let Some(time) = sound["timeLeft"].as_u64() {
                                        if previous > time && left <= time {
                                            if let Some(name) = sound["fileName"].as_str() {
                                                play_sound(name);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            if let Err(e) = sync_hardware(&mut s) {
                s.error = format!("Field disabled: {e:#}");
                stop(&mut s);
                s.hardware = None;
            }
        }
    });
    tauri::Builder::default()
        .manage(shared)
        .invoke_handler(tauri::generate_handler![dispatch])
        .on_window_event(|w, e| {
            if w.label() == "main" {
                if let tauri::WindowEvent::CloseRequested {api,..}=e {
                    let state=w.state::<Shared>();
                    let dirty={let mut s=state.0.lock().unwrap();stop(&mut s);s.dirty};
                    if dirty && rfd::MessageDialog::new().set_title("Unsaved tournament").set_description("Close and discard unsaved changes? Use Save before closing to keep them.").set_buttons(rfd::MessageButtons::YesNo).show()!=rfd::MessageDialogResult::Yes {api.prevent_close();}
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("Tauri application failed");
}

#[cfg(test)]
mod preview_tests {
    use super::*;
    #[test]
    fn score_preview_refreshes_during_a_running_match() {
        let mut s=Session::default();let g=game(&s.event).unwrap();
        s.event.matches=vec![g.new_match(1,2,&[1,2,3,4])];s.active=Some(1);s.timer.start();
        s.event.matches[0].values.insert("red_quad_1_red_pins".into(),3);
        changed(&mut s);assert_eq!(s.active_score["alliances"][0]["score"].as_f64(),Some(15.));
        s.event.matches[0].values.insert("red_quad_1_red_pins".into(),5);
        changed(&mut s);assert_eq!(s.active_score["alliances"][0]["score"].as_f64(),Some(25.));
    }
    #[test]
    fn draft_autosave_keeps_in_match_screen_until_final_submission() {
        let mut s=Session::default();let g=game(&s.event).unwrap();s.event.matches=vec![g.new_match(1,2,&[1,2,3,4])];s.auto_results=true;s.audience="In-Match".into();
        remember_score(&mut s,&g,0,true).unwrap();assert_eq!(s.audience,"In-Match");
        remember_score(&mut s,&g,0,false).unwrap();assert_eq!(s.audience,"Saved Match Results");
    }
}
