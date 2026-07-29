/// Everything input can ask the app to do. Input handlers emit these; the
/// router in `App::execute_command` interprets them against the current focus.
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
    /// The X button: rub out a character on the on-screen keyboard.
    Erase,
    Shutdown,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}
