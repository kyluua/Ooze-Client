//! Developer preview of the launcher interface over the simulated fixture backend.
//!
//! Not a product: every screen shows the "Developer preview" banner, sign-in and installs are
//! simulated, and the binary only builds with `--features dev-fixtures` in debug builds.
//!
//! Usage: `cargo run --features dev-fixtures --bin ooze-launcher-preview -- [scenario]`
//! Scenarios: first-run (default), signed-out, ready, auth-not-configured, app-not-approved,
//! offline, crash.

use ooze_launcher_ui::fixtures::{FixtureBackend, Scenario};
use ooze_launcher_ui::{AppOptions, LauncherApp};

fn main() -> eframe::Result {
    let arg = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "first-run".to_owned());
    let Some(scenario) = Scenario::parse(&arg) else {
        eprintln!("unknown scenario {arg:?}");
        std::process::exit(2);
    };
    let options = LauncherApp::native_options("Ooze Launcher (developer preview)");
    eframe::run_native(
        "ooze-launcher-preview",
        options,
        Box::new(move |_cc| {
            Ok(Box::new(LauncherApp::new(
                Box::new(FixtureBackend::simulated(scenario)),
                AppOptions {
                    developer_preview: true,
                },
            )))
        }),
    )
}
