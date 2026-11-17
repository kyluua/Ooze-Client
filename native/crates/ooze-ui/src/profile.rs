//! Profiles (presets): module enable states and settings, HUD arrangement and visual settings.
//!
//! Effective values are layered: catalog defaults → built-in preset → the user's overrides.
//! Switching the base preset never discards overrides; reset removes them. Documents are data
//! only (`ooze.profile/1`), validated on import; unknown entries are reported and skipped.

use std::collections::BTreeMap;

use crate::catalog::{ModuleId, VISUAL_SETTINGS, visual_setting};
use crate::hud::{Anchor, Placement};
use crate::json::{self, Json};
use crate::settings::{SettingDef, Value};

pub const SCHEMA: &str = "ooze.profile/1";
pub const PRESET_SCHEMA: &str = "ooze.preset/1";

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PresetId {
    Fps,
    Pvp,
    Minimal,
}

impl PresetId {
    pub const ALL: [Self; 3] = [Self::Fps, Self::Pvp, Self::Minimal];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Fps => "fps",
            Self::Pvp => "pvp",
            Self::Minimal => "minimal",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|p| p.as_str() == s)
    }

    pub fn name_key(self) -> &'static str {
        match self {
            Self::Fps => "profile.preset.fps",
            Self::Pvp => "profile.preset.pvp",
            Self::Minimal => "profile.preset.minimal",
        }
    }

    fn source(self) -> &'static str {
        match self {
            Self::Fps => include_str!("../presets/fps.json"),
            Self::Pvp => include_str!("../presets/pvp.json"),
            Self::Minimal => include_str!("../presets/minimal.json"),
        }
    }

    /// The preset's overrides of the catalog defaults (parsed from the embedded data file).
    pub fn overrides(self) -> Overrides {
        // Embedded presets are validated by tests; a malformed one degrades to no overrides.
        json::parse(self.source())
            .ok()
            .and_then(|doc| {
                doc.get("overrides")
                    .map(|o| Overrides::from_json(o, &mut Vec::new()))
            })
            .unwrap_or_default()
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ModuleOverride {
    pub enabled: Option<bool>,
    pub settings: BTreeMap<String, Value>,
}

/// A sparse layer over the catalog defaults.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Overrides {
    pub modules: BTreeMap<ModuleId, ModuleOverride>,
    pub hud: BTreeMap<ModuleId, Placement>,
    pub visual: BTreeMap<String, Value>,
}

/// Why an imported entry was skipped. The rest of the document still imports.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImportWarning {
    pub path: String,
    pub reason: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ImportError {
    NotJson(json::JsonError),
    WrongSchema,
    MissingField(&'static str),
    BadId,
}

impl Overrides {
    fn layer(&mut self, top: &Overrides) {
        for (id, m) in &top.modules {
            let entry = self.modules.entry(*id).or_default();
            if m.enabled.is_some() {
                entry.enabled = m.enabled;
            }
            for (k, v) in &m.settings {
                entry.settings.insert(k.clone(), v.clone());
            }
        }
        for (id, p) in &top.hud {
            self.hud.insert(*id, *p);
        }
        for (k, v) in &top.visual {
            self.visual.insert(k.clone(), v.clone());
        }
    }

    fn from_json(value: &Json, warnings: &mut Vec<ImportWarning>) -> Self {
        let mut out = Self::default();
        let warn = |warnings: &mut Vec<ImportWarning>, path: String, reason| {
            warnings.push(ImportWarning { path, reason })
        };
        for (name, m) in value
            .get("modules")
            .and_then(Json::as_object)
            .unwrap_or_default()
        {
            let Some(id) = ModuleId::parse(name) else {
                warn(warnings, format!("modules.{name}"), "unknown module");
                continue;
            };
            let def = id.def();
            let mut o = ModuleOverride::default();
            match m.get("enabled") {
                Some(Json::Bool(b)) => o.enabled = Some(*b),
                Some(_) => warn(warnings, format!("modules.{name}.enabled"), "not a boolean"),
                None => {}
            }
            for (key, v) in m
                .get("settings")
                .and_then(Json::as_object)
                .unwrap_or_default()
            {
                match def.setting(key) {
                    None => warn(
                        warnings,
                        format!("modules.{name}.settings.{key}"),
                        "unknown setting",
                    ),
                    Some(s) => match s.validate(v) {
                        Ok(value) => {
                            o.settings.insert(key.clone(), value);
                        }
                        Err(_) => warn(
                            warnings,
                            format!("modules.{name}.settings.{key}"),
                            "invalid value",
                        ),
                    },
                }
            }
            out.modules.insert(id, o);
        }
        for (name, p) in value
            .get("hud")
            .and_then(Json::as_object)
            .unwrap_or_default()
        {
            let path = format!("hud.{name}");
            match ModuleId::parse(name) {
                Some(id) if id.def().hud.is_some() => match placement_from_json(p) {
                    Some(placement) => {
                        out.hud.insert(id, placement);
                    }
                    None => warn(warnings, path, "invalid placement"),
                },
                Some(_) => warn(warnings, path, "module has no HUD widget"),
                None => warn(warnings, path, "unknown module"),
            }
        }
        for (key, v) in value
            .get("visual")
            .and_then(Json::as_object)
            .unwrap_or_default()
        {
            match visual_setting(key) {
                None => warn(warnings, format!("visual.{key}"), "unknown setting"),
                Some(s) => match s.validate(v) {
                    Ok(value) => {
                        out.visual.insert(key.clone(), value);
                    }
                    Err(_) => warn(warnings, format!("visual.{key}"), "invalid value"),
                },
            }
        }
        out
    }

    fn to_json(&self) -> Json {
        let modules = self
            .modules
            .iter()
            .map(|(id, m)| {
                let mut fields = Vec::new();
                if let Some(e) = m.enabled {
                    fields.push(("enabled".to_owned(), Json::Bool(e)));
                }
                if !m.settings.is_empty() {
                    let s = m
                        .settings
                        .iter()
                        .map(|(k, v)| (k.clone(), v.to_json()))
                        .collect();
                    fields.push(("settings".to_owned(), Json::Object(s)));
                }
                (id.as_str().to_owned(), Json::Object(fields))
            })
            .collect();
        let hud = self
            .hud
            .iter()
            .map(|(id, p)| (id.as_str().to_owned(), placement_to_json(p)))
            .collect();
        let visual = self
            .visual
            .iter()
            .map(|(k, v)| (k.clone(), v.to_json()))
            .collect();
        Json::Object(vec![
            ("modules".into(), Json::Object(modules)),
            ("hud".into(), Json::Object(hud)),
            ("visual".into(), Json::Object(visual)),
        ])
    }
}

fn placement_from_json(v: &Json) -> Option<Placement> {
    let p = Placement {
        anchor: Anchor::parse(v.get("anchor")?.as_str()?)?,
        x: v.get("x")?.as_f64()? as f32,
        y: v.get("y")?.as_f64()? as f32,
        scale: v.get("scale").and_then(Json::as_f64).unwrap_or(1.0) as f32,
    };
    (p.valid() && p.x.abs() <= 10_000.0 && p.y.abs() <= 10_000.0).then_some(p)
}

fn placement_to_json(p: &Placement) -> Json {
    Json::Object(vec![
        ("anchor".into(), Json::String(p.anchor.as_str().into())),
        ("x".into(), Json::Number(f64::from(p.x))),
        ("y".into(), Json::Number(f64::from(p.y))),
        ("scale".into(), Json::Number(f64::from(p.scale))),
    ])
}

#[derive(Clone, Debug, PartialEq)]
pub struct ModuleState {
    pub enabled: bool,
    pub settings: BTreeMap<&'static str, Value>,
}

/// Fully resolved values for every module, widget and visual setting.
#[derive(Clone, Debug, PartialEq)]
pub struct Effective {
    pub modules: BTreeMap<ModuleId, ModuleState>,
    pub hud: BTreeMap<ModuleId, Placement>,
    pub visual: BTreeMap<&'static str, Value>,
}

/// One difference between two effective configurations, for "what this changes".
#[derive(Clone, Debug, PartialEq)]
pub struct Change {
    /// Stable path such as `modules.fps.enabled`, `modules.crosshair.settings.size`, `hud.fps`.
    pub path: String,
    pub from: String,
    pub to: String,
}

/// A user profile document.
#[derive(Clone, Debug, PartialEq)]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub base: Option<PresetId>,
    pub overrides: Overrides,
}

pub fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 48
        && id
            .bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-' || c == b'_')
}

impl Profile {
    /// A new profile following a built-in preset, with no overrides yet.
    pub fn from_preset(preset: PresetId, id: &str, name: &str) -> Self {
        Self {
            id: id.to_owned(),
            name: name.to_owned(),
            base: Some(preset),
            overrides: Overrides::default(),
        }
    }

    pub fn resolve(&self) -> Effective {
        let mut layers = Overrides::default();
        if let Some(p) = self.base {
            layers.layer(&p.overrides());
        }
        layers.layer(&self.overrides);
        resolve_layers(&layers)
    }

    /// Changes `self` would make compared with `current` (for the apply confirmation).
    pub fn diff_from(&self, current: &Effective) -> Vec<Change> {
        diff(current, &self.resolve())
    }

    pub fn set_enabled(&mut self, id: ModuleId, enabled: bool) {
        self.overrides.modules.entry(id).or_default().enabled = Some(enabled);
    }

    /// Validated through the catalog; invalid values are refused, never stored.
    pub fn set_setting(
        &mut self,
        id: ModuleId,
        key: &str,
        value: &Json,
    ) -> Result<(), crate::settings::SettingError> {
        let def = id
            .def()
            .setting(key)
            .ok_or(crate::settings::SettingError::WrongType)?;
        let v = def.validate(value)?;
        self.overrides
            .modules
            .entry(id)
            .or_default()
            .settings
            .insert(key.to_owned(), v);
        Ok(())
    }

    pub fn set_visual(
        &mut self,
        key: &str,
        value: &Json,
    ) -> Result<(), crate::settings::SettingError> {
        let def = visual_setting(key).ok_or(crate::settings::SettingError::WrongType)?;
        let v = def.validate(value)?;
        self.overrides.visual.insert(key.to_owned(), v);
        Ok(())
    }

    pub fn set_placement(&mut self, id: ModuleId, placement: Placement) -> bool {
        if id.def().hud.is_none() || !placement.valid() {
            return false;
        }
        self.overrides.hud.insert(id, placement);
        true
    }

    /// Remove the user's override at `path` (a path from [`Change`]); returns whether one existed.
    pub fn reset(&mut self, path: &str) -> bool {
        let parts: Vec<&str> = path.split('.').collect();
        match parts.as_slice() {
            ["modules", m, "enabled"] => ModuleId::parse(m)
                .and_then(|id| self.overrides.modules.get_mut(&id))
                .is_some_and(|o| o.enabled.take().is_some()),
            ["modules", m, "settings", key] => ModuleId::parse(m)
                .and_then(|id| self.overrides.modules.get_mut(&id))
                .is_some_and(|o| o.settings.remove(*key).is_some()),
            ["modules", m] => {
                ModuleId::parse(m).is_some_and(|id| self.overrides.modules.remove(&id).is_some())
            }
            ["hud", m] => {
                ModuleId::parse(m).is_some_and(|id| self.overrides.hud.remove(&id).is_some())
            }
            ["hud"] => !std::mem::take(&mut self.overrides.hud).is_empty(),
            ["visual", key] => self.overrides.visual.remove(*key).is_some(),
            _ => false,
        }
    }

    pub fn reset_all(&mut self) {
        self.overrides = Overrides::default();
    }

    /// A copy with a new id and name, keeping the base preset and every override.
    pub fn duplicate(&self, id: &str, name: &str) -> Self {
        Self {
            id: id.to_owned(),
            name: name.to_owned(),
            ..self.clone()
        }
    }

    pub fn to_json(&self) -> String {
        Json::Object(vec![
            ("schema".into(), Json::String(SCHEMA.into())),
            ("id".into(), Json::String(self.id.clone())),
            ("name".into(), Json::String(self.name.clone())),
            (
                "base".into(),
                self.base
                    .map_or(Json::Null, |b| Json::String(b.as_str().into())),
            ),
            ("overrides".into(), self.overrides.to_json()),
        ])
        .to_pretty()
    }

    /// Import a data-only document. Unknown or invalid entries are skipped with warnings.
    pub fn from_json(text: &str) -> Result<(Self, Vec<ImportWarning>), ImportError> {
        let doc = json::parse(text).map_err(ImportError::NotJson)?;
        if doc.get("schema").and_then(Json::as_str) != Some(SCHEMA) {
            return Err(ImportError::WrongSchema);
        }
        let id = doc
            .get("id")
            .and_then(Json::as_str)
            .ok_or(ImportError::MissingField("id"))?;
        if !valid_id(id) {
            return Err(ImportError::BadId);
        }
        let name = doc
            .get("name")
            .and_then(Json::as_str)
            .ok_or(ImportError::MissingField("name"))?;
        let mut warnings = Vec::new();
        let base = match doc.get("base") {
            None | Some(Json::Null) => None,
            Some(Json::String(s)) => {
                let p = PresetId::parse(s);
                if p.is_none() {
                    warnings.push(ImportWarning {
                        path: "base".into(),
                        reason: "unknown preset",
                    });
                }
                p
            }
            Some(_) => {
                warnings.push(ImportWarning {
                    path: "base".into(),
                    reason: "not a string",
                });
                None
            }
        };
        let overrides = doc
            .get("overrides")
            .map(|o| Overrides::from_json(o, &mut warnings))
            .unwrap_or_default();
        let name: String = name.chars().filter(|c| !c.is_control()).take(64).collect();
        Ok((
            Self {
                id: id.to_owned(),
                name,
                base,
                overrides,
            },
            warnings,
        ))
    }
}

fn resolve_layers(layers: &Overrides) -> Effective {
    let mut modules = BTreeMap::new();
    let mut hud = BTreeMap::new();
    for id in ModuleId::ALL {
        let def = id.def();
        let o = layers.modules.get(&id);
        let settings = def
            .settings
            .iter()
            .map(|s: &SettingDef| {
                (
                    s.key,
                    o.and_then(|o| o.settings.get(s.key))
                        .cloned()
                        .unwrap_or_else(|| s.default()),
                )
            })
            .collect();
        modules.insert(
            id,
            ModuleState {
                enabled: o.and_then(|o| o.enabled).unwrap_or(false),
                settings,
            },
        );
        if let Some(default) = def.hud {
            hud.insert(id, layers.hud.get(&id).copied().unwrap_or(default));
        }
    }
    let visual = VISUAL_SETTINGS
        .iter()
        .map(|s| {
            (
                s.key,
                layers
                    .visual
                    .get(s.key)
                    .cloned()
                    .unwrap_or_else(|| s.default()),
            )
        })
        .collect();
    Effective {
        modules,
        hud,
        visual,
    }
}

/// Differences between two effective configurations, in catalog order.
pub fn diff(from: &Effective, to: &Effective) -> Vec<Change> {
    let mut out = Vec::new();
    for id in ModuleId::ALL {
        let (a, b) = (&from.modules[&id], &to.modules[&id]);
        let m = id.as_str();
        if a.enabled != b.enabled {
            out.push(Change {
                path: format!("modules.{m}.enabled"),
                from: a.enabled.to_string(),
                to: b.enabled.to_string(),
            });
        }
        for (key, va) in &a.settings {
            if let Some(vb) = b.settings.get(key).filter(|vb| *vb != va) {
                out.push(Change {
                    path: format!("modules.{m}.settings.{key}"),
                    from: va.display(),
                    to: vb.display(),
                });
            }
        }
        if let (Some(pa), Some(pb)) = (from.hud.get(&id), to.hud.get(&id))
            && pa != pb
        {
            let show =
                |p: &Placement| format!("{} {:+}/{:+} ×{}", p.anchor.as_str(), p.x, p.y, p.scale);
            out.push(Change {
                path: format!("hud.{m}"),
                from: show(pa),
                to: show(pb),
            });
        }
    }
    for (key, va) in &from.visual {
        if let Some(vb) = to.visual.get(key).filter(|vb| *vb != va) {
            out.push(Change {
                path: format!("visual.{key}"),
                from: va.display(),
                to: vb.display(),
            });
        }
    }
    out
}

/// The catalog defaults with no preset (everything optional off).
pub fn defaults() -> Effective {
    resolve_layers(&Overrides::default())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn preset_doc(p: PresetId) -> Json {
        json::parse(p.source()).unwrap()
    }

    #[test]
    fn embedded_presets_are_valid_and_clean() {
        for p in PresetId::ALL {
            let doc = preset_doc(p);
            assert_eq!(
                doc.get("schema").and_then(Json::as_str),
                Some(PRESET_SCHEMA),
                "{p:?}"
            );
            assert_eq!(doc.get("id").and_then(Json::as_str), Some(p.as_str()));
            let mut warnings = Vec::new();
            let _ = Overrides::from_json(doc.get("overrides").unwrap(), &mut warnings);
            assert!(warnings.is_empty(), "{p:?}: {warnings:?}");
        }
    }

    #[test]
    fn presets_match_the_product_definition() {
        let on = |p: PresetId| -> Vec<ModuleId> {
            let e = Profile::from_preset(p, "x", "x").resolve();
            ModuleId::ALL
                .into_iter()
                .filter(|id| e.modules[id].enabled)
                .collect()
        };
        assert_eq!(on(PresetId::Fps), vec![ModuleId::Fps]);
        assert_eq!(
            on(PresetId::Pvp),
            vec![
                ModuleId::Fps,
                ModuleId::Armor,
                ModuleId::Effects,
                ModuleId::Keystrokes,
                ModuleId::Cps,
                ModuleId::Ping,
                ModuleId::Crosshair
            ]
        );
        assert!(on(PresetId::Minimal).is_empty());
        let fps = Profile::from_preset(PresetId::Fps, "x", "x").resolve();
        assert_eq!(fps.visual["effects_quality"], Value::Choice("low".into()));
        assert_eq!(fps.visual["ambient_motion"], Value::Bool(false));
    }

    #[test]
    fn user_overrides_survive_preset_switches_and_reset_removes_them() {
        let mut p = Profile::from_preset(PresetId::Pvp, "mine", "Mine");
        p.set_setting(ModuleId::Crosshair, "size", &Json::Number(10.0))
            .unwrap();
        p.set_enabled(ModuleId::Ping, false);
        p.base = Some(PresetId::Fps);
        let e = p.resolve();
        assert_eq!(
            e.modules[&ModuleId::Crosshair].settings["size"],
            Value::Int(10)
        );
        assert!(!e.modules[&ModuleId::Ping].enabled);
        assert!(p.reset("modules.crosshair.settings.size"));
        assert_eq!(
            p.resolve().modules[&ModuleId::Crosshair].settings["size"],
            Value::Int(6)
        );
        assert!(
            !p.reset("modules.crosshair.settings.size"),
            "nothing left to reset"
        );
        p.reset_all();
        assert_eq!(
            p.resolve(),
            Profile::from_preset(PresetId::Fps, "y", "y").resolve()
        );
    }

    #[test]
    fn invalid_values_are_refused() {
        let mut p = Profile::from_preset(PresetId::Minimal, "m", "M");
        assert!(
            p.set_setting(ModuleId::Crosshair, "size", &Json::Number(99.0))
                .is_err()
        );
        assert!(
            p.set_setting(ModuleId::Crosshair, "nope", &Json::Number(1.0))
                .is_err()
        );
        assert!(
            !p.set_placement(
                ModuleId::Zoom,
                Placement {
                    anchor: Anchor::Top,
                    x: 0.0,
                    y: 0.0,
                    scale: 1.0
                }
            ),
            "zoom has no widget"
        );
        assert_eq!(p, Profile::from_preset(PresetId::Minimal, "m", "M"));
    }

    #[test]
    fn diff_lists_exactly_what_changes() {
        let current = Profile::from_preset(PresetId::Minimal, "a", "a").resolve();
        let fps = Profile::from_preset(PresetId::Fps, "b", "b");
        let changes = fps.diff_from(&current);
        let paths: Vec<_> = changes.iter().map(|c| c.path.as_str()).collect();
        assert!(paths.contains(&"modules.fps.enabled"));
        assert!(paths.contains(&"visual.effects_quality"));
        assert!(
            !paths.iter().any(|p| p.starts_with("modules.crosshair")),
            "{paths:?}"
        );
        assert!(fps.diff_from(&fps.resolve()).is_empty());
    }

    #[test]
    fn export_import_round_trip_and_hostile_imports() {
        let mut p = Profile::from_preset(PresetId::Pvp, "pvp-copy", "PvP copy");
        p.set_setting(
            ModuleId::Crosshair,
            "color",
            &Json::String("#FF00FF00".into()),
        )
        .unwrap();
        p.set_placement(
            ModuleId::Fps,
            Placement {
                anchor: Anchor::Bottom,
                x: 0.0,
                y: 12.0,
                scale: 1.5,
            },
        );
        p.set_visual("drips", &Json::Bool(false)).unwrap();
        let (back, warnings) = Profile::from_json(&p.to_json()).unwrap();
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(back, p);

        let hostile = r#"{"schema":"ooze.profile/1","id":"x","name":"X\u0007","base":"aimbot",
            "overrides":{"modules":{"reach":{"enabled":true},"crosshair":{"settings":{"size":999,"shape":"cross"}}},
                         "hud":{"zoom":{"anchor":"top","x":0,"y":0},"fps":{"anchor":"nowhere","x":0,"y":0}},
                         "visual":{"x_ray":true}}}"#;
        let (imported, warnings) = Profile::from_json(hostile).unwrap();
        assert_eq!(imported.name, "X");
        assert_eq!(imported.base, None);
        let paths: Vec<_> = warnings.iter().map(|w| w.path.as_str()).collect();
        for expected in [
            "base",
            "modules.reach",
            "modules.crosshair.settings.size",
            "hud.zoom",
            "hud.fps",
            "visual.x_ray",
        ] {
            assert!(paths.contains(&expected), "{expected} in {paths:?}");
        }
        assert_eq!(
            imported.overrides.modules[&ModuleId::Crosshair]
                .settings
                .len(),
            1,
            "only the valid setting kept"
        );
        assert_eq!(Profile::from_json("{}"), Err(ImportError::WrongSchema));
        assert_eq!(
            Profile::from_json(r#"{"schema":"ooze.profile/1","id":"../evil","name":"x"}"#),
            Err(ImportError::BadId)
        );
    }

    #[test]
    fn duplicate_keeps_everything_but_identity() {
        let mut p = Profile::from_preset(PresetId::Pvp, "a", "A");
        p.set_enabled(ModuleId::Coordinates, true);
        let d = p.duplicate("b", "B");
        assert_eq!((d.id.as_str(), d.name.as_str()), ("b", "B"));
        assert_eq!(d.resolve(), p.resolve());
    }
}
