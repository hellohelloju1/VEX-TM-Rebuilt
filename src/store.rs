use crate::model::*;
use anyhow::{Context, Result, bail};
use rusqlite::{Connection, OpenFlags, params};
use std::path::Path;
pub fn save(path: &Path, event: &Event) -> Result<()> {
    let mut c = Connection::open(path)?;
    let original: bool = c.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='teams')",
        [],
        |r| r.get(0),
    )?;
    if original {
        bail!("This is an original TM database. Save the reconstruction to a new .tmrs file.")
    };
    c.execute_batch("PRAGMA journal_mode=DELETE; CREATE TABLE IF NOT EXISTS tmrs_meta (id INTEGER PRIMARY KEY CHECK(id=1),format INTEGER NOT NULL);CREATE TABLE IF NOT EXISTS tmrs_event (id INTEGER PRIMARY KEY CHECK(id=1),data TEXT NOT NULL);CREATE TABLE IF NOT EXISTS tmrs_history (id INTEGER PRIMARY KEY, saved_at TEXT DEFAULT CURRENT_TIMESTAMP,data TEXT NOT NULL);")?;
    let tx = c.transaction()?;
    let payload = serde_json::to_string(event)?;
    tx.execute(
        "INSERT INTO tmrs_history(data) SELECT data FROM tmrs_event WHERE id=1",
        [],
    )?;
    tx.execute("INSERT OR REPLACE INTO tmrs_meta VALUES(1,1)", [])?;
    tx.execute(
        "INSERT OR REPLACE INTO tmrs_event VALUES(1,?)",
        params![payload],
    )?;
    tx.commit()?;
    Ok(())
}
pub fn load(path: &Path) -> Result<Event> {
    let c = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let format:u32=c.query_row("SELECT format FROM tmrs_meta WHERE id=1",[],|r|r.get(0)).context("Not a Rust Tournament Manager file. Original TM files can be imported using Import teams from original TM database.")?;
    if format != 1 {
        bail!("unsupported file version {format}")
    };
    let s: String = c.query_row("SELECT data FROM tmrs_event WHERE id=1", [], |r| r.get(0))?;
    let event: Event = serde_json::from_str(&s)?;
    if event.format != 1 {
        bail!("invalid payload version")
    };
    if !crate::scoring::Game::all()
        .iter()
        .any(|g| g.id() == event.game)
    {
        bail!("This game is not included in the installed game definitions")
    };
    Ok(event)
}
pub fn original_teams(path: &Path, start: u32) -> Result<Vec<Team>> {
    let c = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let mut q = c.prepare("SELECT number,name,school,city,checked_in FROM teams ORDER BY id")?;
    let rows = q.query_map([], |r| {
        Ok(Team {
            number: r.get(0)?,
            name: r.get(1)?,
            organization: r.get(2)?,
            city: r.get(3)?,
            checked_in: r.get::<_, i64>(4)? != 0,
            ..Default::default()
        })
    })?;
    let mut v = vec![];
    for (i, r) in rows.enumerate() {
        let mut t = r?;
        t.id = start + i as u32;
        v.push(t)
    }
    Ok(v)
}
// Legacy CSV exports from macOS spreadsheets commonly use Mac Roman.
// Prefer valid UTF-8 verbatim; decode legacy bytes without replacement characters.
fn csv_text(bytes: &[u8]) -> String {
    if let Ok(text) = std::str::from_utf8(bytes) {
        return text.trim_start_matches('\u{feff}').to_owned();
    }
    const MAC_ROMAN: &str = "ÄÅÇÉÑÖÜáàâäãåçéèêëíìîïñóòôöõúùûü†°¢£§•¶ß®©™´¨≠ÆØ∞±≤≥¥µ∂∑∏π∫ªºΩæø¿¡¬√ƒ≈∆«»… ÀÃÕŒœ–—“”‘’÷◊ÿŸ⁄€‹›ﬁﬂ‡·‚„‰ÂÊÁËÈÍÎÏÌÓÔÒÚÛÙıˆ˜¯˘˙˚¸˝˛ˇ";
    let high: Vec<char> = MAC_ROMAN.chars().collect();
    bytes.iter().map(|&b| if b < 128 { b as char } else { high[(b - 128) as usize] }).collect()
}

pub fn import_csv(path: &Path, start: u32) -> Result<Vec<Team>> {
    let text = csv_text(&std::fs::read(path)?);
    let mut r = csv::ReaderBuilder::new()
        .flexible(true)
        .trim(csv::Trim::All)
        .from_reader(text.as_bytes());
    let h = r
        .headers()?
        .iter()
        .map(|s| s.to_lowercase().replace([' ', '_'], ""))
        .collect::<Vec<_>>();
    let find = |names: &[&str]| h.iter().position(|s| names.contains(&s.as_str()));
    let num = find(&["number", "teamnumber", "team", "teamnum"])
        .context("CSV needs a Number or Team Number column")?;
    let mut out = vec![];
    for row in r.records() {
        let row = row?;
        let get = |names: &[&str]| {
            find(names)
                .and_then(|i| row.get(i))
                .unwrap_or("")
                .to_string()
        };
        let number = row.get(num).unwrap_or("").to_string();
        if number.is_empty() {
            continue;
        }
        out.push(Team {
            id: start + out.len() as u32,
            number,
            name: get(&["name", "teamname"]),
            age_group: get(&["agegroup", "age"]),
            organization: get(&["organization", "school"]),
            city: get(&["city"]),
            ..Default::default()
        })
    }
    Ok(out)
}
pub fn export_teams(path: &Path, event: &Event) -> Result<()> {
    let mut w = csv::Writer::from_path(path)?;
    w.write_record([
        "Number",
        "Name",
        "Organization",
        "City",
        "Checked In",
        "Inspected",
    ])?;
    for t in &event.teams {
        w.write_record([
            &t.number,
            &t.name,
            &t.organization,
            &t.city,
            &t.checked_in.to_string(),
            &t.inspected.to_string(),
        ])?;
    }
    w.flush()?;
    Ok(())
}

/// Reserve a new file before setup without overwriting an existing tournament.
pub fn create(path: &Path, event: &Event) -> Result<()> {
    let file=std::fs::OpenOptions::new().write(true).create_new(true).open(path)?;
    drop(file);
    if let Err(error)=save(path,event){let _=std::fs::remove_file(path);return Err(error);}
    Ok(())
}
