//! Pure UI state machines — no egui here; `crate::ui` renders them. The
//! split (retsurf's convention) keeps navigation logic unit-testable and the
//! renderers swappable.

pub mod browser;
pub mod history;
pub mod home;
pub mod osk;
pub mod routes;
pub mod settings;
pub mod tabs;
pub mod toast;
pub mod transfer;

/// Which surface owns navigation input right now, in precedence order.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Focus {
    /// The on-screen keyboard — finish typing before anything else.
    Osk,
    /// The incoming-request modal.
    Prompt,
    /// The file browser (send picks, the save-dir setting, or a route's
    /// folder) — outranks the routes editor it can be opened from.
    Browser,
    /// The routes editor (reached from the Settings tab).
    Routes,
    /// The transfer progress/summary screen, a full-screen takeover.
    Transfer,
    /// The tab base (Send / Receive / Settings); the active tab decides what
    /// Nav/Confirm/Back mean.
    Tabs,
}
