//! Ooze Launcher desktop interface (L0).
//!
//! Screens, components, navigation, focus, localisation and product UI state for the standalone
//! Ooze Launcher. The backend (Microsoft sign-in, installation, Java and process lifecycle,
//! credential storage, packaging) is Sol's `ooze-launcher-core` (LAUNCHER_API_V1); this crate
//! reaches it only through [`core_backend::CoreBackend`], which implements [`api::Backend`].
//!
//! Hard rules for this crate:
//! - It never receives, stores, logs or displays a credential. Accounts are display summaries.
//! - It never fakes backend state: progress, accounts and install status come from events.
//! - Test and preview fixtures exist only under `cfg(test)` or the `dev-fixtures` feature, which a
//!   release build refuses.

#![forbid(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used))]

#[cfg(all(feature = "dev-fixtures", not(debug_assertions)))]
compile_error!(
    "the dev-fixtures feature is for tests and the developer preview only; it must never be enabled in a release build"
);

pub mod api;
pub mod app;
pub mod appearance;
pub mod config;
pub mod core_backend;
pub mod core_library;
pub mod i18n;
pub mod library;
pub mod library_model;
pub mod motion;
pub mod paint;
pub mod redact;
pub mod state;
pub mod theme;
pub mod widgets;

#[cfg(any(test, feature = "dev-fixtures"))]
pub mod fixtures;
#[cfg(any(test, feature = "dev-fixtures"))]
pub mod library_fixture;

#[cfg(test)]
mod library_ui_tests;
#[cfg(test)]
mod ui_tests;

pub use app::{AppOptions, LauncherApp};

/// Open the launcher window over `backend` and run until it closes.
pub fn run(backend: Box<dyn api::Backend>) -> eframe::Result {
    run_with_library(backend, None)
}

/// As [`run`], with Sol's library attached (instances, profiles, content). Without it the window
/// is the L0 launcher.
pub fn run_with_library(
    backend: Box<dyn api::Backend>,
    library: Option<Box<dyn library::LibraryBackend>>,
) -> eframe::Result {
    let title = i18n::Strings::en_us().get("app.name").to_owned();
    eframe::run_native(
        "ooze-launcher",
        LauncherApp::native_options(&title),
        Box::new(move |_cc| {
            let app = LauncherApp::new(backend, AppOptions::default());
            Ok(Box::new(match library {
                Some(l) => app.with_library(l),
                None => app,
            }))
        }),
    )
}
