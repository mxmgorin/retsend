//! App orchestration: construction, the main loop, and the command router.

mod command;

pub use command::{AppCommand, Direction};

use crate::config::AppConfig;
use crate::event::user::UserEventSender;
use crate::event::AppEventHandler;
use crate::net::NetService;
use crate::overlay::transfer::Viewed;
use crate::overlay::Focus;
use crate::platform::window::AppWindow;
use crate::transfer::outbound::{self, OutboundSession};
use crate::ui::AppUi;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::Arc;

/// Rows a shoulder-button page jump moves in a list.
const PAGE_JUMP: i32 = 8;

pub struct App {
    window: AppWindow,
    ui: AppUi,
    event_handler: AppEventHandler,
    config: AppConfig,
    net: NetService,
    wake: Arc<UserEventSender>,
    /// The running (or last) send; `outbound_active` mirrors its liveness.
    outbound: Option<Arc<OutboundSession>>,
    /// Files queued for sending, from CLI arguments — the file browser
    /// replaces this as the on-device source in the next milestone.
    staged: Vec<PathBuf>,
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
        let net = NetService::spawn(
            &config.device,
            &config.network,
            &config.transfer,
            wake.clone(),
        )
        .map_err(|e| format!("failed to start networking: {e}"))?;

        let staged: Vec<PathBuf> = std::env::args_os()
            .skip(1)
            .map(PathBuf::from)
            .filter(|p| p.is_file())
            .collect();
        if !staged.is_empty() {
            log::info!("{} files staged for sending", staged.len());
        }

        Ok(Self {
            window,
            ui,
            event_handler,
            config,
            net,
            wake,
            outbound: None,
            staged,
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
            // Adopt/release the active inbound session and surface its
            // transitions (quick-save finished, request expired) as toasts.
            let active = self.net.shared.active.lock().unwrap().clone();
            for toast in self.ui.transfer.sync(active) {
                self.ui.toasts.push(toast);
            }
            // A finished send stops blocking incoming transfers.
            if self.outbound.as_ref().is_some_and(|o| o.is_finished()) {
                self.net
                    .shared
                    .outbound_active
                    .store(false, Ordering::SeqCst);
            }
            self.ui.update(&self.net, &self.config);
            self.ui.draw(&self.window);
        }
        self.ui.destroy();
        self.net.stop();
    }

    /// Input precedence: the incoming modal outranks everything, then the
    /// full-screen overlays, then the radar.
    fn focus(&self) -> Focus {
        if self.net.shared.pending.lock().unwrap().is_some() {
            Focus::Prompt
        } else if self.ui.settings.open {
            Focus::Settings
        } else if self.ui.transfer.opened {
            Focus::Transfer
        } else {
            Focus::Home
        }
    }

    /// Interpret a command against the current focus — the single place where
    /// "what does A do right now" is decided.
    fn execute_command(&mut self, command: AppCommand) {
        match (self.focus(), command) {
            (_, AppCommand::Shutdown) => self.running = false,

            (_, AppCommand::ReAnnounce) => {
                self.net.re_announce();
                self.ui.toasts.push("Announcing…");
            }

            // Incoming-request modal: A accepts (the parked handler answers
            // 200 and installs the session), B declines (it answers 403).
            (Focus::Prompt, AppCommand::Confirm) => {
                if let Some(pending) = self.net.shared.pending.lock().unwrap().take() {
                    pending.accept(std::path::PathBuf::from(&self.config.transfer.save_dir));
                    self.ui.settings.open = false;
                    self.ui.transfer.open();
                }
            }
            (Focus::Prompt, AppCommand::Back) => {
                if let Some(pending) = self.net.shared.pending.lock().unwrap().take() {
                    let sender = pending.sender.alias.clone();
                    pending.decline();
                    self.ui.toasts.push(format!("Declined {sender}"));
                }
            }
            (Focus::Prompt, _) => {}

            (Focus::Transfer, AppCommand::Back) => {
                if self.ui.transfer.confirm_cancel {
                    self.ui.transfer.confirm_cancel = false; // keep going
                } else if self.ui.transfer.is_active() {
                    self.ui.transfer.confirm_cancel = true;
                } else {
                    self.ui.transfer.close();
                }
            }
            (Focus::Transfer, AppCommand::Confirm) => {
                if self.ui.transfer.confirm_cancel {
                    self.cancel_active_transfer();
                }
            }
            (Focus::Transfer, _) => {}

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
                if let Some(index) = self.ui.home.cursor(self.ui.peer_count) {
                    self.start_send(index);
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

    /// Send the staged files to the radar's selected peer.
    fn start_send(&mut self, index: usize) {
        if self.staged.is_empty() {
            self.ui
                .toasts
                .push("Nothing staged — pass files as arguments (browser coming soon)");
            return;
        }
        if self.outbound.as_ref().is_some_and(|o| !o.is_finished()) {
            self.ui.toasts.push("A send is already running");
            return;
        }
        let peers = self.net.shared.peers.snapshot();
        let Some(peer) = peers.get(index) else { return };
        if peer.info.protocol.as_deref() == Some("https") {
            self.ui
                .toasts
                .push("HTTPS peers aren't supported yet — turn off encryption on the other side");
            return;
        }
        let me = self.net.shared.me.lock().unwrap().clone();
        let base = format!("http://{}:{}", peer.ip, peer.port);
        match outbound::spawn(
            peer.info.alias.clone(),
            base,
            me,
            self.staged.clone(),
            self.wake.clone(),
        ) {
            Ok(session) => {
                self.net
                    .shared
                    .outbound_active
                    .store(true, Ordering::SeqCst);
                self.outbound = Some(session.clone());
                self.ui.transfer.view_outbound(session);
            }
            Err(e) => self.ui.toasts.push(format!("Can't send: {e}")),
        }
    }

    /// Cancel whichever direction is on screen. Inbound: flip the session's
    /// flag (streaming threads abort and clean their `.part`s) and free the
    /// active slot. Outbound: the worker aborts and POSTs /cancel itself.
    fn cancel_active_transfer(&mut self) {
        self.ui.transfer.confirm_cancel = false;
        match &self.ui.transfer.viewed {
            Some(Viewed::In(session)) => {
                let session = session.clone();
                session.cancel("cancelled");
                let mut active = self.net.shared.active.lock().unwrap();
                if active
                    .as_ref()
                    .is_some_and(|s| s.session_id == session.session_id)
                {
                    *active = None;
                }
                log::info!("transfer from `{}` cancelled by user", session.peer_alias);
            }
            Some(Viewed::Out(session)) => {
                session.cancel.store(true, Ordering::SeqCst);
                log::info!("send to `{}` cancelled by user", session.peer_alias);
            }
            None => {}
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
