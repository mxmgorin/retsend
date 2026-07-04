//! App orchestration: construction, the main loop, and the command router.

mod command;

pub use command::{AppCommand, Direction};

use crate::config::AppConfig;
use crate::event::AppEventHandler;
use crate::overlay::Focus;
use crate::platform::window::AppWindow;
use crate::ui::AppUi;

/// Rows a shoulder-button page jump moves in a list.
const PAGE_JUMP: i32 = 8;

pub struct App {
    window: AppWindow,
    ui: AppUi,
    event_handler: AppEventHandler,
    config: AppConfig,
    running: bool,
}

impl App {
    pub fn new(sdl: &sdl2::Sdl, config: AppConfig) -> Result<Self, String> {
        let window = AppWindow::new(sdl, &config.display)?;
        let ui = AppUi::new(&window);
        let event_handler = AppEventHandler::new(sdl, config.input.clone())?;

        Ok(Self {
            window,
            ui,
            event_handler,
            config,
            running: true,
        })
    }

    pub fn run(mut self) {
        log::info!("running as `{}`", self.config.device.alias);
        let mut commands: Vec<AppCommand> = Vec::new();
        while self.running {
            self.event_handler
                .wait(&self.window, &mut self.ui, &mut commands);
            for command in commands.drain(..) {
                self.execute_command(command);
            }
            self.ui.update(&self.config);
            self.ui.draw(&self.window);
        }
        self.ui.destroy();
    }

    /// Interpret a command against the current focus — the single place where
    /// "what does A do right now" is decided.
    fn execute_command(&mut self, command: AppCommand) {
        match (self.ui.focus(), command) {
            (_, AppCommand::Shutdown) => self.running = false,

            // The binding exists shell-first; discovery wires it up next.
            (_, AppCommand::ReAnnounce) => self.ui.toasts.push("Discovery is coming soon"),

            (Focus::Home, AppCommand::Nav(dir)) => {
                let count = self.ui.peer_count;
                self.ui.home.move_cursor(nav_delta(dir), count);
            }
            (Focus::Home, AppCommand::PageUp) => {
                let count = self.ui.peer_count;
                self.ui.home.move_cursor(-PAGE_JUMP, count);
            }
            (Focus::Home, AppCommand::PageDown) => {
                let count = self.ui.peer_count;
                self.ui.home.move_cursor(PAGE_JUMP, count);
            }
            (Focus::Home, AppCommand::Confirm) => {
                if self.ui.home.cursor(self.ui.peer_count).is_some() {
                    // The send flow (file browser → confirm → upload) comes
                    // after the transfer milestones.
                    self.ui.toasts.push("Sending files is coming soon");
                }
            }
            (Focus::Home, AppCommand::ToggleSettings) => self.ui.settings.open = true,
            (Focus::Home, AppCommand::Back) => {}

            (Focus::Settings, AppCommand::Nav(dir)) => {
                self.ui.settings.move_cursor(nav_delta(dir));
            }
            (Focus::Settings, AppCommand::Back | AppCommand::ToggleSettings) => {
                self.ui.settings.open = false;
            }
            (Focus::Settings, _) => {}
        }
    }
}

/// Vertical lists: up/down move the cursor; left/right are reserved for value
/// steppers and do nothing in plain lists.
fn nav_delta(dir: Direction) -> i32 {
    match dir {
        Direction::Up => -1,
        Direction::Down => 1,
        Direction::Left | Direction::Right => 0,
    }
}
