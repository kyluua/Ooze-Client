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
        let re = regex::Regex::new(r#""((?:app|nav|first_run|play|sign_in|account|install|game|step|error|notice|settings|diag|close|a11y|common|preview|about)\.[a-z0-9_.]+)""#).unwrap();
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
        keys
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
