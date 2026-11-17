//! Adapter from Sol's Library API 1 (`ooze_launcher_core::library`) to the interface's
//! [`crate::library`] types, in the same way `core_backend` adapts Launcher API 1.
//!
//! It holds no business logic. Commands map one to one (a project's details take two library
//! operations, reported as one), replies map to typed payloads, and errors keep the library's safe
//! message. **Capabilities report only what Sol's API supports today**; everything else stays
//! hidden in the interface:
//! - themes: off until the library persists `ooze.theme/1` (CLAUDE-P0-request R1);
//! - data packs: off until the library lists an instance's worlds;
//! - search sort and categories: off;
//! - switching to Managed Fair Play: off (the library offers only conversion to Custom).

use std::collections::VecDeque;
use std::path::Path;

use ooze_launcher_core::api::{ErrorCode, LauncherError as CoreError};
use ooze_launcher_core::library as sl;
use ooze_ui::theme_doc::ThemeDoc;

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
    raw: sl::Snapshot,
    snap: LibrarySnapshot,
    /// (library operation, interface operation, what it was).
    pending: Option<(u64, LibOpId, Pending)>,
    out: VecDeque<LibEvent>,
    synthetic: u64,
    /// The last plan's id, kind, selected project and file names (for the installed notice).
    plan: Option<(String, ContentKind, Vec<String>)>,
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
    /// window opens, beside `CoreBackend::open`).
    pub fn open(data_dir: &Path) -> Result<Self, LibError> {
        let lib = sl::Library::open(data_dir).map_err(|e| map_error(&e))?;
        let raw = lib.snapshot();
        let snap = map_snapshot(&raw);
        Ok(Self {
            lib,
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
        let snap = map_snapshot(&self.raw);
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
                            author: None,
                            downloads: None,
                            updated: None,
                        })
                        .collect(),
                    total: page.total_hits,
                    categories: Vec::new(),
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
                    *kind,
                    plan.install.iter().map(|i| i.title.clone()).collect(),
                ));
                LibPayload::Plan(plan)
            }
            (LibCommand::Apply { plan_id }, _) => {
                let (kind, titles) = match self.plan.take() {
                    Some((id, kind, titles)) if id == *plan_id => (kind, titles),
                    _ => (ContentKind::Mod, Vec::new()),
                };
                let instance = self.snap.selected.clone().ok_or_else(unexpected)?;
                LibPayload::Installed {
                    instance,
                    kind,
                    titles,
                }
            }
            (LibCommand::CheckIntegrity(id), sl::Reply::Integrity(r)) => LibPayload::Integrity {
                instance: id.clone(),
                report: map_integrity(&r),
            },
            (_, sl::Reply::None) => LibPayload::Done,
            _ => return Err(unexpected()),
        })
    }
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
                    mode: sl::Mode::Custom,
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
            LibCommand::SetMode {
                id: i,
                mode: IntegrityMode::Custom,
            } => self.start(sl::Command::ConvertToCustom(id(i)), plain(&c)),
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
                    sl::Command::Search(sl::Search {
                        query: q.text.clone(),
                        kind: to_core_kind(q.kind),
                        target,
                        offset: q.offset,
                        limit: 20,
                    }),
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
            LibCommand::ListInstalled { instance, kind } => {
                let s = self
                    .raw_instance(instance)
                    .ok_or_else(|| LibError::new(LibErrorKind::NotFound))?;
                let items = installed_items(&s.instance.content, *kind);
                let payload = LibPayload::InstalledList {
                    instance: instance.clone(),
                    kind: *kind,
                    items,
                };
                Ok(self.synthetic_op(payload))
            }
            LibCommand::SetEnabled {
                instance,
                record_id,
                enabled,
            } => self.start(
                sl::Command::SetEnabled {
                    instance: id(instance),
                    project: record_id.clone(),
                    enabled: *enabled,
                },
                plain(&c),
            ),
            LibCommand::Remove {
                instance,
                record_id,
            } => self.start(
                sl::Command::RemoveContent {
                    instance: id(instance),
                    project: record_id.clone(),
                },
                plain(&c),
            ),
            LibCommand::CheckIntegrity(i) => self.start(sl::Command::Verify(id(i)), plain(&c)),
            // Not offered by this library: the interface never shows these (capabilities and
            // per-instance actions exclude them), so reaching here is a programming error.
            LibCommand::SetMode { .. }
            | LibCommand::Install(_)
            | LibCommand::SaveTheme(_)
            | LibCommand::ListWorlds(_)
            | LibCommand::Update { .. }
            | LibCommand::Resolve { .. } => Err(LibError::new(LibErrorKind::Invalid)),
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
            if let sl::Event::Finished { id, result } = event {
                self.on_finished(id, result);
            }
        }
        out.extend(self.out.drain(..));
    }

    fn wants_polling(&self) -> bool {
        true
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

/// Content kinds this library can install for a target: Modrinth's loader-specific kinds need
/// Fabric; data packs wait for a world list (see module docs).
fn kinds_for(t: &sl::Target) -> Vec<ContentKind> {
    match t.loader {
        sl::Loader::Fabric => vec![
            ContentKind::Mod,
            ContentKind::ResourcePack,
            ContentKind::Shader,
        ],
        _ => vec![ContentKind::ResourcePack],
    }
}

pub fn map_snapshot(raw: &sl::Snapshot) -> LibrarySnapshot {
    let instances = raw
        .instances
        .iter()
        .filter(|s| !s.instance.archived)
        .map(|s| {
            let i = &s.instance;
            let mode = match i.mode {
                sl::Mode::Managed => IntegrityMode::ManagedFairPlay,
                sl::Mode::Custom => IntegrityMode::Custom,
            };
            let mut actions = vec![
                InstanceAction::Rename,
                InstanceAction::Duplicate,
                InstanceAction::Archive,
            ];
            if mode == IntegrityMode::ManagedFairPlay {
                actions.push(InstanceAction::ChangeMode);
            }
            InstanceSummary {
                id: InstanceId(i.id.0.clone()),
                name: i.name.clone(),
                target: from_core_target(&i.target),
                preset: i.profile.document.base,
                overrides: override_count(&i.profile.document),
                mode,
                install: None,
                actions,
                content: kinds_for(&i.target),
                running: false,
                revision: i.revision,
                game_dir: s.game_dir.clone(),
            }
        })
        .collect();
    LibrarySnapshot {
        capabilities: LibraryCapabilities {
            create: vec![
                from_core_target(&sl::Target::fabric()),
                from_core_target(&sl::Target::vanilla()),
            ],
            instances: true,
            profiles: true,
            themes: false,
            content: vec![
                ContentKind::Mod,
                ContentKind::ResourcePack,
                ContentKind::Shader,
            ],
            managed_mode: false,
            sort: false,
        },
        instances,
        selected: raw.selected.as_ref().map(|i| InstanceId(i.0.clone())),
        theme: ThemeDoc::default(),
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
                channel: None,
                published: None,
                game_versions: v.game_versions.clone(),
                loaders: v.loaders.clone(),
                size_bytes: None,
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

fn installed_items(receipts: &[sl::ContentReceipt], kind: ContentKind) -> Vec<InstalledItem> {
    receipts
        .iter()
        .filter(|r| from_core_kind(r.kind) == kind)
        .map(|r| {
            let file = r.path.rsplit('/').next().unwrap_or(&r.path).to_owned();
            let world = r
                .path
                .strip_prefix("saves/")
                .and_then(|rest| rest.split('/').next())
                .map(str::to_owned);
            InstalledItem {
                record_id: r.project.clone(),
                kind,
                title: file.clone(),
                version_number: r.version.clone(),
                file_name: file,
                state: if r.enabled {
                    ContentState::Enabled
                } else {
                    ContentState::Disabled
                },
                world,
                update: None,
            }
        })
        .collect()
}

fn map_integrity(r: &sl::IntegrityReport) -> IntegrityReport {
    let mut issues: Vec<IntegrityIssue> = Vec::new();
    if r.managed && !r.baseline_verified {
        issues.push(IntegrityIssue {
            file: String::new(),
            problem: IntegrityProblem::BaselineUnverified,
            actions: Vec::new(),
        });
    }
    issues.extend(r.issues.iter().map(|text| IntegrityIssue {
        file: String::new(),
        problem: IntegrityProblem::Reported(crate::redact::redact(text).into_owned()),
        actions: Vec::new(),
    }));
    IntegrityReport {
        mode: if r.managed {
            IntegrityMode::ManagedFairPlay
        } else {
            IntegrityMode::Custom
        },
        checked_files: r.verified_files,
        issues,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert!(caps.instances && caps.profiles);
        assert!(
            !caps.themes,
            "hidden until the library persists ooze.theme/1"
        );
        assert!(
            !caps.content.contains(&ContentKind::DataPack),
            "no world list yet"
        );
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
        assert!(report.issues.is_empty(), "{:?}", report.issues);
        assert_eq!(report.checked_files as usize, items.len());
        let _ = std::fs::remove_dir_all(&dir);
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
