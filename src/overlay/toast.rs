//! Bottom-anchored toast queue: transient status lines ("Announcing…",
//! "Send arrives in M3", net errors). Expired toasts are dropped on read.

use std::time::{Duration, Instant};

const TOAST_TTL: Duration = Duration::from_secs(3);

pub struct Toasts {
    items: Vec<(String, Instant)>,
}

impl Toasts {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    pub fn push(&mut self, text: impl Into<String>) {
        self.items.push((text.into(), Instant::now()));
    }

    /// Live toast texts, oldest first. Prunes expired ones.
    pub fn live(&mut self) -> impl Iterator<Item = &str> {
        self.items.retain(|(_, at)| at.elapsed() < TOAST_TTL);
        self.items.iter().map(|(text, _)| text.as_str())
    }

    /// When the next toast expires — the frame loop folds this into its idle
    /// wait so a toast disappears without other input.
    pub fn next_expiry(&self) -> Option<Duration> {
        self.items
            .iter()
            .map(|(_, at)| TOAST_TTL.saturating_sub(at.elapsed()))
            .min()
    }
}
