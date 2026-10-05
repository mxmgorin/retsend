//! The way out that no key names: the signal a launcher sends to take the device
//! down. It arrives as SDL's quit event, which the loop answers by stopping the
//! net threads.

/// Ask the loop to close. Callable from any thread: `SDL_PushEvent` is
/// thread-safe, and the pump wakes on what it queues.
#[cfg(all(unix, not(any(target_os = "android", target_os = "vita"))))]
pub fn push() {
    use sdl2::sys::{SDL_Event, SDL_EventType, SDL_PushEvent, SDL_QuitEvent};
    unsafe {
        let mut evt = SDL_Event {
            quit: SDL_QuitEvent {
                type_: SDL_EventType::SDL_QUIT as u32,
                timestamp: 0,
            },
        };
        SDL_PushEvent(&mut evt);
    }
}

/// Close on `SIGTERM` rather than dying where the frame stands: Allium sends it
/// to power off, then `SIGKILL`s five seconds later. `SIGINT` is left alone —
/// Ctrl-C has to kill a wedged run.
///
/// Call before the first thread is spawned: the mask is inherited, and blocking
/// it everywhere is what leaves the signal to the waiter.
#[cfg(all(unix, not(any(target_os = "android", target_os = "vita"))))]
pub fn on_termination() {
    let mut set: libc::sigset_t = unsafe { std::mem::zeroed() };
    unsafe {
        libc::sigemptyset(&mut set);
        libc::sigaddset(&mut set, libc::SIGTERM);
        libc::pthread_sigmask(libc::SIG_BLOCK, &set, std::ptr::null_mut());
    }
    std::thread::spawn(move || {
        let mut signal = 0;
        // Blocked everywhere else, so no handler runs and nothing here has to be
        // signal-safe.
        if unsafe { libc::sigwait(&set, &mut signal) } == 0 {
            log::info!("SIGTERM: quitting");
            push();
        }
        // A wedged loop would never draw the quit out, so hand the second signal
        // back to the kernel — parked, because a thread that returns takes its
        // unblocked mask with it.
        unsafe {
            libc::signal(libc::SIGTERM, libc::SIG_DFL);
            libc::pthread_sigmask(libc::SIG_UNBLOCK, &set, std::ptr::null_mut());
        }
        loop {
            std::thread::park();
        }
    });
}

/// Android terminates its own way, and a platform without POSIX signals (the
/// Vita's newlib among them) has nothing to install.
#[cfg(not(all(unix, not(any(target_os = "android", target_os = "vita")))))]
pub fn on_termination() {}
