use crate::{
    api::*,
    auth, install,
    net::Http,
    process,
    util::{self, Result, error, io_error},
};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, SyncSender},
    },
};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Saved {
    schema: u32,
    settings: Settings,
    account: Option<AccountSummary>,
}
struct Active {
    id: OperationId,
    cancel: Arc<AtomicBool>,
    game_committed: bool,
}
struct State {
    saved: Saved,
    next_id: u64,
    active: Option<Active>,
    game_pid: Option<u32>,
}
struct Inner {
    config: LauncherConfig,
    state: Mutex<State>,
    events: SyncSender<LauncherEvent>,
    _lock: fs::File,
}
/// Own on the desktop UI thread; all submitted work runs on a separate backend thread.
pub struct Launcher {
    inner: Arc<Inner>,
    events: Receiver<LauncherEvent>,
}
impl Launcher {
    pub fn open(config: LauncherConfig) -> Result<Self> {
        if !cfg!(all(windows, target_arch = "x86_64")) {
            return Err(error(
                ErrorCode::UnsupportedPlatform,
                "L0 supports Windows x64 only.",
            ));
        }
        util::absolute(&config.data_dir)?;
        util::absolute(&config.release_manifest)?;
        util::mkdir(&config.data_dir)?;
        let lock_path = util::join(&config.data_dir, "launcher.lock")?;
        let lock = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(lock_path)
            .map_err(|_| io_error())?;
        fs2::FileExt::try_lock_exclusive(&lock).map_err(|_| {
            error(
                ErrorCode::Busy,
                "Another Ooze launcher is using this data directory.",
            )
        })?;
        let settings_path = util::join(&config.data_dir, "settings.json")?;
        let saved: Saved = if settings_path.exists() {
            util::read_json(&settings_path)?
        } else {
            Saved {
                schema: 1,
                settings: Settings {
                    instance_dir: config.data_dir.join("instances").join("ooze"),
                    java_path: None,
                    memory_mib: 4096,
                },
                account: None,
            }
        };
        if saved.schema != 1 {
            return Err(error(
                ErrorCode::Configuration,
                "Unsupported launcher settings schema. Preserve the file and use its matching launcher version.",
            ));
        }
        validate_settings(&saved.settings)?;
        let (tx, rx) = mpsc::sync_channel(256);
        Ok(Self {
            inner: Arc::new(Inner {
                config,
                state: Mutex::new(State {
                    saved,
                    next_id: 1,
                    active: None,
                    game_pid: None,
                }),
                events: tx,
                _lock: lock,
            }),
            events: rx,
        })
    }
    pub fn snapshot(&self) -> StateSnapshot {
        let state = self.inner.state.lock().unwrap_or_else(|e| e.into_inner());
        StateSnapshot {
            api_version: LAUNCHER_API_VERSION,
            settings: state.saved.settings.clone(),
            account: state.saved.account.clone(),
            active_operation: state.active.as_ref().map(|a| a.id),
            game_pid: state.game_pid,
        }
    }
    pub fn try_event(&self) -> Option<LauncherEvent> {
        self.events.try_recv().ok()
    }
    pub fn submit(&self, command: LauncherCommand) -> Result<OperationId> {
        let mut state = self.inner.state.lock().map_err(|_| {
            error(
                ErrorCode::Internal,
                "Launcher state is unavailable. Restart Ooze.",
            )
        })?;
        if state.active.is_some() {
            return Err(error(
                ErrorCode::Busy,
                "Wait for the current operation or close the running game.",
            ));
        }
        let id = OperationId(state.next_id);
        state.next_id = state.next_id.checked_add(1).ok_or_else(|| {
            error(
                ErrorCode::Internal,
                "Operation IDs exhausted. Restart Ooze.",
            )
        })?;
        let cancel = Arc::new(AtomicBool::new(false));
        state.active = Some(Active {
            id,
            cancel: cancel.clone(),
            game_committed: false,
        });
        drop(state);
        let inner = self.inner.clone();
        std::thread::Builder::new()
            .name("ooze-launcher-operation".into())
            .spawn(move || {
                let context = Context {
                    inner: inner.clone(),
                    id,
                    cancel,
                };
                context.control(LauncherEvent::Started { id });
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    context.execute(command)
                }))
                .unwrap_or_else(|_| {
                    Err(error(
                        ErrorCode::Internal,
                        "A launcher operation failed internally. Restart Ooze and retry.",
                    ))
                });
                if let Ok(mut state) = inner.state.lock() {
                    state.active = None;
                    state.game_pid = None;
                }
                let event = match result {
                    Ok(()) => LauncherEvent::Completed { id },
                    Err(e) if e.code == ErrorCode::Cancelled => LauncherEvent::Cancelled { id },
                    Err(error) => LauncherEvent::Failed { id, error },
                };
                context.control(event);
            })
            .map_err(|_| {
                if let Ok(mut state) = self.inner.state.lock() {
                    state.active = None;
                }
                error(ErrorCode::Internal, "Couldn't start a backend worker.")
            })?;
        Ok(id)
    }
    pub fn cancel(&self, id: OperationId) -> bool {
        let state = self.inner.state.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(active) = &state.active
            && active.id == id
            && !active.game_committed
        {
            active.cancel.store(true, Ordering::Release);
            return true;
        }
        false
    }
}
impl Drop for Launcher {
    fn drop(&mut self) {
        if let Ok(state) = self.inner.state.lock()
            && let Some(active) = &state.active
            && !active.game_committed
        {
            active.cancel.store(true, Ordering::Release);
        }
    }
}
struct Context {
    inner: Arc<Inner>,
    id: OperationId,
    cancel: Arc<AtomicBool>,
}
impl Context {
    fn check(&self) -> Result<()> {
        if self.cancel.load(Ordering::Acquire) {
            Err(error(ErrorCode::Cancelled, "Operation cancelled."))
        } else {
            Ok(())
        }
    }
    fn control(&self, event: LauncherEvent) {
        let _ = self.inner.events.send(event);
    }
    fn progress(&self, stage: Stage, completed: u64, total: Option<u64>) {
        let _ = self.inner.events.try_send(LauncherEvent::Progress {
            id: self.id,
            stage,
            completed,
            total,
        });
    }
    fn client_id(&self) -> Result<&str> {
        self.inner.config.microsoft_client_id.as_deref().filter(|s|!s.is_empty()).ok_or_else(||error(ErrorCode::Configuration,"The developer must configure Ooze's owned Microsoft OAuth client ID. Players do not register applications."))
    }
    fn credential_key(&self) -> Result<String> {
        use sha2::Digest;
        let path = self
            .inner
            .config
            .data_dir
            .canonicalize()
            .map_err(|_| io_error())?
            .to_string_lossy()
            .to_ascii_lowercase();
        Ok(format!(
            "{}-{}",
            self.client_id()?,
            hex::encode(sha2::Sha256::digest(path.as_bytes()))
        ))
    }
    fn set_account(&self, account: Option<AccountSummary>) -> Result<()> {
        let mut state = self.inner.state.lock().map_err(|_| io_error())?;
        let previous = state.saved.account.clone();
        state.saved.account = account.clone();
        let result = util::atomic_json(
            &util::join(&self.inner.config.data_dir, "settings.json")?,
            &state.saved,
        );
        if result.is_err() {
            state.saved.account = previous;
        }
        result?;
        drop(state);
        self.control(LauncherEvent::AccountChanged {
            id: self.id,
            account,
        });
        Ok(())
    }
    fn execute(&self, command: LauncherCommand) -> Result<()> {
        self.check()?;
        if let LauncherCommand::SaveSettings(settings) = command {
            validate_settings(&settings)?;
            let mut state = self.inner.state.lock().map_err(|_| io_error())?;
            let previous = state.saved.settings.clone();
            state.saved.settings = settings;
            let result = util::atomic_json(
                &util::join(&self.inner.config.data_dir, "settings.json")?,
                &state.saved,
            );
            if result.is_err() {
                state.saved.settings = previous;
            }
            return result;
        }
        if matches!(command, LauncherCommand::SignOut) {
            auth::forget(&self.credential_key()?)?;
            return self.set_account(None);
        }
        if let LauncherCommand::ExportDiagnostics(path) = command {
            return crate::diagnostics::export(&self.inner.config, &path, &|| self.check());
        }
        let http = Http::new()?;
        if matches!(command, LauncherCommand::SignIn) {
            self.progress(Stage::Authorizing, 0, None);
            let session = auth::authenticate(
                self.client_id()?,
                &self.credential_key()?,
                true,
                &http,
                &|| self.check(),
                &|| self.progress(Stage::AwaitingBrowser, 0, None),
            )?;
            return self.set_account(Some(session.account));
        }
        let settings = self
            .inner
            .state
            .lock()
            .map_err(|_| io_error())?
            .saved
            .settings
            .clone();
        let session = if matches!(command, LauncherCommand::Launch) {
            self.progress(Stage::Authorizing, 0, None);
            let session = auth::authenticate(
                self.client_id()?,
                &self.credential_key()?,
                false,
                &http,
                &|| self.check(),
                &|| {},
            )?;
            self.set_account(Some(session.account.clone()))?;
            Some(session)
        } else {
            None
        };
        let _instance_lock = install::claim_instance(&settings.instance_dir)?;
        crate::library::recover_game(&settings.instance_dir)?;
        if session.is_some() {
            crate::library::before_launch(&settings.instance_dir, &|| self.check())?;
        }
        let prepared = install::prepare(
            &settings,
            &self.inner.config.release_manifest,
            &self.inner.config.data_dir,
            &http,
            &|| self.check(),
            &|stage, n, total| self.progress(stage, n, total),
        )?;
        if let Some(session) = session {
            self.progress(Stage::StartingGame, 0, None);
            {
                let mut state = self.inner.state.lock().map_err(|_| io_error())?;
                self.check()?;
                if let Some(active) = &mut state.active {
                    active.game_committed = true;
                }
            }
            let tx = self.inner.events.clone();
            let id = self.id;
            let ready_sent = AtomicBool::new(false);
            let code = process::launch(
                &prepared,
                &settings,
                session,
                self.client_id()?,
                &self.inner.config.data_dir,
                |pid| {
                    if let Ok(mut state) = self.inner.state.lock() {
                        state.game_pid = Some(pid);
                    }
                    self.control(LauncherEvent::GameStarted { id, pid });
                },
                move |line, engine_state| {
                    publish_game_line(&tx, id, line, engine_state, &ready_sent);
                },
            )?;
            self.control(LauncherEvent::GameExited { id, code });
            if code != Some(0) {
                return Err(error(
                    ErrorCode::Process,
                    "Minecraft exited unsuccessfully. Inspect the launcher's redacted game log and repair if needed.",
                ));
            }
        }
        Ok(())
    }
}
fn publish_game_line(
    events: &SyncSender<LauncherEvent>,
    id: OperationId,
    line: String,
    engine_state: Option<EngineState>,
    ready_sent: &AtomicBool,
) {
    let _ = events.try_send(LauncherEvent::LogLine { id, line });
    if let Some(engine_state) = engine_state
        && !ready_sent.swap(true, Ordering::Relaxed)
    {
        let _ = events.send(LauncherEvent::GameReady { id, engine_state });
    }
}

fn validate_settings(settings: &Settings) -> Result<()> {
    if !(1024..=16384).contains(&settings.memory_mib) {
        return Err(error(
            ErrorCode::Configuration,
            "Choose between 1024 and 16384 MiB of game memory.",
        ));
    }
    util::absolute(&settings.instance_dir)?;
    if let Some(java) = &settings.java_path {
        util::absolute(java)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ready_event_survives_a_full_log_queue_and_is_emitted_once() {
        let (tx, rx) = mpsc::sync_channel(1);
        let id = OperationId(7);
        tx.send(LauncherEvent::Started { id }).unwrap();
        let sent = Arc::new(AtomicBool::new(false));
        let worker_flag = sent.clone();
        let worker = std::thread::spawn(move || {
            for _ in 0..2 {
                publish_game_line(
                    &tx,
                    id,
                    "ready".into(),
                    Some(EngineState::Loaded),
                    &worker_flag,
                );
            }
        });
        // Wait for the worker to attempt readiness with no spare queue capacity.
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while !sent.load(Ordering::Relaxed) {
            assert!(
                std::time::Instant::now() < deadline,
                "worker did not reach the control send"
            );
            std::thread::yield_now();
        }
        let ready: Vec<_> = rx.into_iter().filter(|event| {
            matches!(event, LauncherEvent::GameReady { id: event_id, engine_state: EngineState::Loaded } if *event_id == id)
        }).collect();
        worker.join().unwrap();
        assert_eq!(ready.len(), 1);
    }
    #[test]
    #[cfg(windows)]
    fn repeated_play_busy_and_cancel_are_bounded() {
        let dir = tempfile::tempdir().unwrap();
        let launcher = Launcher::open(LauncherConfig {
            data_dir: dir.path().to_owned(),
            release_manifest: dir.path().join("release.json"),
            microsoft_client_id: None,
        })
        .unwrap();
        let flag = Arc::new(AtomicBool::new(false));
        launcher.inner.state.lock().unwrap().active = Some(Active {
            id: OperationId(42),
            cancel: flag.clone(),
            game_committed: false,
        });
        assert_eq!(
            launcher.submit(LauncherCommand::Launch).unwrap_err().code,
            ErrorCode::Busy
        );
        assert!(!launcher.cancel(OperationId(41)));
        assert!(launcher.cancel(OperationId(42)));
        assert!(flag.load(Ordering::Acquire));
        launcher
            .inner
            .state
            .lock()
            .unwrap()
            .active
            .as_mut()
            .unwrap()
            .game_committed = true;
        assert!(!launcher.cancel(OperationId(42)));
    }
    #[test]
    fn invalid_memory_and_relative_paths() {
        let settings = Settings {
            instance_dir: std::path::Path::new("relative").to_owned(),
            java_path: None,
            memory_mib: 4096,
        };
        assert!(validate_settings(&settings).is_err());
    }
    #[test]
    #[cfg(windows)]
    fn missing_client_id_and_settings_restart() {
        let dir = tempfile::tempdir().unwrap();
        let config = LauncherConfig {
            data_dir: dir.path().to_owned(),
            release_manifest: dir.path().join("release.json"),
            microsoft_client_id: None,
        };
        let launcher = Launcher::open(config.clone()).unwrap();
        let id = launcher.submit(LauncherCommand::SignIn).unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            if let Some(LauncherEvent::Failed { id: event, error }) = launcher.try_event() {
                assert_eq!(event, id);
                assert_eq!(error.code, ErrorCode::Configuration);
                break;
            }
            assert!(std::time::Instant::now() < deadline);
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        drop(launcher);
        let launcher = Launcher::open(config).unwrap();
        assert!(launcher.snapshot().account.is_none());
    }
}
