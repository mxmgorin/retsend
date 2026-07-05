//! Pure UI state machines — no egui here; `crate::ui` renders them. The
//! split (retsurf's convention) keeps navigation logic unit-testable and the
//! renderers swappable.

pub mod browser;
pub mod home;
pub mod settings;
pub mod toast;
pub mod transfer;

/// Which surface owns navigation input right now, in precedence order.
/// Osk joins in a later milestone.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Focus {
    /// The incoming-request modal — outranks everything below it.
    Prompt,
    Settings,
    /// The file browser (picking files to send).
    Browser,
    /// The transfer progress/summary screen.
    Transfer,
    Home,
}
