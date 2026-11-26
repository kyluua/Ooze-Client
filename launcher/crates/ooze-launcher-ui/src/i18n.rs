//! Localised strings. Every user-facing string comes from `launcher/resources/lang/<locale>.json`.
//!
//! Placeholders are `{0}`, `{1}`, ... A missing key renders as the key itself (never a panic) and
//! is caught by the `every_key_used_in_source_exists` test.

use std::collections::HashMap;

const EN_US: &str = include_str!("../../../resources/lang/en_us.json");

#[derive(Debug)]
pub struct Strings {
    map: HashMap<String, String>,
}

impl Strings {
    pub fn en_us() -> Self {
        Self::parse(EN_US).unwrap_or_else(|_| Self {
            map: HashMap::new(),
        })
    }

    pub fn parse(json: &str) -> Result<Self, serde_json::Error> {
        let map: HashMap<String, String> = serde_json::from_str(json)?;
        Ok(Self { map })
    }

    pub fn get<'a>(&'a self, key: &'a str) -> &'a str {
        self.map.get(key).map_or(key, String::as_str)
    }

    pub fn has(&self, key: &str) -> bool {
        self.map.contains_key(key)
    }

    /// Substitute `{0}`, `{1}`, ... with `args`.
    pub fn fmt(&self, key: &str, args: &[&str]) -> String {
        let mut out = self.get(key).to_owned();
        for (i, arg) in args.iter().enumerate() {
            out = out.replace(&format!("{{{i}}}"), arg);
        }
        out
    }

    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.map.keys().map(String::as_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    fn source_keys() -> BTreeSet<String> {
        let re = regex::Regex::new(r#""((?:app|nav|first_run|play|sign_in|account|install|game|step|error|notice|settings|diag|close|a11y|common|preview|about|library|instance|instances|integrity|profile|content|appearance|loader)\.[a-zA-Z0-9_.]+)""#).unwrap();
        let mut keys = BTreeSet::new();
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/src");
        let mut stack = vec![std::path::PathBuf::from(dir)];
        while let Some(path) = stack.pop() {
            for entry in std::fs::read_dir(&path).unwrap() {
                let p = entry.unwrap().path();
                if p.is_dir() {
                    stack.push(p);
                } else if p.extension().is_some_and(|e| e == "rs") {
                    let text = std::fs::read_to_string(&p).unwrap();
                    for c in re.captures_iter(&text) {
                        keys.insert(c[1].to_owned());
                    }
                }
            }
        }
        keys.extend(data_keys());
        keys
    }

    /// Keys built from data rather than written as literals: the module catalog (names,
    /// descriptions, setting labels, choice options), fair-play classes, presets, accent swatches,
    /// loaders, content kinds and library errors.
    fn data_keys() -> Vec<String> {
        use crate::library::{ContentKind, LibErrorKind, Loader};
        use ooze_ui::catalog::{FairPlay, MODULES, VISUAL_SETTINGS};
        use ooze_ui::profile::PresetId;
        use ooze_ui::settings::SettingKind;
        let mut out = Vec::new();
        let setting = |d: &ooze_ui::settings::SettingDef, out: &mut Vec<String>| {
            out.push(d.label_key.to_owned());
            if let SettingKind::Choice { options, .. } = d.kind {
                out.extend(options.iter().map(|o| format!("{}.{o}", d.label_key)));
            }
        };
        for m in &MODULES {
            out.push(m.name_key.to_owned());
            out.push(m.description_key.to_owned());
            for d in m.settings {
                setting(d, &mut out);
            }
        }
        for d in &VISUAL_SETTINGS {
            setting(d, &mut out);
        }
        for f in [
            FairPlay::Cosmetic,
            FairPlay::Informational,
            FairPlay::QualityOfLife,
            FairPlay::Gameplay,
        ] {
            out.push(f.key().to_owned());
        }
        for p in PresetId::ALL {
            out.push(p.name_key().to_owned());
            out.push(format!("{}.description", p.name_key()));
        }
        for name in ooze_ui::theme_doc::ACCENT_SWATCHES {
            out.push(format!("appearance.accent.{name}"));
        }
        for l in [
            Loader::Vanilla,
            Loader::Fabric,
            Loader::Quilt,
            Loader::Forge,
            Loader::NeoForge,
        ] {
            out.push(l.key().to_owned());
        }
        for k in ContentKind::ALL {
            out.push(k.key().to_owned());
        }
        for e in [
            LibErrorKind::Busy,
            LibErrorKind::GameRunning,
            LibErrorKind::Incompatible,
            LibErrorKind::Conflict,
            LibErrorKind::HashMismatch,
            LibErrorKind::RateLimited,
            LibErrorKind::Offline,
            LibErrorKind::NotFound,
            LibErrorKind::Config,
            LibErrorKind::Invalid,
            LibErrorKind::Stale,
            LibErrorKind::Cancelled,
            LibErrorKind::Internal,
        ] {
            let (t, b) = crate::app::lib_error_keys_for_tests(e);
            out.push(t.to_owned());
            out.push(b.to_owned());
        }
        out
    }

    /// Product wording rules (LIBRARY_UI.md §7, AGENTS.md): no claim that an instance or module is
    /// cheat-proof, undetectable or server-approved, and no claim that content "works".
    #[test]
    fn no_banned_claims_in_any_string() {
        let banned = [
            "cheat-proof",
            "cheat proof",
            "undetectable",
            "anticheat",
            "anti-cheat",
            "server-approved",
            "server approved",
            "safe on all servers",
            "safe on every server",
            "guaranteed",
            "ready to use",
        ];
        let strings = Strings::en_us();
        for k in strings.keys() {
            let v = strings.get(k).to_lowercase();
            for b in banned {
                assert!(!v.contains(b), "{k}: {v:?} contains {b:?}");
            }
        }
    }

    #[test]
    fn every_key_used_in_source_exists() {
        let strings = Strings::en_us();
        let missing: Vec<_> = source_keys()
            .into_iter()
            .filter(|k| !strings.has(k))
            .collect();
        assert!(missing.is_empty(), "missing en_us keys: {missing:?}");
    }

    #[test]
    fn every_key_in_the_file_is_used() {
        let used = source_keys();
        let strings = Strings::en_us();
        let unused: Vec<_> = strings.keys().filter(|k| !used.contains(*k)).collect();
        assert!(unused.is_empty(), "unused en_us keys: {unused:?}");
    }

    #[test]
    fn placeholders_substitute_and_missing_keys_do_not_panic() {
        let s = Strings::parse(r#"{"a.b": "Hello {0}, {1}"}"#).unwrap();
        assert_eq!(s.fmt("a.b", &["Steve", "welcome"]), "Hello Steve, welcome");
        assert_eq!(s.get("missing.key"), "missing.key");
    }

    #[test]
    fn strings_contain_no_credential_wording_requests() {
        // The launcher never asks for a Microsoft password: sign-in happens in the browser.
        let strings = Strings::en_us();
        for k in strings.keys() {
            let v = strings.get(k).to_lowercase();
            assert!(!v.contains("enter your password"), "{k}");
            assert!(!v.contains("access token"), "{k}");
        }
    }
}
