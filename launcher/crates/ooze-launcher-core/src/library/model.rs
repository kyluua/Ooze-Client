use crate::{
    api::ErrorCode,
    util::{Result, error},
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const LIBRARY_API_VERSION: u32 = 1;
pub const SCHEMA: u32 = 1;
pub const THEME_SCHEMA: &str = "ooze.launcher.theme/1";
#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
pub enum ProgressStage {
    Contacting,
    Downloading,
    Verifying,
    Committing,
}
pub(super) type Reporter<'a> =
    &'a dyn Fn(ProgressStage, u64, Option<u64>, Option<&str>) -> Result<()>;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(transparent)]
pub struct Id(pub String);
impl Id {
    pub(crate) fn new() -> Self {
        Self(uuid::Uuid::new_v4().simple().to_string())
    }
    pub(crate) fn validate(&self) -> Result<()> {
        if self.0.len() != 32
            || !self
                .0
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(invalid("Invalid library identity."));
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Loader {
    Vanilla,
    Fabric,
    Quilt,
    Forge,
    NeoForge,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Target {
    pub minecraft: String,
    pub loader: Loader,
    pub loader_version: Option<String>,
    pub java_major: u32,
    pub ooze: bool,
}
impl Target {
    pub fn fabric() -> Self {
        Self {
            minecraft: "1.21.11".into(),
            loader: Loader::Fabric,
            loader_version: Some("0.20.1".into()),
            java_major: 21,
            ooze: true,
        }
    }
    pub fn vanilla() -> Self {
        Self {
            loader: Loader::Vanilla,
            loader_version: None,
            ooze: false,
            ..Self::fabric()
        }
    }
    pub(crate) fn validate(&self) -> Result<()> {
        if self.minecraft != "1.21.11"
            || self.java_major != 21
            || match self.loader {
                Loader::Vanilla => self.loader_version.is_some() || self.ooze,
                Loader::Fabric => self.loader_version.as_deref() != Some("0.20.1"),
                _ => true,
            }
        {
            return Err(invalid(
                "This build supports Minecraft 1.21.11 on Vanilla or Fabric 0.20.1 with Java 21. Other adapters are not available.",
            ));
        }
        Ok(())
    }
    pub(crate) fn modrinth_loader(&self) -> Option<&'static str> {
        match self.loader {
            Loader::Fabric => Some("fabric"),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Preset {
    Fps,
    Pvp,
    Minimal,
}
// Product profile schema and defaults have one owner: Claude's pure Rust model.
pub use ooze_ui::catalog::ModuleId;
#[derive(Clone, Debug, PartialEq)]
pub struct Profile {
    pub document: ooze_ui::profile::Profile,
}
impl Profile {
    pub fn new(preset: Preset) -> Self {
        let preset = match preset {
            Preset::Fps => ooze_ui::profile::PresetId::Fps,
            Preset::Pvp => ooze_ui::profile::PresetId::Pvp,
            Preset::Minimal => ooze_ui::profile::PresetId::Minimal,
        };
        Self {
            document: ooze_ui::profile::Profile::from_preset(preset, "default", "Ooze profile"),
        }
    }
    pub fn preset(&self) -> Preset {
        match self.document.base {
            Some(ooze_ui::profile::PresetId::Pvp) => Preset::Pvp,
            Some(ooze_ui::profile::PresetId::Minimal) => Preset::Minimal,
            _ => Preset::Fps,
        }
    }
    pub fn to_json(&self) -> String {
        self.document.to_json()
    }
    pub fn from_json(text: &str) -> Result<(Self, Vec<String>)> {
        if text.len() > 65536 {
            return Err(invalid("Profile exceeds 64 KiB."));
        }
        let (document, warnings) = ooze_ui::profile::Profile::from_json(text)
            .map_err(|_| invalid("Unsupported or malformed ooze.profile/1 document."))?;
        name(&document.name)?;
        Ok((
            Self { document },
            warnings
                .into_iter()
                .map(|w| format!("{}: {}", w.path, w.reason))
                .collect(),
        ))
    }
    pub(crate) fn validate(&self) -> Result<()> {
        let (parsed, warnings) = Self::from_json(&self.to_json())?;
        if !warnings.is_empty() || parsed != *self {
            return Err(invalid("Profile contains invalid settings or identity."));
        }
        Ok(())
    }
}
impl Serialize for Profile {
    fn serialize<S: serde::Serializer>(
        &self,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        let value: serde_json::Value =
            serde_json::from_str(&self.to_json()).map_err(serde::ser::Error::custom)?;
        value.serialize(serializer)
    }
}
impl<'de> Deserialize<'de> for Profile {
    fn deserialize<D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> std::result::Result<Self, D::Error> {
        let value = serde_json::Value::deserialize(deserializer)?;
        let (profile, warnings) =
            Self::from_json(&value.to_string()).map_err(serde::de::Error::custom)?;
        if !warnings.is_empty() {
            return Err(serde::de::Error::custom(
                "Persisted profile contains unsupported settings; preserve it for recovery.",
            ));
        }
        Ok(profile)
    }
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Motion {
    Full,
    Reduced,
    Off,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Theme {
    pub schema: String,
    pub name: String,
    pub background: [u8; 4],
    pub foreground: [u8; 4],
    pub accent: [u8; 4],
    pub ui_scale: f32,
    pub font: String,
    pub motion: Motion,
}
impl Default for Theme {
    fn default() -> Self {
        Self {
            schema: THEME_SCHEMA.into(),
            name: "Ooze classic".into(),
            background: [248, 249, 255, 255],
            foreground: [23, 25, 35, 255],
            accent: [65, 77, 141, 255],
            ui_scale: 1.0,
            font: "inter".into(),
            motion: Motion::Full,
        }
    }
}
impl Theme {
    pub(crate) fn validate(&self) -> Result<()> {
        name(&self.name)?;
        if self.schema != THEME_SCHEMA
            || !range(self.ui_scale, 1.0, 2.0)
            || !matches!(self.font.as_str(), "inter" | "bubblegum_sans")
            || self.background[3] != 255
            || self.foreground[3] != 255
            || contrast(self.background, self.foreground) < 4.5
        {
            return Err(invalid(
                "Theme must use supported tokens, scale/font and readable foreground contrast (4.5:1).",
            ));
        }
        Ok(())
    }
}
fn range(v: f32, lo: f32, hi: f32) -> bool {
    v.is_finite() && (lo..=hi).contains(&v)
}
fn luminance(c: [u8; 4]) -> f64 {
    let linear = |v: u8| {
        let v = f64::from(v) / 255.0;
        if v <= 0.04045 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * linear(c[0]) + 0.7152 * linear(c[1]) + 0.0722 * linear(c[2])
}
fn contrast(a: [u8; 4], b: [u8; 4]) -> f64 {
    let (a, b) = (luminance(a), luminance(b));
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}
pub(crate) fn name(value: &str) -> Result<()> {
    if value.trim().is_empty() || value.len() > 96 || value.chars().any(char::is_control) {
        return Err(invalid(
            "Choose a name of 1–96 bytes without control characters.",
        ));
    }
    Ok(())
}
pub(crate) fn invalid(message: &str) -> crate::api::LauncherError {
    error(ErrorCode::Configuration, message)
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    Managed,
    Custom,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ContentKind {
    Mod,
    Resourcepack,
    Datapack,
    Shader,
}
impl ContentKind {
    pub(crate) fn api_name(self) -> &'static str {
        match self {
            Self::Mod => "mod",
            Self::Resourcepack => "resourcepack",
            Self::Datapack => "datapack",
            Self::Shader => "shader",
        }
    }
    pub(crate) fn directory(self) -> &'static str {
        match self {
            Self::Mod => "mods",
            Self::Resourcepack => "resourcepacks",
            Self::Datapack => "datapacks",
            Self::Shader => "shaderpacks",
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContentReceipt {
    pub project: String,
    pub version: String,
    pub kind: ContentKind,
    pub path: String,
    pub size: u64,
    pub sha512: String,
    pub enabled: bool,
    pub required_projects: Vec<String>,
    pub required_versions: BTreeMap<String, String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Instance {
    pub schema: u32,
    pub id: Id,
    pub name: String,
    pub revision: u64,
    pub target: Target,
    pub mode: Mode,
    pub profile: Profile,
    pub theme: Theme,
    pub archived: bool,
    pub content: Vec<ContentReceipt>,
}
impl Instance {
    pub(crate) fn validate(&self) -> Result<()> {
        self.id.validate()?;
        name(&self.name)?;
        self.target.validate()?;
        self.profile.validate()?;
        self.theme.validate()?;
        if self.schema != SCHEMA || self.revision == 0 || self.content.len() > 512 {
            return Err(invalid("Unsupported instance schema or size."));
        }
        let mut paths = std::collections::HashSet::new();
        let mut projects = std::collections::HashSet::new();
        for c in &self.content {
            crate::util::relative(&c.path)?;
            super::modrinth::identifier(&c.project)?;
            super::modrinth::identifier(&c.version)?;
            if c.required_versions.len() > 128
                || c.required_projects
                    .iter()
                    .any(|p| super::modrinth::identifier(p).is_err())
                || c.required_versions.iter().any(|(p, v)| {
                    super::modrinth::identifier(p).is_err()
                        || super::modrinth::identifier(v).is_err()
                })
            {
                return Err(invalid("Invalid recorded dependency identity."));
            }
            let parts: Vec<_> = c.path.split('/').collect();
            let scope = match c.kind {
                ContentKind::Datapack
                    if parts.len() == 4 && parts[0] == "saves" && parts[2] == "datapacks" =>
                {
                    parts[1]
                }
                ContentKind::Mod | ContentKind::Resourcepack | ContentKind::Shader
                    if parts.len() == 2 && parts[0] == c.kind.directory() =>
                {
                    ""
                }
                _ => {
                    return Err(invalid(
                        "Content receipts must point only to their kind's pack/mod files.",
                    ));
                }
            };
            let filename = parts
                .last()
                .ok_or_else(|| invalid("Missing content filename."))?;
            let filename = if c.enabled {
                if filename.ends_with(".disabled") {
                    return Err(invalid("Invalid enabled content filename."));
                }
                *filename
            } else {
                filename
                    .strip_suffix(".disabled")
                    .ok_or_else(|| invalid("Invalid disabled content filename."))?
            };
            let extension = if c.kind == ContentKind::Mod {
                ".jar"
            } else {
                ".zip"
            };
            if !filename.ends_with(extension) {
                return Err(invalid(
                    "Content receipt file type does not match its kind.",
                ));
            }
            if !projects.insert((c.project.as_str(), c.kind.api_name(), scope)) {
                return Err(invalid(
                    "A project is recorded more than once for one content destination.",
                ));
            }
            if !c.path.starts_with("mods/")
                && !c.path.starts_with("resourcepacks/")
                && !c.path.starts_with("shaderpacks/")
                && !c.path.starts_with("saves/")
            {
                return Err(invalid(
                    "Content path is outside the supported instance directories.",
                ));
            }
            if !paths.insert(c.path.to_ascii_lowercase())
                || c.size == 0
                || c.size > 1024 * 1024 * 1024
                || c.sha512.len() != 128
                || !c.sha512.bytes().all(|b| b.is_ascii_hexdigit())
                || c.required_projects.len() > 128
            {
                return Err(invalid("Invalid installed content receipt."));
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct InstanceSummary {
    pub instance: Instance,
    pub game_dir: std::path::PathBuf,
}
#[derive(Clone, Debug, Serialize)]
pub struct Snapshot {
    pub api_version: u32,
    pub instances: Vec<InstanceSummary>,
    pub selected: Option<Id>,
    pub active_operation: Option<u64>,
    pub appearance: super::Appearance,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Search {
    pub query: String,
    pub kind: ContentKind,
    pub target: Target,
    pub offset: u32,
    pub limit: u32,
}
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchOrder {
    #[default]
    Relevance,
    Downloads,
    Updated,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SearchOptions {
    pub order: SearchOrder,
    pub categories: Vec<String>,
}
#[derive(Clone, Debug)]
pub struct Capabilities {
    pub api_version: u32,
    pub targets: Vec<Target>,
    pub content_kinds: Vec<ContentKind>,
    pub runtime_modules: Vec<&'static str>,
    pub appearance: bool,
    pub world_listing: bool,
    pub search_options: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LocalWorld {
    pub key: String,
    pub name: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProjectSummary {
    pub project_id: String,
    pub slug: String,
    pub title: String,
    pub description: String,
    pub project_type: ContentKind,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub downloads: Option<u64>,
    #[serde(default)]
    pub date_modified: Option<String>,
    #[serde(default)]
    pub categories: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearchPage {
    pub hits: Vec<ProjectSummary>,
    pub offset: u32,
    pub limit: u32,
    pub total_hits: u32,
    /// Category slugs discovered in this page's real search hits.
    #[serde(default)]
    pub categories: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub slug: String,
    pub title: String,
    pub description: String,
    pub project_type: ContentKind,
    pub client_side: String,
    pub license: License,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct License {
    pub id: String,
    pub name: String,
    pub url: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VersionSummary {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub version_number: String,
    pub game_versions: Vec<String>,
    pub loaders: Vec<String>,
    #[serde(default)]
    pub version_type: Option<String>,
    #[serde(default)]
    pub date_published: Option<String>,
    #[serde(default)]
    pub size_bytes: Option<u64>,
}
#[derive(Clone, Debug, Serialize)]
pub struct PlanFile {
    pub project: String,
    pub version: String,
    pub kind: ContentKind,
    pub filename: String,
    pub size: u64,
}
#[derive(Clone, Debug, Serialize)]
pub struct PlanSummary {
    pub id: Id,
    pub instance: Id,
    pub revision: u64,
    pub files: Vec<PlanFile>,
    pub optional: Vec<String>,
    pub embedded: Vec<String>,
    pub expires_seconds: u32,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IntegrityReport {
    pub managed: bool,
    pub baseline_verified: bool,
    pub verified_files: u32,
    pub issues: Vec<String>,
    #[serde(default)]
    pub content: Vec<CheckedContent>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ContentStatus {
    Verified,
    Modified,
    Missing,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CheckedContent {
    pub receipt: ContentReceipt,
    pub status: ContentStatus,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn content_receipts_cannot_target_world_or_settings_data() {
        let mut instance = Instance {
            schema: 1,
            id: Id::new(),
            name: "Test".into(),
            revision: 1,
            target: Target::fabric(),
            mode: Mode::Custom,
            profile: Profile::new(Preset::Minimal),
            theme: Theme::default(),
            archived: false,
            content: vec![ContentReceipt {
                project: "A".into(),
                version: "a".into(),
                kind: ContentKind::Datapack,
                path: "saves/world/level.dat".into(),
                size: 1,
                sha512: "a".repeat(128),
                enabled: true,
                required_projects: Vec::new(),
                required_versions: BTreeMap::new(),
            }],
        };
        assert!(instance.validate().is_err());
        instance.content[0].path = "saves/world/datapacks/a.zip".into();
        instance.validate().unwrap();
        instance.content[0].kind = ContentKind::Mod;
        assert!(instance.validate().is_err());
    }
    #[test]
    fn profiles_preserve_overrides_and_share_product_schema() {
        let mut p = Profile::new(Preset::Pvp);
        assert!(p.document.resolve().modules[&ModuleId::Armor].enabled);
        p.document.set_enabled(ModuleId::Armor, false);
        let p: Profile = serde_json::from_slice(&serde_json::to_vec(&p).unwrap()).unwrap();
        assert!(!p.document.resolve().modules[&ModuleId::Armor].enabled);
        p.validate().unwrap();
        assert!(p.to_json().contains("ooze.profile/1"));
    }
    #[test]
    fn theme_is_data_only_bounded_and_contrasting() {
        let mut t = Theme::default();
        t.validate().unwrap();
        t.foreground = t.background;
        assert!(t.validate().is_err());
        t = Theme::default();
        t.font = "C:/untrusted.ttf".into();
        assert!(t.validate().is_err());
        assert!(serde_json::from_str::<Theme>("{\"schema\":1,\"script\":\"x\"}").is_err());
    }
    #[test]
    fn unsupported_adapters_and_forged_ids_are_rejected() {
        Target::vanilla().validate().unwrap();
        Target::fabric().validate().unwrap();
        let mut t = Target::fabric();
        t.loader = Loader::Forge;
        assert!(t.validate().is_err());
        for id in ["../x", "ABCDEF", "0000000000000000000000000000000/"] {
            assert!(Id(id.into()).validate().is_err());
        }
    }
}
