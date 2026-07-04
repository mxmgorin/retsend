//! Transfer-screen state machine: which session the UI is viewing, whether
//! the screen is open, the cancel confirmation, and the speed meter. Synced
//! against [`crate::net::NetShared::active`] once per frame; produces toast
//! texts for transitions the user shouldn't miss.

use crate::transfer::inbound::InboundSession;
use std::collections::VecDeque;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Speed is measured over this trailing window.
const SPEED_WINDOW: Duration = Duration::from_secs(2);
/// An accepted request whose session never materialized (the handler timed
/// out just as the user pressed A) closes the screen after this long.
const ADOPTION_GRACE: Duration = Duration::from_secs(2);

pub struct TransferView {
    /// The session being viewed. Held independently of `NetShared::active` so
    /// the summary stays on screen after the net side clears the slot.
    pub session: Option<Arc<InboundSession>>,
    pub opened: bool,
    pub confirm_cancel: bool,
    opened_at: Option<Instant>,
    toasted: bool,
    samples: VecDeque<(Instant, u64)>,
}

impl TransferView {
    pub fn new() -> Self {
        Self {
            session: None,
            opened: false,
            confirm_cancel: false,
            opened_at: None,
            toasted: false,
            samples: VecDeque::new(),
        }
    }

    /// The user accepted the incoming request: show the screen and wait for
    /// the handler thread to install the session.
    pub fn open(&mut self) {
        self.opened = true;
        self.opened_at = Some(Instant::now());
        self.confirm_cancel = false;
    }

    pub fn close(&mut self) {
        self.opened = false;
        self.session = None;
        self.confirm_cancel = false;
        self.samples.clear();
    }

    /// Still receiving (cancel is meaningful).
    pub fn is_receiving(&self) -> bool {
        self.session.as_ref().is_some_and(|s| !s.is_finished())
    }

    /// Per-frame sync with the net side. Returns toast texts to show.
    pub fn sync(&mut self, active: Option<Arc<InboundSession>>) -> Vec<String> {
        let mut toasts = Vec::new();

        if let Some(active) = active {
            let same = self
                .session
                .as_ref()
                .is_some_and(|s| s.session_id == active.session_id);
            if !same {
                // Auto-accepted sessions don't steal the screen; the user
                // opened it via the modal when `opened` is already true.
                self.session = Some(active);
                self.toasted = false;
                self.samples.clear();
            }
        }

        if let Some(session) = &self.session {
            let now = Instant::now();
            self.samples
                .push_back((now, session.received_total.load(Ordering::Relaxed)));
            while self
                .samples
                .front()
                .is_some_and(|(t, _)| now.duration_since(*t) > SPEED_WINDOW)
            {
                self.samples.pop_front();
            }

            if session.is_finished() && !self.toasted {
                self.toasted = true;
                let summary = summary_line(session);
                if self.opened {
                    self.confirm_cancel = false; // nothing left to cancel
                } else {
                    // Quick-save finished in the background.
                    toasts.push(summary);
                    self.session = None;
                }
            }
        } else if self.opened && self.opened_at.is_some_and(|t| t.elapsed() > ADOPTION_GRACE) {
            // Accepted, but the handler had already timed out — no session.
            self.opened = false;
            toasts.push("Request expired".to_string());
        }

        toasts
    }

    /// Bytes per second over the trailing window; `None` until measurable.
    pub fn speed_bps(&self) -> Option<f64> {
        let (first, last) = (self.samples.front()?, self.samples.back()?);
        let dt = last.0.duration_since(first.0).as_secs_f64();
        if dt < 0.2 || last.1 <= first.1 {
            return None;
        }
        Some((last.1 - first.1) as f64 / dt)
    }
}

fn summary_line(session: &InboundSession) -> String {
    let done = session.done_count();
    let total = session.files.len();
    if done == total {
        format!("Received {done} files from {}", session.peer_alias)
    } else {
        format!(
            "Received {done} of {total} files from {}",
            session.peer_alias
        )
    }
}
