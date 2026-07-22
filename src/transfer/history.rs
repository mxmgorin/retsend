//! The transfer log shown on the History tab: one entry per finished session
//! (sent or received), persisted to `history.json` in the data dir so it
//! survives restarts. Recorded from [`crate::overlay::transfer::TransferView`]
//! at the completion edge; owned and persisted by `App`.

use super::inbound::InboundSession;
use super::outbound::{OutboundPhase, OutboundSession};
use serde::{Deserialize, Serialize};
use std::sync::atomic::Ordering;

/// Keep the log bounded — the tail is dropped on write.
const MAX_ENTRIES: usize = 200;

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Direction {
    Sent,
    Received,
}

/// How a session ended. `Completed` = every file done; `Partial` = some.
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Outcome {
    Completed,
    Partial,
    Cancelled,
    Declined,
    Failed,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub direction: Direction,
    /// The other device's alias.
    pub peer: String,
    /// Files that reached `Done`, out of the session total.
    pub done: usize,
    pub total: usize,
    /// Bytes actually moved.
    pub bytes: u64,
    pub outcome: Outcome,
    /// Unix seconds when it finished (for the relative "…ago" label).
    pub at: u64,
}

impl HistoryEntry {
    pub fn from_inbound(s: &InboundSession) -> Self {
        let done = s.done_count();
        let total = s.files.len();
        let outcome = if s.cancelled.load(Ordering::Relaxed) {
            Outcome::Cancelled
        } else {
            counts_outcome(done, total)
        };
        Self {
            direction: Direction::Received,
            peer: s.peer_alias.clone(),
            done,
            total,
            bytes: s.received_total.load(Ordering::Relaxed),
            outcome,
            at: now_unix(),
        }
    }

    pub fn from_outbound(s: &OutboundSession) -> Self {
        let done = s.done_count();
        let total = s.files.len();
        let outcome = match s.phase() {
            OutboundPhase::Done => counts_outcome(done, total),
            OutboundPhase::Cancelled => Outcome::Cancelled,
            OutboundPhase::Declined => Outcome::Declined,
            // `Failed`, or the can't-happen live phases.
            _ => Outcome::Failed,
        };
        Self {
            direction: Direction::Sent,
            peer: s.peer_alias.clone(),
            done,
            total,
            bytes: s.sent_total.load(Ordering::Relaxed),
            outcome,
            at: now_unix(),
        }
    }
}

fn counts_outcome(done: usize, total: usize) -> Outcome {
    if total > 0 && done == total {
        Outcome::Completed
    } else if done > 0 {
        Outcome::Partial
    } else {
        Outcome::Failed
    }
}

/// The persisted log, oldest-first. `App` owns one and appends to it.
pub struct History {
    path: String,
    entries: Vec<HistoryEntry>,
}

impl History {
    /// Load `history.json` from the data dir; a missing or corrupt file yields
    /// an empty log (best-effort, like the config).
    pub fn load(data_dir: &str) -> Self {
        let path = format!("{data_dir}history.json");
        let entries = match std::fs::read_to_string(&path) {
            Ok(text) => serde_json::from_str(&text).unwrap_or_else(|e| {
                log::warn!("invalid history `{path}`: {e}; starting empty");
                Vec::new()
            }),
            Err(_) => Vec::new(),
        };
        Self { path, entries }
    }

    /// Append and persist. Best-effort write — a read-only SD degrades to an
    /// in-memory log, not a crash.
    pub fn record(&mut self, entry: HistoryEntry) {
        self.entries.push(entry);
        let overflow = self.entries.len().saturating_sub(MAX_ENTRIES);
        if overflow > 0 {
            self.entries.drain(..overflow);
        }
        match serde_json::to_string_pretty(&self.entries) {
            Ok(text) => {
                if let Err(e) = std::fs::write(&self.path, text) {
                    log::warn!("could not write history `{}`: {e}", self.path);
                }
            }
            Err(e) => log::warn!("could not serialize history: {e}"),
        }
    }

    /// Entries oldest-first (the renderer walks them newest-first).
    pub fn entries(&self) -> &[HistoryEntry] {
        &self.entries
    }
}

fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
