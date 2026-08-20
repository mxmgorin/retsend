use crate::overlay::tabs::Tab;

/// Everything input can ask the app to do. Input handlers emit these; the
/// router in `App::execute_command` interprets them against the current focus.
///
/// The `Pick*` three are the exception: a tap names what it landed on, so they
/// carry an absolute target and are routed by it rather than by focus.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum AppCommand {
    Nav(Direction),
    Confirm,
    Back,
    /// The Start button: confirm a send in the browser, commit the keyboard.
    Start,
    /// Multicast our announce right now (the radar's manual refresh).
    ReAnnounce,
    PageUp,
    PageDown,
    /// The Y button: pin or unpin the browser's current folder.
    TogglePin,
    /// The X button: rub out a character on the on-screen keyboard, take every
    /// file in the folder in the browser.
    Alt,
    Shutdown,
    /// Put the showing list's cursor on this row. A tap emits it with a
    /// [`Self::Confirm`] behind it, which is what makes a tap act.
    PickRow(usize),
    /// Put the on-screen keyboard's cursor on this key.
    PickKey {
        row: usize,
        col: usize,
    },
    /// Switch to this tab.
    PickTab(Tab),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}
