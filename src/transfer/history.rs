//! The transfer log shown on the History tab: one entry per finished session
//! (sent or received), persisted to `history.json` in the data dir so it
//! survives restarts. Recorded from [`crate::overlay::transfer::TransferView`]
//! at the completion edge; owned and persisted by `App`.

use super::inbound::InboundSession;
use super::outbound::{OutboundPhase, OutboundSession};
use serde::{Deserialize, Serialize};
use std::sync::atomic::Ordering;

/// Default cap on retained entries when the config omits `transfer.history_limit`.
pub const DEFAULT_MAX_ENTRIES: usize = 200;

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
    /// Max retained entries; the oldest are dropped past this (`transfer.history_limit`).
    limit: usize,
}

impl History {
    /// Load `history.json` from the data dir; a missing or corrupt file yields
    /// an empty log (best-effort, like the config). `limit` caps retained
    /// entries — a file over the (possibly lowered) cap is trimmed on load.
    pub fn load(data_dir: &str, limit: usize) -> Self {
        let path = format!("{data_dir}history.json");
        let mut entries: Vec<HistoryEntry> = match std::fs::read_to_string(&path) {
            Ok(text) => serde_json::from_str(&text).unwrap_or_else(|e| {
                log::warn!("invalid history `{path}`: {e}; starting empty");
                Vec::new()
            }),
            Err(_) => Vec::new(),
        };
        let overflow = entries.len().saturating_sub(limit);
        if overflow > 0 {
            entries.drain(..overflow);
        }
        Self {
            path,
            entries,
            limit,
        }
    }

    /// Append and persist. Best-effort write — a read-only SD degrades to an
    /// in-memory log, not a crash.
    pub fn record(&mut self, entry: HistoryEntry) {
        self.entries.push(entry);
        let overflow = self.entries.len().saturating_sub(self.limit);
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

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(tag: &str) -> String {
        let dir =
            std::env::temp_dir().join(format!("retsend-history-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        format!("{}/", dir.display())
    }

    fn entry(peer: &str) -> HistoryEntry {
        HistoryEntry {
            direction: Direction::Sent,
            peer: peer.to_string(),
            done: 1,
            total: 1,
            bytes: 1,
            outcome: Outcome::Completed,
            at: 0,
        }
    }

    fn peers(h: &History) -> Vec<String> {
        h.entries().iter().map(|e| e.peer.clone()).collect()
    }

    #[test]
    fn record_drops_oldest_past_limit() {
        let dir = temp_dir("record");
        let mut h = History::load(&dir, 3);
        for i in 0..5 {
            h.record(entry(&format!("p{i}")));
        }
        assert_eq!(peers(&h), ["p2", "p3", "p4"]);
        std::fs::remove_dir_all(dir.trim_end_matches('/')).unwrap();
    }

    #[test]
    fn load_trims_to_a_lowered_limit() {
        let dir = temp_dir("load");
        {
            let mut h = History::load(&dir, 100);
            for i in 0..5 {
                h.record(entry(&format!("p{i}")));
            }
        }
        // Re-open under a lower cap: the persisted file is trimmed on load.
        let h = History::load(&dir, 2);
        assert_eq!(peers(&h), ["p3", "p4"]);
        std::fs::remove_dir_all(dir.trim_end_matches('/')).unwrap();
    }
}
