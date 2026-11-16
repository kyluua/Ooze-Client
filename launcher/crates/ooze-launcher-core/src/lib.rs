//! Ooze desktop launcher API. No Minecraft/JNI objects or credentials cross this API.
pub mod api;
mod auth;
mod backend;
mod game_guard;
mod install;
mod metadata;
mod net;
mod process;
mod util;
pub use api::*;
pub use backend::Launcher;
