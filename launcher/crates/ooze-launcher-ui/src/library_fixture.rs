//! Fixture library for tests and the developer preview only (`cfg(test)` or `dev-fixtures`, which
//! release builds refuse). Every name it produces says "fixture"; it never contacts Modrinth and
//! never writes files. The real library is Sol's `ooze_launcher_core::library`.

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, PoisonError};

use ooze_ui::profile::{PresetId, Profile};
use ooze_ui::theme_doc::ThemeDoc;

use crate::library::*;

pub fn fabric_target() -> GameTarget {
    GameTarget {
        game_version: "1.21.11".into(),
        loader: Loader::Fabric,
        loader_version: Some("0.20.1".into()),
        java_major: 21,
        ooze: true,
    }
}

pub fn vanilla_target() -> GameTarget {
    GameTarget {
        game_version: "1.21.11".into(),
        loader: Loader::Vanilla,
        loader_version: None,
        java_major: 21,
        ooze: false,
    }
}

/// Every combination the multi-version release offers, in the order Sol's library reports them
/// (Ooze targets first, then the catalog's Vanilla and plain Fabric records per version): Fabric
/// 0.20.1 with and without Ooze, and Vanilla, on 1.21.11 (Java 21) and 26.1 to 26.3 (Java 25).
pub fn all_targets() -> Vec<GameTarget> {
    const VERSIONS: [&str; 6] = ["1.21.11", "26.1", "26.1.1", "26.1.2", "26.2", "26.3"];
    let java = |v: &str| if v == "1.21.11" { 21 } else { 25 };
    let with = |base: GameTarget, v: &str| GameTarget {
        game_version: v.into(),
        java_major: java(v),
        ..base
    };
    let mut out: Vec<GameTarget> = VERSIONS.iter().map(|v| with(fabric_target(), v)).collect();
    for v in VERSIONS {
        out.push(with(vanilla_target(), v));
        out.push(GameTarget {
            ooze: false,
            ..with(fabric_target(), v)
        });
    }
    out
}

pub fn fabric_id() -> InstanceId {
    InstanceId("0f1e2d3c4b5a69788796a5b4c3d2e1f0".into())
}

pub fn vanilla_id() -> InstanceId {
    InstanceId("a1b2c3d4e5f60718293a4b5c6d7e8f90".into())
}

const ALL_ACTIONS: [InstanceAction; 4] = [
    InstanceAction::Rename,
    InstanceAction::Duplicate,
    InstanceAction::Repair,
    InstanceAction::Archive,
];

fn instance(
    id: InstanceId,
    name: &str,
    target: GameTarget,
    preset: Option<PresetId>,
) -> InstanceSummary {
    let content = if target.loader == Loader::Vanilla {
        vec![ContentKind::ResourcePack, ContentKind::DataPack]
    } else {
        ContentKind::ALL.to_vec()
    };
    InstanceSummary {
        game_dir: PathBuf::from(format!(
            r"C:\Users\Player\AppData\Roaming\.ooze\instances\{}",
            id.0
        )),
        id,
        name: name.into(),
        target,
        preset,
        overrides: 0,
        install: Some(InstallState::Ready),
        actions: ALL_ACTIONS.to_vec(),
        content,
        running: false,
        revision: 1,
    }
}

/// A library with two instances (Fabric with Ooze, Vanilla) and every capability.
pub fn snapshot() -> LibrarySnapshot {
    let fabric = instance(
        fabric_id(),
        "PvP practice",
        fabric_target(),
        Some(PresetId::Pvp),
    );
    LibrarySnapshot {
        capabilities: LibraryCapabilities {
            create: all_targets(),
            instances: true,
            profiles: true,
            themes: true,
            content: ContentKind::ALL.to_vec(),
            sort: true,
        },
        instances: vec![
            fabric,
            instance(vanilla_id(), "Plain survival", vanilla_target(), None),
        ],
        selected: Some(fabric_id()),
        theme: ThemeDoc::default(),
    }
}

pub fn hits(kind: ContentKind) -> Vec<SearchHit> {
    let base = match kind {
        ContentKind::Mod => "Example Utility Mod",
        ContentKind::ResourcePack => "Example Texture Pack",
        ContentKind::DataPack => "Example Data Pack",
        ContentKind::Shader => "Example Shader Pack",
    };
    (1..=3)
        .map(|i| SearchHit {
            project_id: format!("fixture-{kind:?}-{i}").to_lowercase(),
            title: format!("{base} {i} (fixture)"),
            summary: "Fixture data for tests and the developer preview. Not a real project.".into(),
            author: Some("Fixture Author".into()),
            downloads: Some(1_000 * i),
            updated: Some("2026-11-01".into()),
        })
        .collect()
}

pub fn details(instance: InstanceId, kind: ContentKind, project_id: &str) -> ProjectDetails {
    let title = hits(kind)
        .into_iter()
        .find(|h| h.project_id == project_id)
        .map_or_else(|| "Unknown fixture".to_owned(), |h| h.title);
    let loaders = match kind {
        ContentKind::Mod => vec!["fabric".to_owned()],
        ContentKind::Shader => vec!["iris".to_owned()],
        ContentKind::DataPack => vec!["datapack".to_owned()],
        ContentKind::ResourcePack => vec!["minecraft".to_owned()],
    };
    ProjectDetails {
        instance,
        kind,
        project_id: project_id.into(),
        title,
        author: Some("Fixture Author".into()),
        description: "Fixture project. <b>Markup</b> is shown as text.".into(),
        license: Some("MIT".into()),
        links: vec![("Source".into(), "https://example.invalid/source".into())],
        versions: vec![
            VersionSummary {
                version_id: format!("{project_id}-v2"),
                number: "2.0.0".into(),
                channel: Some(Channel::Release),
                published: Some("2026-11-01".into()),
                game_versions: vec!["1.21.11".into()],
                loaders: loaders.clone(),
                size_bytes: Some(1_250_000),
                compatible: Ok(()),
            },
            VersionSummary {
                version_id: format!("{project_id}-v1"),
                number: "1.0.0".into(),
                channel: Some(Channel::Release),
                published: Some("2026-06-01".into()),
                game_versions: vec!["1.21.10".into()],
                loaders,
                size_bytes: Some(1_100_000),
                compatible: Err("Needs Minecraft 1.21.10".into()),
            },
        ],
    }
}

pub fn plan(
    instance: InstanceId,
    kind: ContentKind,
    version_id: &str,
    world: Option<String>,
) -> DependencyPlan {
    let mut install = vec![PlanItem {
        title: version_id.trim_end_matches("-v2").replace('-', " "),
        version_number: "2.0.0".into(),
        kind,
        size_bytes: 1_250_000,
        role: PlanRole::Selected,
    }];
    let mut shader_loader = None;
    match kind {
        ContentKind::Mod => install.push(PlanItem {
            title: "Example Library (fixture)".into(),
            version_number: "0.9.0".into(),
            kind: ContentKind::Mod,
            size_bytes: 300_000,
            role: PlanRole::Required,
        }),
        ContentKind::Shader => {
            install.push(PlanItem {
                title: "Example Shader Loader (fixture)".into(),
                version_number: "1.8.0".into(),
                kind: ContentKind::Mod,
                size_bytes: 2_400_000,
                role: PlanRole::Required,
            });
            shader_loader = Some(ShaderLoader::InPlan(
                "Example Shader Loader (fixture)".into(),
            ));
        }
        _ => {}
    }
    DependencyPlan {
        plan_id: "42".into(),
        instance,
        kind,
        install,
        optional: if kind == ContentKind::Mod {
            vec!["Example Config Screen (fixture)".into()]
        } else {
            Vec::new()
        },
        embedded: Vec::new(),
        conflicts: Vec::new(),
        world,
        shader_loader,
    }
}

pub fn worlds() -> Vec<World> {
    vec![
        World {
            key: "w1".into(),
            name: "Fixture World".into(),
        },
        World {
            key: "w2".into(),
            name: "Creative Test (fixture)".into(),
        },
    ]
}

pub fn installed(kind: ContentKind) -> Vec<InstalledItem> {
    vec![InstalledItem {
        record_id: format!("rec-{kind:?}").to_lowercase(),
        kind,
        title: format!(
            "{} (fixture)",
            hits(kind)[0].title.trim_end_matches(" (fixture)")
        ),
        version_number: "1.0.0".into(),
        file_name: "example-1.0.0.jar".into(),
        state: ContentState::Enabled,
        world: (kind == ContentKind::DataPack).then(|| "Fixture World".into()),
        update: Some("2.0.0".into()),
    }]
}

/// The fixture's verification report. The Fabric instance has a changed and a missing file
/// that Ooze installed; other instances verify cleanly.
pub fn integrity(instance: &InstanceId) -> IntegrityReport {
    IntegrityReport {
        checked_files: 214,
        issues: if *instance == fabric_id() {
            vec![
                IntegrityIssue {
                    file: "mods/fabric-api-0.141.6+1.21.11.jar".into(),
                    problem: IntegrityProblem::HashMismatch,
                    actions: vec![IntegrityAction::Repair, IntegrityAction::OpenFolder],
                },
                IntegrityIssue {
                    file: "resourcepacks/faithful-32x.zip".into(),
                    problem: IntegrityProblem::Missing,
                    actions: vec![IntegrityAction::Repair],
                },
            ]
        } else {
            Vec::new()
        },
    }
}

/// A report whose only issue is the instance's game installation (not installed or changed).
pub fn integrity_installation() -> IntegrityReport {
    IntegrityReport {
        checked_files: 0,
        issues: vec![IntegrityIssue {
            file: String::new(),
            problem: IntegrityProblem::InstallationFiles,
            actions: Vec::new(),
        }],
    }
}

#[derive(Debug, Default)]
struct Inner {
    queue: VecDeque<LibEvent>,
    submitted: Vec<LibCommand>,
    cancelled: Vec<LibOpId>,
    next: u64,
    reject_next: Option<LibError>,
}

/// Test-side control of a manual fixture.
#[derive(Clone, Debug)]
pub struct LibFixtureHandle(Arc<Mutex<Inner>>);

impl LibFixtureHandle {
    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }

    pub fn push(&self, event: LibEvent) {
        self.lock().queue.push_back(event);
    }

    pub fn submitted(&self) -> Vec<LibCommand> {
        self.lock().submitted.clone()
    }

    pub fn cancelled(&self) -> Vec<LibOpId> {
        self.lock().cancelled.clone()
    }

    pub fn last_op(&self) -> LibOpId {
        LibOpId(self.lock().next)
    }

    pub fn reject_next(&self, error: LibError) {
        self.lock().reject_next = Some(error);
    }
}

#[derive(Debug)]
pub struct LibraryFixture {
    snap: LibrarySnapshot,
    last_kind: ContentKind,
    inner: Arc<Mutex<Inner>>,
    simulated: bool,
    profiles: Vec<(InstanceId, Profile, u64)>,
}

impl LibraryFixture {
    /// Answers nothing by itself: the test pushes every event.
    pub fn manual(snap: LibrarySnapshot) -> (Self, LibFixtureHandle) {
        let inner = Arc::new(Mutex::new(Inner::default()));
        (
            Self {
                snap,
                inner: Arc::clone(&inner),
                simulated: false,
                profiles: Vec::new(),
                last_kind: ContentKind::Mod,
            },
            LibFixtureHandle(inner),
        )
    }

    /// Answers every command at once with fixture data (developer preview).
    pub fn simulated() -> Self {
        let snap = snapshot();
        let profiles = snap
            .instances
            .iter()
            .filter(|i| i.target.ooze)
            .map(|i| {
                let preset = i.preset.unwrap_or(PresetId::Minimal);
                (
                    i.id.clone(),
                    Profile::from_preset(preset, "default", "Default"),
                    1,
                )
            })
            .collect();
        Self {
            snap,
            inner: Arc::new(Mutex::new(Inner::default())),
            simulated: true,
            profiles,
            last_kind: ContentKind::Mod,
        }
    }

    fn respond(&mut self, op: LibOpId, command: &LibCommand) -> Vec<LibEvent> {
        let ok = |p: LibPayload| LibEvent::Finished { op, result: Ok(p) };
        let mut out = Vec::new();
        let mut changed = false;
        match command.clone() {
            LibCommand::Select(id) => {
                self.snap.selected = Some(id);
                changed = true;
                out.push(ok(LibPayload::Done));
            }
            LibCommand::Create { name, target } => {
                let id = InstanceId(format!(
                    "{:032x}",
                    0xf1c7_u64 + self.snap.instances.len() as u64
                ));
                self.snap
                    .instances
                    .push(instance(id.clone(), &name, target, None));
                changed = true;
                out.push(ok(LibPayload::Created(id)));
            }
            LibCommand::Rename { id, name } => {
                if let Some(i) = self.snap.instances.iter_mut().find(|i| i.id == id) {
                    i.name = name;
                    i.revision += 1;
                }
                changed = true;
                out.push(ok(LibPayload::Done));
            }
            LibCommand::Duplicate { id, name } => {
                if let Some(src) = self.snap.instance(&id).cloned() {
                    let new_id = InstanceId(format!(
                        "{:032x}",
                        0xd0_u64 + self.snap.instances.len() as u64
                    ));
                    let mut copy = src;
                    copy.id = new_id.clone();
                    copy.name = name;
                    self.snap.instances.push(copy);
                    if let Some(p) = self.profiles.iter().find(|p| p.0 == id).cloned() {
                        self.profiles.push((new_id.clone(), p.1, 1));
                    }
                    out.push(ok(LibPayload::Created(new_id)));
                }
                changed = true;
            }
            LibCommand::Install(_) => {
                out.push(LibEvent::Progress {
                    op,
                    progress: LibProgress {
                        step: LibStep::Verifying,
                        done: 1,
                        total: Some(1),
                        item: None,
                    },
                });
                out.push(ok(LibPayload::Done));
            }
            LibCommand::Archive(id) => {
                self.snap.instances.retain(|i| i.id != id);
                if self.snap.selected.as_ref() == Some(&id) {
                    self.snap.selected = self.snap.instances.first().map(|i| i.id.clone());
                }
                changed = true;
                out.push(ok(LibPayload::Done));
            }
            LibCommand::LoadProfile(id) => {
                if let Some((_, p, r)) = self.profiles.iter().find(|p| p.0 == id) {
                    out.push(ok(LibPayload::Profile {
                        id,
                        profile: p.clone(),
                        revision: *r,
                    }));
                }
            }
            LibCommand::SaveProfile { id, profile, .. } => {
                let mut revision = 1;
                if let Some(slot) = self.profiles.iter_mut().find(|p| p.0 == id) {
                    slot.1 = profile.clone();
                    slot.2 += 1;
                    revision = slot.2;
                }
                if let Some(i) = self.snap.instances.iter_mut().find(|i| i.id == id) {
                    i.preset = profile.base;
                    i.overrides = override_count(&profile);
                }
                changed = true;
                out.push(ok(LibPayload::Profile {
                    id,
                    profile,
                    revision,
                }));
            }
            LibCommand::SaveTheme(doc) => {
                self.snap.theme = doc;
                changed = true;
                out.push(ok(LibPayload::Done));
            }
            LibCommand::Search(query) => {
                let hits: Vec<_> = hits(query.kind)
                    .into_iter()
                    .filter(|h| {
                        query.text.is_empty()
                            || h.title.to_lowercase().contains(&query.text.to_lowercase())
                    })
                    .collect();
                out.push(ok(LibPayload::Search(SearchPage {
                    total: hits.len() as u32,
                    hits,
                    categories: vec!["utility".into(), "optimization".into()],
                    query,
                })));
            }
            LibCommand::Project {
                instance,
                kind,
                project_id,
            } => out.push(ok(LibPayload::Project(details(
                instance,
                kind,
                &project_id,
            )))),
            LibCommand::ListWorlds(instance) => out.push(ok(LibPayload::Worlds {
                instance,
                worlds: worlds(),
            })),
            LibCommand::Plan {
                instance,
                kind,
                version_id,
                world,
                ..
            } => {
                self.last_kind = kind;
                let world =
                    world.and_then(|k| worlds().into_iter().find(|w| w.key == k).map(|w| w.name));
                out.push(ok(LibPayload::Plan(plan(
                    instance,
                    kind,
                    &version_id,
                    world,
                ))));
            }
            LibCommand::Apply { .. } => {
                for step in [
                    LibStep::Downloading,
                    LibStep::Verifying,
                    LibStep::Committing,
                ] {
                    out.push(LibEvent::Progress {
                        op,
                        progress: LibProgress {
                            step,
                            done: 1,
                            total: Some(2),
                            item: None,
                        },
                    });
                }
                let instance = self.snap.selected.clone().unwrap_or_else(fabric_id);
                out.push(ok(LibPayload::Installed {
                    instance,
                    kind: self.last_kind,
                    titles: vec![format!(
                        "{} 1 (fixture)",
                        match self.last_kind {
                            ContentKind::Mod => "Example Utility Mod",
                            ContentKind::ResourcePack => "Example Texture Pack",
                            ContentKind::DataPack => "Example Data Pack",
                            ContentKind::Shader => "Example Shader Pack",
                        }
                    )],
                }));
            }
            LibCommand::ListInstalled { instance, kind } => {
                out.push(ok(LibPayload::InstalledList {
                    instance,
                    kind,
                    items: installed(kind),
                }));
            }
            LibCommand::SetEnabled { .. }
            | LibCommand::Update { .. }
            | LibCommand::Remove { .. } => {
                out.push(ok(LibPayload::Done));
            }
            LibCommand::CheckIntegrity(instance) => {
                out.push(ok(LibPayload::Integrity {
                    report: integrity(&instance),
                    instance,
                }));
            }
            LibCommand::Resolve { instance, file, .. } => {
                let mut report = integrity(&instance);
                report.issues.retain(|i| i.file != file);
                out.push(ok(LibPayload::Integrity { instance, report }));
            }
        }
        if changed {
            out.push(LibEvent::Changed(self.snap.clone()));
        }
        out
    }
}

impl LibraryBackend for LibraryFixture {
    fn api_version(&self) -> (u16, u16) {
        (LIBRARY_API_MAJOR, 0)
    }

    fn snapshot(&self) -> LibrarySnapshot {
        self.snap.clone()
    }

    fn submit(&mut self, command: LibCommand) -> Result<LibOpId, LibError> {
        let op = {
            let mut inner = self.inner.lock().unwrap_or_else(PoisonError::into_inner);
            if let Some(e) = inner.reject_next.take() {
                return Err(e);
            }
            inner.submitted.push(command.clone());
            inner.next += 1;
            LibOpId(inner.next)
        };
        if self.simulated {
            let events = self.respond(op, &command);
            self.inner
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .queue
                .extend(events);
        }
        Ok(op)
    }

    fn cancel(&mut self, op: LibOpId) -> bool {
        self.inner
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .cancelled
            .push(op);
        true
    }

    fn drain_events(&mut self, out: &mut Vec<LibEvent>) {
        out.extend(
            self.inner
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .queue
                .drain(..),
        );
    }
}
