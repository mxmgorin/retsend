//! Pure UI state machines — no egui here; `crate::ui` renders them. The
//! split (retsurf's convention) keeps navigation logic unit-testable and the
//! renderers swappable.

pub mod home;
pub mod settings;
pub mod toast;
pub mod transfer;

/// Which surface owns navigation input right now, in precedence order.
/// Browser/Osk join in later milestones.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Focus {
    /// The incoming-request modal — outranks everything below it.
    Prompt,
    Settings,
    /// The transfer progress/summary screen.
    Transfer,
    Home,
}
