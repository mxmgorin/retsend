//! egui integration and the per-frame render pass. Owns the overlay state
//! machines; `App` drives them through commands, this module draws them.

mod home;
mod settings;
pub mod theme;

use crate::config::AppConfig;
use crate::net::NetService;
use crate::overlay::{home::Home, settings::Settings, toast::Toasts, Focus};
use crate::platform::window::AppWindow;
use egui_sdl2::egui;
use egui_sdl2::EguiGlow;
use std::time::Duration;

/// Radar snapshots at most this stale while idle — covers peer-expiry pruning
/// and freshly announced ports without waking every frame.
const IDLE_REFRESH: Duration = Duration::from_secs(1);

pub struct AppUi {
    egui: EguiGlow,
    repaint_delay: Option<Duration>,
    pub home: Home,
    pub settings: Settings,
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
            toasts: Toasts::new(),
            peer_count: 0,
        }
    }

    pub fn focus(&self) -> Focus {
        if self.settings.open {
            Focus::Settings
        } else {
            Focus::Home
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

    /// Build the frame. Reads shared net state (brief registry lock) before
    /// entering the egui closure.
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
            if settings_open {
                settings::render(&mut root, settings_state, config, actual_port);
            } else {
                home::render(&mut root, &data);
            }
            render_toasts(ctx, &toasts);
        });

        // Fold the frame-timing sources into one idle bound: egui's own
        // request (animations/sizing passes), toast expiry, radar staleness.
        let mut delay = self.egui.repaint_delay().min(IDLE_REFRESH);
        if let Some(t) = self.toasts.next_expiry() {
            delay = delay.min(t);
        }
        self.repaint_delay = Some(delay);
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

fn endpoint_line(net: &NetService) -> String {
    let port = net.http_port();
    match crate::net::local_ip() {
        Some(ip) => format!("HTTP · {ip}:{port}"),
        None => format!("HTTP · port {port} · no network"),
    }
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
