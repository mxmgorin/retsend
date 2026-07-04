//! Controller input → [`AppCommand`]s. A trimmed-down take on retsurf's
//! gesture resolver: taps dispatch on press, and held directions (D-pad or
//! left stick past the dead zone) auto-repeat navigation. The tap/hold/chord
//! machinery and bindings.toml arrive with the settings milestone.

use crate::app::{AppCommand, Direction};
use crate::config::InputConfig;
use sdl2::controller::{Axis, Button};
use std::time::Instant;

pub struct Gamepad {
    config: InputConfig,
    /// Currently held direction (last one pressed wins) and its repeat clock.
    held: Option<Held>,
    /// Left-stick state folded into digital directions with hysteresis.
    stick: (f32, f32),
    stick_dir: Option<Direction>,
}

struct Held {
    dir: Direction,
    pressed_at: Instant,
    last_repeat: Option<Instant>,
}

impl Gamepad {
    pub fn new(config: InputConfig) -> Self {
        Self {
            config,
            held: None,
            stick: (0.0, 0.0),
            stick_dir: None,
        }
    }

    /// While a direction is held the main loop must keep ticking (repeats fire
    /// from [`Self::tick`]); otherwise it may block on the event pump.
    pub fn is_active(&self) -> bool {
        self.held.is_some()
    }

    pub fn on_button(&mut self, button: Button, pressed: bool, commands: &mut Vec<AppCommand>) {
        if let Some(dir) = dpad_dir(button) {
            if pressed {
                commands.push(AppCommand::Nav(dir));
                self.hold(dir);
            } else if self.held.as_ref().is_some_and(|h| h.dir == dir) {
                self.held = None;
            }
            return;
        }
        if !pressed {
            return;
        }
        match button {
            Button::A => commands.push(AppCommand::Confirm),
            Button::B => commands.push(AppCommand::Back),
            Button::Start => commands.push(AppCommand::ToggleSettings),
            Button::Back => commands.push(AppCommand::ReAnnounce),
            Button::LeftShoulder => commands.push(AppCommand::PageUp),
            Button::RightShoulder => commands.push(AppCommand::PageDown),
            _ => {}
        }
    }

    pub fn on_axis(&mut self, axis: Axis, value: i16, commands: &mut Vec<AppCommand>) {
        let v = value as f32 / i16::MAX as f32;
        match axis {
            Axis::LeftX => self.stick.0 = v,
            Axis::LeftY => self.stick.1 = v,
            _ => return,
        }
        // Digitalize with hysteresis: engage past the dead zone, release only
        // when well back inside it, so a wobbling stick doesn't chatter.
        let engage = self.config.deadzone;
        let release = engage * 0.6;
        let (x, y) = self.stick;
        let dir = if x.abs().max(y.abs()) >= engage {
            Some(if x.abs() > y.abs() {
                if x > 0.0 {
                    Direction::Right
                } else {
                    Direction::Left
                }
            } else if y > 0.0 {
                Direction::Down // SDL Y axis is positive downward
            } else {
                Direction::Up
            })
        } else if x.abs().max(y.abs()) <= release {
            None
        } else {
            self.stick_dir // in the hysteresis band: keep the current state
        };

        if dir != self.stick_dir {
            if let Some(old) = self.stick_dir {
                if self.held.as_ref().is_some_and(|h| h.dir == old) {
                    self.held = None;
                }
            }
            if let Some(d) = dir {
                commands.push(AppCommand::Nav(d));
                self.hold(d);
            }
            self.stick_dir = dir;
        }
    }

    /// Fire navigation repeats for the held direction. Called once per frame.
    pub fn tick(&mut self, commands: &mut Vec<AppCommand>) {
        let Some(held) = &mut self.held else { return };
        let now = Instant::now();
        let initial = std::time::Duration::from_millis(self.config.repeat_initial_delay_ms);
        let interval = std::time::Duration::from_millis(self.config.repeat_interval_ms);
        let due = match held.last_repeat {
            None => now.duration_since(held.pressed_at) >= initial,
            Some(last) => now.duration_since(last) >= interval,
        };
        if due {
            commands.push(AppCommand::Nav(held.dir));
            held.last_repeat = Some(now);
        }
    }

    fn hold(&mut self, dir: Direction) {
        self.held = Some(Held {
            dir,
            pressed_at: Instant::now(),
            last_repeat: None,
        });
    }
}

fn dpad_dir(button: Button) -> Option<Direction> {
    match button {
        Button::DPadUp => Some(Direction::Up),
        Button::DPadDown => Some(Direction::Down),
        Button::DPadLeft => Some(Direction::Left),
        Button::DPadRight => Some(Direction::Right),
        _ => None,
    }
}
