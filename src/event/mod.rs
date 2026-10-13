//! SDL event pump ownership and routing: gamepad, keyboard, window/quit, and
//! the cross-thread user events. Blocks when idle (retsurf's pattern) so the
//! app costs nothing while nobody is transferring or pressing buttons.

mod gamepad;
pub mod hints;
mod keyboard;
pub mod quit;
pub mod user;

use crate::app::{AppCommand, Direction};
use crate::config::input::PadButton;
use crate::config::InputConfig;
use crate::ui::AppUi;
use gamepad::Gamepad;

/// What `button` asks of the app, before any A/B swap.
pub(crate) fn command(button: PadButton) -> AppCommand {
    match button {
        PadButton::Up => AppCommand::Nav(Direction::Up),
        PadButton::Down => AppCommand::Nav(Direction::Down),
        PadButton::Left => AppCommand::Nav(Direction::Left),
        PadButton::Right => AppCommand::Nav(Direction::Right),
        PadButton::A => AppCommand::Confirm,
        PadButton::B => AppCommand::Back,
        PadButton::X => AppCommand::Alt,
        PadButton::Y => AppCommand::TogglePin,
        PadButton::Start => AppCommand::Start,
        PadButton::Select => AppCommand::ReAnnounce,
        PadButton::L1 => AppCommand::PageUp,
        PadButton::R1 => AppCommand::PageDown,
        PadButton::Menu => AppCommand::Shutdown,
    }
}

fn is_face(button: PadButton) -> bool {
    matches!(
        button,
        PadButton::A | PadButton::B | PadButton::X | PadButton::Y
    )
}

/// What a face button does once A/B and X/Y are swapped: the other button of
/// its pair. Applied to the pad and to a pad that arrives as keys, never to a
/// keyboard proper, whose Enter and Esc carry no printed letter.
pub(crate) fn swap_face(cmd: AppCommand) -> AppCommand {
    if !crate::config::input::swap_abxy() {
        return cmd;
    }
    match cmd {
        AppCommand::Confirm => AppCommand::Back,
        AppCommand::Back => AppCommand::Confirm,
        AppCommand::Alt => AppCommand::TogglePin,
        AppCommand::TogglePin => AppCommand::Alt,
        other => other,
    }
}
use keyboard::Keymap;
pub use keyboard::{bind_key, effective_keys, key_label, Layout};
use sdl2::event::Event;

pub struct AppEventHandler {
    event_pump: sdl2::EventPump,
    game_controllers: Vec<sdl2::controller::GameController>,
    game_controller_subsystem: sdl2::GameControllerSubsystem,
    gamepad: Gamepad,
    /// The layout's keys with the `[input.keys]` overrides applied.
    keymap: Keymap,
}

impl AppEventHandler {
    pub fn new(sdl: &sdl2::Sdl, input_cfg: InputConfig) -> Result<Self, String> {
        let mut game_controllers = vec![];
        let game_controller_subsystem = sdl.game_controller()?;

        for id in 0..game_controller_subsystem.num_joysticks()? {
            if game_controller_subsystem.is_game_controller(id) {
                match game_controller_subsystem.open(id) {
                    Ok(controller) => game_controllers.push(controller),
                    Err(e) => log::warn!("could not open controller {id}: {e}"),
                }
            }
        }

        let layout = Layout::detect(sdl.video()?.current_video_driver());
        log::info!("keyboard layout: {layout:?}");
        let keymap = Keymap::new(layout, &input_cfg.keys);

        Ok(Self {
            event_pump: sdl.event_pump()?,
            game_controllers,
            game_controller_subsystem,
            gamepad: Gamepad::new(input_cfg),
            keymap,
        })
    }

    pub fn layout(&self) -> Layout {
        self.keymap.layout()
    }

    /// Rebuild the keymap after `[input.keys]` changed.
    pub fn set_key_overrides(&mut self, overrides: &crate::config::input::KeyBindings) {
        self.keymap.set_overrides(overrides);
    }

    /// Arm or disarm reporting the next key press as [`AppCommand::BindKey`].
    pub fn capture_key(&mut self, on: bool) {
        self.keymap.capture(on);
    }

    /// Block for the next event when idle, then drain everything queued this
    /// frame and emit gamepad repeats. The block is bounded by the earlier of
    /// egui's repaint request and the next auto-repeat — a held D-pad wakes
    /// the loop per repeat instead of spinning it.
    pub fn wait(&mut self, ui: &mut AppUi, commands: &mut Vec<AppCommand>) {
        let deadline = match (ui.take_repaint_delay(), self.gamepad.next_repeat_in()) {
            (Some(repaint), Some(repeat)) => Some(repaint.min(repeat)),
            (repaint, repeat) => repaint.or(repeat),
        };
        match deadline {
            Some(delay) => {
                let ms = delay.as_millis().min(u32::MAX as u128) as u32;
                if ms > 0 {
                    if let Some(event) = self.event_pump.wait_event_timeout(ms) {
                        self.handle_event(event, ui, commands);
                    }
                }
            }
            None => {
                let event = self.event_pump.wait_event();
                self.handle_event(event, ui, commands);
            }
        }

        while let Some(event) = self.event_pump.poll_event() {
            self.handle_event(event, ui, commands);
        }

        self.gamepad.tick(commands);
    }

    fn handle_event(&mut self, event: Event, ui: &mut AppUi, commands: &mut Vec<AppCommand>) {
        if is_indirect_touch(&event) {
            return;
        }
        // Feed egui first (pointer hover, window resize/DPI bookkeeping). Our
        // navigation is command-driven, so egui never consumes the keys we
        // care about — no text fields exist yet.
        ui.handle_event(&event);

        match event {
            Event::ControllerDeviceAdded { which, .. } => {
                if let Ok(controller) = self.game_controller_subsystem.open(which) {
                    log::info!("controller {} connected", controller.name());
                    self.game_controllers.push(controller);
                }
            }
            Event::ControllerDeviceRemoved { which, .. } => {
                self.game_controllers.retain(|c| c.instance_id() != which);
                log::info!("controller {which} disconnected");
            }
            Event::ControllerButtonDown { button, .. } => {
                hints::pad_used();
                self.gamepad.on_button(button, true, commands);
            }
            Event::ControllerButtonUp { button, .. } => {
                self.gamepad.on_button(button, false, commands);
            }
            Event::ControllerAxisMotion { axis, value, .. } => {
                let before = commands.len();
                self.gamepad.on_axis(axis, value, commands);
                // Only a push past the dead zone counts, not stick drift.
                if commands.len() > before {
                    hints::pad_used();
                }
            }
            Event::KeyDown {
                keycode: Some(kc),
                repeat,
                ..
            } => self.keymap.on_key_down(kc, repeat, commands),
            Event::Quit { .. } => commands.push(AppCommand::Shutdown),
            // User events exist purely to unblock the wait; per-frame reads of
            // the shared net state pick up whatever changed.
            Event::User { .. } => {}
            _ => {}
        }
    }
}

/// A finger on a pad that does not overlay the screen, such as the Vita's rear
/// touchpad: its coordinates are not window positions, so it must not tap.
fn is_indirect_touch(event: &Event) -> bool {
    use sdl2::sys::{SDL_GetTouchDeviceType, SDL_TouchDeviceType};
    let touch_id = match event {
        Event::FingerDown { touch_id, .. }
        | Event::FingerUp { touch_id, .. }
        | Event::FingerMotion { touch_id, .. } => *touch_id,
        _ => return false,
    };
    // SAFETY: a lookup by id; an unknown id yields SDL_TOUCH_DEVICE_INVALID.
    let kind = unsafe { SDL_GetTouchDeviceType(touch_id) };
    kind != SDL_TouchDeviceType::SDL_TOUCH_DEVICE_DIRECT
}
