//! Ooze Launcher desktop interface (L0).
//!
//! Screens, components, navigation, focus, localisation and product UI state for the standalone
//! Ooze Launcher. The backend (Microsoft sign-in, installation, Java and process lifecycle,
//! credential storage, packaging) is Codex's `ooze-launcher-core`; this crate talks to it only
//! through [`api::Backend`].
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
pub mod i18n;
pub mod motion;
pub mod paint;
pub mod redact;
pub mod state;
pub mod theme;
pub mod widgets;

#[cfg(any(test, feature = "dev-fixtures"))]
pub mod fixtures;

#[cfg(test)]
mod ui_tests;

pub use app::{AppOptions, LauncherApp};
