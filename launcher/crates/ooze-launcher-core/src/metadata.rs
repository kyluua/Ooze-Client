use crate::{
    net::Artifact,
    util::{self, Result, metadata_error},
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Release {
    pub schema: u32,
    pub id: String,
    pub minecraft: String,
    pub loader: String,
    pub minecraft_metadata: Artifact,
    pub fabric_profile: Artifact,
    pub fabric_libraries: Vec<PinnedLibrary>,
    pub fabric_api: Artifact,
    pub ooze: Artifact,
    pub java: JavaRelease,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PinnedLibrary {
    pub name: String,
    pub path: String,
    pub artifact: Artifact,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct JavaRelease {
    pub archive: Artifact,
    pub java_relative: String,
}
impl Release {
    pub fn validate(&self) -> Result<()> {
        if self.schema != 1
            || self.minecraft != "1.21.11"
            || self.loader != "0.20.1"
            || self.id.len() > 128
            || !self
                .id
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'-' | b'.' | b'_'))
            || self.id.is_empty()
        {
            return Err(metadata_error());
        }
        for a in [
            &self.minecraft_metadata,
            &self.fabric_profile,
            &self.fabric_api,
            &self.ooze,
            &self.java.archive,
        ] {
            a.validate()?;
        }
        if self.minecraft_metadata.file.is_none()
            || self.fabric_profile.file.is_none()
            || self.ooze.file.is_none()
            || self.ooze.sha256.is_none()
            || self.java.archive.sha256.is_none()
        {
            return Err(metadata_error());
        }
        util::relative(&self.java.java_relative)?;
        let mut names = HashSet::new();
        for lib in &self.fabric_libraries {
            lib.artifact.validate()?;
            util::relative(&lib.path)?;
            if !names.insert(&lib.name) || maven_path(&lib.name)? != lib.path {
                return Err(metadata_error());
            }
        }
        Ok(())
    }
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Minecraft {
    pub id: String,
    pub main_class: String,
    pub java_version: JavaVersion,
    pub downloads: BTreeMap<String, Download>,
    pub asset_index: AssetIndex,
    pub libraries: Vec<Library>,
    pub arguments: Arguments,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct JavaVersion {
    pub major_version: u32,
}
#[derive(Clone, Deserialize)]
pub(crate) struct Download {
    pub url: String,
    pub sha1: String,
    pub size: u64,
    pub path: Option<String>,
}
impl Download {
    pub fn artifact(&self) -> Artifact {
        Artifact {
            url: Some(self.url.clone()),
            file: None,
            size: self.size,
            sha1: Some(self.sha1.clone()),
            sha256: None,
        }
    }
}
#[derive(Deserialize)]
pub(crate) struct AssetIndex {
    pub id: String,
    pub url: String,
    pub sha1: String,
    pub size: u64,
}
impl AssetIndex {
    pub fn artifact(&self) -> Artifact {
        Artifact {
            url: Some(self.url.clone()),
            file: None,
            size: self.size,
            sha1: Some(self.sha1.clone()),
            sha256: None,
        }
    }
}
#[derive(Deserialize)]
pub(crate) struct Library {
    pub name: String,
    pub downloads: LibraryDownloads,
    #[serde(default)]
    pub rules: Vec<Rule>,
    #[serde(default)]
    pub natives: BTreeMap<String, String>,
}
#[derive(Deserialize)]
pub(crate) struct LibraryDownloads {
    pub artifact: Option<Download>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Fabric {
    pub id: String,
    pub inherits_from: String,
    pub main_class: String,
    pub libraries: Vec<FabricLibrary>,
    pub arguments: Arguments,
}
#[derive(Deserialize)]
pub(crate) struct FabricLibrary {
    pub name: String,
}
#[derive(Default, Deserialize)]
pub(crate) struct Arguments {
    #[serde(default)]
    pub jvm: Vec<Argument>,
    #[serde(default)]
    pub game: Vec<Argument>,
}
#[derive(Deserialize)]
#[serde(untagged)]
pub(crate) enum Argument {
    Plain(String),
    Conditional {
        rules: Vec<Rule>,
        value: ArgumentValues,
    },
}
#[derive(Deserialize)]
#[serde(untagged)]
pub(crate) enum ArgumentValues {
    One(String),
    Many(Vec<String>),
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Rule {
    pub action: String,
    pub os: Option<OsRule>,
    pub features: Option<BTreeMap<String, bool>>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct OsRule {
    pub name: Option<String>,
    pub arch: Option<String>,
    pub version: Option<String>,
}

pub(crate) fn allowed(rules: &[Rule]) -> Result<bool> {
    if rules.is_empty() {
        return Ok(true);
    }
    let mut allowed = false;
    for rule in rules {
        if rule.action != "allow" && rule.action != "disallow" {
            return Err(metadata_error());
        }
        let mut matches = true;
        if let Some(os) = &rule.os {
            if let Some(name) = &os.name {
                matches &= name == "windows";
            }
            if let Some(arch) = &os.arch {
                matches &= matches!(arch.as_str(), "x86_64" | "amd64");
            }
            // Pinned 1.21.11 has no matching Windows version condition. Fail closed if that changes.
            if os.version.is_some() && matches {
                return Err(metadata_error());
            }
        }
        if let Some(features) = &rule.features {
            for (name, value) in features {
                if !matches!(
                    name.as_str(),
                    "is_demo_user"
                        | "has_custom_resolution"
                        | "has_quick_plays_support"
                        | "is_quick_play_singleplayer"
                        | "is_quick_play_multiplayer"
                        | "is_quick_play_realms"
                ) {
                    return Err(metadata_error());
                }
                matches &= !*value;
            }
        }
        if matches {
            allowed = rule.action == "allow";
        }
    }
    Ok(allowed)
}
pub(crate) fn arguments(args: &[Argument]) -> Result<Vec<String>> {
    let mut result = Vec::new();
    for arg in args {
        match arg {
            Argument::Plain(s) => result.push(s.clone()),
            Argument::Conditional { rules, value } if allowed(rules)? => match value {
                ArgumentValues::One(s) => result.push(s.clone()),
                ArgumentValues::Many(v) => result.extend(v.iter().cloned()),
            },
            _ => {}
        }
    }
    if result.len() > 256 || result.iter().any(|v| v.len() > 4096 || v.contains('\0')) {
        return Err(metadata_error());
    }
    Ok(result)
}
pub(crate) fn expand(args: &[String], values: &BTreeMap<&str, String>) -> Result<Vec<String>> {
    let re = regex::Regex::new(r"\$\{([^}]+)\}").map_err(|_| metadata_error())?;
    args.iter()
        .map(|arg| {
            for capture in re.captures_iter(arg) {
                if !values.contains_key(&capture[1]) {
                    return Err(metadata_error());
                }
            }
            Ok(re
                .replace_all(arg, |capture: &regex::Captures<'_>| {
                    values.get(&capture[1]).cloned().unwrap_or_default()
                })
                .into_owned())
        })
        .collect()
}
pub(crate) fn maven_path(name: &str) -> Result<String> {
    let parts = name.split(':').collect::<Vec<_>>();
    if !(3..=4).contains(&parts.len())
        || parts.iter().any(|p| {
            p.is_empty()
                || !p
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || b"._-+".contains(&c))
        })
    {
        return Err(metadata_error());
    }
    let classifier = parts.get(3).map(|s| format!("-{s}")).unwrap_or_default();
    let path = format!(
        "{}/{}/{}/{}-{}{}.jar",
        parts[0].replace('.', "/"),
        parts[1],
        parts[2],
        parts[1],
        parts[2],
        classifier
    );
    util::relative(&path)?;
    Ok(path)
}
#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct InstalledFile {
    pub path: String,
    pub sha256: String,
    pub size: u64,
}
#[derive(Serialize, Deserialize)]
pub(crate) struct Installation {
    pub schema: u32,
    pub release_sha256: String,
    pub directory: String,
    pub files: Vec<InstalledFile>,
    pub classpath: Vec<String>,
    pub main_class: String,
    pub jvm_args: Vec<String>,
    pub game_args: Vec<String>,
    pub asset_index: String,
    pub managed_java: Option<String>,
    pub version_name: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rules_last_match_and_feature_conditions() {
        let rules: Vec<Rule> = serde_json::from_str(
            r#"[{"action":"allow"},{"action":"disallow","os":{"name":"windows"}}]"#,
        )
        .unwrap();
        assert!(!allowed(&rules).unwrap());
        let args:Arguments=serde_json::from_str(r#"{"game":["--username","${auth_player_name}",{"rules":[{"action":"allow","features":{"is_demo_user":true}}],"value":"--demo"}]}"#).unwrap();
        let args = arguments(&args.game).unwrap();
        assert_eq!(args.len(), 2);
        assert!(expand(&args, &BTreeMap::new()).is_err());
        let values = BTreeMap::from([("auth_player_name", "Player With Spaces".to_owned())]);
        assert_eq!(expand(&args, &values).unwrap()[1], "Player With Spaces");
    }
    #[test]
    fn maven_coordinates_confined() {
        assert_eq!(
            maven_path("net.fabricmc:intermediary:1.21.11").unwrap(),
            "net/fabricmc/intermediary/1.21.11/intermediary-1.21.11.jar"
        );
        assert!(maven_path("../evil:jar:1").is_err());
    }
}
