//! Bottom-anchored toast queue: transient status lines ("Announcing…",
//! "Send arrives in M3", net errors). Expired toasts are dropped on read.

use std::time::{Duration, Instant};

const TOAST_TTL: Duration = Duration::from_secs(3);

/// How a toast is framed: a refusal or a failure cannot look like news.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ToastKind {
    Info,
    Warn,
}

pub struct Toast {
    pub text: String,
    pub kind: ToastKind,
}

pub struct Toasts {
    items: Vec<(Toast, Instant)>,
}

impl Toasts {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    pub fn push(&mut self, text: impl Into<String>) {
        self.push_kind(text, ToastKind::Info);
    }

    /// Something refused or failed.
    pub fn warn(&mut self, text: impl Into<String>) {
        self.push_kind(text, ToastKind::Warn);
    }

    fn push_kind(&mut self, text: impl Into<String>, kind: ToastKind) {
        let toast = Toast {
            text: text.into(),
            kind,
        };
        self.items.push((toast, Instant::now()));
    }

    /// Live toasts, oldest first. Prunes expired ones.
    pub fn live(&mut self) -> impl Iterator<Item = &Toast> {
        self.items.retain(|(_, at)| at.elapsed() < TOAST_TTL);
        self.items.iter().map(|(toast, _)| toast)
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
