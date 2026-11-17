//! The interface's view of Sol's **Library API 1** (`docs/launcher/LIBRARY_API_V1.md`): instances,
//! profiles, themes, Modrinth content and integrity.
//!
//! The interface submits typed commands and renders typed results; it never computes
//! compatibility, dependency plans, paths or hashes. Every field here is display data or an opaque
//! backend identifier: no credential, no download URL chosen by the interface, no absolute path
//! other than an instance's game directory (needed for API 1 `SaveSettings`).
//!
//! These types are the interface's side of the contract (requested in
//! `docs/handoffs/CLAUDE-P0-request.md`); an adapter maps Sol's `ooze_launcher_core::library`
//! onto them, exactly as `core_backend` does for API 1.

use std::path::PathBuf;

use ooze_ui::profile::{PresetId, Profile};
use ooze_ui::theme_doc::ThemeDoc;

pub const LIBRARY_API_MAJOR: u16 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LibOpId(pub u64);

/// A random 32-hex identifier assigned by the library. Never a name or a path.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct InstanceId(pub String);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Loader {
    Vanilla,
    Fabric,
    Quilt,
    Forge,
    NeoForge,
}

impl Loader {
    pub fn key(self) -> &'static str {
        match self {
            Self::Vanilla => "loader.vanilla",
            Self::Fabric => "loader.fabric",
            Self::Quilt => "loader.quilt",
            Self::Forge => "loader.forge",
            Self::NeoForge => "loader.neoforge",
        }
    }
}

/// A game version + loader combination exactly as the library supports it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GameTarget {
    pub game_version: String,
    pub loader: Loader,
    pub loader_version: Option<String>,
    pub java_major: u16,
    /// The Ooze mod and its modules are available (Fabric 1.21.11 only).
    pub ooze: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum IntegrityMode {
    Custom,
    ManagedFairPlay,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InstallState {
    Ready,
    NeedsInstall,
    NeedsRepair,
}

/// Per-instance actions the library allows right now. The interface shows only these.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum InstanceAction {
    Rename,
    Duplicate,
    Install,
    Repair,
    Archive,
    ChangeMode,
}

#[derive(Clone, Debug, PartialEq)]
pub struct InstanceSummary {
    pub id: InstanceId,
    pub name: String,
    pub target: GameTarget,
    /// `None`: no preset base (all settings are the player's own).
    pub preset: Option<PresetId>,
    /// Number of user overrides on top of the preset.
    pub overrides: u32,
    pub mode: IntegrityMode,
    /// `None` when the library does not report install state (Play still verifies via API 1).
    pub install: Option<InstallState>,
    pub actions: Vec<InstanceAction>,
    /// Content kinds the library can manage for this instance (for example, no mods or shader
    /// packs for Vanilla). The interface offers the intersection with the capability.
    pub content: Vec<ContentKind>,
    /// This instance's game is running; mutations are refused until it exits.
    pub running: bool,
    pub revision: u64,
    /// For API 1 `SaveSettings` when this instance is played.
    pub game_dir: PathBuf,
}

impl InstanceSummary {
    pub fn allows(&self, action: InstanceAction) -> bool {
        !self.running && self.actions.contains(&action)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ContentKind {
    Mod,
    ResourcePack,
    DataPack,
    Shader,
}

impl ContentKind {
    pub const ALL: [Self; 4] = [Self::Mod, Self::ResourcePack, Self::DataPack, Self::Shader];

    pub fn key(self) -> &'static str {
        match self {
            Self::Mod => "content.kind.mod",
            Self::ResourcePack => "content.kind.resourcepack",
            Self::DataPack => "content.kind.datapack",
            Self::Shader => "content.kind.shader",
        }
    }
}

/// What the library can do. A capability that is false or empty hides its interface.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LibraryCapabilities {
    /// Combinations offered when creating an instance. Empty: instances are listed only.
    pub create: Vec<GameTarget>,
    pub instances: bool,
    pub profiles: bool,
    pub themes: bool,
    /// Content kinds the library can search and install (for supported instances).
    pub content: Vec<ContentKind>,
    pub managed_mode: bool,
    /// Search accepts a sort order.
    pub sort: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct LibrarySnapshot {
    pub capabilities: LibraryCapabilities,
    pub instances: Vec<InstanceSummary>,
    pub selected: Option<InstanceId>,
    pub theme: ThemeDoc,
}

impl LibrarySnapshot {
    pub fn instance(&self, id: &InstanceId) -> Option<&InstanceSummary> {
        self.instances.iter().find(|i| &i.id == id)
    }

    pub fn selected(&self) -> Option<&InstanceSummary> {
        self.selected.as_ref().and_then(|id| self.instance(id))
    }
}

// ---------------------------------------------------------------------------------------------
// Modrinth content

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SortBy {
    Relevance,
    Downloads,
    Updated,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SearchQuery {
    pub instance: InstanceId,
    pub kind: ContentKind,
    pub text: String,
    pub sort: SortBy,
    pub categories: Vec<String>,
    pub offset: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SearchHit {
    pub project_id: String,
    pub title: String,
    pub summary: String,
    /// Shown only when the library reports them.
    pub author: Option<String>,
    pub downloads: Option<u64>,
    /// ISO date as reported by Modrinth (display only).
    pub updated: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SearchPage {
    pub query: SearchQuery,
    pub hits: Vec<SearchHit>,
    pub total: u32,
    /// Categories the backend offers as filters for this kind.
    pub categories: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Channel {
    Release,
    Beta,
    Alpha,
}

#[derive(Clone, Debug, PartialEq)]
pub struct VersionSummary {
    pub version_id: String,
    pub number: String,
    pub channel: Option<Channel>,
    pub published: Option<String>,
    pub game_versions: Vec<String>,
    pub loaders: Vec<String>,
    pub size_bytes: Option<u64>,
    /// `Err(reason)`: the backend's safe explanation, for example "Needs Minecraft 1.21.10".
    pub compatible: Result<(), String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ProjectDetails {
    pub instance: InstanceId,
    pub kind: ContentKind,
    pub project_id: String,
    pub title: String,
    pub author: Option<String>,
    /// Plain text; markup is never interpreted.
    pub description: String,
    pub license: Option<String>,
    /// (label, https URL) pairs, shown with their host before opening.
    pub links: Vec<(String, String)>,
    pub versions: Vec<VersionSummary>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlanRole {
    Selected,
    Required,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PlanItem {
    pub title: String,
    pub version_number: String,
    pub kind: ContentKind,
    pub size_bytes: u64,
    pub role: PlanRole,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PlanConflict {
    pub title: String,
    /// The installed item it conflicts with.
    pub with: String,
}

/// Shader loader state for a shader-pack plan.
#[derive(Clone, Debug, PartialEq)]
pub enum ShaderLoader {
    /// A compatible loader is installed in this instance.
    Present(String),
    /// None installed; the plan includes it as a required item.
    InPlan(String),
    /// None installed and none the library can install for this instance.
    Unavailable,
}

/// A backend-held, opaque install plan. Apply sends only `plan_id`.
#[derive(Clone, Debug, PartialEq)]
pub struct DependencyPlan {
    /// Opaque backend identifier.
    pub plan_id: String,
    pub instance: InstanceId,
    pub kind: ContentKind,
    pub install: Vec<PlanItem>,
    /// Listed, never installed automatically.
    pub optional: Vec<String>,
    /// Bundled inside another file; informational.
    pub embedded: Vec<String>,
    pub conflicts: Vec<PlanConflict>,
    /// Data packs: the world the plan installs into (display name).
    pub world: Option<String>,
    pub shader_loader: Option<ShaderLoader>,
}

impl DependencyPlan {
    pub fn blocked(&self) -> bool {
        !self.conflicts.is_empty() || self.shader_loader == Some(ShaderLoader::Unavailable)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct World {
    /// Opaque to the interface (the backend's world key).
    pub key: String,
    pub name: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContentState {
    Enabled,
    Disabled,
    /// The recorded file's hash no longer matches.
    Modified,
    Missing,
}

#[derive(Clone, Debug, PartialEq)]
pub struct InstalledItem {
    pub record_id: String,
    pub kind: ContentKind,
    pub title: String,
    pub version_number: String,
    pub file_name: String,
    pub state: ContentState,
    pub world: Option<String>,
    /// A newer compatible version the library found, if any.
    pub update: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum IntegrityProblem {
    HashMismatch,
    Missing,
    UnexpectedMod,
    UnapprovedPack,
    /// The backend's safe name for the setting (for example "custom JVM argument").
    ProhibitedLaunchSetting(String),
    /// A problem the library describes only in its own safe words.
    Reported(String),
    /// Managed mode: the release manifest itself could not be verified.
    BaselineUnverified,
}

/// Safe remedies. There is deliberately no "delete": the library never deletes player files.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IntegrityAction {
    Repair,
    Quarantine,
    OpenFolder,
}

#[derive(Clone, Debug, PartialEq)]
pub struct IntegrityIssue {
    /// Instance-relative file name for display.
    pub file: String,
    pub problem: IntegrityProblem,
    pub actions: Vec<IntegrityAction>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct IntegrityReport {
    pub mode: IntegrityMode,
    pub checked_files: u32,
    pub issues: Vec<IntegrityIssue>,
}

// ---------------------------------------------------------------------------------------------
// Commands, progress, results

#[derive(Clone, Debug, PartialEq)]
pub enum LibCommand {
    Select(InstanceId),
    Create {
        name: String,
        target: GameTarget,
    },
    Rename {
        id: InstanceId,
        name: String,
    },
    Duplicate {
        id: InstanceId,
        name: String,
    },
    /// Install or repair the instance's game files.
    Install(InstanceId),
    Archive(InstanceId),
    SetMode {
        id: InstanceId,
        mode: IntegrityMode,
    },
    LoadProfile(InstanceId),
    /// The library revalidates with `ooze-ui` and refuses a stale `revision`.
    SaveProfile {
        id: InstanceId,
        profile: Profile,
        revision: u64,
    },
    SaveTheme(ThemeDoc),
    Search(SearchQuery),
    Project {
        instance: InstanceId,
        kind: ContentKind,
        project_id: String,
    },
    ListWorlds(InstanceId),
    Plan {
        instance: InstanceId,
        kind: ContentKind,
        project_id: String,
        version_id: String,
        world: Option<String>,
    },
    Apply {
        plan_id: String,
    },
    ListInstalled {
        instance: InstanceId,
        kind: ContentKind,
    },
    SetEnabled {
        instance: InstanceId,
        record_id: String,
        enabled: bool,
    },
    Update {
        instance: InstanceId,
        record_id: String,
    },
    Remove {
        instance: InstanceId,
        record_id: String,
    },
    CheckIntegrity(InstanceId),
    Resolve {
        instance: InstanceId,
        file: String,
        action: IntegrityAction,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LibStep {
    Contacting,
    Downloading,
    Verifying,
    /// Past the commit boundary: cancellation no longer applies.
    Committing,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LibProgress {
    pub step: LibStep,
    pub done: u64,
    pub total: Option<u64>,
    pub item: Option<String>,
}

impl LibProgress {
    pub fn fraction(&self) -> Option<f32> {
        self.total
            .filter(|t| *t > 0)
            .map(|t| (self.done as f32 / t as f32).clamp(0.0, 1.0))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LibErrorKind {
    Busy,
    GameRunning,
    Incompatible,
    Conflict,
    HashMismatch,
    RateLimited,
    Offline,
    NotFound,
    /// Library data couldn't be read; the original bytes are preserved.
    Config,
    /// The submitted document or value was refused by validation.
    Invalid,
    /// The instance changed since the interface read it (revision mismatch).
    Stale,
    Cancelled,
    Internal,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LibError {
    pub kind: LibErrorKind,
    /// Safe, redacted, displayable detail from the backend.
    pub detail: Option<String>,
    pub retry_after_s: Option<u32>,
}

impl LibError {
    pub fn new(kind: LibErrorKind) -> Self {
        Self {
            kind,
            detail: None,
            retry_after_s: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum LibPayload {
    Done,
    Created(InstanceId),
    Profile {
        id: InstanceId,
        profile: Profile,
        revision: u64,
    },
    Search(SearchPage),
    Project(ProjectDetails),
    Worlds {
        instance: InstanceId,
        worlds: Vec<World>,
    },
    Plan(DependencyPlan),
    /// Files verified (SHA-512) and recorded. Not a statement that the content works.
    Installed {
        instance: InstanceId,
        kind: ContentKind,
        titles: Vec<String>,
    },
    InstalledList {
        instance: InstanceId,
        kind: ContentKind,
        items: Vec<InstalledItem>,
    },
    Integrity {
        instance: InstanceId,
        report: IntegrityReport,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub enum LibEvent {
    Progress {
        op: LibOpId,
        progress: LibProgress,
    },
    Finished {
        op: LibOpId,
        result: Result<LibPayload, LibError>,
    },
    /// The library's persisted state changed (any source).
    Changed(LibrarySnapshot),
}

/// Number of user overrides in a profile (shown as "N changes" on instance rows).
pub fn override_count(p: &Profile) -> u32 {
    let o = &p.overrides;
    let modules: usize = o
        .modules
        .values()
        .map(|m| usize::from(m.enabled.is_some()) + m.settings.len())
        .sum();
    u32::try_from(modules + o.hud.len() + o.visual.len()).unwrap_or(u32::MAX)
}

/// The library as the interface uses it. Never blocks; results arrive through `drain_events`.
pub trait LibraryBackend {
    fn api_version(&self) -> (u16, u16);
    /// Set when the library could not open (for example a configuration error). The interface then
    /// shows that error on library pages and hides library actions.
    fn open_error(&self) -> Option<LibError> {
        None
    }
    fn snapshot(&self) -> LibrarySnapshot;
    fn submit(&mut self, command: LibCommand) -> Result<LibOpId, LibError>;
    /// Cooperative; returns whether the operation was still cancellable.
    fn cancel(&mut self, op: LibOpId) -> bool;
    fn drain_events(&mut self, out: &mut Vec<LibEvent>);
    fn wants_polling(&self) -> bool {
        false
    }
}
