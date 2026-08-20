mod app;
// Public: the headless integration tests (tests/) drive the net layer and its
// config types directly, without SDL.
pub mod config;
mod event;
pub mod net;
mod overlay;
mod platform;
pub mod transfer;
mod ui;

use crate::app::App;

/// Shared startup for the desktop binary, the handheld launcher (same binary,
/// different env), and Android's `SDL_main`. Mirrors retsurf's `run_app`.
pub fn run_app() {
    // Capture panics before anything else can panic. On the handheld the
    // launcher usually discards stderr, so a bare panic leaves no trace beyond
    // exit code 101; mirroring it to a file is how we recover the message.
    install_panic_hook();

    init_logging();

    log::info!("init retsend {}", env!("CARGO_PKG_VERSION"));

    net::tls::install_provider();

    let mut app_config = config::AppConfig::load();
    if let Ok(v) = std::env::var("RETSEND_GLES") {
        app_config.display.use_gles = v != "0";
    }
    #[cfg(target_os = "android")]
    android_startup(&mut app_config);

    transfer::files::sweep_stale_parts(std::path::Path::new(&app_config.transfer.save_dir));

    // Headless receiver: no SDL, no screen — auto-accept into save_dir and
    // log to stdout. For ssh sessions on the device and scripted testing.
    if std::env::args().any(|a| a == "--receive") {
        run_headless(app_config);
        return;
    }

    // On a Wayland desktop SDL still often defaults to x11; align it to
    // Wayland. On the handheld (no WAYLAND_DISPLAY) this is skipped and SDL
    // falls back to its kmsdrm driver. An explicit SDL_VIDEODRIVER wins.
    // Android has its own video driver and no WAYLAND_DISPLAY, so skip it.
    #[cfg(not(target_os = "android"))]
    if std::env::var_os("SDL_VIDEODRIVER").is_none()
        && std::env::var_os("WAYLAND_DISPLAY").is_some()
    {
        std::env::set_var("SDL_VIDEODRIVER", "wayland");
    }

    let sdl = sdl2::init().unwrap();
    let app = App::new(&sdl, app_config).unwrap();

    app.run();
}

/// SDL's Android shell (`SDLActivity`) `dlopen`s our cdylib and calls this on
/// its own thread — Android's `src/main.rs`.
#[cfg(target_os = "android")]
#[no_mangle]
pub extern "C" fn SDL_main(
    _argc: std::os::raw::c_int,
    _argv: *const *const std::os::raw::c_char,
) -> std::os::raw::c_int {
    run_app();
    0
}

/// What only Android decides. Its paths, alias and UI scale arrive as the same
/// `RETSEND_*` env vars the handheld launchers set, written by `RetsendActivity`
/// before SDL starts.
#[cfg(target_os = "android")]
fn android_startup(config: &mut config::AppConfig) {
    // Mali/Adreno/PowerVR expose only GLES, so desktop GL is never an option.
    config.display.use_gles = true;

    // egui-sdl2 synthesizes a pointer stream from finger events itself; SDL's own
    // synthesis would deliver every tap twice.
    std::env::set_var("SDL_TOUCH_MOUSE_EVENTS", "0");

    // Untrapped, Back backgrounds the activity and the app never sees it. Trapped,
    // it arrives as an AC_BACK key — the only way out of a screen without a pad.
    std::env::set_var("SDL_ANDROID_TRAP_BACK_BUTTON", "1");
}

#[cfg(target_os = "android")]
fn init_logging() {
    // No stderr on Android; route `log` to logcat (`adb logcat -s retsend`).
    android_logger::init_once(
        android_logger::Config::default()
            .with_max_level(log::LevelFilter::Info)
            .with_tag("retsend"),
    );
}

#[cfg(not(target_os = "android"))]
fn init_logging() {
    let env = env_logger::Env::default()
        .filter_or("RETSEND_LOG_LEVEL", "info")
        .write_style_or("RETSEND_LOG_STYLE", "always");
    let mut builder = env_logger::Builder::from_env(env);
    // The handheld launcher discards stderr too: mirror logs to a file when asked.
    if let Ok(path) = std::env::var("RETSEND_LOG_FILE") {
        match std::fs::File::create(&path) {
            Ok(file) => {
                builder.target(env_logger::Target::Pipe(Box::new(file)));
            }
            Err(e) => eprintln!("failed to open RETSEND_LOG_FILE `{path}`: {e}"),
        }
    }
    builder.init();
}

/// `--receive`: run just the net stack with auto-accept forced and progress
/// on stdout, until Ctrl-C. Everything is reused; only the waker is a no-op
/// (there is no event loop to wake — the poll loop below reads shared state).
fn run_headless(mut config: config::AppConfig) {
    use std::sync::atomic::Ordering;

    struct NoopWake;
    impl net::Wake for NoopWake {
        fn wake(&self, _reason: net::WakeReason) {}
    }

    config.transfer.auto_accept = true;
    let net = match net::NetService::spawn(
        &config.device,
        &config.network,
        &config.transfer,
        std::path::Path::new(&config::data_dir()),
        std::sync::Arc::new(NoopWake),
    ) {
        Ok(net) => net,
        Err(e) => {
            eprintln!("failed to start networking: {e}");
            std::process::exit(1);
        }
    };
    println!(
        "receiving as `{}` on port {} into {}  (Ctrl-C to quit)",
        config.device.alias,
        net.http_port(),
        config.transfer.save_dir
    );

    let mut known_peers = std::collections::HashSet::new();
    let mut last: Option<std::sync::Arc<transfer::inbound::InboundSession>> = None;
    loop {
        std::thread::sleep(std::time::Duration::from_millis(500));

        for peer in net.shared.peers.snapshot() {
            if known_peers.insert(peer.info.fingerprint.clone()) {
                println!("found `{}` at {}", peer.info.alias, peer.ip);
            }
        }

        let active = net.shared.active.lock().unwrap().clone();
        if let Some(session) = &active {
            let received = session.received_total.load(Ordering::Relaxed);
            let percent = (received * 100)
                .checked_div(session.total_bytes)
                .unwrap_or(100);
            println!(
                "receiving from `{}`: {percent}% ({}/{} files)",
                session.peer_alias,
                session.done_count(),
                session.files.len()
            );
        }
        // The slot cleared (or was replaced): report the outcome once.
        if let Some(session) = last.take() {
            let gone = active
                .as_ref()
                .is_none_or(|a| a.session_id != session.session_id);
            if gone && session.is_finished() {
                println!(
                    "done: {}/{} files from `{}`",
                    session.done_count(),
                    session.files.len(),
                    session.peer_alias
                );
            }
        }
        last = active;
    }
}

/// Mirror panics to a file in addition to stderr: `RETSEND_PANIC_FILE` if set,
/// else `retsend-panic.log` in the working directory. The default
/// backtrace hook still runs after us, so desktop behavior is unchanged.
fn install_panic_hook() {
    let default = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let path =
            std::env::var("RETSEND_PANIC_FILE").unwrap_or_else(|_| "retsend-panic.log".to_string());
        let backtrace = std::backtrace::Backtrace::force_capture();
        let _ = std::fs::write(&path, format!("{info}\n\nbacktrace:\n{backtrace}\n"));
        default(info);
    }));
}
