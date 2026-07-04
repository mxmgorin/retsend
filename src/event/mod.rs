//! SDL event pump ownership and routing: gamepad, keyboard, window/quit.
//! Blocks when idle (retsurf's pattern) so the app costs nothing while nobody
//! is pressing buttons.

mod gamepad;
mod keyboard;

use crate::app::AppCommand;
use crate::config::InputConfig;
use crate::platform::window::AppWindow;
use crate::ui::AppUi;
use gamepad::Gamepad;
use sdl2::event::Event;

pub struct AppEventHandler {
    event_pump: sdl2::EventPump,
    game_controllers: Vec<sdl2::controller::GameController>,
    game_controller_subsystem: sdl2::GameControllerSubsystem,
    gamepad: Gamepad,
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

        Ok(Self {
            event_pump: sdl.event_pump()?,
            game_controllers,
            game_controller_subsystem,
            gamepad: Gamepad::new(input_cfg),
        })
    }

    /// Block for the next event when idle, then drain everything queued this
    /// frame and emit gamepad repeats. `ui.take_repaint_delay()` bounds the
    /// block when egui or a toast wants a timed follow-up frame.
    pub fn wait(&mut self, window: &AppWindow, ui: &mut AppUi, commands: &mut Vec<AppCommand>) {
        if !self.gamepad.is_active() {
            match ui.take_repaint_delay() {
                Some(delay) => {
                    let ms = delay.as_millis().min(u32::MAX as u128) as u32;
                    if ms > 0 {
                        if let Some(event) = self.event_pump.wait_event_timeout(ms) {
                            self.handle_event(event, window, ui, commands);
                        }
                    }
                }
                None => {
                    let event = self.event_pump.wait_event();
                    self.handle_event(event, window, ui, commands);
                }
            }
        }

        while let Some(event) = self.event_pump.poll_event() {
            self.handle_event(event, window, ui, commands);
        }

        self.gamepad.tick(commands);
    }

    fn handle_event(
        &mut self,
        event: Event,
        window: &AppWindow,
        ui: &mut AppUi,
        commands: &mut Vec<AppCommand>,
    ) {
        // Feed egui first (pointer hover, window resize/DPI bookkeeping). Our
        // navigation is command-driven, so egui never consumes the keys we
        // care about — no text fields exist yet.
        ui.handle_event(window, &event);

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
                self.gamepad.on_button(button, true, commands);
            }
            Event::ControllerButtonUp { button, .. } => {
                self.gamepad.on_button(button, false, commands);
            }
            Event::ControllerAxisMotion { axis, value, .. } => {
                self.gamepad.on_axis(axis, value, commands);
            }
            Event::KeyDown {
                keycode: Some(kc),
                repeat,
                ..
            } => keyboard::on_key_down(kc, repeat, commands),
            Event::Quit { .. } => commands.push(AppCommand::Shutdown),
            _ => {}
        }
    }
}
