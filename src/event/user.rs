//! Cross-thread wake events. Net/transfer threads push an SDL user event to
//! unblock the main loop's `wait_event`; the handlers return no command — the
//! per-frame reads in `App::run`/`AppUi` pick up the new shared state.
//! Same mechanism as retsurf's `event::user`.

use sdl2::sys::{SDL_Event, SDL_PushEvent, SDL_UserEvent};

#[repr(i32)]
#[derive(Copy, Clone)]
pub enum UserEvent {
    /// A peer appeared/refreshed in the registry; the radar re-reads it.
    PeersUpdated = 0,
    /// A prepare-upload is parked awaiting the user (M2: pops the modal).
    IncomingRequest = 1,
    /// Transfer bytes moved (throttled worker-side); progress bars re-read.
    TransferProgress = 2,
    /// A transfer finished or failed.
    TransferDone = 3,
}

#[derive(Clone)]
pub struct UserEventSender {
    event_type: u32,
}

impl UserEventSender {
    pub fn new() -> Self {
        Self {
            event_type: unsafe { sdl2::sys::SDL_RegisterEvents(1) },
        }
    }

    pub fn send(&self, event: UserEvent) {
        unsafe {
            let mut evt = SDL_Event {
                user: SDL_UserEvent {
                    type_: self.event_type,
                    timestamp: 0,
                    windowID: 0,
                    code: event as i32,
                    data1: std::ptr::null_mut(),
                    data2: std::ptr::null_mut(),
                },
            };
            SDL_PushEvent(&mut evt);
        }
    }
}

/// Net threads wake the UI through this; keeping `net` SDL-free is what lets
/// the integration tests run headless with a no-op waker.
impl crate::net::Wake for UserEventSender {
    fn wake(&self, reason: crate::net::WakeReason) {
        self.send(match reason {
            crate::net::WakeReason::Peers => UserEvent::PeersUpdated,
            crate::net::WakeReason::Incoming => UserEvent::IncomingRequest,
            crate::net::WakeReason::Progress => UserEvent::TransferProgress,
            crate::net::WakeReason::Done => UserEvent::TransferDone,
        });
    }
}
