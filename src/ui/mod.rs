//! egui integration and the per-frame render pass. Owns the overlay state
//! machines; `App` drives them through commands, this module draws them.

mod about;
mod browser;
mod history;
mod home;
mod osk;
mod prompt;
mod receive;
mod routes;
mod settings;
mod tabs;
pub mod theme;
mod transfer;
mod wordmark;

use crate::app::AppCommand;
use crate::config::AppConfig;
use crate::net::server::DECISION_TIMEOUT;
use crate::net::NetService;
use crate::overlay::{
    about::AboutView,
    browser::FileBrowser,
    history::HistoryView,
    home::Home,
    osk::Osk,
    routes::RoutesView,
    settings::Settings,
    tabs::{Tab, Tabs},
    toast::Toasts,
    transfer::{TransferView, Viewed},
};
use crate::platform::window;
use crate::transfer::history::History;
use crate::transfer::inbound::FileState;
use crate::transfer::outbound::{OutboundPhase, OutboundSession};
use egui_sdl2::egui;
use egui_sdl2::EguiWindow;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

/// Radar snapshots at most this stale while idle — covers peer-expiry pruning
/// and freshly announced ports without waking every frame.
const IDLE_REFRESH: Duration = Duration::from_secs(1);
/// The incoming modal's countdown bar animates at this cadence.
const PROMPT_REFRESH: Duration = Duration::from_millis(100);
/// Cap on how often the Receive screen re-samples IP/SSID — the probe shells
/// out to `iw`, so not per frame.
const NET_STATUS_TTL: Duration = Duration::from_secs(2);

/// The panel every screen is laid out for; the zoom fits it to the real one.
const BASE_SIZE: egui::Vec2 = egui::vec2(640.0, 480.0);
/// Bounds on the zoom actually installed, whatever the fit and the setting
/// between them ask for.
const SCALE_RANGE: std::ops::RangeInclusive<f32> = 0.5..=4.0;
const SCALE_STEP: f32 = crate::config::display::SCALE.step;
/// A panel reported past this is a driver answering nonsense, not a screen.
const MAX_PANEL: f32 = 8192.0;
/// How far past a whole number a fit may reach before the fraction is worth it.
/// Fractional zoom lands glyphs between pixels, which reads as uneven type on a
/// panel with no subpixel positioning; the spare pixels become margin instead.
const WHOLE_ZOOM_REACH: f32 = 0.25;

/// Shared ids for the one top and one bottom panel every base screen draws.
/// egui paints a red seam at a panel edge whenever a panel id changes between
/// frames (its changed-id-between-passes check — see the tab-bar painter note
/// in `tabs.rs`). Reusing a single id per slot keeps the panel sequence
/// identical across screen switches, so opening a takeover (Browser / Routes /
/// About / Transfer) from a tab never reshuffles the ids and never flashes.
pub(crate) const TOP_PANEL_ID: &str = "screen_top";
pub(crate) const BOTTOM_PANEL_ID: &str = "screen_bottom";

/// Throttled cache of the device's network status behind the Receive screen.
struct NetStatusCache {
    status: crate::net::NetStatus,
    sampled_at: Option<Instant>,
}

impl NetStatusCache {
    fn new() -> Self {
        Self {
            status: crate::net::NetStatus::default(),
            sampled_at: None,
        }
    }

    /// The current status, re-sampling only once the cache is older than
    /// [`NET_STATUS_TTL`].
    fn get(&mut self) -> &crate::net::NetStatus {
        if self
            .sampled_at
            .is_none_or(|t| t.elapsed() >= NET_STATUS_TTL)
        {
            self.status = crate::net::sample_status();
            self.sampled_at = Some(Instant::now());
        }
        &self.status
    }
}

/// Data for the one base screen this frame renders. Built by
/// [`AppUi::screen_data`] with the same precedence the render pass uses, so
/// covered screens cost nothing.
enum Screen {
    /// Renders straight from `AppUi::browser`.
    Browser,
    Routes(routes::RoutesData),
    About,
    Transfer(transfer::TransferData),
    Send(home::HomeData),
    Receive(receive::ReceiveData),
    History(history::HistoryData),
    Settings,
}

pub struct AppUi {
    egui: EguiWindow,
    repaint_delay: Option<Duration>,
    pub tabs: Tabs,
    pub home: Home,
    pub history: HistoryView,
    pub settings: Settings,
    pub routes: RoutesView,
    pub about: AboutView,
    pub browser: FileBrowser,
    pub osk: Osk,
    pub transfer: TransferView,
    pub toasts: Toasts,
    /// Peer count as of the last frame that showed the Send tab — the command
    /// router clamps the home cursor against it without re-locking the registry.
    pub peer_count: usize,
    /// History entry count as of the last frame that showed the History tab —
    /// clamps the history cursor.
    pub history_count: usize,
    /// Commands from taps on the frame just drawn, for `App` to run next pass:
    /// the screens are painted from a cursor, so a tap has to become a command.
    taps: Vec<AppCommand>,
    /// Throttled IP/SSID for the Receive screen's diagnostic line.
    net_status: NetStatusCache,
    /// `RETSEND_SCALE`, standing in for the panel's own fit where a launcher
    /// knows better.
    forced_scale: Option<f32>,
    /// The zoom in force, so a frame that wants the same one installs nothing.
    scale: f32,
}

impl AppUi {
    pub fn new(sdl: &sdl2::Sdl, config: &crate::config::DisplayConfig) -> Result<Self, String> {
        let egui = window::open(sdl, config)?;
        theme::apply(egui.ctx());
        // The fit needs a frame's measurements; a pinned scale is known now, so
        // the first frame is already at it rather than at 1.0.
        let forced_scale = crate::config::device_scale();
        if let Some(scale) = forced_scale {
            log::info!("applying RETSEND_SCALE {scale}");
            egui.ctx().set_zoom_factor(scale);
        }
        Ok(Self {
            egui,
            repaint_delay: None,
            tabs: Tabs::new(),
            home: Home::new(),
            history: HistoryView::new(),
            settings: Settings::new(),
            routes: RoutesView::new(),
            about: AboutView::new(),
            browser: FileBrowser::new(),
            osk: Osk::new(),
            transfer: TransferView::new(),
            toasts: Toasts::new(),
            peer_count: 0,
            history_count: 0,
            taps: Vec::new(),
            net_status: NetStatusCache::new(),
            forced_scale,
            scale: forced_scale.unwrap_or(1.0),
        })
    }

    /// Feed an SDL event to egui (resize/DPI bookkeeping, pointer hover).
    pub fn handle_event(&mut self, event: &sdl2::event::Event) {
        let _ = self.egui.on_event(event);
    }

    /// How long the event loop may block before the next frame is due.
    pub fn take_repaint_delay(&mut self) -> Option<Duration> {
        self.repaint_delay.take()
    }

    /// What the last frame's taps amount to, for the command router.
    pub fn take_taps(&mut self) -> Vec<AppCommand> {
        std::mem::take(&mut self.taps)
    }

    /// Scale the whole UI to the panel it is on, then by what the settings ask
    /// for. Every size in the renderers is in points against a 640×480 design,
    /// so one zoom factor carries the lot — fonts, rows, gaps and all — rather
    /// than each screen carrying its own notion of a small display. Re-read
    /// every frame, since a desktop window resizes.
    fn sync_scale(&mut self, wanted_by_user: f32) {
        let ctx = self.egui.ctx();
        let native = panel_points(ctx.content_rect().size(), ctx.zoom_factor());
        if !(1.0..=MAX_PANEL).contains(&native.x) || !(1.0..=MAX_PANEL).contains(&native.y) {
            return;
        }
        let wanted = wanted_scale(native, self.forced_scale, wanted_by_user);
        if (wanted - self.scale).abs() < f32::EPSILON {
            return;
        }
        // Two decimals: snapping to the step grid leaves float noise in the tail.
        log::info!("ui scale {wanted:.2} for {}x{} points", native.x, native.y);
        ctx.set_zoom_factor(wanted);
        self.scale = wanted;
    }

    /// Build the frame. Reads shared net state (brief locks) before entering
    /// the egui closure.
    pub fn update(&mut self, net: &NetService, config: &AppConfig, history: &History) {
        self.sync_scale(config.display.scale);
        let screen = self.screen_data(net, config, history);
        let prompt_data = prompt_data(net);
        // The destination picker stands in for the modal while it is up: the
        // request keeps counting down, so the browser shows what is left of
        // the deadline and the modal itself stays hidden behind it.
        let picking_incoming = self.browser.open
            && self.browser.dir_purpose == crate::overlay::browser::DirPurpose::Incoming;
        let deadline_secs = prompt_data.as_ref().filter(|_| picking_incoming).map(|p| {
            (p.remaining * DECISION_TIMEOUT.as_secs_f32())
                .max(0.0)
                .ceil() as u32
        });
        let active_tab = self.tabs.active();
        let settings_state = &self.settings;
        let toasts: Vec<String> = self.toasts.live().map(str::to_string).collect();
        let actual_port = net.http_port();

        // Local, not `self.taps`: the closure already borrows the state it draws.
        let mut taps: Vec<AppCommand> = Vec::new();
        self.egui.run(|ctx| {
            // egui 0.34 panels are shown inside an explicit root Ui spanning
            // the window (retsurf's pattern; top-level `show` is deprecated).
            let mut root = egui::Ui::new(
                ctx.clone(),
                egui::Id::new("root_ui"),
                egui::UiBuilder::new().max_rect(ctx.content_rect()),
            );
            root.set_clip_rect(ctx.content_rect());
            match &screen {
                Screen::Browser => browser::render(
                    &mut root,
                    &self.browser,
                    &self.browser.target_alias,
                    deadline_secs,
                    &mut taps,
                ),
                Screen::Routes(data) => routes::render(&mut root, data, &mut taps),
                Screen::About => about::render(&mut root, &mut taps),
                Screen::Transfer(data) => transfer::render(&mut root, data, &mut taps),
                Screen::Send(data) => {
                    tabs::render_bar(&mut root, active_tab, &mut taps);
                    home::render(&mut root, data, &mut taps);
                }
                Screen::Receive(data) => {
                    tabs::render_bar(&mut root, active_tab, &mut taps);
                    receive::render(&mut root, data, &mut taps);
                }
                Screen::History(data) => {
                    tabs::render_bar(&mut root, active_tab, &mut taps);
                    history::render(&mut root, data, &mut taps);
                }
                Screen::Settings => {
                    tabs::render_bar(&mut root, active_tab, &mut taps);
                    settings::render(&mut root, settings_state, config, actual_port, &mut taps);
                }
            }
            if let Some(p) = prompt_data.as_ref().filter(|_| !picking_incoming) {
                prompt::render(ctx, p, &mut taps);
            }
            if self.osk.active {
                osk::render(ctx, &self.osk, &mut taps);
            }
            render_toasts(ctx, &toasts);
        });

        // Fold the frame-timing sources into one idle bound: egui's own
        // request (animations/sizing passes), toast expiry, radar staleness,
        // and the modal's countdown animation.
        let mut delay = self.egui.repaint_delay().min(IDLE_REFRESH);
        if let Some(t) = self.toasts.next_expiry() {
            delay = delay.min(t);
        }
        if prompt_data.is_some() {
            delay = delay.min(PROMPT_REFRESH);
        }
        // Acted on next pass, so the loop must not block on an event first.
        if !taps.is_empty() {
            delay = Duration::ZERO;
        }
        self.taps = taps;
        self.repaint_delay = Some(delay);
    }

    /// Snapshot what the rendered screen needs, and only that. Precedence
    /// mirrors Focus: browser, routes editor, About, the transfer takeover,
    /// then the active tab. Skipping covered screens also keeps the Receive
    /// tab's network probe (shells out to `iw`) off every other screen.
    fn screen_data(&mut self, net: &NetService, config: &AppConfig, history: &History) -> Screen {
        if self.browser.open {
            return Screen::Browser;
        }
        if self.routes.open {
            let rows: Vec<(String, String)> = config
                .transfer
                .routes
                .iter()
                .map(|(ext, dir)| (ext.clone(), dir.clone()))
                .collect();
            let auto_rows = self.routes.auto_rows(&config.transfer.routes);
            return Screen::Routes(routes::RoutesData {
                cursor: self.routes.cursor(rows.len(), auto_rows.len()),
                rows,
                auto_rows,
                auto_on: config.transfer.auto_routes,
            });
        }
        if self.about.open {
            return Screen::About;
        }
        if let Some(data) = self.transfer_data() {
            return Screen::Transfer(data);
        }
        match self.tabs.active() {
            Tab::Send => {
                let peers = net.shared.peers.snapshot();
                self.peer_count = peers.len();
                Screen::Send(home::HomeData {
                    cursor: self.home.cursor(peers.len()),
                    peers: peers
                        .iter()
                        .map(|p| home::PeerRow {
                            alias: p.info.alias.clone(),
                            detail: format!(
                                "{} · {}",
                                p.info.device_model.as_deref().unwrap_or("unknown"),
                                p.ip
                            ),
                            insecure: !p
                                .info
                                .protocol
                                .as_deref()
                                .is_some_and(|s| s.eq_ignore_ascii_case("https")),
                        })
                        .collect(),
                })
            }
            Tab::Receive => {
                let (scheme, port) = endpoint_scheme_port(net);
                let status = self.net_status.get();
                Screen::Receive(receive::ReceiveData {
                    alias: config.device.alias.clone(),
                    scheme,
                    port,
                    ip: status.ip.map(|ip| ip.to_string()),
                    ssid: status.ssid.clone(),
                    save_dir: config.transfer.save_dir.clone(),
                    quick_save: config.transfer.auto_accept,
                })
            }
            Tab::History => {
                self.history_count = history.entries().len();
                let now = unix_now();
                let cursor = self.history.cursor(self.history_count);
                Screen::History(history::HistoryData {
                    cursor,
                    can_resend: cursor
                        .and_then(|row| history.get(row))
                        .is_some_and(|e| e.resendable()),
                    rows: history
                        .newest_first()
                        .map(|e| history::row(e, now))
                        .collect(),
                })
            }
            Tab::Settings => Screen::Settings,
        }
    }

    /// Snapshot the viewed session for the renderer (per-slot locks, brief).
    fn transfer_data(&self) -> Option<transfer::TransferData> {
        if !self.transfer.opened {
            return None;
        }
        let viewed = self.transfer.viewed.as_ref()?;
        let (title, rows, transferred, total) = match viewed {
            Viewed::In(session) => (
                if session.is_finished() {
                    crate::overlay::transfer::inbound_summary(session)
                } else {
                    format!("Receiving from {}", session.peer_alias)
                },
                session
                    .files
                    .iter()
                    .map(|slot| {
                        file_row(
                            slot.dest
                                .file_name()
                                .map(|n| n.to_string_lossy().into_owned())
                                .unwrap_or_else(|| slot.meta.file_name.clone()),
                            slot.meta.size,
                            &slot.state.lock().unwrap(),
                            slot.received.load(Ordering::Relaxed),
                        )
                    })
                    .collect(),
                session.received_total.load(Ordering::Relaxed),
                session.total_bytes,
            ),
            Viewed::Out(session) => (
                outbound_title(session),
                session
                    .files
                    .iter()
                    .map(|file| {
                        file_row(
                            file.meta.file_name.clone(),
                            file.meta.size,
                            &file.state.lock().unwrap(),
                            file.sent.load(Ordering::Relaxed),
                        )
                    })
                    .collect(),
                session.sent_total.load(Ordering::Relaxed),
                session.total_bytes,
            ),
        };
        Some(transfer::TransferData {
            title,
            finished: viewed.is_finished(),
            transferred,
            total,
            speed_bps: self.transfer.speed_bps(),
            rows,
            confirm_cancel: self.transfer.confirm_cancel,
        })
    }

    pub fn draw(&mut self) {
        self.egui.paint(theme::BACKGROUND);
    }

    pub fn destroy(&mut self) {
        self.egui.destroy();
    }
}

/// Room a transfer row has for a name. Reached by the files of a sent folder,
/// which are named by their path.
const ROW_NAME_CHARS: usize = 44;

fn file_row(name: String, size: u64, state: &FileState, moved: u64) -> transfer::FileRow {
    transfer::FileRow {
        name: truncate_middle(&name, ROW_NAME_CHARS),
        size,
        glyph: match state {
            FileState::Done => "√",
            FileState::Failed(_) => "×",
            FileState::Pending | FileState::Receiving => "",
        },
        frac: if size > 0 {
            (moved as f32 / size as f32).clamp(0.0, 1.0)
        } else {
            1.0
        },
    }
}

fn outbound_title(session: &OutboundSession) -> String {
    match session.phase() {
        OutboundPhase::Waiting => format!("Waiting for {} to accept…", session.peer_alias),
        OutboundPhase::Sending => format!("Sending to {}", session.peer_alias),
        OutboundPhase::Done => {
            let done = session.done_count();
            let total = session.files.len();
            if done == total {
                format!("Sent {done} files")
            } else {
                format!("Sent {done} of {total} files")
            }
        }
        OutboundPhase::Declined => format!("{} declined", session.peer_alias),
        OutboundPhase::Cancelled => "Send cancelled".to_string(),
        OutboundPhase::Failed(message) => format!("Send failed: {message}"),
    }
}

fn prompt_data(net: &NetService) -> Option<prompt::PromptData> {
    let pending = net.shared.pending.lock().unwrap();
    let p = pending.as_ref()?;
    let elapsed = p.received_at.elapsed().as_secs_f32();
    Some(prompt::PromptData {
        sender: p.sender.alias.clone(),
        files: p
            .files
            .iter()
            .take(prompt::SHOWN_FILES)
            .map(|f| (f.file_name.clone(), f.size))
            .collect(),
        hidden: p.files.len().saturating_sub(prompt::SHOWN_FILES),
        count: p.files.len(),
        total_bytes: p.total_bytes,
        dests: p.dests.iter().take(prompt::SHOWN_DESTS).cloned().collect(),
        hidden_dests: p.dests.len().saturating_sub(prompt::SHOWN_DESTS),
        remaining: 1.0 - elapsed / DECISION_TIMEOUT.as_secs_f32(),
    })
}

/// Announced scheme (uppercased) and bound port. IP/SSID come from the
/// throttled `NetStatusCache`, not here.
fn endpoint_scheme_port(net: &NetService) -> (String, u16) {
    let me = net.shared.me.lock().unwrap();
    (
        me.protocol.as_deref().unwrap_or("http").to_uppercase(),
        me.port.unwrap_or(0),
    )
}

/// The points a panel covers at `zoom` — what the design is fitted against.
fn panel_points(content: egui::Vec2, zoom: f32) -> egui::Vec2 {
    (content * zoom).round()
}

/// The zoom a panel of `native` points wants: what the design fits into it, or
/// `forced` where a launcher said so, taken `user` times over and snapped to a
/// step the layout can settle on.
fn wanted_scale(native: egui::Vec2, forced: Option<f32>, user: f32) -> f32 {
    // Long edge to long edge: a phone held upright is the same screen, so the
    // rows keep their size and the list gets a narrower measure instead.
    let fit =
        (native.max_elem() / BASE_SIZE.max_elem()).min(native.min_elem() / BASE_SIZE.min_elem());
    let fit = if fit >= 1.0 && fit.fract() <= WHOLE_ZOOM_REACH {
        fit.floor()
    } else {
        fit
    };
    let wanted = forced.unwrap_or(fit) * user;
    ((wanted / SCALE_STEP).round() * SCALE_STEP).clamp(*SCALE_RANGE.start(), *SCALE_RANGE.end())
}

/// Unix seconds now — for the history's relative-time labels.
fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// "999 B", "12.3 KB", "1.2 GB" — one decimal above bytes.
pub(crate) fn fmt_bytes(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["KB", "MB", "GB", "TB"];
    if bytes < 1000 {
        return format!("{bytes} B");
    }
    // Start in KB (the first unit above bytes) so `value` and `unit` stay in
    // step — dividing straight from bytes would land a unit too high.
    let mut value = bytes as f64 / 1000.0;
    let mut unit = 0;
    while value >= 1000.0 && unit < UNITS.len() - 1 {
        value /= 1000.0;
        unit += 1;
    }
    format!("{value:.1} {}", UNITS[unit])
}

/// One-line budget for a path at [`theme::DETAIL_FONT`], sized for the
/// narrowest place one is shown (the 460 px incoming-request modal).
pub(crate) const PATH_CHARS: usize = 64;

/// Keep the tail of a long path visible: `/very/…/deep/folder`.
pub(crate) fn truncate_middle(s: &str, max_chars: usize) -> String {
    let count = s.chars().count();
    if count <= max_chars {
        return s.to_string();
    }
    let keep = max_chars.saturating_sub(1);
    let head = keep / 3;
    let tail = keep - head;
    let head_str: String = s.chars().take(head).collect();
    let tail_str: String = s.chars().skip(count - tail).collect();
    format!("{head_str}…{tail_str}")
}

fn render_toasts(ctx: &egui::Context, toasts: &[String]) {
    if toasts.is_empty() {
        return;
    }
    egui::Area::new(egui::Id::new("toasts"))
        .anchor(egui::Align2::CENTER_BOTTOM, egui::vec2(0.0, -40.0))
        .interactable(false)
        .show(ctx, |ui| {
            for text in toasts {
                egui::Frame::new()
                    .fill(theme::PANEL_FILL)
                    .stroke(egui::Stroke::new(1.0, theme::ACCENT))
                    .corner_radius(6.0)
                    .inner_margin(egui::Margin::symmetric(12, 6))
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new(text).size(theme::DETAIL_FONT));
                    });
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::display::SCALE;

    /// The panel the screens were drawn for, a handheld a little past it (the
    /// Miyoo Flip), a desktop window, and a phone held upright.
    const HANDHELD: egui::Vec2 = BASE_SIZE;
    const FLIP: egui::Vec2 = egui::vec2(752.0, 560.0);
    const DESKTOP: egui::Vec2 = egui::vec2(1280.0, 720.0);
    const UPRIGHT: egui::Vec2 = egui::vec2(BASE_SIZE.y, BASE_SIZE.x);

    /// Snapping multiplies the step back out, so the answers land a rounding
    /// short of the round number they read as.
    fn assert_scale(got: f32, want: f32) {
        assert!((got - want).abs() < SCALE_STEP / 2.0, "{got} is not {want}");
    }

    /// Every scale the setting can be stepped to.
    fn user_scales() -> impl Iterator<Item = f32> {
        let steps = ((SCALE.max - SCALE.min) / SCALE.step).round() as i32;
        (0..=steps).map(move |i| SCALE.min + i as f32 * SCALE.step)
    }

    #[test]
    fn the_design_lands_at_one_on_the_panel_it_was_drawn_for() {
        assert_scale(wanted_scale(HANDHELD, None, 1.0), 1.0);
        assert_scale(wanted_scale(DESKTOP, None, 1.0), 1.5);
    }

    /// The Flip's panel is 17% past the design, which is not worth rendering
    /// type between pixels for; the spare pixels become margin instead.
    #[test]
    fn a_panel_a_little_past_the_design_keeps_a_whole_zoom() {
        assert_scale(wanted_scale(FLIP, None, 1.0), 1.0);
        // Far enough past it, the fraction is the point.
        assert_scale(wanted_scale(DESKTOP, None, 1.0), 1.5);
    }

    /// A phone in portrait is the same screen turned, so the rows keep their
    /// size and the lists get a narrower measure.
    #[test]
    fn a_panel_held_upright_asks_for_the_same_scale() {
        assert_scale(wanted_scale(UPRIGHT, None, 1.0), 1.0);
        for panel in [HANDHELD, FLIP, DESKTOP] {
            let upright = egui::vec2(panel.y, panel.x);
            for user in user_scales() {
                assert_eq!(
                    wanted_scale(panel, None, user),
                    wanted_scale(upright, None, user),
                    "{panel:?} at {user} resizes itself when held upright"
                );
            }
        }
    }

    #[test]
    fn the_setting_is_read_against_whatever_the_panel_asked_for() {
        // The point of a factor: one setting means the same thing on both.
        assert_scale(wanted_scale(HANDHELD, None, 1.2), 1.2);
        assert_scale(wanted_scale(DESKTOP, None, 1.2), 1.8);
        // A launcher's RETSEND_SCALE is a base like any other.
        assert_scale(wanted_scale(DESKTOP, Some(2.0), 1.2), 2.4);
    }

    #[test]
    fn neither_end_of_the_setting_can_push_the_zoom_out_of_range() {
        for panel in [HANDHELD, DESKTOP, egui::vec2(7680.0, 4320.0)] {
            for user in [SCALE.min, SCALE.max] {
                let scale = wanted_scale(panel, None, user);
                assert!(
                    SCALE_RANGE.contains(&scale),
                    "{panel:?} at {user} → {scale}"
                );
            }
        }
    }

    /// What `Context::content_rect` hands back for a panel at `zoom`: the
    /// points it lays out in, snapped to egui's 1/32pt grid.
    fn content_rect(panel: egui::Vec2, zoom: f32) -> egui::Vec2 {
        use egui::emath::GuiRounding as _;
        (panel / zoom).round_ui()
    }

    /// One frame of the loop: measure the panel through the zoom in force, and
    /// ask what the next frame should be at.
    fn next_zoom(panel: egui::Vec2, zoom: f32, user: f32) -> f32 {
        wanted_scale(panel_points(content_rect(panel, zoom), zoom), None, user)
    }

    #[test]
    fn a_scale_settles_rather_than_swapping_between_two_steps_every_frame() {
        // The measurement is taken through the zoom set last frame, so a scale
        // landing exactly half a step from the grid could round either way
        // depending on the zoom it was measured through — and then change it.
        for panel in [HANDHELD, UPRIGHT, FLIP, DESKTOP] {
            for user in user_scales() {
                let mut zoom = 1.0;
                for _ in 0..8 {
                    zoom = next_zoom(panel, zoom, user);
                }
                assert_eq!(
                    next_zoom(panel, zoom, user),
                    zoom,
                    "{panel:?} at {user} swaps between the two"
                );
            }
        }
    }

    #[test]
    fn every_step_of_the_setting_moves_the_panel_the_design_was_drawn_for() {
        // A setting stepping by 5% into a zoom snapped to 10% would spend half
        // its presses on nothing at all.
        let mut zooms: Vec<f32> = user_scales()
            .map(|user| wanted_scale(HANDHELD, None, user))
            .collect();
        let asked = zooms.len();
        zooms.dedup();
        assert_eq!(zooms.len(), asked, "{zooms:?} repeats a zoom");
    }

    #[test]
    fn a_scale_is_snapped_so_a_resize_drag_does_not_relayout_on_every_pixel() {
        // 683×512 fits at 1.066…, and 1.15 asks for a step and a half.
        for (panel, user) in [(egui::vec2(683.0, 512.0), 1.0), (HANDHELD, 1.15)] {
            let steps = wanted_scale(panel, None, user) / SCALE_STEP;
            assert!((steps - steps.round()).abs() < 1e-3, "{steps} is not whole");
        }
    }

    #[test]
    fn a_step_lands_back_on_the_grid_rather_than_drifting() {
        let mut scale = 1.0;
        for _ in 0..4 {
            scale = SCALE.step(scale, 1);
        }
        assert_scale(scale, 1.2);
        // Neither end walks past itself.
        assert_eq!(SCALE.step(SCALE.max, 1), SCALE.max);
        assert_eq!(SCALE.step(SCALE.min, -1), SCALE.min);
    }

    #[test]
    fn fmt_bytes_scales_units() {
        assert_eq!(fmt_bytes(0), "0 B");
        assert_eq!(fmt_bytes(999), "999 B");
        assert_eq!(fmt_bytes(1_500), "1.5 KB");
        assert_eq!(fmt_bytes(1_048_576), "1.0 MB");
        assert_eq!(fmt_bytes(45_678_901), "45.7 MB");
        assert_eq!(fmt_bytes(1_000_000_000), "1.0 GB");
    }
}
