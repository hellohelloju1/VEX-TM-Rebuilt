use crate::model::*;
use anyhow::{Result, bail};
pub fn validate(event: &Event, displays: &[DisplayConfig], sponsors: &[Sponsor]) -> Result<()> {
    let mut ids = std::collections::HashSet::new();
    let mut names = std::collections::HashSet::new();
    for d in displays {
        if d.id == 0
            || !ids.insert(d.id)
            || d.name.trim().is_empty()
            || !names.insert(d.name.trim().to_lowercase())
        {
            bail!("Displays need unique IDs and nonempty, unique names")
        }
        if !["Pit", "Field", "Audience"].contains(&d.kind.as_str()) {
            bail!("Choose Pit, Field, or Audience")
        }
        if d.kind == "Field" && !event.config.fields().contains(&d.field) {
            bail!("Choose a configured field for each field display")
        }
        if !["Rankings", "Skills"].contains(&d.screen.as_str()) {
            bail!("Choose Rankings or Skills as the default screen")
        }
    }
    if sponsors.iter().map(|s| s.image.len()).sum::<usize>() > 30_000_000 {
        bail!("Keep sponsor logos under 30 MB in total")
    }
    for s in sponsors {
        if s.name.trim().is_empty() || s.image.is_empty() || s.image.len() > 10_000_000 {
            bail!("Each sponsor needs a name and an image smaller than 10 MB")
        }
        let reader =
            image::ImageReader::new(std::io::Cursor::new(&s.image)).with_guessed_format()?;
        let (w, h) = reader.into_dimensions()?;
        if w > 8192 || h > 8192 || u64::from(w) * u64::from(h) > 20_000_000 {
            bail!("Resize sponsor logos to at most 20 megapixels and 8192 pixels per side")
        }
    }
    Ok(())
}

/// Live pit screen switch; it does not change the roster or match schedule.
pub fn pit_screen(event: &mut Event, id: u32, screen: &str) -> Result<()> {
    if !["Rankings","Skills"].contains(&screen){bail!("Choose Rankings or Skills");}
    let display=event.config.displays.iter_mut().find(|d|d.id==id&&d.kind=="Pit").ok_or_else(||anyhow::anyhow!("Choose a pit display"))?;
    display.screen=screen.into();
    Ok(())
}

/// Published scores only: the two highest-numbered completed qualifications.
pub fn recent_qual_results(event:&Event)->Vec<serde_json::Value>{
    let Some(game)=crate::scoring::Game::all().into_iter().find(|g|g.id()==event.game) else{return vec![];};
    let mut matches=event.matches.iter().filter(|m|m.round==2&&m.state==MatchState::Submitted).collect::<Vec<_>>();
    matches.sort_by_key(|m|std::cmp::Reverse((m.number,m.id)));
    matches.into_iter().take(2).filter_map(|m|game.score(m).ok().map(|score|serde_json::json!({"match":m,"score":score}))).collect()
}
