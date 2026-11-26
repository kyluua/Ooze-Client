//! Ooze desktop launcher API. No Minecraft/JNI objects or credentials cross this API.
pub mod api;
mod auth;
mod backend;
mod catalog;
mod diagnostics;
mod game_guard;
mod install;
pub mod library;
mod metadata;
mod net;
mod process;
mod readiness;
mod util;
pub use api::*;
pub use backend::Launcher;
