//! App orchestration: construction, the main loop, and the command router.

mod command;

pub use command::{AppCommand, Direction};

use crate::config::AppConfig;
use crate::event::user::UserEventSender;
use crate::event::AppEventHandler;
use crate::net::NetService;
use crate::overlay::Focus;
use crate::platform::window::AppWindow;
use crate::ui::AppUi;
use std::sync::Arc;

/// Rows a shoulder-button page jump moves in a list.
const PAGE_JUMP: i32 = 8;

pub struct App {
    window: AppWindow,
    ui: AppUi,
    event_handler: AppEventHandler,
    config: AppConfig,
    net: NetService,
    running: bool,
}

impl App {
    pub fn new(sdl: &sdl2::Sdl, config: AppConfig) -> Result<Self, String> {
        let window = AppWindow::new(sdl, &config.display)?;
        let ui = AppUi::new(&window);
        let event_handler = AppEventHandler::new(sdl, config.input.clone())?;

        // Net threads wake the blocked event loop through SDL user events;
        // the sender must be created on this (video) thread.
        let wake = Arc::new(UserEventSender::new());
        let net = NetService::spawn(&config.device, &config.network, wake)
            .map_err(|e| format!("failed to start networking: {e}"))?;

        Ok(Self {
            window,
            ui,
            event_handler,
            config,
            net,
            running: true,
        })
    }

    pub fn run(mut self) {
        log::info!(
            "running as `{}` on port {}",
            self.config.device.alias,
            self.net.http_port()
        );
        let mut commands: Vec<AppCommand> = Vec::new();
        while self.running {
            self.event_handler
                .wait(&self.window, &mut self.ui, &mut commands);
            for command in commands.drain(..) {
                self.execute_command(command);
            }
            self.ui.update(&self.net, &self.config);
            self.ui.draw(&self.window);
        }
        self.ui.destroy();
        self.net.stop();
    }

    /// Interpret a command against the current focus — the single place where
    /// "what does A do right now" is decided.
    fn execute_command(&mut self, command: AppCommand) {
        match (self.ui.focus(), command) {
            (_, AppCommand::Shutdown) => self.running = false,

            (_, AppCommand::ReAnnounce) => {
                self.net.re_announce();
                self.ui.toasts.push("Announcing…");
            }

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
                    // The send flow (file browser → confirm → upload) is M3/M4.
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
