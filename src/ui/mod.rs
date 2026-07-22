//! egui integration and the per-frame render pass. Owns the overlay state
//! machines; `App` drives them through commands, this module draws them.

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

use crate::config::AppConfig;
use crate::net::server::DECISION_TIMEOUT;
use crate::net::NetService;
use crate::overlay::{
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
use crate::platform::window::AppWindow;
use crate::transfer::history::History;
use crate::transfer::inbound::FileState;
use crate::transfer::outbound::{OutboundPhase, OutboundSession};
use egui_sdl2::egui;
use egui_sdl2::EguiGlow;
use std::sync::atomic::Ordering;
use std::time::Duration;

/// Radar snapshots at most this stale while idle — covers peer-expiry pruning
/// and freshly announced ports without waking every frame.
const IDLE_REFRESH: Duration = Duration::from_secs(1);
/// The incoming modal's countdown bar animates at this cadence.
const PROMPT_REFRESH: Duration = Duration::from_millis(100);

pub struct AppUi {
    egui: EguiGlow,
    repaint_delay: Option<Duration>,
    pub tabs: Tabs,
    pub home: Home,
    pub history: HistoryView,
    pub settings: Settings,
    pub routes: RoutesView,
    pub browser: FileBrowser,
    pub osk: Osk,
    pub transfer: TransferView,
    pub toasts: Toasts,
    /// Peer count as of the last frame — the command router clamps the home
    /// cursor against it without re-locking the registry.
    pub peer_count: usize,
    /// History entry count as of the last frame — clamps the history cursor.
    pub history_count: usize,
}

impl AppUi {
    pub fn new(window: &AppWindow) -> Self {
        let egui = EguiGlow::new(window.sdl2_window(), window.glow_ctx(), None, false);
        theme::apply(&egui.ctx);
        let scale = crate::config::device_scale();
        if scale != 1.0 {
            log::info!("applying RETSEND_SCALE {scale}");
            egui.ctx.set_zoom_factor(scale);
        }
        Self {
            egui,
            repaint_delay: None,
            tabs: Tabs::new(),
            home: Home::new(),
            history: HistoryView::new(),
            settings: Settings::new(),
            routes: RoutesView::new(),
            browser: FileBrowser::new(),
            osk: Osk::new(),
            transfer: TransferView::new(),
            toasts: Toasts::new(),
            peer_count: 0,
            history_count: 0,
        }
    }

    /// Feed an SDL event to egui (resize/DPI bookkeeping, pointer hover).
    pub fn handle_event(&mut self, window: &AppWindow, event: &sdl2::event::Event) {
        let _ = self.egui.state.on_event(window.sdl2_window(), event);
    }

    /// How long the event loop may block before the next frame is due.
    pub fn take_repaint_delay(&mut self) -> Option<Duration> {
        self.repaint_delay.take()
    }

    /// Build the frame. Reads shared net state (brief locks) before entering
    /// the egui closure.
    pub fn update(&mut self, net: &NetService, config: &AppConfig, history: &History) {
        let peers = net.shared.peers.snapshot();
        self.peer_count = peers.len();
        let send_data = home::HomeData {
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
                    proto: p.info.protocol.as_deref().unwrap_or("http").to_uppercase(),
                })
                .collect(),
        };
        let receive_data = receive::ReceiveData {
            alias: config.device.alias.clone(),
            endpoint: endpoint_line(net),
            quick_save: config.transfer.auto_accept,
        };
        self.history_count = history.entries().len();
        let now = unix_now();
        let history_data = history::HistoryData {
            cursor: self.history.cursor(self.history_count),
            rows: history
                .entries()
                .iter()
                .rev()
                .map(|e| history::row(e, now))
                .collect(),
        };

        let routes_open = self.routes.open;
        let route_rows: Vec<(String, String)> = config
            .transfer
            .routes
            .iter()
            .map(|(ext, dir)| (ext.clone(), dir.clone()))
            .collect();
        let routes_data = routes::RoutesData {
            cursor: self.routes.cursor(route_rows.len()),
            rows: route_rows,
        };

        let prompt_data = prompt_data(net, config);
        let transfer_data = self.transfer_data();
        let active_tab = self.tabs.active();
        let settings_state = &self.settings;
        let toasts: Vec<String> = self.toasts.live().map(str::to_string).collect();
        let actual_port = net.http_port();

        self.egui.run(|ctx| {
            // egui 0.34 panels are shown inside an explicit root Ui spanning
            // the window (retsurf's pattern; top-level `show` is deprecated).
            let mut root = egui::Ui::new(
                ctx.clone(),
                egui::Id::new("root_ui"),
                egui::UiBuilder::new().max_rect(ctx.content_rect()),
            );
            root.set_clip_rect(ctx.content_rect());
            // Base-screen precedence mirrors Focus: the browser, the routes
            // editor, and the transfer takeover outrank the tabs; otherwise the
            // tab bar plus the active tab's body.
            if self.browser.open {
                browser::render(&mut root, &self.browser, &self.browser.target_alias);
            } else if routes_open {
                routes::render(&mut root, &routes_data);
            } else if let Some(t) = &transfer_data {
                transfer::render(&mut root, t);
            } else {
                tabs::render_bar(&mut root, active_tab);
                match active_tab {
                    Tab::Send => home::render(&mut root, &send_data),
                    Tab::Receive => receive::render(&mut root, &receive_data),
                    Tab::History => history::render(&mut root, &history_data),
                    Tab::Settings => {
                        settings::render(&mut root, settings_state, config, actual_port)
                    }
                }
            }
            if let Some(p) = &prompt_data {
                prompt::render(ctx, p);
            }
            if self.osk.active {
                osk::render(ctx, &self.osk);
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
        self.repaint_delay = Some(delay);
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

    pub fn draw(&mut self, window: &AppWindow) {
        self.egui.clear(theme::BACKGROUND);
        self.egui.paint();
        window.present();
    }

    pub fn destroy(&mut self) {
        self.egui.destroy();
    }
}

fn file_row(name: String, size: u64, state: &FileState, moved: u64) -> transfer::FileRow {
    transfer::FileRow {
        name,
        size,
        glyph: match state {
            FileState::Done => "✓",
            FileState::Failed(_) => "✗",
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

fn prompt_data(net: &NetService, config: &AppConfig) -> Option<prompt::PromptData> {
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
        dest: config.transfer.save_dir.clone(),
        remaining: 1.0 - elapsed / DECISION_TIMEOUT.as_secs_f32(),
    })
}

fn endpoint_line(net: &NetService) -> String {
    let (port, scheme) = {
        let me = net.shared.me.lock().unwrap();
        (
            me.port.unwrap_or(0),
            me.protocol.as_deref().unwrap_or("http").to_uppercase(),
        )
    };
    match crate::net::local_ip() {
        Some(ip) => format!("{scheme} · {ip}:{port}"),
        None => format!("{scheme} · port {port} · no network"),
    }
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
    use super::fmt_bytes;

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
