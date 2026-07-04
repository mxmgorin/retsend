mod app;
// Public: headless tests and tools reach the config layer without SDL.
pub mod config;
mod event;
mod overlay;
mod platform;
mod ui;

use crate::app::App;

/// Shared startup for the desktop binary (and later the handheld launcher —
/// same binary, different env). Mirrors retsurf's `run_app`.
pub fn run_app() {
    // Capture panics before anything else can panic. On the handheld the
    // launcher usually discards stderr, so a bare panic leaves no trace beyond
    // exit code 101; mirroring it to a file is how we recover the message.
    install_panic_hook();

    init_logging();

    log::info!("init localsend-retro {}", env!("CARGO_PKG_VERSION"));

    let mut app_config = config::AppConfig::load();
    if let Ok(v) = std::env::var("LSRETRO_GLES") {
        app_config.display.use_gles = v != "0";
    }

    // On a Wayland desktop SDL still often defaults to x11; align it to
    // Wayland. On the handheld (no WAYLAND_DISPLAY) this is skipped and SDL
    // falls back to its kmsdrm driver. An explicit SDL_VIDEODRIVER wins.
    if std::env::var_os("SDL_VIDEODRIVER").is_none()
        && std::env::var_os("WAYLAND_DISPLAY").is_some()
    {
        std::env::set_var("SDL_VIDEODRIVER", "wayland");
    }

    let sdl = sdl2::init().unwrap();
    let app = App::new(&sdl, app_config).unwrap();

    app.run();
}

fn init_logging() {
    let env = env_logger::Env::default()
        .filter_or("LSRETRO_LOG_LEVEL", "info")
        .write_style_or("LSRETRO_LOG_STYLE", "always");
    let mut builder = env_logger::Builder::from_env(env);
    // The handheld launcher discards stderr too: mirror logs to a file when asked.
    if let Ok(path) = std::env::var("LSRETRO_LOG_FILE") {
        match std::fs::File::create(&path) {
            Ok(file) => {
                builder.target(env_logger::Target::Pipe(Box::new(file)));
            }
            Err(e) => eprintln!("failed to open LSRETRO_LOG_FILE `{path}`: {e}"),
        }
    }
    builder.init();
}

/// Mirror panics to a file in addition to stderr: `LSRETRO_PANIC_FILE` if set,
/// else `localsend-retro-panic.log` in the working directory. The default
/// backtrace hook still runs after us, so desktop behavior is unchanged.
fn install_panic_hook() {
    let default = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let path = std::env::var("LSRETRO_PANIC_FILE")
            .unwrap_or_else(|_| "localsend-retro-panic.log".to_string());
        let backtrace = std::backtrace::Backtrace::force_capture();
        let _ = std::fs::write(&path, format!("{info}\n\nbacktrace:\n{backtrace}\n"));
        default(info);
    }));
}
