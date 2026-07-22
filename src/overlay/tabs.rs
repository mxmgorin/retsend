//! Top-level tab selection: which of Send / Receive / Settings is showing,
//! plus the tab to return to when Start toggles Settings back off. Pure state
//! (no egui); `crate::ui::tabs` draws the bar, `App` drives the switching.

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Tab {
    Send,
    Receive,
    History,
    Settings,
}

/// Left-to-right order in the bar, and the cycle L1/R1 walks.
const ORDER: [Tab; 4] = [Tab::Send, Tab::Receive, Tab::History, Tab::Settings];

pub struct Tabs {
    active: Tab,
    /// Where Start (or B on Settings) returns to when it toggles Settings off.
    prev: Tab,
}

impl Tabs {
    /// Land on Receive — the device's primary role is to be received-to.
    pub fn new() -> Self {
        Self {
            active: Tab::Receive,
            prev: Tab::Send,
        }
    }

    pub fn active(&self) -> Tab {
        self.active
    }

    /// L1/R1: step to the previous/next tab, wrapping around.
    pub fn cycle(&mut self, delta: i32) {
        let idx = ORDER.iter().position(|t| *t == self.active).unwrap_or(0) as i32;
        let next = (idx + delta).rem_euclid(ORDER.len() as i32) as usize;
        self.set(ORDER[next]);
    }

    /// Start: jump to Settings, or back to the previously-active tab when
    /// already there.
    pub fn toggle_settings(&mut self) {
        let target = if self.active == Tab::Settings {
            self.prev
        } else {
            Tab::Settings
        };
        self.set(target);
    }

    fn set(&mut self, tab: Tab) {
        if tab != self.active {
            self.prev = self.active;
            self.active = tab;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cycle_wraps_both_ways() {
        let mut tabs = Tabs::new(); // Receive
        tabs.cycle(1);
        assert_eq!(tabs.active(), Tab::History);
        tabs.cycle(1);
        assert_eq!(tabs.active(), Tab::Settings);
        tabs.cycle(1); // wrap past the end
        assert_eq!(tabs.active(), Tab::Send);
        tabs.cycle(-1); // wrap past the front
        assert_eq!(tabs.active(), Tab::Settings);
    }

    #[test]
    fn start_toggles_settings_and_restores_previous() {
        let mut tabs = Tabs::new(); // Receive
        tabs.toggle_settings();
        assert_eq!(tabs.active(), Tab::Settings);
        tabs.toggle_settings();
        assert_eq!(tabs.active(), Tab::Receive);

        // Reached via the cycle, Start still returns to wherever we came from.
        tabs.cycle(-1); // Receive -> Send
        tabs.cycle(1); // Send -> Receive
        tabs.toggle_settings(); // -> Settings, prev = Receive
        tabs.toggle_settings(); // -> Receive
        assert_eq!(tabs.active(), Tab::Receive);
    }
}
