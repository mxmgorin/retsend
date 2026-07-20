//! Pure UI state machines — no egui here; `crate::ui` renders them. The
//! split (retsurf's convention) keeps navigation logic unit-testable and the
//! renderers swappable.

pub mod browser;
pub mod home;
pub mod osk;
pub mod settings;
pub mod toast;
pub mod transfer;

/// Which surface owns navigation input right now, in precedence order.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Focus {
    /// The on-screen keyboard — finish typing before anything else.
    Osk,
    /// The incoming-request modal.
    Prompt,
    /// The file browser (send picks or the save-dir setting) — checked
    /// before Settings so a browser opened *from* Settings gets the input.
    Browser,
    Settings,
    /// The transfer progress/summary screen.
    Transfer,
    Home,
}
