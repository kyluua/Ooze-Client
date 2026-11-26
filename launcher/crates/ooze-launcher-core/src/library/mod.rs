//! Additive library API 1. Credentials and Minecraft objects never enter this facade.
mod appearance;
mod content;
mod model;
mod modrinth;
pub(crate) mod store;
use crate::{
    api::{ErrorCode, LauncherError},
    util::{self, Result, error, io_error},
};
pub use appearance::Appearance;
pub use model::*;
use modrinth::Source;
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc::{self, Receiver, SyncSender},
    },
};

#[derive(Clone, Debug)]
pub enum Command {
    SaveAppearance(Appearance),
    ImportAppearance(String),
    ExportAppearance,
    ListWorlds(Id),
    Create {
        name: String,
        target: Target,
        preset: Preset,
    },
    Select(Id),
    Rename {
        instance: Id,
        name: String,
    },
    Duplicate {
        instance: Id,
        name: String,
    },
    Archive(Id),
    SaveProfile {
        instance: Id,
        profile: Profile,
    },
    SaveTheme {
        instance: Id,
        theme: Theme,
    },
    ImportProfile {
        instance: Id,
        json: String,
    },
    ImportTheme {
        instance: Id,
        json: String,
    },
    ExportProfile(Id),
    ExportTheme(Id),
    Search(Search),
    SearchFiltered {
        search: Search,
        options: SearchOptions,
    },
    Project(String),
    Versions {
        project: String,
        target: Target,
    },
    Plan {
        instance: Id,
        project: String,
        version: String,
        world: Option<String>,
    },
    Apply(Id),
    SetEnabled {
        instance: Id,
        project: String,
        enabled: bool,
    },
    RemoveContent {
        instance: Id,
        project: String,
    },
    /// One exact installed receipt (opaque record identity from the snapshot).
    /// None removes; Some toggles. Unlike project commands, never spans worlds.
    ChangeContent {
        instance: Id,
        record_id: String,
        enabled: Option<bool>,
    },
    Verify(Id),
    InspectContent(Id),
    RepairRecord {
        instance: Id,
        record_id: String,
    },
    RepairContent {
        instance: Id,
        project: String,
    },
}
#[derive(Clone, Debug, serde::Serialize)]
pub enum Reply {
    Worlds {
        instance: Id,
        worlds: Vec<LocalWorld>,
    },
    None,
    Instance(Id),
    Search(SearchPage),
    Project(Project),
    Versions(Vec<VersionSummary>),
    Plan(PlanSummary),
    Integrity(IntegrityReport),
    Installed(Vec<CheckedContent>),
    Export(String),
    Imported {
        warnings: Vec<String>,
    },
}
#[derive(Clone, Debug)]
pub enum Event {
    Progress {
        id: u64,
        stage: ProgressStage,
        done: u64,
        total: Option<u64>,
        item: Option<String>,
    },
    Started {
        id: u64,
    },
    Finished {
        id: u64,
        result: std::result::Result<Reply, LauncherError>,
    },
}
struct Active {
    id: u64,
    cancel: Arc<AtomicBool>,
    cancellable: bool,
}
type Notify = Arc<dyn Fn() + Send + Sync>;
struct Inner {
    data: PathBuf,
    state: Mutex<Snapshot>,
    active: Mutex<Option<Active>>,
    next: AtomicU64,
    events: SyncSender<Event>,
    http: modrinth::Modrinth,
    plans: Mutex<BTreeMap<Id, content::Plan>>,
    notify: Mutex<Option<Notify>>,
    _lock: fs::File,
}
pub struct Library {
    inner: Arc<Inner>,
    events: Receiver<Event>,
}
impl Library {
    pub fn capabilities(&self) -> Capabilities {
        Capabilities {
            api_version: LIBRARY_API_VERSION,
            targets: vec![Target::fabric(), Target::vanilla()],
            content_kinds: vec![
                ContentKind::Mod,
                ContentKind::Resourcepack,
                ContentKind::Datapack,
                ContentKind::Shader,
            ],
            runtime_modules: Vec::new(),
            appearance: true,
            world_listing: true,
            search_options: true,
        }
    }
    pub fn open(data_dir: impl AsRef<Path>) -> Result<Self> {
        let data = data_dir.as_ref().to_owned();
        util::absolute(&data)?;
        let root = util::join(&data, "library")?;
        util::mkdir(&root)?;
        let lock = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(util::join(&root, "library.lock")?)
            .map_err(|_| io_error())?;
        fs2::FileExt::try_lock_exclusive(&lock).map_err(|_| {
            error(
                ErrorCode::Busy,
                "Another Ooze library is using this data directory.",
            )
        })?;
        let store = store::Store::open(&data)?;
        let state = store.snapshot()?;
        let (events, receiver) = mpsc::sync_channel(64);
        Ok(Self {
            inner: Arc::new(Inner {
                data,
                state: Mutex::new(state),
                active: Mutex::new(None),
                next: AtomicU64::new(1),
                events,
                http: modrinth::Modrinth::new()?,
                plans: Mutex::new(BTreeMap::new()),
                notify: Mutex::new(None),
                _lock: lock,
            }),
            events: receiver,
        })
    }
    pub fn snapshot(&self) -> Snapshot {
        let mut snapshot = self
            .inner
            .state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        snapshot.active_operation = self
            .inner
            .active
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
            .map(|a| a.id);
        snapshot
    }
    pub fn try_event(&self) -> Option<Event> {
        self.events.try_recv().ok()
    }
    /// A non-blocking notification hook. It must only schedule a UI repaint.
    pub fn set_notify(&self, notify: impl Fn() + Send + Sync + 'static) {
        *self.inner.notify.lock().unwrap_or_else(|e| e.into_inner()) = Some(Arc::new(notify));
    }
    pub fn submit(&self, command: Command) -> Result<u64> {
        let mut active = self.inner.active.lock().map_err(|_| io_error())?;
        if active.is_some() {
            return Err(error(
                ErrorCode::Busy,
                "A library operation is already running.",
            ));
        }
        let id = self
            .inner
            .next
            .try_update(Ordering::Relaxed, Ordering::Relaxed, |v| v.checked_add(1))
            .map_err(|_| {
                error(
                    ErrorCode::Internal,
                    "Library operation identities exhausted.",
                )
            })?;
        let cancel = Arc::new(AtomicBool::new(false));
        *active = Some(Active {
            id,
            cancel: cancel.clone(),
            cancellable: true,
        });
        drop(active);
        let inner = self.inner.clone();
        if std::thread::Builder::new()
            .name("ooze-library-operation".into())
            .spawn(move || {
                inner.emit(Event::Started { id });
                let check = || {
                    if cancel.load(Ordering::Acquire) {
                        Err(error(ErrorCode::Cancelled, "Library operation cancelled."))
                    } else {
                        Ok(())
                    }
                };
                let mut result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    inner.execute(command, &check, &|stage, done, total, item| {
                        inner.progress(id, stage, done, total, item)
                    })
                }))
                .unwrap_or_else(|_| {
                    Err(error(
                        ErrorCode::Internal,
                        "A library operation failed internally; restart Ooze.",
                    ))
                });
                match store::Store::open(&inner.data).and_then(|store| store.snapshot()) {
                    Ok(snapshot) => {
                        *inner.state.lock().unwrap_or_else(|e| e.into_inner()) = snapshot
                    }
                    Err(e) if result.is_ok() => result = Err(e),
                    Err(_) => {}
                }
                *inner.active.lock().unwrap_or_else(|e| e.into_inner()) = None;
                inner.emit(Event::Finished { id, result });
                inner.wake();
            })
            .is_err()
        {
            *self.inner.active.lock().map_err(|_| io_error())? = None;
            return Err(error(
                ErrorCode::Internal,
                "Could not start the library worker.",
            ));
        }
        Ok(id)
    }
    pub fn cancel(&self, id: u64) -> bool {
        if let Some(active) = self
            .inner
            .active
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
            && active.id == id
            && active.cancellable
        {
            active.cancel.store(true, Ordering::Release);
            return true;
        }
        false
    }
}
impl Drop for Library {
    fn drop(&mut self) {
        if let Some(active) = self
            .inner
            .active
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
            && active.cancellable
        {
            active.cancel.store(true, Ordering::Release);
        }
    }
}
impl Inner {
    fn progress(
        &self,
        id: u64,
        stage: ProgressStage,
        done: u64,
        total: Option<u64>,
        item: Option<&str>,
    ) -> Result<()> {
        if stage == ProgressStage::Committing {
            let mut active = self.active.lock().map_err(|_| io_error())?;
            if let Some(active) = active.as_mut() {
                if active.cancel.load(Ordering::Acquire) {
                    return Err(error(
                        ErrorCode::Cancelled,
                        "Library operation cancelled before commit.",
                    ));
                }
                active.cancellable = false;
            }
        }
        let event = Event::Progress {
            id,
            stage,
            done,
            total,
            item: item.map(str::to_owned),
        };
        if stage == ProgressStage::Committing {
            self.emit(event);
        } else {
            let _ = self.events.try_send(event);
            self.wake();
        }
        Ok(())
    }
    fn wake(&self) {
        let notify = self
            .notify
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        if let Some(notify) = notify {
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| notify()));
        }
    }
    fn emit(&self, event: Event) {
        let _ = self.events.send(event);
        self.wake();
    }
    fn execute(
        &self,
        command: Command,
        check: &dyn Fn() -> Result<()>,
        report: Reporter<'_>,
    ) -> Result<Reply> {
        check()?;
        report(ProgressStage::Contacting, 0, None, None)?;
        let store = store::Store::open(&self.data)?;
        match command {
            Command::SaveAppearance(appearance) => {
                appearance.validate()?;
                report(ProgressStage::Committing, 0, None, None)?;
                util::atomic_json(&util::join(&store.root, "appearance.json")?, &appearance)?;
                Ok(Reply::None)
            }
            Command::ImportAppearance(json) => {
                let appearance = Appearance::from_json(&json)?;
                self.execute(Command::SaveAppearance(appearance), check, report)
            }
            Command::ExportAppearance => Ok(Reply::Export(store.appearance()?.to_json())),
            Command::ListWorlds(id) => {
                let (_instance, _lock) = store.claim(&id)?;
                Ok(Reply::Worlds {
                    instance: id.clone(),
                    worlds: content::list_worlds(&store.game(&id)?, check)?,
                })
            }
            Command::Create {
                name,
                target,
                preset,
            } => {
                report(ProgressStage::Committing, 0, None, None)?;
                Ok(Reply::Instance(store.create(name, target, preset)?))
            }
            Command::Select(id) => {
                report(ProgressStage::Committing, 0, None, None)?;
                store.select(&id)?;
                Ok(Reply::None)
            }
            Command::Duplicate { instance, name } => Ok(Reply::Instance(
                store.duplicate_reported(&instance, name, check, report)?,
            )),
            Command::Archive(id) => {
                report(ProgressStage::Committing, 0, None, None)?;
                store.archive(&id)?;
                Ok(Reply::None)
            }
            Command::Rename {
                instance,
                name: name_value,
            } => {
                name(&name_value)?;
                let (mut value, _lock) = store.claim(&instance)?;
                value.name = name_value;
                report(ProgressStage::Committing, 0, None, None)?;
                store.save(&mut value)?;
                Ok(Reply::None)
            }
            Command::SaveProfile { instance, profile } => {
                profile.validate()?;
                let (mut value, _lock) = store.claim(&instance)?;
                value.profile = profile;
                report(ProgressStage::Committing, 0, None, None)?;
                store.save(&mut value)?;
                Ok(Reply::None)
            }
            Command::SaveTheme { instance, theme } => {
                theme.validate()?;
                let (mut value, _lock) = store.claim(&instance)?;
                value.theme = theme;
                report(ProgressStage::Committing, 0, None, None)?;
                store.save(&mut value)?;
                Ok(Reply::None)
            }
            Command::ImportProfile { instance, json } => {
                let (profile, warnings) = Profile::from_json(&json)?;
                self.execute(Command::SaveProfile { instance, profile }, check, report)?;
                Ok(Reply::Imported { warnings })
            }
            Command::ImportTheme { instance, json } => {
                let theme = import::<Theme>(&json)?;
                self.execute(Command::SaveTheme { instance, theme }, check, report)
            }
            Command::ExportProfile(id) => Ok(Reply::Export(store.instance(&id)?.profile.to_json())),
            Command::ExportTheme(id) => Ok(Reply::Export(
                serde_json::to_string_pretty(&store.instance(&id)?.theme)
                    .map_err(|_| io_error())?,
            )),
            Command::Search(request) => Ok(Reply::Search(self.http.search(&request, check)?)),
            Command::SearchFiltered { search, options } => Ok(Reply::Search(
                self.http.search_filtered(&search, &options, check)?,
            )),
            Command::Project(project) => Ok(Reply::Project(self.http.project(&project, check)?)),
            Command::Versions { project, target } => {
                let project = self.http.project(&project, check)?;
                Ok(Reply::Versions(
                    self.http
                        .versions(&project.id, &target, check)?
                        .into_iter()
                        .filter(|v| modrinth::compatible(&project, v, &target))
                        .map(|v| v.summary)
                        .collect(),
                ))
            }
            Command::Plan {
                instance,
                project,
                version,
                world,
            } => {
                let (value, _lock) = store.claim(&instance)?;
                let resolution = modrinth::resolve(&self.http, &value, &project, &version, check)?;
                for file in &resolution.files {
                    content::destination(
                        &store.game(&instance)?,
                        file.kind,
                        &file.file.filename,
                        world.as_deref(),
                    )?;
                }
                let plan = content::Plan {
                    id: Id::new(),
                    instance,
                    revision: value.revision,
                    created: std::time::Instant::now(),
                    resolution,
                    world,
                };
                let summary = plan.summary();
                let mut plans = self.plans.lock().map_err(|_| io_error())?;
                plans.retain(|_, p| p.created.elapsed() < std::time::Duration::from_secs(600));
                if plans.len() >= 8 {
                    return Err(error(
                        ErrorCode::Busy,
                        "Too many pending content plans. Apply one or wait for expiry.",
                    ));
                }
                plans.insert(plan.id.clone(), plan);
                Ok(Reply::Plan(summary))
            }
            Command::Apply(id) => {
                id.validate()?;
                let plan = self
                    .plans
                    .lock()
                    .map_err(|_| io_error())?
                    .remove(&id)
                    .ok_or_else(|| invalid("Unknown or consumed content plan."))?;
                content::apply(&store, &self.http, &plan, check, report)?;
                Ok(Reply::None)
            }
            Command::SetEnabled {
                instance,
                project,
                enabled,
            } => {
                content::change_reported(
                    &store,
                    &instance,
                    content::Selection::Project(&project),
                    Some(enabled),
                    check,
                    report,
                )?;
                Ok(Reply::None)
            }
            Command::RemoveContent { instance, project } => {
                content::change_reported(
                    &store,
                    &instance,
                    content::Selection::Project(&project),
                    None,
                    check,
                    report,
                )?;
                Ok(Reply::None)
            }
            Command::ChangeContent {
                instance,
                record_id,
                enabled,
            } => {
                content::change_reported(
                    &store,
                    &instance,
                    content::Selection::Record(&record_id),
                    enabled,
                    check,
                    report,
                )?;
                Ok(Reply::None)
            }
            Command::RepairContent { instance, project } => {
                content::repair(
                    &store,
                    &self.http,
                    &instance,
                    content::Selection::Project(&project),
                    check,
                    report,
                )?;
                Ok(Reply::None)
            }
            Command::RepairRecord {
                instance,
                record_id,
            } => {
                content::repair(
                    &store,
                    &self.http,
                    &instance,
                    content::Selection::Record(&record_id),
                    check,
                    report,
                )?;
                self.execute(Command::Verify(instance), check, report)
            }
            Command::InspectContent(id) => {
                let (instance, _lock) = store.claim(&id)?;
                Ok(Reply::Installed(content::inspect(
                    &instance,
                    &store.game(&id)?,
                    check,
                )?))
            }
            Command::Verify(id) => {
                let (value, _lock) = store.claim(&id)?;
                let game = store.game(&id)?;
                let mut report = content::verify(&value, &game, check)?;
                if let Some(verified) = verify_baseline(&value, &game, check)? {
                    report.baseline_verified = verified;
                    if !verified {
                        report.issues.push(
                            "Installed game files changed or are missing. Use game Repair.".into(),
                        );
                    }
                }
                Ok(Reply::Integrity(report))
            }
        }
    }
}
fn import<T: serde::de::DeserializeOwned>(json: &str) -> Result<T> {
    if json.len() > 65536 {
        return Err(invalid("Imported configuration exceeds 64 KiB."));
    }
    serde_json::from_str(json).map_err(|_| {
        invalid("Invalid or unsupported imported configuration; existing settings were preserved.")
    })
}

pub(crate) fn before_launch(game: &Path, check: &dyn Fn() -> Result<()>) -> Result<()> {
    if let Some(instance) = store::for_game(game)? {
        let report = content::verify(&instance, game, check)?;
        if !report.issues.is_empty() {
            return Err(error(
                ErrorCode::Integrity,
                "Instance content integrity check failed. Inspect Library Verify and repair the affected content.",
            ));
        }
    }
    Ok(())
}

fn verify_baseline(
    instance: &Instance,
    game: &Path,
    check: &dyn Fn() -> Result<()>,
) -> Result<Option<bool>> {
    let managed = util::join(game, ".ooze-launcher")?;
    let pointer = util::join(&managed, "active-install.json")?;
    if !pointer.is_file() {
        return Ok(None);
    }
    let install: crate::metadata::Installation = util::read_json(&pointer)?;
    if install.schema != 1 || install.files.is_empty() || install.files.len() > 100000 {
        return Ok(Some(false));
    }
    let expected = if instance.target.loader == Loader::Fabric {
        "net.fabricmc.loader.impl.launch.knot.KnotClient"
    } else {
        "net.minecraft.client.main.Main"
    };
    if install.main_class != expected {
        return Ok(Some(false));
    }
    let root = util::join(&managed, &install.directory)?;
    for file in &install.files {
        check()?;
        let path = util::join(&root, &file.path)?;
        if !path.is_file()
            || fs::metadata(&path).map_err(|_| io_error())?.len() != file.size
            || util::sha256(&path)? != file.sha256
        {
            return Ok(Some(false));
        }
    }
    Ok(Some(true))
}

pub(crate) fn recover_game(game: &Path) -> Result<()> {
    if let Some(instance) = store::for_game(game)? {
        let data = game.ancestors().nth(4).ok_or_else(io_error)?;
        let store = store::Store::open(data)?;
        if store.game(&instance.id)? != game {
            return Err(invalid("Library game path does not match its identity."));
        }
        content::recover(&store, &instance.id)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn durable_commit_progress_closes_cancellation_before_finished() {
        let dir = tempfile::tempdir().unwrap();
        let library = Library::open(dir.path()).unwrap();
        let mut appearance = Appearance::default();
        appearance.document.ui_scale = 125;
        let id = library
            .submit(Command::SaveAppearance(appearance.clone()))
            .unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        let mut commit = false;
        let mut finished = false;
        while std::time::Instant::now() < deadline {
            match library.try_event() {
                Some(Event::Progress {
                    id: event,
                    stage: ProgressStage::Committing,
                    ..
                }) if event == id => {
                    commit = true;
                    assert!(!library.cancel(id));
                }
                Some(Event::Finished { id: event, result }) if event == id => {
                    result.unwrap();
                    finished = true;
                    break;
                }
                _ => std::thread::sleep(std::time::Duration::from_millis(1)),
            }
        }
        assert!(commit && finished);
        assert_eq!(library.snapshot().appearance, appearance);
    }
    fn wait(library: &Library, id: u64) -> Result<Reply> {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while std::time::Instant::now() < deadline {
            if let Some(Event::Finished { id: event, result }) = library.try_event()
                && event == id
            {
                while library.snapshot().active_operation.is_some() {
                    std::thread::yield_now();
                }
                return result;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        panic!("worker timed out");
    }
    #[test]
    fn real_facade_persists_profiles_without_credentials() {
        let dir = tempfile::tempdir().unwrap();
        let library = Library::open(dir.path()).unwrap();
        let id = library
            .submit(Command::Create {
                name: "PvP".into(),
                target: Target::fabric(),
                preset: Preset::Pvp,
            })
            .unwrap();
        let Reply::Instance(instance) = wait(&library, id).unwrap() else {
            panic!()
        };
        let id = library.submit(Command::ExportProfile(instance)).unwrap();
        let Reply::Export(json) = wait(&library, id).unwrap() else {
            panic!()
        };
        assert!(json.contains("pvp"));
        assert!(!json.contains("account"));
        assert!(!json.contains("token"));
        drop(library);
        assert_eq!(
            Library::open(dir.path())
                .unwrap()
                .snapshot()
                .instances
                .len(),
            1
        );
    }
    #[test]
    fn separate_facades_cannot_mutate_one_library() {
        let dir = tempfile::tempdir().unwrap();
        let _first = Library::open(dir.path()).unwrap();
        assert!(matches!(Library::open(dir.path()),Err(e)if e.code==ErrorCode::Busy));
    }
}
