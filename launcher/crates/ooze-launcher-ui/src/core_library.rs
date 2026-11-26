//! Adapter from Sol's Library API 1 (`ooze_launcher_core::library`) to the interface's
//! [`crate::library`] types, in the same way `core_backend` adapts Launcher API 1.
//!
//! It holds no business logic. Commands map one to one (a project's details take two library
//! operations, reported as one), replies map to typed payloads, and errors keep the library's safe
//! message. **Capabilities report only what Sol's API supports today**; everything else stays
//! hidden in the interface:
//! - profiles (module switches): off until the library reports runtime modules (the game can't
//!   run any before M0; Real or absent);
//! - themes and local data packs use the library's persisted appearance and opaque world keys;
//! - search sort, category filters and metadata map the library's real catalog response;
//! - verification and repair remain backend-validated, never optimistic interface state.
//!
//! Instances have no mode (owner decision 2026-11-25, `SOL-INSTANCE-MODE-REMOVAL.md`): every
//! instance is a normal instance where players add compatible mods and packs. The library
//! migrates legacy instances on read; the interface never converts them.

use std::collections::VecDeque;
use std::path::Path;

use ooze_launcher_core::api::{ErrorCode, LauncherError as CoreError};
use ooze_launcher_core::library as sl;

use crate::library::*;

/// Interface operation IDs for results the adapter answers from the snapshot without a library
/// operation (profile load, installed list). Kept apart from the library's own IDs.
const SYNTHETIC: u64 = 1 << 62;

#[derive(Debug)]
enum Pending {
    Plain(LibCommand),
    /// Project details: waiting for the project, then for its versions.
    Project {
        instance: InstanceId,
        kind: ContentKind,
        target: sl::Target,
    },
    Versions {
        instance: InstanceId,
        kind: ContentKind,
        project: Box<sl::Project>,
    },
}

pub struct CoreLibrary {
    lib: sl::Library,
    caps: sl::Capabilities,
    raw: sl::Snapshot,
    snap: LibrarySnapshot,
    /// (library operation, interface operation, what it was).
    pending: Option<(u64, LibOpId, Pending)>,
    out: VecDeque<LibEvent>,
    synthetic: u64,
    /// The last plan's id, kind, selected project and file names (for the installed notice).
    plan: Option<(String, InstanceId, ContentKind, Vec<String>)>,
}

impl std::fmt::Debug for CoreLibrary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CoreLibrary")
            .field("instances", &self.snap.instances.len())
            .finish_non_exhaustive()
    }
}

impl CoreLibrary {
    /// Opens the library under the launcher's data directory (bounded local I/O; call before the
    /// window opens, beside `CoreBackend::open`). Without a release catalog the library offers only
    /// its built-in 1.21.11 targets (tests and fixtures).
    pub fn open(data_dir: &Path) -> Result<Self, LibError> {
        let lib = sl::Library::open(data_dir).map_err(|e| map_error(&e))?;
        Self::from_library(lib, data_dir)
    }

    /// Opens the library with the package's release catalog (`release.json`, the same path as
    /// `LauncherConfig::release_manifest`), so its capabilities list every target the package can
    /// install (`SOL-MULTIVERSION.md`).
    pub fn open_with_release(data_dir: &Path, release_manifest: &Path) -> Result<Self, LibError> {
        let lib = sl::Library::open_with_release(data_dir, release_manifest)
            .map_err(|e| map_error(&e))?;
        Self::from_library(lib, data_dir)
    }

    fn from_library(lib: sl::Library, data_dir: &Path) -> Result<Self, LibError> {
        migrate_legacy_appearance(&lib, data_dir)?;
        let raw = lib.snapshot();
        let caps = lib.capabilities();
        let snap = map_snapshot(&raw, &caps);
        Ok(Self {
            lib,
            caps,
            raw,
            snap,
            pending: None,
            out: VecDeque::new(),
            synthetic: 0,
            plan: None,
        })
    }

    fn synthetic_op(&mut self, payload: LibPayload) -> LibOpId {
        self.synthetic += 1;
        let op = LibOpId(SYNTHETIC | self.synthetic);
        self.out.push_back(LibEvent::Finished {
            op,
            result: Ok(payload),
        });
        op
    }

    fn raw_instance(&self, id: &InstanceId) -> Option<&sl::InstanceSummary> {
        self.raw.instances.iter().find(|s| s.instance.id.0 == id.0)
    }

    fn refresh(&mut self) {
        self.raw = self.lib.snapshot();
        let snap = map_snapshot(&self.raw, &self.caps);
        if snap != self.snap {
            self.snap = snap.clone();
            self.out.push_back(LibEvent::Changed(snap));
        }
    }

    fn start(&mut self, command: sl::Command, pending: Pending) -> Result<LibOpId, LibError> {
        let id = self.lib.submit(command).map_err(|e| map_error(&e))?;
        let op = LibOpId(id);
        self.pending = Some((id, op, pending));
        Ok(op)
    }

    fn finish(&mut self, op: LibOpId, result: Result<LibPayload, LibError>) {
        self.out.push_back(LibEvent::Finished { op, result });
    }

    fn on_finished(&mut self, id: u64, result: Result<sl::Reply, CoreError>) {
        let Some((pid, op, pending)) = self.pending.take() else {
            return;
        };
        if pid != id {
            self.pending = Some((pid, op, pending));
            return;
        }
        let reply = match result {
            Ok(r) => r,
            Err(e) => {
                self.refresh();
                self.finish(op, Err(map_error(&e)));
                return;
            }
        };
        match pending {
            Pending::Project {
                instance,
                kind,
                target,
            } => {
                let sl::Reply::Project(project) = reply else {
                    self.finish(op, Err(unexpected()));
                    return;
                };
                let next = sl::Command::Versions {
                    project: project.id.clone(),
                    target,
                };
                match self.lib.submit(next) {
                    Ok(id) => {
                        self.pending = Some((
                            id,
                            op,
                            Pending::Versions {
                                instance,
                                kind,
                                project: Box::new(project),
                            },
                        ))
                    }
                    Err(e) => self.finish(op, Err(map_error(&e))),
                }
            }
            Pending::Versions {
                instance,
                kind,
                project,
            } => {
                let sl::Reply::Versions(versions) = reply else {
                    self.finish(op, Err(unexpected()));
                    return;
                };
                let details = map_project(instance, kind, &project, &versions);
                self.finish(op, Ok(LibPayload::Project(details)));
            }
            Pending::Plain(command) => {
                self.refresh();
                let payload = self.map_reply(&command, reply);
                self.finish(op, payload);
            }
        }
    }

    fn map_reply(
        &mut self,
        command: &LibCommand,
        reply: sl::Reply,
    ) -> Result<LibPayload, LibError> {
        Ok(match (command, reply) {
            (LibCommand::Create { .. } | LibCommand::Duplicate { .. }, sl::Reply::Instance(id)) => {
                LibPayload::Created(InstanceId(id.0))
            }
            (LibCommand::SaveProfile { id, .. }, _) => {
                let s = self.raw_instance(id).ok_or_else(unexpected)?;
                LibPayload::Profile {
                    id: id.clone(),
                    profile: s.instance.profile.document.clone(),
                    revision: s.instance.revision,
                }
            }
            (LibCommand::ListWorlds(_), sl::Reply::Worlds { instance, worlds }) => {
                LibPayload::Worlds {
                    instance: InstanceId(instance.0),
                    worlds: worlds
                        .into_iter()
                        .map(|w| World {
                            key: w.key,
                            name: w.name,
                        })
                        .collect(),
                }
            }
            (LibCommand::Search(query), sl::Reply::Search(page)) => {
                LibPayload::Search(SearchPage {
                    query: query.clone(),
                    hits: page
                        .hits
                        .into_iter()
                        .map(|h| SearchHit {
                            project_id: h.project_id,
                            title: h.title,
                            summary: h.description,
                            author: h.author,
                            downloads: h.downloads,
                            updated: h.date_modified,
                        })
                        .collect(),
                    total: page.total_hits,
                    categories: page.categories,
                })
            }
            (
                LibCommand::Plan {
                    instance,
                    kind,
                    project_id,
                    world,
                    ..
                },
                sl::Reply::Plan(p),
            ) => {
                let plan = map_plan(instance, *kind, project_id, world.clone(), &p);
                self.plan = Some((
                    plan.plan_id.clone(),
                    instance.clone(),
                    *kind,
                    plan.install.iter().map(|i| i.title.clone()).collect(),
                ));
                LibPayload::Plan(plan)
            }
            (LibCommand::Apply { plan_id }, _) => {
                let (instance, kind, titles) = match self.plan.take() {
                    Some((id, instance, kind, titles)) if id == *plan_id => {
                        (instance, kind, titles)
                    }
                    _ => return Err(unexpected()),
                };
                LibPayload::Installed {
                    instance,
                    kind,
                    titles,
                }
            }
            (LibCommand::ListInstalled { instance, kind }, sl::Reply::Installed(records)) => {
                LibPayload::InstalledList {
                    instance: instance.clone(),
                    kind: *kind,
                    items: installed_items(&records, *kind),
                }
            }
            (
                LibCommand::CheckIntegrity(id) | LibCommand::Resolve { instance: id, .. },
                sl::Reply::Integrity(r),
            ) => LibPayload::Integrity {
                instance: id.clone(),
                report: map_integrity(&r),
            },
            (_, sl::Reply::None) => LibPayload::Done,
            _ => return Err(unexpected()),
        })
    }
}

/// Bounded, local-only startup migration, before the window/paint loop exists.
/// The old preference bytes stay intact; all new persistence goes through the library API.
fn migrate_legacy_appearance(lib: &sl::Library, data: &Path) -> Result<(), LibError> {
    use std::io::Read;
    if data.join("library/appearance.json").exists() {
        return Ok(());
    }
    let Ok(file) = std::fs::File::open(data.join("ui-preferences.json")) else {
        return Ok(());
    };
    let mut bytes = Vec::new();
    if file.take(65537).read_to_end(&mut bytes).is_err() || bytes.len() > 65536 {
        return Ok(());
    }
    let Ok(prefs) = serde_json::from_slice::<crate::core_backend::UiPrefs>(&bytes) else {
        return Ok(());
    };
    if prefs.schema != 1 || !ooze_ui::theme_doc::UI_SCALES.contains(&prefs.ui_scale_percent) {
        return Ok(());
    }
    let motion = match prefs.motion.as_str() {
        "off" => ooze_ui::theme_doc::Motion::Off,
        "reduced" => ooze_ui::theme_doc::Motion::Reduced,
        "full" => ooze_ui::theme_doc::Motion::Full,
        _ => return Ok(()),
    };
    let document = ooze_ui::theme_doc::ThemeDoc {
        motion,
        ui_scale: prefs.ui_scale_percent,
        ..Default::default()
    };
    let op = lib
        .submit(sl::Command::SaveAppearance(sl::Appearance { document }))
        .map_err(|e| map_error(&e))?;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
    while std::time::Instant::now() < deadline {
        if let Some(sl::Event::Finished { id, result }) = lib.try_event()
            && id == op
        {
            return result.map(|_| ()).map_err(|e| map_error(&e));
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    Err(LibError {
        kind: LibErrorKind::Config,
        detail: Some(
            "Appearance migration timed out; the original interface preferences were preserved."
                .into(),
        ),
        retry_after_s: None,
    })
}

impl LibraryBackend for CoreLibrary {
    fn api_version(&self) -> (u16, u16) {
        (LIBRARY_API_MAJOR, 0)
    }

    fn snapshot(&self) -> LibrarySnapshot {
        self.snap.clone()
    }

    fn submit(&mut self, command: LibCommand) -> Result<LibOpId, LibError> {
        if self.pending.is_some() {
            return Err(LibError::new(LibErrorKind::Busy));
        }
        let id = |i: &InstanceId| sl::Id(i.0.clone());
        let plain = |c: &LibCommand| Pending::Plain(c.clone());
        let c = command.clone();
        match &command {
            LibCommand::Select(i) => self.start(sl::Command::Select(id(i)), plain(&c)),
            LibCommand::Create { name, target } => self.start(
                sl::Command::Create {
                    name: name.clone(),
                    target: to_core_target(target),
                    preset: sl::Preset::Minimal,
                },
                plain(&c),
            ),
            LibCommand::Rename { id: i, name } => self.start(
                sl::Command::Rename {
                    instance: id(i),
                    name: name.clone(),
                },
                plain(&c),
            ),
            LibCommand::Duplicate { id: i, name } => self.start(
                sl::Command::Duplicate {
                    instance: id(i),
                    name: name.clone(),
                },
                plain(&c),
            ),
            LibCommand::Archive(i) => self.start(sl::Command::Archive(id(i)), plain(&c)),
            LibCommand::LoadProfile(i) => {
                let s = self
                    .raw_instance(i)
                    .ok_or_else(|| LibError::new(LibErrorKind::NotFound))?;
                let payload = LibPayload::Profile {
                    id: i.clone(),
                    profile: s.instance.profile.document.clone(),
                    revision: s.instance.revision,
                };
                Ok(self.synthetic_op(payload))
            }
            LibCommand::SaveProfile {
                id: i,
                profile,
                revision,
            } => {
                let current = self
                    .raw_instance(i)
                    .ok_or_else(|| LibError::new(LibErrorKind::NotFound))?;
                if current.instance.revision != *revision {
                    return Err(LibError::new(LibErrorKind::Stale));
                }
                self.start(
                    sl::Command::SaveProfile {
                        instance: id(i),
                        profile: sl::Profile {
                            document: profile.clone(),
                        },
                    },
                    plain(&c),
                )
            }
            LibCommand::Search(q) => {
                let target = self
                    .raw_instance(&q.instance)
                    .ok_or_else(|| LibError::new(LibErrorKind::NotFound))?
                    .instance
                    .target
                    .clone();
                self.start(
                    sl::Command::SearchFiltered {
                        search: sl::Search {
                            query: q.text.clone(),
                            kind: to_core_kind(q.kind),
                            target,
                            offset: q.offset,
                            limit: 20,
                        },
                        options: sl::SearchOptions {
                            order: match q.sort {
                                SortBy::Relevance => sl::SearchOrder::Relevance,
                                SortBy::Downloads => sl::SearchOrder::Downloads,
                                SortBy::Updated => sl::SearchOrder::Updated,
                            },
                            categories: q.categories.clone(),
                        },
                    },
                    plain(&c),
                )
            }
            LibCommand::Project {
                instance,
                kind,
                project_id,
            } => {
                let target = self
                    .raw_instance(instance)
                    .ok_or_else(|| LibError::new(LibErrorKind::NotFound))?
                    .instance
                    .target
                    .clone();
                self.start(
                    sl::Command::Project(project_id.clone()),
                    Pending::Project {
                        instance: instance.clone(),
                        kind: *kind,
                        target,
                    },
                )
            }
            LibCommand::Plan {
                instance,
                project_id,
                version_id,
                world,
                ..
            } => self.start(
                sl::Command::Plan {
                    instance: id(instance),
                    project: project_id.clone(),
                    version: version_id.clone(),
                    world: world.clone(),
                },
                plain(&c),
            ),
            LibCommand::Apply { plan_id } => {
                self.start(sl::Command::Apply(sl::Id(plan_id.clone())), plain(&c))
            }
            LibCommand::ListInstalled { instance, .. } => {
                self.start(sl::Command::InspectContent(id(instance)), plain(&c))
            }
            LibCommand::SetEnabled {
                instance,
                record_id,
                enabled,
            } => self.start(
                sl::Command::ChangeContent {
                    instance: id(instance),
                    record_id: record_id.clone(),
                    enabled: Some(*enabled),
                },
                plain(&c),
            ),
            LibCommand::Remove {
                instance,
                record_id,
            } => self.start(
                sl::Command::ChangeContent {
                    instance: id(instance),
                    record_id: record_id.clone(),
                    enabled: None,
                },
                plain(&c),
            ),
            LibCommand::CheckIntegrity(i) => self.start(sl::Command::Verify(id(i)), plain(&c)),
            LibCommand::SaveTheme(document) => self.start(
                sl::Command::SaveAppearance(sl::Appearance {
                    document: document.clone(),
                }),
                plain(&c),
            ),
            LibCommand::ListWorlds(i) => self.start(sl::Command::ListWorlds(id(i)), plain(&c)),
            LibCommand::Resolve {
                instance,
                file,
                action: IntegrityAction::Repair,
            } => self.start(
                sl::Command::RepairRecord {
                    instance: id(instance),
                    record_id: file.clone(),
                },
                plain(&c),
            ),
            // Not offered by this library: the interface never shows these (capabilities and
            // per-instance actions exclude them), so reaching here is a programming error.
            LibCommand::Install(_) | LibCommand::Update { .. } | LibCommand::Resolve { .. } => {
                Err(LibError::new(LibErrorKind::Invalid))
            }
        }
    }

    fn cancel(&mut self, op: LibOpId) -> bool {
        match &self.pending {
            Some((id, pop, _)) if *pop == op => self.lib.cancel(*id),
            _ => false,
        }
    }

    fn drain_events(&mut self, out: &mut Vec<LibEvent>) {
        while let Some(event) = self.lib.try_event() {
            match event {
                sl::Event::Finished { id, result } => self.on_finished(id, result),
                sl::Event::Progress {
                    id,
                    stage,
                    done,
                    total,
                    item,
                } => {
                    if let Some((pending_id, op, _)) = &self.pending
                        && *pending_id == id
                    {
                        self.out.push_back(LibEvent::Progress {
                            op: *op,
                            progress: LibProgress {
                                step: match stage {
                                    sl::ProgressStage::Contacting => LibStep::Contacting,
                                    sl::ProgressStage::Downloading => LibStep::Downloading,
                                    sl::ProgressStage::Verifying => LibStep::Verifying,
                                    sl::ProgressStage::Committing => LibStep::Committing,
                                },
                                done,
                                total,
                                item,
                            },
                        });
                    }
                }
                sl::Event::Started { .. } => {}
            }
        }
        out.extend(self.out.drain(..));
    }

    fn set_waker(&mut self, waker: crate::api::Waker) {
        self.lib.set_notify(waker);
    }
}

/// Shown when the library could not open: the Instances page explains, nothing else appears.
#[derive(Debug)]
pub struct FailedLibrary(pub LibError);

impl LibraryBackend for FailedLibrary {
    fn api_version(&self) -> (u16, u16) {
        (LIBRARY_API_MAJOR, 0)
    }
    fn open_error(&self) -> Option<LibError> {
        Some(self.0.clone())
    }
    fn snapshot(&self) -> LibrarySnapshot {
        LibrarySnapshot {
            capabilities: LibraryCapabilities {
                instances: true,
                ..LibraryCapabilities::default()
            },
            ..LibrarySnapshot::default()
        }
    }
    fn submit(&mut self, _: LibCommand) -> Result<LibOpId, LibError> {
        Err(self.0.clone())
    }
    fn cancel(&mut self, _: LibOpId) -> bool {
        false
    }
    fn drain_events(&mut self, _: &mut Vec<LibEvent>) {}
}

fn unexpected() -> LibError {
    LibError::new(LibErrorKind::Internal)
}

pub fn map_error(e: &CoreError) -> LibError {
    let kind = match e.code {
        ErrorCode::Busy => LibErrorKind::Busy,
        ErrorCode::Cancelled => LibErrorKind::Cancelled,
        ErrorCode::Network => LibErrorKind::Offline,
        // The library reports validation and unsupported combinations as configuration
        // errors with a specific safe message, shown under "That wasn't accepted".
        ErrorCode::Configuration => LibErrorKind::Invalid,
        ErrorCode::Integrity => LibErrorKind::HashMismatch,
        ErrorCode::Process => LibErrorKind::GameRunning,
        _ => LibErrorKind::Internal,
    };
    LibError {
        kind,
        detail: (!e.message.trim().is_empty())
            .then(|| crate::redact::redact(&e.message).into_owned()),
        retry_after_s: None,
    }
}

pub fn to_core_kind(k: ContentKind) -> sl::ContentKind {
    match k {
        ContentKind::Mod => sl::ContentKind::Mod,
        ContentKind::ResourcePack => sl::ContentKind::Resourcepack,
        ContentKind::DataPack => sl::ContentKind::Datapack,
        ContentKind::Shader => sl::ContentKind::Shader,
    }
}

pub fn from_core_kind(k: sl::ContentKind) -> ContentKind {
    match k {
        sl::ContentKind::Mod => ContentKind::Mod,
        sl::ContentKind::Resourcepack => ContentKind::ResourcePack,
        sl::ContentKind::Datapack => ContentKind::DataPack,
        sl::ContentKind::Shader => ContentKind::Shader,
    }
}

fn from_core_loader(l: sl::Loader) -> Loader {
    match l {
        sl::Loader::Vanilla => Loader::Vanilla,
        sl::Loader::Fabric => Loader::Fabric,
        sl::Loader::Quilt => Loader::Quilt,
        sl::Loader::Forge => Loader::Forge,
        sl::Loader::NeoForge => Loader::NeoForge,
    }
}

fn to_core_loader(l: Loader) -> sl::Loader {
    match l {
        Loader::Vanilla => sl::Loader::Vanilla,
        Loader::Fabric => sl::Loader::Fabric,
        Loader::Quilt => sl::Loader::Quilt,
        Loader::Forge => sl::Loader::Forge,
        Loader::NeoForge => sl::Loader::NeoForge,
    }
}

pub fn from_core_target(t: &sl::Target) -> GameTarget {
    GameTarget {
        game_version: t.minecraft.clone(),
        loader: from_core_loader(t.loader),
        loader_version: t.loader_version.clone(),
        java_major: u16::try_from(t.java_major).unwrap_or(u16::MAX),
        ooze: t.ooze,
    }
}

pub fn to_core_target(t: &GameTarget) -> sl::Target {
    sl::Target {
        minecraft: t.game_version.clone(),
        loader: to_core_loader(t.loader),
        loader_version: t.loader_version.clone(),
        java_major: u32::from(t.java_major),
        ooze: t.ooze,
    }
}

/// Content kinds the interface can offer: the library's kinds, without data packs until the
/// library lists an instance's worlds (the interface never reads the saves folder itself).
fn offered_kinds(caps: &sl::Capabilities) -> Vec<ContentKind> {
    caps.content_kinds
        .iter()
        .map(|k| from_core_kind(*k))
        .filter(|k| *k != ContentKind::DataPack || caps.world_listing)
        .collect()
}

/// Per instance: mods and shader packs need a mod loader (shaders need Iris, a Fabric mod).
fn kinds_for(t: &sl::Target, offered: &[ContentKind]) -> Vec<ContentKind> {
    offered
        .iter()
        .copied()
        .filter(|k| {
            t.loader != sl::Loader::Vanilla || !matches!(k, ContentKind::Mod | ContentKind::Shader)
        })
        .collect()
}

pub fn map_snapshot(raw: &sl::Snapshot, caps: &sl::Capabilities) -> LibrarySnapshot {
    let offered = offered_kinds(caps);
    let instances = raw
        .instances
        .iter()
        .filter(|s| !s.instance.archived)
        .map(|s| {
            let i = &s.instance;
            InstanceSummary {
                id: InstanceId(i.id.0.clone()),
                name: i.name.clone(),
                target: from_core_target(&i.target),
                preset: i.profile.document.base,
                overrides: override_count(&i.profile.document),
                install: None,
                actions: vec![
                    InstanceAction::Rename,
                    InstanceAction::Duplicate,
                    InstanceAction::Archive,
                ],
                content: kinds_for(&i.target, &offered),
                running: false,
                revision: i.revision,
                game_dir: s.game_dir.clone(),
            }
        })
        .collect();
    LibrarySnapshot {
        capabilities: LibraryCapabilities {
            // Fabric (with Ooze) first: it's the product's main target.
            create: {
                let mut t: Vec<GameTarget> = caps.targets.iter().map(from_core_target).collect();
                t.sort_by_key(|t| !t.ooze);
                t
            },
            instances: true,
            // Real or absent: module switches appear only once the game can run modules
            // (`runtime_modules` from the library; empty until M0).
            profiles: !caps.runtime_modules.is_empty(),
            themes: caps.appearance,
            content: offered,
            sort: caps.search_options,
        },
        instances,
        selected: raw.selected.as_ref().map(|i| InstanceId(i.0.clone())),
        theme: raw.appearance.document.clone(),
    }
}

fn map_project(
    instance: InstanceId,
    kind: ContentKind,
    p: &sl::Project,
    versions: &[sl::VersionSummary],
) -> ProjectDetails {
    let links = p
        .license
        .url
        .as_ref()
        .filter(|u| u.starts_with("https://"))
        .map(|u| vec![(p.license.name.clone(), u.clone())])
        .unwrap_or_default();
    ProjectDetails {
        instance,
        kind,
        project_id: p.id.clone(),
        title: p.title.clone(),
        author: None,
        description: p.description.clone(),
        license: (!p.license.name.is_empty()).then(|| p.license.name.clone()),
        links,
        versions: versions
            .iter()
            .map(|v| VersionSummary {
                version_id: v.id.clone(),
                number: v.version_number.clone(),
                channel: match v.version_type.as_deref() {
                    Some("release") => Some(Channel::Release),
                    Some("beta") => Some(Channel::Beta),
                    Some("alpha") => Some(Channel::Alpha),
                    _ => None,
                },
                published: v.date_published.clone(),
                game_versions: v.game_versions.clone(),
                loaders: v.loaders.clone(),
                size_bytes: v.size_bytes,
                // The library lists versions for this instance's game version and revalidates
                // the exact version when planning; an incompatible choice fails there with the
                // library's reason.
                compatible: Ok(()),
            })
            .collect(),
    }
}

fn map_plan(
    instance: &InstanceId,
    kind: ContentKind,
    project: &str,
    world: Option<String>,
    p: &sl::PlanSummary,
) -> DependencyPlan {
    DependencyPlan {
        plan_id: p.id.0.clone(),
        instance: instance.clone(),
        kind,
        install: p
            .files
            .iter()
            .map(|f| PlanItem {
                title: f.filename.clone(),
                version_number: f.version.clone(),
                kind: from_core_kind(f.kind),
                size_bytes: f.size,
                role: if f.project == project {
                    PlanRole::Selected
                } else {
                    PlanRole::Required
                },
            })
            .collect(),
        optional: p.optional.clone(),
        embedded: p.embedded.clone(),
        // The library fails closed on conflicts and missing loaders (an error, not a plan).
        conflicts: Vec::new(),
        world,
        shader_loader: None,
    }
}

fn installed_items(receipts: &[sl::CheckedContent], kind: ContentKind) -> Vec<InstalledItem> {
    receipts
        .iter()
        .filter(|r| from_core_kind(r.receipt.kind) == kind)
        .map(|checked| {
            let r = &checked.receipt;
            let file = r.path.rsplit('/').next().unwrap_or(&r.path).to_owned();
            let world = r
                .path
                .strip_prefix("saves/")
                .and_then(|rest| rest.split('/').next())
                .map(str::to_owned);
            InstalledItem {
                record_id: r.path.clone(),
                kind,
                title: file.clone(),
                version_number: r.version.clone(),
                file_name: file,
                state: match checked.status {
                    sl::ContentStatus::Missing => ContentState::Missing,
                    sl::ContentStatus::Modified => ContentState::Modified,
                    sl::ContentStatus::Verified if r.enabled => ContentState::Enabled,
                    sl::ContentStatus::Verified => ContentState::Disabled,
                },
                world,
                update: None,
            }
        })
        .collect()
}

/// The library's sentence for installed game files that fail verification (for its CLI). The
/// interface shows the typed [`IntegrityProblem::InstallationFiles`] for `baseline_verified ==
/// false` instead, so the same problem isn't listed twice.
const BASELINE_ISSUE: &str = "Installed game files changed or are missing. Use game Repair.";

fn map_integrity(r: &sl::IntegrityReport) -> IntegrityReport {
    let mut issues: Vec<IntegrityIssue> = Vec::new();
    // Every instance: `false` also means no installed generation yet (Library API 1).
    if !r.baseline_verified {
        issues.push(IntegrityIssue {
            file: String::new(),
            problem: IntegrityProblem::InstallationFiles,
            actions: Vec::new(),
        });
    }
    issues.extend(
        r.content
            .iter()
            .filter(|f| f.status != sl::ContentStatus::Verified)
            .map(|f| IntegrityIssue {
                file: f.receipt.path.clone(),
                problem: match f.status {
                    sl::ContentStatus::Missing => IntegrityProblem::Missing,
                    _ => IntegrityProblem::HashMismatch,
                },
                actions: vec![IntegrityAction::Repair],
            }),
    );
    let recorded_messages: std::collections::BTreeSet<_> = r
        .content
        .iter()
        .filter(|f| f.status != sl::ContentStatus::Verified)
        .map(|f| {
            format!(
                "Registered content changed or is missing: {}",
                f.receipt.path
            )
        })
        .collect();
    issues.extend(
        r.issues
            .iter()
            .filter(|s| !recorded_messages.contains(s.as_str()) && *s != BASELINE_ISSUE)
            .map(|text| IntegrityIssue {
                file: String::new(),
                problem: IntegrityProblem::Reported(crate::redact::redact(text).into_owned()),
                actions: Vec::new(),
            }),
    );
    IntegrityReport {
        checked_files: r.verified_files,
        issues,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_motion_and_scale_migrate_once_without_changing_the_original_preferences() {
        let dir = temp_dir("legacy-appearance");
        let path = dir.join("ui-preferences.json");
        let prefs = crate::core_backend::UiPrefs {
            motion: "off".into(),
            ui_scale_percent: 125,
            ..Default::default()
        };
        prefs.save(&path).unwrap();
        let original = std::fs::read(&path).unwrap();
        let mut lib = CoreLibrary::open(&dir).unwrap();
        let mut doc = lib.snapshot().theme;
        assert_eq!(doc.motion, ooze_ui::theme_doc::Motion::Off);
        assert_eq!(doc.ui_scale, 125);
        assert_eq!(std::fs::read(&path).unwrap(), original);
        doc.motion = ooze_ui::theme_doc::Motion::Reduced;
        let op = lib.submit(LibCommand::SaveTheme(doc.clone())).unwrap();
        wait(&mut lib, op).unwrap();
        drop(lib);
        let lib = CoreLibrary::open(&dir).unwrap();
        assert_eq!(
            lib.snapshot().theme,
            doc,
            "a global theme takes precedence over legacy preferences"
        );
        assert_eq!(std::fs::read(&path).unwrap(), original);
        drop(lib);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn global_appearance_and_opaque_local_worlds_use_the_real_library() {
        use std::sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        };
        let dir = temp_dir("appearance-worlds");
        let mut lib = CoreLibrary::open(&dir).unwrap();
        let woke = Arc::new(AtomicBool::new(false));
        let worker_woke = Arc::clone(&woke);
        lib.set_waker(Box::new(move || {
            worker_woke.store(true, Ordering::Release);
        }));
        let doc = ooze_ui::theme_doc::ThemeDoc {
            motion: ooze_ui::theme_doc::Motion::Off,
            ui_scale: 125,
            ..Default::default()
        };
        let op = lib.submit(LibCommand::SaveTheme(doc.clone())).unwrap();
        assert_eq!(wait(&mut lib, op), Ok(LibPayload::Done));
        assert!(woke.load(Ordering::Acquire));
        assert_eq!(lib.snapshot().theme, doc);
        let op = lib
            .submit(LibCommand::Create {
                name: "World test".into(),
                target: from_core_target(&sl::Target::vanilla()),
            })
            .unwrap();
        let Ok(LibPayload::Created(id)) = wait(&mut lib, op) else {
            panic!("created")
        };
        let game = lib.snapshot().instance(&id).unwrap().game_dir.clone();
        let world = game.join("saves").join("Local world");
        std::fs::create_dir_all(&world).unwrap();
        std::fs::write(world.join("level.dat"), b"fixture").unwrap();
        let op = lib.submit(LibCommand::ListWorlds(id.clone())).unwrap();
        let Ok(LibPayload::Worlds { instance, worlds }) = wait(&mut lib, op) else {
            panic!("worlds")
        };
        assert_eq!(instance, id);
        assert_eq!(worlds.len(), 1);
        assert_eq!(worlds[0].name, "Local world");
        assert!(worlds[0].key.starts_with("world:"));
        assert!(!worlds[0].key.contains("Local world"));
        drop(lib);
        let lib = CoreLibrary::open(&dir).unwrap();
        assert_eq!(
            lib.snapshot().theme,
            doc,
            "persisted independently of instances and legacy prefs"
        );
        drop(lib);
        std::fs::remove_dir_all(dir).unwrap();
    }

    fn temp_dir(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "ooze-ui-lib-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_nanos())
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn wait(lib: &mut CoreLibrary, op: LibOpId) -> Result<LibPayload, LibError> {
        let mut events = Vec::new();
        for _ in 0..3000 {
            lib.drain_events(&mut events);
            for e in events.drain(..) {
                if let LibEvent::Finished { op: o, result } = e
                    && o == op
                {
                    return result;
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        panic!("operation {op:?} did not finish");
    }

    /// Exercises Sol's real library (no network): create, select, rename, profile save with a
    /// revision check, installed list, archive, and the mapped capabilities.
    #[test]
    fn the_real_library_round_trips_through_the_interface_types() {
        let dir = temp_dir("roundtrip");
        let mut lib = CoreLibrary::open(&dir).unwrap();
        let caps = lib.snapshot().capabilities;
        assert!(caps.instances);
        assert!(
            !caps.profiles,
            "no module switches before the game can run modules"
        );
        assert!(caps.themes);
        assert!(caps.content.contains(&ContentKind::DataPack));
        assert_eq!(caps.create.len(), 2);

        let fabric = caps.create[0].clone();
        assert!(fabric.ooze && fabric.loader == Loader::Fabric);
        let op = lib
            .submit(LibCommand::Create {
                name: "Fabric test".into(),
                target: fabric,
            })
            .unwrap();
        let Ok(LibPayload::Created(id)) = wait(&mut lib, op) else {
            panic!("created")
        };
        let snap = lib.snapshot();
        let inst = snap.instance(&id).unwrap().clone();
        assert_eq!(inst.target.game_version, "1.21.11");
        assert!(inst.content.contains(&ContentKind::Mod));
        assert!(inst.game_dir.starts_with(&dir));

        let op = lib.submit(LibCommand::Select(id.clone())).unwrap();
        assert_eq!(wait(&mut lib, op), Ok(LibPayload::Done));
        assert_eq!(lib.snapshot().selected, Some(id.clone()));

        let op = lib.submit(LibCommand::LoadProfile(id.clone())).unwrap();
        let Ok(LibPayload::Profile {
            profile, revision, ..
        }) = wait(&mut lib, op)
        else {
            panic!("profile")
        };
        let mut edited = profile.clone();
        edited.base = Some(ooze_ui::profile::PresetId::Pvp);
        edited.set_enabled(ooze_ui::catalog::ModuleId::Zoom, true);
        assert_eq!(
            lib.submit(LibCommand::SaveProfile {
                id: id.clone(),
                profile: edited.clone(),
                revision: revision + 7,
            })
            .unwrap_err()
            .kind,
            LibErrorKind::Stale
        );
        let op = lib
            .submit(LibCommand::SaveProfile {
                id: id.clone(),
                profile: edited.clone(),
                revision,
            })
            .unwrap();
        let Ok(LibPayload::Profile {
            profile: saved,
            revision: r2,
            ..
        }) = wait(&mut lib, op)
        else {
            panic!("saved")
        };
        assert!(r2 > revision);
        assert_eq!(saved.base, Some(ooze_ui::profile::PresetId::Pvp));
        assert!(saved.resolve().modules[&ooze_ui::catalog::ModuleId::Zoom].enabled);
        let summary = lib.snapshot().instance(&id).cloned().unwrap();
        assert_eq!(summary.preset, Some(ooze_ui::profile::PresetId::Pvp));
        assert_eq!(summary.overrides, 1);

        let op = lib
            .submit(LibCommand::ListInstalled {
                instance: id.clone(),
                kind: ContentKind::Mod,
            })
            .unwrap();
        assert!(matches!(
            wait(&mut lib, op),
            Ok(LibPayload::InstalledList { items, .. }) if items.is_empty()
        ));

        let op = lib
            .submit(LibCommand::Rename {
                id: id.clone(),
                name: "Renamed".into(),
            })
            .unwrap();
        assert_eq!(wait(&mut lib, op), Ok(LibPayload::Done));
        assert_eq!(lib.snapshot().instance(&id).unwrap().name, "Renamed");

        let op = lib.submit(LibCommand::Archive(id.clone())).unwrap();
        assert_eq!(wait(&mut lib, op), Ok(LibPayload::Done));
        assert!(
            lib.snapshot().instance(&id).is_none(),
            "archived, not listed"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Live Modrinth (network): the L1 content flow through the interface types and Sol's real
    /// library. Run with `cargo test -p ooze-launcher-ui live_modrinth -- --ignored`.
    #[test]
    #[ignore = "contacts api.modrinth.com and downloads files"]
    fn live_modrinth_search_details_plan_install_and_verify() {
        let dir = temp_dir("live");
        let mut lib = CoreLibrary::open(&dir).unwrap();
        let fabric = lib.snapshot().capabilities.create[0].clone();
        let op = lib
            .submit(LibCommand::Create {
                name: "Live".into(),
                target: fabric,
            })
            .unwrap();
        let Ok(LibPayload::Created(id)) = wait(&mut lib, op) else {
            panic!()
        };
        let op = lib.submit(LibCommand::Select(id.clone())).unwrap();
        wait(&mut lib, op).unwrap();
        for kind in [
            ContentKind::Mod,
            ContentKind::ResourcePack,
            ContentKind::Shader,
        ] {
            let op = lib
                .submit(LibCommand::Search(SearchQuery {
                    instance: id.clone(),
                    kind,
                    text: String::new(),
                    sort: SortBy::Relevance,
                    categories: Vec::new(),
                    offset: 0,
                }))
                .unwrap();
            let Ok(LibPayload::Search(page)) = wait(&mut lib, op) else {
                panic!("{kind:?} search")
            };
            assert!(!page.hits.is_empty(), "{kind:?}: real results");
            println!("{kind:?}: {} of {} hits", page.hits.len(), page.total);
        }
        // Iris (requires Sodium), the same public project Sol's library test installs.
        let op = lib
            .submit(LibCommand::Project {
                instance: id.clone(),
                kind: ContentKind::Mod,
                project_id: "YL57xq9U".into(),
            })
            .unwrap();
        let Ok(LibPayload::Project(details)) = wait(&mut lib, op) else {
            panic!("details")
        };
        let version = details
            .versions
            .iter()
            .find(|v| v.loaders.iter().any(|l| l == "fabric"))
            .expect("a Fabric 1.21.11 version")
            .version_id
            .clone();
        let op = lib
            .submit(LibCommand::Plan {
                instance: id.clone(),
                kind: ContentKind::Mod,
                project_id: details.project_id.clone(),
                version_id: version,
                world: None,
            })
            .unwrap();
        let Ok(LibPayload::Plan(plan)) = wait(&mut lib, op) else {
            panic!("plan")
        };
        assert!(plan.install.iter().any(|i| i.role == PlanRole::Selected));
        assert!(
            plan.install.iter().any(|i| i.role == PlanRole::Required),
            "Sodium is a required dependency"
        );
        println!(
            "plan: {:?}",
            plan.install.iter().map(|i| &i.title).collect::<Vec<_>>()
        );
        let op = lib
            .submit(LibCommand::Apply {
                plan_id: plan.plan_id.clone(),
            })
            .unwrap();
        let Ok(LibPayload::Installed { titles, .. }) = wait_long(&mut lib, op) else {
            panic!("installed")
        };
        assert_eq!(titles.len(), plan.install.len());
        let op = lib
            .submit(LibCommand::ListInstalled {
                instance: id.clone(),
                kind: ContentKind::Mod,
            })
            .unwrap();
        let Ok(LibPayload::InstalledList { items, .. }) = wait(&mut lib, op) else {
            panic!()
        };
        assert_eq!(items.len(), plan.install.len());
        let op = lib.submit(LibCommand::CheckIntegrity(id.clone())).unwrap();
        let Ok(LibPayload::Integrity { report, .. }) = wait(&mut lib, op) else {
            panic!()
        };
        // The installed content verifies; the only issue is the game installation itself, which
        // this test never installed (Library API 1: no installed generation).
        assert_eq!(
            report.issues,
            vec![IntegrityIssue {
                file: String::new(),
                problem: IntegrityProblem::InstallationFiles,
                actions: Vec::new(),
            }]
        );
        assert_eq!(report.checked_files as usize, items.len());
        let game_dir = lib.snapshot().instance(&id).unwrap().game_dir.clone();
        change_and_repair(
            &mut lib,
            &id,
            &game_dir.join("mods").join(&items[0].file_name),
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Changes one installed file, checks that Verify reports it with Repair (and no policy
    /// text), repairs it through the interface command and checks the original is back.
    fn change_and_repair(lib: &mut CoreLibrary, id: &InstanceId, file: &std::path::Path) {
        let original = std::fs::read(file).unwrap();
        std::fs::write(file, b"changed").unwrap();
        let op = lib.submit(LibCommand::CheckIntegrity(id.clone())).unwrap();
        let Ok(LibPayload::Integrity { report, .. }) = wait(lib, op) else {
            panic!("verify")
        };
        let changed = report
            .issues
            .iter()
            .find(|i| i.problem == IntegrityProblem::HashMismatch)
            .expect("the changed file is reported")
            .clone();
        assert_eq!(changed.actions, vec![IntegrityAction::Repair]);
        assert!(
            report
                .issues
                .iter()
                .all(|i| !matches!(&i.problem, IntegrityProblem::Reported(_))),
            "no policy text: {:?}",
            report.issues
        );
        let op = lib
            .submit(LibCommand::Resolve {
                instance: id.clone(),
                file: changed.file.clone(),
                action: IntegrityAction::Repair,
            })
            .unwrap();
        let Ok(LibPayload::Integrity { report, .. }) = wait_long(lib, op) else {
            panic!("repaired")
        };
        assert!(
            report
                .issues
                .iter()
                .all(|i| i.problem == IntegrityProblem::InstallationFiles),
            "{:?}",
            report.issues
        );
        assert_eq!(std::fs::read(file).unwrap(), original, "repaired");
        println!("repaired {}", changed.file);
    }

    fn wait_long(lib: &mut CoreLibrary, op: LibOpId) -> Result<LibPayload, LibError> {
        for _ in 0..12 {
            let mut events = Vec::new();
            for _ in 0..500 {
                lib.drain_events(&mut events);
                for e in events.drain(..) {
                    if let LibEvent::Finished { op: o, result } = e
                        && o == op
                    {
                        return result;
                    }
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        }
        panic!("operation {op:?} did not finish");
    }

    /// A legacy instance saved as `managed` (schema 1, before the owner's 2026-11-25 decision)
    /// displays as a normal instance: no mode, the usual actions, selectable for Play, and
    /// Verify reports the installation with its own typed issue and no policy text. Written on
    /// disk rather than created, so it holds before and after Sol's read-time migration.
    #[test]
    fn a_legacy_managed_instance_displays_and_verifies_as_a_normal_instance() {
        let dir = temp_dir("legacy-managed");
        let (id, game_dir, manifest) = legacy_managed_instance(&dir);
        let fabric = CoreLibrary::open(&dir)
            .unwrap()
            .snapshot()
            .capabilities
            .create[0]
            .clone();
        let legacy = std::fs::read(&manifest).unwrap();

        let mut lib = CoreLibrary::open(&dir).unwrap();
        let inst = lib.snapshot().instance(&id).cloned().expect("listed");
        assert_eq!(inst.name, "Old managed");
        assert_eq!(inst.target, fabric);
        assert_eq!(inst.game_dir, game_dir);
        assert_eq!(
            inst.actions,
            vec![
                InstanceAction::Rename,
                InstanceAction::Duplicate,
                InstanceAction::Archive
            ]
        );
        assert!(inst.content.contains(&ContentKind::Mod), "mods allowed");
        assert_eq!(
            std::fs::read(&manifest).unwrap(),
            legacy,
            "opening doesn't rewrite it"
        );

        let op = lib.submit(LibCommand::Select(id.clone())).unwrap();
        assert_eq!(wait(&mut lib, op), Ok(LibPayload::Done));
        let snap = lib.snapshot();
        assert_eq!(snap.selected, Some(id.clone()));
        assert_eq!(snap.selected().unwrap().game_dir, game_dir, "Play's folder");

        let op = lib.submit(LibCommand::CheckIntegrity(id.clone())).unwrap();
        let Ok(LibPayload::Integrity { report, .. }) = wait(&mut lib, op) else {
            panic!("verify")
        };
        assert_eq!(
            report.issues,
            vec![IntegrityIssue {
                file: String::new(),
                problem: IntegrityProblem::InstallationFiles,
                actions: Vec::new(),
            }],
            "no policy or mode text"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// An installation record that fails verification: the library reports `baseline_verified ==
    /// false` with its CLI sentence, and the interface lists one typed installation issue.
    #[test]
    fn a_changed_game_installation_is_one_typed_issue() {
        let dir = temp_dir("baseline");
        let (id, game_dir, _) = legacy_managed_instance(&dir);
        let pointer = game_dir.join(".ooze-launcher");
        std::fs::create_dir_all(&pointer).unwrap();
        // A record from an unknown schema never verifies.
        let record = serde_json::json!({
            "schema": 0,
            "release_sha256": "",
            "directory": "generation",
            "files": [],
            "classpath": [],
            "main_class": "",
            "jvm_args": [],
            "game_args": [],
            "asset_index": "",
            "managed_java": null,
            "version_name": "",
        });
        std::fs::write(
            pointer.join("active-install").with_extension("json"),
            serde_json::to_vec(&record).unwrap(),
        )
        .unwrap();
        let mut lib = CoreLibrary::open(&dir).unwrap();
        let op = lib.submit(LibCommand::CheckIntegrity(id)).unwrap();
        let Ok(LibPayload::Integrity { report, .. }) = wait(&mut lib, op) else {
            panic!("verify")
        };
        assert_eq!(
            report.issues,
            vec![IntegrityIssue {
                file: String::new(),
                problem: IntegrityProblem::InstallationFiles,
                actions: Vec::new(),
            }]
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Creates a Fabric instance through the adapter, then saves its manifest as schema 1 did for
    /// a Managed Fair Play instance. Returns its ID, game folder and manifest path.
    fn legacy_managed_instance(
        dir: &std::path::Path,
    ) -> (InstanceId, std::path::PathBuf, std::path::PathBuf) {
        let mut lib = CoreLibrary::open(dir).unwrap();
        let fabric = lib.snapshot().capabilities.create[0].clone();
        let op = lib
            .submit(LibCommand::Create {
                name: "Old managed".into(),
                target: fabric,
            })
            .unwrap();
        let Ok(LibPayload::Created(id)) = wait(&mut lib, op) else {
            panic!("created")
        };
        let game_dir = lib.snapshot().instance(&id).unwrap().game_dir.clone();
        drop(lib);
        // The library's instance manifest beside the game folder.
        let manifest = game_dir
            .parent()
            .unwrap()
            .join("instance")
            .with_extension("json");
        let mut value: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&manifest).unwrap()).unwrap();
        value["mode"] = serde_json::Value::from("managed");
        std::fs::write(&manifest, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
        (id, game_dir, manifest)
    }

    /// Live Modrinth (network): a legacy Managed instance takes compatible mods like any other
    /// instance (Sol's read-time migration, `SOL-INSTANCE-MODE-REMOVAL.md`), and a changed file
    /// is put back by Repair. Run with
    /// `cargo test -p ooze-launcher-ui live_legacy -- --ignored`.
    #[test]
    #[ignore = "contacts api.modrinth.com and downloads files"]
    fn live_legacy_managed_instance_installs_verifies_and_repairs_modrinth_content() {
        let dir = temp_dir("live-legacy");
        let (id, game_dir, _) = legacy_managed_instance(&dir);
        let mut lib = CoreLibrary::open(&dir).unwrap();
        // Sodium: a Fabric mod with no required dependencies on 1.21.11.
        let op = lib
            .submit(LibCommand::Project {
                instance: id.clone(),
                kind: ContentKind::Mod,
                project_id: "AANobbMI".into(),
            })
            .unwrap();
        let Ok(LibPayload::Project(details)) = wait(&mut lib, op) else {
            panic!("details")
        };
        let version = details
            .versions
            .iter()
            .find(|v| v.loaders.iter().any(|l| l == "fabric"))
            .expect("a Fabric 1.21.11 version")
            .version_id
            .clone();
        let op = lib
            .submit(LibCommand::Plan {
                instance: id.clone(),
                kind: ContentKind::Mod,
                project_id: details.project_id.clone(),
                version_id: version,
                world: None,
            })
            .unwrap();
        let plan = match wait(&mut lib, op) {
            Ok(LibPayload::Plan(plan)) => plan,
            other => panic!("a legacy instance must plan normally: {other:?}"),
        };
        let op = lib
            .submit(LibCommand::Apply {
                plan_id: plan.plan_id.clone(),
            })
            .unwrap();
        let Ok(LibPayload::Installed { .. }) = wait_long(&mut lib, op) else {
            panic!("installed")
        };
        let op = lib
            .submit(LibCommand::ListInstalled {
                instance: id.clone(),
                kind: ContentKind::Mod,
            })
            .unwrap();
        let Ok(LibPayload::InstalledList { items, .. }) = wait(&mut lib, op) else {
            panic!("listed")
        };
        assert_eq!(items.len(), plan.install.len());
        change_and_repair(
            &mut lib,
            &id,
            &game_dir.join("mods").join(&items[0].file_name),
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Sol's release catalog (schema 2, `SOL-MULTIVERSION.md`) drives the create choices through
    /// the real library: every record, Fabric with Ooze only where a bundle is pinned, and an exact
    /// target creates an instance with that target. A combination outside the catalog is refused.
    #[test]
    fn a_release_catalog_offers_exactly_its_targets_with_and_without_ooze() {
        use serde_json::{Value, json};
        let dir = temp_dir("catalog");
        let pin = |file: &str| json!({"file": file, "size": 1, "sha256": "0".repeat(64)});
        let fabric = |bundle: bool| {
            json!({
                "loader": "0.20.1",
                "profile": pin("metadata/fabric.json"),
                "libraries": [{
                    "name": "net.fabricmc:fabric-loader:0.20.1",
                    "path": "net/fabricmc/fabric-loader/0.20.1/fabric-loader-0.20.1.jar",
                    "artifact": pin("libraries/fabric-loader.jar")
                }],
                "bundle": if bundle {
                    json!({"api_version": "0.166.2+26.3", "api": pin("mods/fabric-api.jar"), "ooze": pin("mods/ooze-client.jar")})
                } else {
                    Value::Null
                }
            })
        };
        let row = |minecraft: &str, java: u32, fabric: Value| {
            json!({
                "minecraft": minecraft,
                "java_major": java,
                "minecraft_metadata": pin("metadata/minecraft.json"),
                "java": {"archive": pin("runtime/java.zip"), "java_relative": "runtime/bin/java.exe"},
                "fabric": fabric
            })
        };
        let catalog = json!({"schema": 2, "id": "test-catalog", "targets": [
            row("26.3", 25, Value::Null),
            row("26.3", 25, fabric(true)),
            row("26.2", 25, fabric(false)),
            row("1.21.11", 21, Value::Null),
        ]});
        let manifest = dir.join("release").with_extension("json");
        std::fs::write(&manifest, serde_json::to_vec(&catalog).unwrap()).unwrap();
        let mut lib = CoreLibrary::open_with_release(&dir, &manifest).unwrap();

        let target = |version: &str, loader: Loader, ooze: bool| GameTarget {
            game_version: version.into(),
            loader,
            loader_version: (loader == Loader::Fabric).then(|| "0.20.1".to_owned()),
            java_major: if version == "1.21.11" { 21 } else { 25 },
            ooze,
        };
        let create = lib.snapshot().capabilities.create;
        assert_eq!(
            create[0],
            target("26.3", Loader::Fabric, true),
            "Ooze targets first"
        );
        let mut got = create.clone();
        let mut want = vec![
            target("26.3", Loader::Fabric, true),
            target("26.3", Loader::Vanilla, false),
            target("26.3", Loader::Fabric, false),
            target("26.2", Loader::Fabric, false),
            target("1.21.11", Loader::Vanilla, false),
        ];
        let key = |t: &GameTarget| (t.game_version.clone(), t.loader, t.ooze);
        got.sort_by_key(key);
        want.sort_by_key(key);
        assert_eq!(got, want);

        for t in [
            target("26.2", Loader::Fabric, false),
            target("26.3", Loader::Fabric, true),
        ] {
            let op = lib
                .submit(LibCommand::Create {
                    name: format!("{} {}", t.game_version, t.ooze),
                    target: t.clone(),
                })
                .unwrap();
            let Ok(LibPayload::Created(id)) = wait(&mut lib, op) else {
                panic!("created {t:?}")
            };
            assert_eq!(lib.snapshot().instance(&id).unwrap().target, t);
        }
        let refused = lib
            .submit(LibCommand::Create {
                name: "Not in the catalog".into(),
                target: target("26.2", Loader::Fabric, true),
            })
            .and_then(|op| wait(&mut lib, op));
        assert!(refused.is_err(), "no Ooze bundle is pinned for 26.2");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn unsupported_combinations_are_refused_by_the_library_with_its_reason() {
        let dir = temp_dir("refuse");
        let mut lib = CoreLibrary::open(&dir).unwrap();
        let mut target = lib.snapshot().capabilities.create[0].clone();
        target.game_version = "1.20.1".into();
        let r = lib
            .submit(LibCommand::Create {
                name: "Old".into(),
                target,
            })
            .and_then(|op| wait(&mut lib, op));
        let e = r.unwrap_err();
        assert_eq!(e.kind, LibErrorKind::Invalid);
        assert!(e.detail.unwrap_or_default().contains("1.21.11"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn error_codes_map_to_typed_kinds_with_redacted_detail() {
        let e = map_error(&CoreError {
            code: ErrorCode::Network,
            message: "Couldn't reach Modrinth.".into(),
            retryable: true,
        });
        assert_eq!(e.kind, LibErrorKind::Offline);
        assert_eq!(e.detail.as_deref(), Some("Couldn't reach Modrinth."));
        assert_eq!(
            map_error(&CoreError {
                code: ErrorCode::Integrity,
                message: String::new(),
                retryable: false
            })
            .kind,
            LibErrorKind::HashMismatch
        );
    }
}
