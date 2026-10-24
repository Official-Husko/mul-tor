//! Mul-Tor, graphical version.

mod app;
mod model;
mod tasks;
mod theme;
mod view;

#[cfg(test)]
mod ui_tests;

use std::path::PathBuf;

use app::MulTorApp;

fn main() {
    let home = home_from_args();
    init_logging(&home);

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Mul-Tor")
            .with_inner_size([1120.0, 720.0])
            .with_min_inner_size([820.0, 520.0]),
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };

    let result = eframe::run_native(
        "Mul-Tor",
        options,
        Box::new(move |cc| {
            theme::apply(&cc.egui_ctx);
            Ok(Box::new(MulTorApp::new(cc.egui_ctx.clone(), home)?))
        }),
    );

    if let Err(error) = result {
        eprintln!("Mul-Tor could not open its window: {error}");
        eprintln!(
            "The graphical version needs a working GPU driver (Vulkan, Metal or Direct3D 12). \
             The command-line version, `mul-tor`, works without one."
        );
        std::process::exit(1);
    }
}

/// `--home DIR` or `MUL_TOR_HOME`: keep all files in one folder.
fn home_from_args() -> Option<PathBuf> {
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--home" {
            return args.next().map(PathBuf::from);
        }
        if let Some(value) = arg.strip_prefix("--home=") {
            return Some(PathBuf::from(value));
        }
        if arg == "--version" || arg == "-V" {
            println!("mul-tor-gui {}", env!("CARGO_PKG_VERSION"));
            std::process::exit(0);
        }
        if arg == "--help" || arg == "-h" {
            println!(
                "mul-tor-gui {}\n\nUsage: mul-tor-gui [--home DIR]\n\n  --home DIR  keep config and caches in DIR (or set MUL_TOR_HOME)",
                env!("CARGO_PKG_VERSION")
            );
            std::process::exit(0);
        }
    }
    std::env::var_os("MUL_TOR_HOME").map(PathBuf::from)
}

/// Failures go to runtime.log, as in the command-line version.
fn init_logging(home: &Option<PathBuf>) {
    use std::sync::Mutex;
    use tracing_subscriber::{EnvFilter, fmt, prelude::*};

    let Ok(dirs) = mul_tor_core::AppDirs::discover(home.as_deref()) else {
        return;
    };
    let _ = dirs.create_all();
    let Ok(file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(dirs.log_file())
    else {
        return;
    };
    let _ = tracing_subscriber::registry()
        .with(
            fmt::layer()
                .with_ansi(false)
                .with_writer(Mutex::new(file))
                .with_filter(EnvFilter::new("warn")),
        )
        .try_init();
}
