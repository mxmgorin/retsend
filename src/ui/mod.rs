//! egui integration and the per-frame render pass. Owns the overlay state
//! machines; `App` drives them through commands, this module draws them.

mod browser;
mod home;
mod osk;
mod prompt;
mod settings;
pub mod theme;
mod transfer;

use crate::config::AppConfig;
use crate::net::server::DECISION_TIMEOUT;
use crate::net::NetService;
use crate::overlay::{
    browser::FileBrowser,
    home::Home,
    osk::Osk,
    settings::Settings,
    toast::Toasts,
    transfer::{TransferView, Viewed},
};
use crate::platform::window::AppWindow;
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
    pub home: Home,
    pub settings: Settings,
    pub browser: FileBrowser,
    pub osk: Osk,
    pub transfer: TransferView,
    pub toasts: Toasts,
    /// Peer count as of the last frame — the command router clamps the home
    /// cursor against it without re-locking the registry.
    pub peer_count: usize,
}

impl AppUi {
    pub fn new(window: &AppWindow) -> Self {
        let egui = EguiGlow::new(window.sdl2_window(), window.glow_ctx(), None, false);
        theme::apply(&egui.ctx);
        let scale = crate::config::device_scale();
        if scale != 1.0 {
            log::info!("applying LSRETRO_SCALE {scale}");
            egui.ctx.set_zoom_factor(scale);
        }
        Self {
            egui,
            repaint_delay: None,
            home: Home::new(),
            settings: Settings::new(),
            browser: FileBrowser::new(),
            osk: Osk::new(),
            transfer: TransferView::new(),
            toasts: Toasts::new(),
            peer_count: 0,
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
    pub fn update(&mut self, net: &NetService, config: &AppConfig) {
        let peers = net.shared.peers.snapshot();
        self.peer_count = peers.len();
        let data = home::HomeData {
            alias: config.device.alias.clone(),
            endpoint: endpoint_line(net),
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

        let prompt_data = prompt_data(net, config);
        let transfer_data = self.transfer_data();
        let settings_open = self.settings.open;
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
            // Base-screen precedence mirrors Focus: a browser opened from
            // Settings draws (and gets input) above it.
            if self.browser.open {
                browser::render(&mut root, &self.browser, &self.browser.target_alias);
            } else if settings_open {
                settings::render(&mut root, settings_state, config, actual_port);
            } else if let Some(t) = &transfer_data {
                transfer::render(&mut root, t);
            } else {
                home::render(&mut root, &data);
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

/// "999 B", "12.3 KB", "1.2 GB" — one decimal above bytes.
pub(crate) fn fmt_bytes(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["KB", "MB", "GB", "TB"];
    if bytes < 1000 {
        return format!("{bytes} B");
    }
    let mut value = bytes as f64;
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
