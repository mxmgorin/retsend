//! App orchestration: construction, the main loop, and the command router.

mod command;

pub use command::{AppCommand, Direction};

use crate::config::AppConfig;
use crate::event::user::UserEventSender;
use crate::event::AppEventHandler;
use crate::net::NetService;
use crate::overlay::browser::{BrowserMode, DirPurpose};
use crate::overlay::osk::{OskEvent, OskTarget};
use crate::overlay::routes::RouteCursor;
use crate::overlay::settings::SettingsRow;
use crate::overlay::tabs::Tab;
use crate::overlay::transfer::Viewed;
use crate::overlay::Focus;
use crate::platform::window::AppWindow;
use crate::transfer::history::History;
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
    /// The persisted transfer log shown on the History tab.
    history: History,
    /// The running (or last) send; `outbound_active` mirrors its liveness.
    outbound: Option<Arc<OutboundSession>>,
    /// Files pre-selected in the browser, from CLI arguments.
    staged: Vec<PathBuf>,
    /// The radar row A was pressed on; the browser's Start sends here.
    send_target: Option<SendTarget>,
    running: bool,
}

struct SendTarget {
    alias: String,
    /// `http://ip:port`
    base: String,
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
            std::path::Path::new(&crate::config::data_dir()),
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

        let history = History::load(&crate::config::data_dir(), config.transfer.history_limit);

        Ok(Self {
            window,
            ui,
            event_handler,
            config,
            net,
            wake,
            history,
            outbound: None,
            staged,
            send_target: None,
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
            // transitions (quick-save finished, request expired) as toasts;
            // log a finished transfer to the history.
            let active = self.net.shared.active.lock().unwrap().clone();
            let outcome = self.ui.transfer.sync(active);
            for toast in outcome.toasts {
                self.ui.toasts.push(toast);
            }
            // Outcomes reported by net threads (a manual add's probe).
            for notice in self.net.shared.take_notices() {
                self.ui.toasts.push(notice);
            }
            if let Some(entry) = outcome.recorded {
                self.history.record(entry);
            }
            // A finished send stops blocking incoming transfers.
            if self.outbound.as_ref().is_some_and(|o| o.is_finished()) {
                self.net
                    .shared
                    .outbound_active
                    .store(false, Ordering::SeqCst);
            }
            self.ui.update(&self.net, &self.config, &self.history);
            self.ui.draw(&self.window);
        }
        self.ui.destroy();
        self.net.stop();
    }

    /// Input precedence: the incoming modal outranks everything, then the
    /// full-screen overlays, then the tabs. The exception is the folder
    /// browser opened *from* that modal to pick where the files land — the
    /// request stays parked, but the browser owns the buttons.
    fn focus(&self) -> Focus {
        if self.ui.osk.active {
            Focus::Osk
        } else if self.net.shared.pending.lock().unwrap().is_some() && !self.picking_incoming_dir()
        {
            Focus::Prompt
        } else if self.ui.browser.open {
            Focus::Browser
        } else if self.ui.routes.open {
            Focus::Routes
        } else if self.ui.about.open {
            Focus::About
        } else if self.ui.transfer.opened {
            Focus::Transfer
        } else {
            Focus::Tabs
        }
    }

    /// Interpret a command against the current focus — the single place where
    /// "what does A do right now" is decided.
    fn execute_command(&mut self, command: AppCommand) {
        match (self.focus(), command) {
            (_, AppCommand::Shutdown) => self.running = false,

            // Select is the browser's root-carousel; everywhere else it
            // refreshes the radar.
            (Focus::Tabs | Focus::Transfer | Focus::Prompt, AppCommand::ReAnnounce) => {
                self.net.re_announce();
                self.ui.toasts.push("Announcing…");
            }

            // On-screen keyboard: finish typing before anything else.
            (Focus::Osk, AppCommand::Nav(dir)) => {
                let (dx, dy) = match dir {
                    Direction::Up => (0, -1),
                    Direction::Down => (0, 1),
                    Direction::Left => (-1, 0),
                    Direction::Right => (1, 0),
                };
                self.ui.osk.move_cursor(dx, dy);
            }
            (Focus::Osk, AppCommand::Confirm) => {
                if let Some(event) = self.ui.osk.press() {
                    self.handle_osk_event(event);
                }
            }
            (Focus::Osk, AppCommand::Back) => {
                let event = self.ui.osk.cancel();
                self.handle_osk_event(event);
            }
            (Focus::Osk, AppCommand::Alt) => self.ui.osk.erase(),
            (Focus::Osk, AppCommand::Start) => {
                let event = self.ui.osk.commit();
                self.handle_osk_event(event);
            }
            (Focus::Osk, AppCommand::ReAnnounce) => self.ui.osk.cycle_layer(),
            (Focus::Osk, _) => {}

            // Incoming-request modal: A accepts (the parked handler answers
            // 200 and installs the session), B declines (it answers 403).
            (Focus::Prompt, AppCommand::Confirm) => {
                if let Some(pending) = self.net.shared.pending.lock().unwrap().take() {
                    pending.accept(std::path::PathBuf::from(&self.config.transfer.save_dir));
                    // The transfer screen takes over (it outranks the tabs); an
                    // in-progress pick, route edit, or About view is abandoned
                    // (rare: someone sent to us mid-browse).
                    self.ui.browser.close();
                    self.ui.routes.close();
                    self.ui.about.close();
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
            // X: choose the folder for this transfer instead of taking the
            // configured one. The request stays parked (and counting down)
            // while the browser is up.
            (Focus::Prompt, AppCommand::Alt) => self.browse_for_incoming_dir(),
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

            (Focus::Browser, AppCommand::Nav(dir)) => match dir {
                Direction::Up => self.ui.browser.move_cursor(-1),
                Direction::Down => self.ui.browser.move_cursor(1),
                Direction::Left | Direction::Right => {}
            },
            (Focus::Browser, AppCommand::PageUp) => self.ui.browser.move_cursor(-PAGE_JUMP),
            (Focus::Browser, AppCommand::PageDown) => self.ui.browser.move_cursor(PAGE_JUMP),
            (Focus::Browser, AppCommand::Confirm) => {
                if let Err(message) = self.ui.browser.activate() {
                    self.ui.toasts.push(message);
                }
            }
            (Focus::Browser, AppCommand::Back) => {
                if !self.ui.browser.parent() {
                    self.ui.browser.close();
                    self.send_target = None;
                    // Backing out abandons a route whose folder was being picked.
                    self.ui.routes.pending_ext = None;
                }
            }
            // Start: confirm — send the selection, or choose the cwd (for the
            // save-dir setting or a pending route's folder).
            (Focus::Browser, AppCommand::Start) => match self.ui.browser.mode {
                BrowserMode::PickFiles => self.send_selection(),
                BrowserMode::PickDir => match self.ui.browser.dir_purpose {
                    DirPurpose::SaveDir => self.choose_save_dir(),
                    DirPurpose::Route => self.finish_route(),
                    DirPurpose::Incoming => self.accept_incoming_into(),
                },
            },
            // Select: jump between mount points.
            (Focus::Browser, AppCommand::ReAnnounce) => {
                if let Some(root) = self.ui.browser.cycle_root() {
                    let root = root.display().to_string();
                    self.ui.toasts.push(root);
                }
            }
            (Focus::Browser, AppCommand::TogglePin) => self.toggle_pin(),
            (Focus::Browser, AppCommand::Alt) => self.select_folder_contents(),

            // Routes editor: up/down over the routes, the add row and the auto
            // routes; A adds or removes; B goes back to Settings.
            (Focus::Routes, AppCommand::Nav(dir)) => {
                let (routes, auto) = self.route_counts();
                self.ui.routes.move_cursor(nav_delta(dir), routes, auto);
            }
            (Focus::Routes, AppCommand::Confirm) => self.routes_confirm(),
            (Focus::Routes, AppCommand::Back) => self.ui.routes.close(),
            (Focus::Routes, _) => {}

            // About screen: read-only, so B backs out to Settings and nothing
            // else does anything.
            (Focus::About, AppCommand::Back) => self.ui.about.close(),
            (Focus::About, _) => {}

            // L1/R1 cycle tabs (Settings is just another tab). Nav/Confirm
            // branch on the active tab; Start and Back do nothing here.
            (Focus::Tabs, AppCommand::PageUp) => self.switch_tab(-1),
            (Focus::Tabs, AppCommand::PageDown) => self.switch_tab(1),
            (Focus::Tabs, AppCommand::Nav(dir)) => self.tab_nav(dir),
            (Focus::Tabs, AppCommand::Confirm) => self.tab_confirm(),
            // X on the radar: type a peer's address, for the networks where
            // multicast discovery never reaches us.
            (Focus::Tabs, AppCommand::Alt) if self.ui.tabs.active() == Tab::Send => {
                self.ui
                    .osk
                    .open(OskTarget::PeerAddress, &local_subnet_prefix());
            }
            (
                Focus::Tabs,
                AppCommand::Start | AppCommand::Back | AppCommand::TogglePin | AppCommand::Alt,
            ) => {}
        }
    }

    /// L1/R1 on a tab: step to the previous/next tab, persisting settings if
    /// we're stepping off the Settings tab.
    fn switch_tab(&mut self, delta: i32) {
        let leaving_settings = self.ui.tabs.active() == Tab::Settings;
        self.ui.tabs.cycle(delta);
        if leaving_settings {
            self.leave_settings();
        }
        if self.ui.tabs.active() == Tab::Settings {
            self.refresh_auto_route_count();
        }
    }

    /// Nav on a tab: Send scrolls the radar, Settings moves the row cursor and
    /// steps values with left/right, Receive has nothing to navigate.
    fn tab_nav(&mut self, dir: Direction) {
        match self.ui.tabs.active() {
            Tab::Send => {
                let count = self.ui.peer_count;
                self.ui.home.move_cursor(nav_delta(dir), count);
            }
            Tab::Receive => {}
            Tab::History => {
                let count = self.ui.history_count;
                self.ui.history.move_cursor(nav_delta(dir), count);
            }
            Tab::Settings => match dir {
                Direction::Up => self.ui.settings.move_cursor(-1),
                Direction::Down => self.ui.settings.move_cursor(1),
                Direction::Left => self.adjust_setting(-1),
                Direction::Right => self.adjust_setting(1),
            },
        }
    }

    /// A on a tab: Send opens the browser for the selected device, Settings
    /// edits the current row, Receive does nothing.
    fn tab_confirm(&mut self) {
        match self.ui.tabs.active() {
            Tab::Send => {
                if let Some(index) = self.ui.home.cursor(self.ui.peer_count) {
                    self.open_browser_for(index);
                }
            }
            Tab::Receive => {}
            Tab::History => {}
            Tab::Settings => self.edit_setting(),
        }
    }

    /// A on a settings row: open its editor or toggle it.
    fn edit_setting(&mut self) {
        match self.ui.settings.row() {
            SettingsRow::Alias => {
                self.ui
                    .osk
                    .open(OskTarget::Alias, &self.config.device.alias);
            }
            SettingsRow::SaveDir => {
                let start = PathBuf::from(&self.config.transfer.save_dir);
                self.ui.browser.open_for_dir(
                    &start,
                    &self.config.transfer.browser_roots,
                    &self.config.transfer.pinned_paths,
                    DirPurpose::SaveDir,
                    "",
                );
            }
            SettingsRow::QuickSave => self.toggle_quick_save(),
            SettingsRow::AutoRoutes => self.toggle_auto_routes(),
            SettingsRow::Routes => {
                let auto = self.detected_auto_routes();
                self.ui.routes.open(auto);
            }
            SettingsRow::About => self.ui.about.open(),
            SettingsRow::Port => {}
        }
    }

    /// A in the routes editor: on the add row, start the add flow (type an
    /// extension, then pick a folder); on a route row, remove it.
    fn routes_confirm(&mut self) {
        let (routes, auto) = self.route_counts();
        match self.ui.routes.cursor(routes, auto) {
            RouteCursor::Add => self.ui.osk.open(OskTarget::RouteExt, ""),
            RouteCursor::Route(i) => {
                if let Some(ext) = self.config.transfer.routes.keys().nth(i).cloned() {
                    self.config.transfer.routes.remove(&ext);
                    self.apply_routes();
                    self.ui.toasts.push(format!("Removed .{ext} route"));
                }
            }
            // Auto routes are detected, not configured — nothing to remove.
            RouteCursor::Auto(_) => {}
        }
    }

    /// Cursor bounds for the routes editor: the configured routes, then the
    /// auto routes actually listed.
    fn route_counts(&self) -> (usize, usize) {
        let auto = self.ui.routes.auto_rows(&self.config.transfer.routes).len();
        (self.config.transfer.routes.len(), auto)
    }

    /// Start in the folder browser with a pending route extension: the cwd
    /// becomes that extension's destination.
    fn finish_route(&mut self) {
        let dir = self.ui.browser.cwd.clone();
        self.ui.browser.close();
        if let Some(ext) = self.ui.routes.pending_ext.take() {
            self.config
                .transfer
                .routes
                .insert(ext.clone(), dir.display().to_string());
            self.apply_routes();
            self.ui.toasts.push(format!(".{ext} → {}", dir.display()));
        }
    }

    /// Persist the routes and push them to the live receiver.
    fn apply_routes(&mut self) {
        self.config.save();
        self.net.shared.transfer.lock().unwrap().routes = self.config.transfer.routes.clone();
    }

    /// Left/right (and L1/R1) on value rows: port stepper, toggle.
    fn adjust_setting(&mut self, step: i32) {
        match self.ui.settings.row() {
            SettingsRow::Port => {
                let port = (self.config.network.port as i32 + step).clamp(1024, u16::MAX as i32);
                if port as u16 != self.config.network.port {
                    self.config.network.port = port as u16;
                    self.ui.settings.port_dirty = true;
                }
            }
            SettingsRow::QuickSave => self.toggle_quick_save(),
            SettingsRow::AutoRoutes => self.toggle_auto_routes(),
            _ => {}
        }
    }

    fn toggle_quick_save(&mut self) {
        self.config.transfer.auto_accept = !self.config.transfer.auto_accept;
        self.net.shared.transfer.lock().unwrap().auto_accept = self.config.transfer.auto_accept;
    }

    /// The toast reports the folder count because "on" alone cannot say whether
    /// this card has any console folders to route into.
    fn toggle_auto_routes(&mut self) {
        let on = !self.config.transfer.auto_routes;
        self.config.transfer.auto_routes = on;
        self.net.shared.transfer.lock().unwrap().auto_routes = on;
        self.config.save();
        self.refresh_auto_route_count();
        if on {
            let found = self.ui.settings.auto_route_count;
            self.ui
                .toasts
                .push(format!("Auto save routes on — {found} console folders"));
        } else {
            self.ui.toasts.push("Auto save routes off".to_string());
        }
    }

    /// X in the browser: take every file of this folder into the selection (or
    /// drop them again). Subfolders are left alone — the protocol has no notion
    /// of a directory, so a send is always a flat list of files.
    fn select_folder_contents(&mut self) {
        let Some((count, bytes, selected)) = self.ui.browser.toggle_folder_files() else {
            if self.ui.browser.mode == BrowserMode::PickFiles {
                self.ui.toasts.push("No files in this folder");
            }
            return;
        };
        let size = crate::ui::fmt_bytes(bytes);
        self.ui.toasts.push(if selected {
            format!("Selected {count} files · {size}")
        } else {
            format!("Cleared {count} files")
        });
    }

    /// Persist where this send came from, so the next one opens there. Only on
    /// send: doing it per directory step would write the config on every step.
    fn remember_send_dir(&mut self) {
        let dir = self.ui.browser.cwd.display().to_string();
        if dir != self.config.transfer.last_send_dir {
            self.config.transfer.last_send_dir = dir;
            self.config.save();
        }
    }

    /// X in the browser: pin or unpin the folder being looked at, and persist —
    /// a pin is worth nothing if it doesn't survive a restart.
    fn toggle_pin(&mut self) {
        let Some(change) = self.ui.browser.toggle_pin() else {
            return;
        };
        self.config.transfer.pinned_paths = change.paths;
        self.config.save();
        let name = change
            .path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| change.path.display().to_string());
        self.ui.toasts.push(if change.pinned {
            format!("★ {name}")
        } else {
            format!("Unpinned {name}")
        });
    }

    fn refresh_auto_route_count(&mut self) {
        self.ui.settings.auto_route_count = self.detected_auto_routes().len();
    }

    /// What the receiver would route automatically right now; empty when the
    /// setting is off. Walks the save directory, so callers take a snapshot
    /// instead of asking per frame.
    fn detected_auto_routes(&self) -> std::collections::BTreeMap<String, String> {
        if !self.config.transfer.auto_routes {
            return std::collections::BTreeMap::new();
        }
        crate::config::routes::detect(&PathBuf::from(&self.config.transfer.save_dir))
    }

    /// Leaving the Settings tab: persist, and restart the net stack if the
    /// port changed. (The old modal-close behaviour, now triggered by
    /// switching away from the tab.)
    fn leave_settings(&mut self) {
        self.config.save();
        if self.ui.settings.port_dirty {
            self.ui.settings.port_dirty = false;
            self.restart_net();
        }
    }

    fn restart_net(&mut self) {
        self.net.stop();
        match NetService::spawn(
            &self.config.device,
            &self.config.network,
            &self.config.transfer,
            std::path::Path::new(&crate::config::data_dir()),
            self.wake.clone(),
        ) {
            Ok(net) => {
                self.net = net;
                self.ui
                    .toasts
                    .push(format!("Now on port {}", self.net.http_port()));
            }
            Err(e) => self.ui.toasts.push(format!("Network restart failed: {e}")),
        }
    }

    fn handle_osk_event(&mut self, event: OskEvent) {
        match event {
            OskEvent::Committed(OskTarget::Alias, value) => {
                if value.is_empty() {
                    self.ui.toasts.push("Alias can't be empty");
                    return;
                }
                self.config.device.alias = value.clone();
                self.net.shared.me.lock().unwrap().alias = value;
                self.config.save();
                self.net.re_announce();
            }
            OskEvent::Committed(OskTarget::RouteExt, value) => {
                let ext = normalize_ext(&value);
                if ext.is_empty() {
                    self.ui.toasts.push("Extension can't be empty");
                    return;
                }
                // Capture the extension; the folder is picked next.
                self.ui.routes.pending_ext = Some(ext);
                let start = PathBuf::from(&self.config.transfer.save_dir);
                self.ui.browser.open_for_dir(
                    &start,
                    &self.config.transfer.browser_roots,
                    &self.config.transfer.pinned_paths,
                    DirPurpose::Route,
                    "",
                );
            }
            OskEvent::Committed(OskTarget::PeerAddress, value) => {
                match crate::net::manual::parse_address(&value) {
                    // The probe runs on its own thread; its outcome arrives as
                    // a notice the main loop turns into a toast.
                    Ok(addr) => {
                        self.ui.toasts.push(format!("Looking for {addr}…"));
                        crate::net::manual::spawn_probe(self.net.shared.clone(), addr);
                    }
                    Err(message) => self.ui.toasts.push(message),
                }
            }
            OskEvent::Cancelled => {}
        }
    }

    /// Is the browser up to give the parked incoming request a folder?
    fn picking_incoming_dir(&self) -> bool {
        self.ui.browser.open && self.ui.browser.dir_purpose == DirPurpose::Incoming
    }

    /// X on the incoming modal: browse for this transfer's destination,
    /// starting from the configured save directory.
    fn browse_for_incoming_dir(&mut self) {
        let sender = {
            let pending = self.net.shared.pending.lock().unwrap();
            let Some(pending) = pending.as_ref() else {
                return;
            };
            pending.sender.alias.clone()
        };
        let start = PathBuf::from(&self.config.transfer.save_dir);
        self.ui.browser.open_for_dir(
            &start,
            &self.config.transfer.browser_roots,
            &self.config.transfer.pinned_paths,
            DirPurpose::Incoming,
            &sender,
        );
    }

    /// Start in the browser opened from the incoming modal: accept the parked
    /// request into the cwd. The save routes are skipped — the folder on
    /// screen is the answer to where these files go.
    fn accept_incoming_into(&mut self) {
        let dir = self.ui.browser.cwd.clone();
        self.ui.browser.close();
        let Some(pending) = self.net.shared.pending.lock().unwrap().take() else {
            // The 60 s deadline passed while the folder was being picked.
            self.ui.toasts.push("Request expired");
            return;
        };
        pending.accept_into(dir.clone());
        self.ui.routes.close();
        self.ui.about.close();
        self.ui.transfer.open();
        self.ui.toasts.push(format!(
            "Saving into {}",
            crate::ui::truncate_middle(&dir.display().to_string(), crate::ui::PATH_CHARS)
        ));
    }

    /// Start in PickDir mode: the cwd becomes the save directory.
    fn choose_save_dir(&mut self) {
        let dir = self.ui.browser.cwd.clone();
        self.ui.browser.close();
        self.config.transfer.save_dir = dir.display().to_string();
        self.net.shared.transfer.lock().unwrap().save_dir = dir;
        self.config.save();
        // The new folder has its own console folders, so the old count is stale.
        self.refresh_auto_route_count();
        self.ui.toasts.push("Save folder updated");
    }

    /// A on a radar row: remember the target and open the file browser.
    fn open_browser_for(&mut self, index: usize) {
        if self.outbound.as_ref().is_some_and(|o| !o.is_finished()) {
            self.ui.toasts.push("A send is already running");
            return;
        }
        let peers = self.net.shared.peers.snapshot();
        let Some(peer) = peers.get(index) else { return };
        self.send_target = Some(SendTarget {
            alias: peer.info.alias.clone(),
            base: peer.base_url(),
        });
        self.ui.browser.open_for_send(
            &peer.info.alias,
            &self.config.transfer.browser_roots,
            &self.config.transfer.pinned_paths,
            &self.staged,
            &PathBuf::from(&self.config.transfer.last_send_dir),
        );
    }

    /// Start (in the browser): send the selection to the remembered target.
    fn send_selection(&mut self) {
        let (count, _) = self.ui.browser.selection_totals();
        if count == 0 {
            self.ui.toasts.push("Nothing selected — A marks files");
            return;
        }
        let Some(target) = self.send_target.take() else {
            self.ui.browser.close();
            return;
        };
        let files = self.ui.browser.selected_paths();
        self.remember_send_dir();
        self.ui.browser.close();
        let me = self.net.shared.me.lock().unwrap().clone();
        match outbound::spawn(target.alias, target.base, me, files, self.wake.clone()) {
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

/// A head start for typing a peer's address on a d-pad: a peer is nearly
/// always on our own /24, so the keyboard opens on `192.168.1.` and only the
/// last octet is left to type. Empty when we have no IPv4 address to borrow.
fn local_subnet_prefix() -> String {
    match crate::net::local_ip() {
        Some(std::net::IpAddr::V4(ip)) => {
            let [a, b, c, _] = ip.octets();
            format!("{a}.{b}.{c}.")
        }
        _ => String::new(),
    }
}

/// A route key from typed input: lowercase, alphanumeric only, leading dot
/// dropped — matching how `transfer::files::extension_of` reads a filename.
fn normalize_ext(raw: &str) -> String {
    raw.trim()
        .trim_start_matches('.')
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .flat_map(|c| c.to_lowercase())
        .collect()
}
