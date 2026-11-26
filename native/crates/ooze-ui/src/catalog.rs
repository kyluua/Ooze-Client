//! The Ooze module catalog: identities, fair-play classes, typed settings and HUD defaults.
//!
//! This is product data (docs/MODULES.md). Adding a module here does not make it available:
//! the in-game hook (Java) and rendering (bridge) must exist and be Game-launched first, and the
//! launcher hides anything the connected instance cannot run.

use crate::hud::{Anchor, Placement};
use crate::settings::{SettingDef, SettingKind};

/// docs/MODULES.md §3.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FairPlay {
    Cosmetic,
    Informational,
    QualityOfLife,
    /// Blocked until a server-policy design exists.
    Gameplay,
}

impl FairPlay {
    pub fn key(self) -> &'static str {
        match self {
            Self::Cosmetic => "fair_play.cosmetic",
            Self::Informational => "fair_play.informational",
            Self::QualityOfLife => "fair_play.quality_of_life",
            Self::Gameplay => "fair_play.gameplay",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ModuleId {
    Fps,
    Coordinates,
    Armor,
    Effects,
    Keystrokes,
    Cps,
    Ping,
    Zoom,
    Crosshair,
}

impl ModuleId {
    pub const ALL: [Self; 9] = [
        Self::Fps,
        Self::Coordinates,
        Self::Armor,
        Self::Effects,
        Self::Keystrokes,
        Self::Cps,
        Self::Ping,
        Self::Zoom,
        Self::Crosshair,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Fps => "fps",
            Self::Coordinates => "coordinates",
            Self::Armor => "armor",
            Self::Effects => "effects",
            Self::Keystrokes => "keystrokes",
            Self::Cps => "cps",
            Self::Ping => "ping",
            Self::Zoom => "zoom",
            Self::Crosshair => "crosshair",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|m| m.as_str() == s)
    }

    pub fn def(self) -> &'static ModuleDef {
        // ALL and MODULES are declared in the same order (tested).
        &MODULES[self as usize]
    }
}

#[derive(Debug)]
pub struct ModuleDef {
    pub id: ModuleId,
    /// docs/MODULES.md id (M-01 …).
    pub matrix_id: &'static str,
    pub name_key: &'static str,
    pub description_key: &'static str,
    pub class: FairPlay,
    /// Some servers restrict it; the UI shows the server-rules notice.
    pub server_sensitive: bool,
    /// Default HUD placement for widget modules; `None` for non-widget modules.
    pub hud: Option<Placement>,
    pub settings: &'static [SettingDef],
}

impl ModuleDef {
    pub fn setting(&self, key: &str) -> Option<&'static SettingDef> {
        self.settings.iter().find(|s| s.key == key)
    }
}

const fn place(anchor: Anchor, x: f32, y: f32) -> Option<Placement> {
    Some(Placement {
        anchor,
        x,
        y,
        scale: 1.0,
    })
}

const fn b(key: &'static str, label_key: &'static str, default: bool) -> SettingDef {
    SettingDef {
        key,
        label_key,
        kind: SettingKind::Bool { default },
    }
}

const fn i(
    key: &'static str,
    label_key: &'static str,
    min: i64,
    max: i64,
    step: i64,
    default: i64,
) -> SettingDef {
    SettingDef {
        key,
        label_key,
        kind: SettingKind::Int {
            min,
            max,
            step,
            default,
        },
    }
}

pub static MODULES: [ModuleDef; 9] = [
    ModuleDef {
        id: ModuleId::Fps,
        matrix_id: "M-01",
        name_key: "module.fps.name",
        description_key: "module.fps.description",
        class: FairPlay::Informational,
        server_sensitive: false,
        hud: place(Anchor::TopLeft, 4.0, 4.0),
        settings: &[
            b("show_frame_time", "module.fps.show_frame_time", true),
            b("compact", "module.fps.compact", false),
            i("window_seconds", "module.fps.window_seconds", 1, 10, 1, 3),
        ],
    },
    ModuleDef {
        id: ModuleId::Coordinates,
        matrix_id: "M-02",
        name_key: "module.coordinates.name",
        description_key: "module.coordinates.description",
        class: FairPlay::Informational,
        server_sensitive: false,
        hud: place(Anchor::TopLeft, 4.0, 24.0),
        settings: &[
            b("show_facing", "module.coordinates.show_facing", true),
            b("show_biome", "module.coordinates.show_biome", false),
            i("decimals", "module.coordinates.decimals", 0, 2, 1, 0),
        ],
    },
    ModuleDef {
        id: ModuleId::Armor,
        matrix_id: "M-03",
        name_key: "module.armor.name",
        description_key: "module.armor.description",
        class: FairPlay::Informational,
        server_sensitive: false,
        hud: place(Anchor::BottomRight, 4.0, 40.0),
        settings: &[
            b("show_hands", "module.armor.show_hands", true),
            SettingDef {
                key: "value",
                label_key: "module.armor.value",
                kind: SettingKind::Choice {
                    options: &["percent", "remaining", "remaining_of_max"],
                    default: "remaining",
                },
            },
            i(
                "warn_below_percent",
                "module.armor.warn_below_percent",
                5,
                50,
                5,
                20,
            ),
        ],
    },
    ModuleDef {
        id: ModuleId::Effects,
        matrix_id: "M-04",
        name_key: "module.effects.name",
        description_key: "module.effects.description",
        class: FairPlay::Informational,
        server_sensitive: false,
        hud: place(Anchor::Right, 4.0, 0.0),
        settings: &[
            b("show_time", "module.effects.show_time", true),
            b("compact", "module.effects.compact", false),
        ],
    },
    ModuleDef {
        id: ModuleId::Keystrokes,
        matrix_id: "M-05",
        name_key: "module.keystrokes.name",
        description_key: "module.keystrokes.description",
        class: FairPlay::Informational,
        server_sensitive: false,
        hud: place(Anchor::BottomLeft, 4.0, 4.0),
        settings: &[
            b("show_mouse", "module.keystrokes.show_mouse", true),
            b("show_jump", "module.keystrokes.show_jump", true),
            b("show_sneak", "module.keystrokes.show_sneak", false),
        ],
    },
    ModuleDef {
        id: ModuleId::Cps,
        matrix_id: "M-06",
        name_key: "module.cps.name",
        description_key: "module.cps.description",
        class: FairPlay::Informational,
        server_sensitive: false,
        hud: place(Anchor::BottomLeft, 4.0, 72.0),
        settings: &[b("show_right", "module.cps.show_right", true)],
    },
    ModuleDef {
        id: ModuleId::Ping,
        matrix_id: "M-07",
        name_key: "module.ping.name",
        description_key: "module.ping.description",
        class: FairPlay::Informational,
        server_sensitive: false,
        hud: place(Anchor::TopRight, 4.0, 4.0),
        settings: &[i(
            "warn_above_ms",
            "module.ping.warn_above_ms",
            50,
            500,
            10,
            150,
        )],
    },
    ModuleDef {
        id: ModuleId::Zoom,
        matrix_id: "M-08",
        name_key: "module.zoom.name",
        description_key: "module.zoom.description",
        class: FairPlay::QualityOfLife,
        server_sensitive: true,
        hud: None,
        settings: &[
            SettingDef {
                key: "key",
                label_key: "module.zoom.key",
                kind: SettingKind::Key {
                    default: "key.keyboard.c",
                },
            },
            SettingDef {
                key: "factor",
                label_key: "module.zoom.factor",
                kind: SettingKind::Float {
                    min: 2.0,
                    max: 10.0,
                    step: 0.5,
                    default: 4.0,
                },
            },
            b("smooth", "module.zoom.smooth", true),
            b("scroll_adjust", "module.zoom.scroll_adjust", true),
        ],
    },
    ModuleDef {
        id: ModuleId::Crosshair,
        matrix_id: "M-09",
        name_key: "module.crosshair.name",
        description_key: "module.crosshair.description",
        class: FairPlay::Cosmetic,
        server_sensitive: false,
        hud: None,
        settings: &[
            SettingDef {
                key: "shape",
                label_key: "module.crosshair.shape",
                kind: SettingKind::Choice {
                    options: &["cross", "dot", "circle", "cross_dot"],
                    default: "cross",
                },
            },
            i("size", "module.crosshair.size", 2, 20, 1, 6),
            i("gap", "module.crosshair.gap", 0, 10, 1, 2),
            i("thickness", "module.crosshair.thickness", 1, 4, 1, 1),
            SettingDef {
                key: "color",
                label_key: "module.crosshair.color",
                kind: SettingKind::Color {
                    default: 0xFFFF_FFFF,
                },
            },
            b("outline", "module.crosshair.outline", true),
        ],
    },
];

/// Visual settings a profile may carry (validated like module settings).
pub static VISUAL_SETTINGS: [SettingDef; 3] = [
    SettingDef {
        key: "effects_quality",
        label_key: "profile.visual.effects_quality",
        kind: SettingKind::Choice {
            options: &["low", "medium", "high"],
            default: "high",
        },
    },
    b("ambient_motion", "profile.visual.ambient_motion", true),
    b("drips", "profile.visual.drips", true),
];

pub fn visual_setting(key: &str) -> Option<&'static SettingDef> {
    VISUAL_SETTINGS.iter().find(|s| s.key == key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_order_matches_ids_and_keys_are_unique() {
        for (i, id) in ModuleId::ALL.into_iter().enumerate() {
            assert_eq!(MODULES[i].id, id);
            assert_eq!(ModuleId::parse(id.as_str()), Some(id));
            let mut keys: Vec<_> = MODULES[i].settings.iter().map(|s| s.key).collect();
            keys.sort_unstable();
            keys.dedup();
            assert_eq!(keys.len(), MODULES[i].settings.len(), "{id:?}");
        }
    }

    #[test]
    fn no_gameplay_module_is_in_the_catalog_and_defaults_validate() {
        for m in &MODULES {
            assert_ne!(
                m.class,
                FairPlay::Gameplay,
                "{:?}: GAMEPLAY modules are blocked (MODULES.md §3)",
                m.id
            );
            for s in m.settings {
                assert!(
                    s.validate(&s.default().to_json()).is_ok(),
                    "{:?}.{}",
                    m.id,
                    s.key
                );
            }
        }
        for s in &VISUAL_SETTINGS {
            assert!(s.validate(&s.default().to_json()).is_ok());
        }
    }
}
