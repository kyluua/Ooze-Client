//! Ooze Launcher for players: the desktop interface over Sol's real backend.
//!
//! Configuration is resolved exactly like Sol's `ooze-launcher-cli`:
//! - data directory: `%LOCALAPPDATA%\OozeLauncher`;
//! - release manifest: `release.json` beside this executable;
//! - Microsoft public client ID: `OOZE_MICROSOFT_CLIENT_ID` at run time, else the same variable at
//!   build time, else the project's committed ID (`launcher/packaging/public-auth.json`). See `config.rs`.
//!
//! Any startup failure (not Windows x64, another launcher running, unreadable settings) opens the
//! window with a clear error page instead of exiting silently.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;

use ooze_launcher_core::LauncherConfig;
use ooze_launcher_ui::api::{ErrorKind, LauncherError};
use ooze_launcher_ui::core_backend::{CoreBackend, FailedBackend};

fn config() -> Result<LauncherConfig, LauncherError> {
    let fail = |detail: &str| LauncherError {
        kind: ErrorKind::Configuration,
        detail: detail.to_owned(),
        retryable: false,
    };
    let data_dir = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .ok_or_else(|| fail("LOCALAPPDATA is not set, so Ooze can't find its data folder."))?
        .join("OozeLauncher");
    let release_manifest = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|dir| dir.join("release.json")))
        .ok_or_else(|| fail("Ooze can't locate its own folder."))?;
    let microsoft_client_id = ooze_launcher_ui::config::microsoft_client_id(
        std::env::var("OOZE_MICROSOFT_CLIENT_ID").ok(),
        option_env!("OOZE_MICROSOFT_CLIENT_ID"),
    );
    Ok(LauncherConfig {
        data_dir,
        release_manifest,
        microsoft_client_id,
    })
}

fn main() -> eframe::Result {
    let config = config();
    // Sol's library (Library API 1) shares the launcher's data directory.
    let library: Option<Box<dyn ooze_launcher_ui::library::LibraryBackend>> =
        config.as_ref().ok().map(|c| {
            match ooze_launcher_ui::core_library::CoreLibrary::open(&c.data_dir) {
                Ok(lib) => Box::new(lib) as Box<dyn ooze_launcher_ui::library::LibraryBackend>,
                Err(e) => Box::new(ooze_launcher_ui::core_library::FailedLibrary(e)),
            }
        });
    let backend: Box<dyn ooze_launcher_ui::api::Backend> = match config.and_then(CoreBackend::open)
    {
        Ok(backend) => Box::new(backend),
        Err(error) => Box::new(FailedBackend { error }),
    };
    ooze_launcher_ui::run_with_library(backend, library)
}
