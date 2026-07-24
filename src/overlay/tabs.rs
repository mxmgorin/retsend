//! Top-level tab selection: which of Send / Receive / History / Settings is
//! showing. Pure state (no egui); `crate::ui::tabs` draws the bar, `App` drives
//! the switching.

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
}

impl Tabs {
    /// Land on Receive — the device's primary role is to be received-to.
    pub fn new() -> Self {
        Self {
            active: Tab::Receive,
        }
    }

    pub fn active(&self) -> Tab {
        self.active
    }

    /// L1/R1: step to the previous/next tab, wrapping around.
    pub fn cycle(&mut self, delta: i32) {
        let idx = ORDER.iter().position(|t| *t == self.active).unwrap_or(0) as i32;
        let next = (idx + delta).rem_euclid(ORDER.len() as i32) as usize;
        self.active = ORDER[next];
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
}
