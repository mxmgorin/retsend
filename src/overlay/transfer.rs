//! Transfer-screen state machine: which session (inbound or outbound) the UI
//! is viewing, whether the screen is open, the cancel confirmation, and the
//! speed meter. Synced once per frame; produces toast texts for transitions
//! the user shouldn't miss.

use crate::transfer::inbound::InboundSession;
use crate::transfer::outbound::OutboundSession;
use std::collections::VecDeque;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Speed is measured over this trailing window.
const SPEED_WINDOW: Duration = Duration::from_secs(2);
/// An accepted request whose session never materialized (the handler timed
/// out just as the user pressed A) closes the screen after this long.
const ADOPTION_GRACE: Duration = Duration::from_secs(2);

/// The session on the transfer screen. Held independently of the net-side
/// slots so the summary stays visible after they're cleared.
pub enum Viewed {
    In(Arc<InboundSession>),
    Out(Arc<OutboundSession>),
}

impl Viewed {
    pub fn is_finished(&self) -> bool {
        match self {
            Viewed::In(s) => s.is_finished(),
            Viewed::Out(s) => s.is_finished(),
        }
    }

    fn moved_bytes(&self) -> u64 {
        match self {
            Viewed::In(s) => s.received_total.load(Ordering::Relaxed),
            Viewed::Out(s) => s.sent_total.load(Ordering::Relaxed),
        }
    }
}

pub struct TransferView {
    pub viewed: Option<Viewed>,
    pub opened: bool,
    pub confirm_cancel: bool,
    opened_at: Option<Instant>,
    toasted: bool,
    samples: VecDeque<(Instant, u64)>,
}

impl TransferView {
    pub fn new() -> Self {
        Self {
            viewed: None,
            opened: false,
            confirm_cancel: false,
            opened_at: None,
            toasted: false,
            samples: VecDeque::new(),
        }
    }

    /// The user accepted an incoming request: show the screen and wait for
    /// the handler thread to install the session.
    pub fn open(&mut self) {
        self.opened = true;
        self.opened_at = Some(Instant::now());
        self.confirm_cancel = false;
    }

    /// The user started a send: view it immediately.
    pub fn view_outbound(&mut self, session: Arc<OutboundSession>) {
        self.viewed = Some(Viewed::Out(session));
        self.toasted = false;
        self.samples.clear();
        self.open();
    }

    pub fn close(&mut self) {
        self.opened = false;
        self.viewed = None;
        self.confirm_cancel = false;
        self.samples.clear();
    }

    /// Still moving bytes (cancel is meaningful).
    pub fn is_active(&self) -> bool {
        self.viewed.as_ref().is_some_and(|v| !v.is_finished())
    }

    /// Per-frame sync with the net side. Returns toast texts to show.
    pub fn sync(&mut self, active_in: Option<Arc<InboundSession>>) -> Vec<String> {
        let mut toasts = Vec::new();

        // Adopt a new inbound session — unless an unfinished send owns the
        // screen (can't happen in practice: incoming prepares answer 409
        // while outbound_active is up).
        if let Some(active) = active_in {
            let viewing_live_send =
                matches!(&self.viewed, Some(v @ Viewed::Out(_)) if !v.is_finished());
            let same =
                matches!(&self.viewed, Some(Viewed::In(s)) if s.session_id == active.session_id);
            if !same && !viewing_live_send {
                self.viewed = Some(Viewed::In(active));
                self.toasted = false;
                self.samples.clear();
            }
        }

        if let Some(viewed) = &self.viewed {
            let now = Instant::now();
            self.samples.push_back((now, viewed.moved_bytes()));
            while self
                .samples
                .front()
                .is_some_and(|(t, _)| now.duration_since(*t) > SPEED_WINDOW)
            {
                self.samples.pop_front();
            }

            if viewed.is_finished() && !self.toasted {
                self.toasted = true;
                if self.opened {
                    self.confirm_cancel = false; // nothing left to cancel
                } else {
                    // Quick-save finished in the background.
                    if let Viewed::In(session) = viewed {
                        toasts.push(inbound_summary(session));
                    }
                    self.viewed = None;
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

pub fn inbound_summary(session: &InboundSession) -> String {
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
