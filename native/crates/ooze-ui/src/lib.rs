//! Ooze product model shared by the launcher and the in-game engine.
//!
//! - [`catalog`]: modules, fair-play classes, typed settings, HUD defaults (docs/MODULES.md).
//! - [`profile`]: FPS / PvP / Minimal presets, user overrides, diff, reset, import/export.
//! - [`hud`]: anchors, placement, clamping, snapping, nudging.
//! - [`stats`]: CPS and frame-time statistics with fixed storage.
//! - [`json`]: bounded JSON for the data documents above (standard library only).
//!
//! Nothing here touches Minecraft, the GPU or the filesystem. Game hooks are Java's, rendering
//! goes through Sol's bridge, and persistence is the backend's.

#![forbid(unsafe_code)]

pub mod catalog;
pub mod hud;
pub mod json;
pub mod profile;
pub mod settings;
pub mod stats;
