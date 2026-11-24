//! Operator match queue. Viewing or queueing a match never enables hardware.
use crate::model::{Event, MatchState};
use anyhow::{Result, bail};
use serde::Serialize;
#[derive(Default, Clone, Serialize)]
pub struct FieldQueue {
    pub pending: Vec<u32>,
}
impl FieldQueue {
    pub fn enqueue(&mut self, event: &Event, id: u32, active: Option<u32>) -> Result<()> {
        if !event
            .matches
            .iter()
            .any(|m| m.id == id && m.state != MatchState::Submitted)
        {
            bail!("Choose an unsubmitted match")
        }
        if active == Some(id) {
            bail!("This match is already on the field")
        }
        if !self.pending.contains(&id) {
            self.pending.push(id)
        };
        Ok(())
    }
    pub fn next(&self, event: &Event, running: bool) -> Result<u32> {
        if running {
            bail!("Finish or abort the current match before loading another")
        }
        self.pending
            .iter()
            .find(|id| {
                event
                    .matches
                    .iter()
                    .any(|m| m.id == **id && m.state != MatchState::Submitted)
            })
            .copied()
            .ok_or_else(|| anyhow::anyhow!("Queue a match from the Matches screen first"))
    }
    pub fn loaded(&mut self, event: &Event, id: u32) {
        self.pending.retain(|x| {
            *x != id
                && event
                    .matches
                    .iter()
                    .any(|m| m.id == *x && m.state != MatchState::Submitted)
        });
    }
    pub fn remove(&mut self, id: u32) {
        self.pending.retain(|x| *x != id);
    }
}
pub const AUDIENCE_MODES: [&str; 12] = [
    "None",
    "Logo",
    "Up Next",
    "In-Match",
    "Saved Match Results",
    "Schedule",
    "Rankings",
    "Skills Rankings",
    "Alliance Selection",
    "Elim Bracket",
    "Award Slides",
    "Inspection",
];
