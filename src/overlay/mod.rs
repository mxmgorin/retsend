//! Pure UI state machines — no egui here; `crate::ui` renders them. The
//! split (retsurf's convention) keeps navigation logic unit-testable and the
//! renderers swappable.

pub mod home;
pub mod settings;
pub mod toast;

/// Which surface owns navigation input right now, in precedence order.
/// Prompt/Browser/Osk join in later milestones.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Focus {
    Home,
    Settings,
}
