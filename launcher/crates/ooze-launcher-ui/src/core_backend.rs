//! Adapter from Sol's `ooze-launcher-core` (LAUNCHER_API_V1, API 1) to the interface's view.
//!
//! Contract facts this adapter relies on (docs/launcher/LAUNCHER_API_V1.md):
//! - One operation at a time; a running game keeps its Launch operation active until it exits.
//! - `Started` → progress/account/process events → exactly one terminal `Completed` /
//!   `Cancelled` / `Failed`. `GameStarted` means Java spawned; `GameExited` precedes the terminal
//!   event (`Failed { Process }` on a nonzero exit).
//! - Progress and log lines are best effort; there is no waker, so the window polls.
//!
//! Mapping decisions (all covered by tests below):
//! - The interface ends its Launch operation at `GameStarted` and follows the game as a session;
//!   the backend's later terminal event for that operation is absorbed, and a nonzero exit is
//!   reported once, as a crash, not again as "couldn't start".
//! - "Reached the title screen" is shown only after a game log line containing `OOZE_READY v1`.
//! - Interface-only preferences (motion, interface size, window behaviour, first-run completion)
//!   are stored by this adapter in `<data_dir>/ui-preferences.json`; the backend's settings file
//!   is never touched directly.

use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};

use ooze_launcher_core as lc;
use serde::{Deserialize, Serialize};

use crate::api::*;

/// Interface-only preferences. Never contains account data.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UiPrefs {
    pub schema: u32,
    pub setup_complete: bool,
    pub motion: String,
    pub ui_scale_percent: u16,
    pub while_playing: String,
}

impl Default for UiPrefs {
    fn default() -> Self {
        Self {
            schema: 1,
            setup_complete: false,
            motion: "full".into(),
            ui_scale_percent: 100,
            while_playing: "keep_open".into(),
        }
    }
}

impl UiPrefs {
    fn motion(&self) -> MotionPreference {
        match self.motion.as_str() {
            "reduced" => MotionPreference::Reduced,
            "off" => MotionPreference::Off,
            _ => MotionPreference::Full,
        }
    }

    fn while_playing(&self) -> WhilePlaying {
        if self.while_playing == "minimize" {
            WhilePlaying::Minimize
        } else {
            WhilePlaying::KeepOpen
        }
    }

    fn apply(&mut self, s: &Settings) {
        self.motion = match s.motion {
            MotionPreference::Reduced => "reduced",
            MotionPreference::Off => "off",
            MotionPreference::Full | MotionPreference::FollowSystem => "full",
        }
        .into();
        self.ui_scale_percent = s.ui_scale_percent.clamp(100, 200);
        self.while_playing = match s.while_playing {
            WhilePlaying::Minimize => "minimize",
            WhilePlaying::KeepOpen => "keep_open",
        }
        .into();
    }

    pub fn load(path: &Path) -> Self {
        std::fs::read(path)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<UiPrefs>(&bytes).ok())
            .filter(|p| p.schema == 1)
            .unwrap_or_default()
    }

    /// Atomic replace: write a sibling temp file, flush, rename over the old file.
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        use std::io::Write;
        let tmp = path.with_extension("json.tmp");
        {
            let mut f = std::fs::File::create(&tmp)?;
            f.write_all(&serde_json::to_vec_pretty(self).map_err(std::io::Error::other)?)?;
            f.sync_all()?;
        }
        std::fs::rename(&tmp, path)
    }
}

/// True only for the Ooze mod's own readiness line, logged on the render thread as the whole
/// message: `[…] [Render thread/INFO]: OOZE_READY v1 screen=title engine=<STATE>` (optionally with
/// Fabric's `(logger)` tag, and the backend's `[stdout]` prefix). Chat lines (`[CHAT] …`), other
/// threads, and text that merely contains the marker never count, so a server message or another
/// mod cannot make the launcher claim the title screen was reached.
pub fn is_ready_marker(line: &str) -> bool {
    const THREAD: &str = "[Render thread/INFO]";
    const MARKER: &str = "OOZE_READY v1 screen=title engine=";
    let Some(at) = line.find(THREAD) else {
        return false;
    };
    let mut rest = line[at + THREAD.len()..].trim_start();
    if let Some(r) = rest.strip_prefix(':') {
        rest = r.trim_start();
    }
    if rest.starts_with('(')
        && let Some(close) = rest.find(')')
    {
        let tag = &rest[1..close];
        if tag.is_empty()
            || !tag
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
        {
            return false;
        }
        rest = rest[close + 1..].trim_start();
        if let Some(r) = rest.strip_prefix(':') {
            rest = r.trim_start();
        }
    }
    let Some(state) = rest.strip_prefix(MARKER) else {
        return false;
    };
    let state = state.trim_end();
    !state.is_empty() && state.bytes().all(|b| b.is_ascii_uppercase() || b == b'_')
}

pub const MEMORY: MemoryBounds = MemoryBounds {
    min_mib: 1024,
    max_mib: 16384,
    recommended_mib: 4096,
};

pub fn capabilities() -> Capabilities {
    Capabilities {
        reopen_browser: false,
        check_java: false,
        export_diagnostics: false,
        open_folders: cfg!(windows),
        install_status: false,
        concurrent_operations: false,
    }
}

/// Pure translation state, independent of the live `Launcher` so it can be tested anywhere.
#[derive(Debug, Default)]
pub struct Mapper {
    kinds: HashMap<u64, OperationKind>,
    /// Backend operations whose Launch already ended for the interface (game started).
    in_game: HashMap<u64, GameSessionId>,
    next_session: u64,
    title_screen_seen: bool,
    /// Pending interface-side follow-up when an operation completes.
    pending_settings: HashMap<u64, Settings>,
}

impl Mapper {
    pub fn track(&mut self, id: lc::OperationId, kind: OperationKind) {
        self.kinds.insert(id.0, kind);
    }

    pub fn track_settings(&mut self, id: lc::OperationId, settings: Settings) {
        self.pending_settings.insert(id.0, settings);
    }

    pub fn active(&self) -> bool {
        !self.kinds.is_empty()
    }

    pub fn game_session(&self) -> Option<GameSessionId> {
        self.in_game.values().next().copied()
    }

    /// Translate one backend event. `on_settings_saved` receives settings whose save completed.
    pub fn map(
        &mut self,
        event: lc::LauncherEvent,
        crash_reports: Option<PathBuf>,
        out: &mut Vec<Event>,
    ) -> Option<Settings> {
        use lc::LauncherEvent as E;
        match event {
            E::Started { id } => {
                if let Some(kind) = self.kinds.get(&id.0) {
                    out.push(Event::Started {
                        op: OperationId(id.0),
                        kind: *kind,
                    });
                }
            }
            E::Progress {
                id,
                stage,
                completed,
                total,
            } => {
                let kind = self.kinds.get(&id.0).copied()?;
                if self.in_game.contains_key(&id.0) {
                    return None;
                }
                let (step, unit) = map_stage(stage, kind);
                out.push(Event::Progress {
                    op: OperationId(id.0),
                    progress: Progress {
                        step,
                        done: completed,
                        total,
                        unit,
                    },
                });
            }
            E::AccountChanged { account, .. } => {
                out.push(Event::AccountChanged(account.map(|a| AccountSummary {
                    username: a.username,
                    uuid: a.uuid,
                })));
            }
            E::GameStarted { id, .. } => {
                self.next_session += 1;
                let session = GameSessionId(self.next_session);
                self.in_game.insert(id.0, session);
                self.title_screen_seen = false;
                out.push(Event::InstallChanged(InstallStatus::Ready));
                out.push(Event::Finished {
                    op: OperationId(id.0),
                    outcome: Outcome::Succeeded(Payload::None),
                });
                out.push(Event::Game {
                    session,
                    state: GameState::Running {
                        title_screen: false,
                    },
                });
            }
            E::LogLine { id, line } => {
                let session = self.in_game.get(&id.0).copied();
                let source = if session.is_some() {
                    LogSource::Game
                } else {
                    LogSource::Launcher
                };
                let level = if line.contains("/ERROR]") || line.contains("/FATAL]") {
                    LogLevel::Error
                } else if line.contains("/WARN]") {
                    LogLevel::Warn
                } else {
                    LogLevel::Info
                };
                let ready = session.is_some() && !self.title_screen_seen && is_ready_marker(&line);
                out.push(Event::Log(vec![LogLine {
                    level,
                    source,
                    text: line,
                }]));
                if let (true, Some(session)) = (ready, session) {
                    self.title_screen_seen = true;
                    out.push(Event::Game {
                        session,
                        state: GameState::Running { title_screen: true },
                    });
                }
            }
            E::GameExited { id, code } => {
                if let Some(session) = self.in_game.get(&id.0).copied() {
                    let state = if code == Some(0) {
                        GameState::Exited { code: 0 }
                    } else {
                        GameState::Crashed {
                            code,
                            crash_report: crash_reports,
                        }
                    };
                    out.push(Event::Game { session, state });
                }
            }
            E::Completed { id } => return self.finish(id, Outcome::Succeeded(Payload::None), out),
            E::Cancelled { id } => return self.finish(id, Outcome::Cancelled, out),
            E::Failed { id, error } => {
                let kind = self.kinds.get(&id.0).copied();
                let outcome = Outcome::Failed(map_error(&error, kind));
                if matches!(error.code, lc::ErrorCode::Integrity) {
                    out.push(Event::InstallChanged(InstallStatus::NeedsRepair));
                }
                return self.finish(id, outcome, out);
            }
        }
        None
    }

    fn finish(
        &mut self,
        id: lc::OperationId,
        outcome: Outcome,
        out: &mut Vec<Event>,
    ) -> Option<Settings> {
        let kind = self.kinds.remove(&id.0)?;
        let saved = self.pending_settings.remove(&id.0);
        if self.in_game.remove(&id.0).is_some() {
            // The interface already ended this Launch at GameStarted; the game's end was
            // reported by GameExited. Absorb the terminal event (including Failed{Process}).
            return None;
        }
        let succeeded = matches!(outcome, Outcome::Succeeded(_));
        if succeeded && matches!(kind, OperationKind::Prepare { .. }) {
            out.push(Event::InstallChanged(InstallStatus::Ready));
        }
        out.push(Event::Finished {
            op: OperationId(id.0),
            outcome,
        });
        if succeeded { saved } else { None }
    }
}

fn map_stage(stage: lc::Stage, kind: OperationKind) -> (Step, ProgressUnit) {
    use lc::Stage as S;
    match stage {
        S::AwaitingBrowser => (Step::WaitingForBrowser, ProgressUnit::Steps),
        S::Authorizing if kind == OperationKind::Launch => {
            (Step::RefreshingSession, ProgressUnit::Steps)
        }
        S::Authorizing => (Step::Authorizing, ProgressUnit::Steps),
        S::Resolving => (Step::Resolving, ProgressUnit::Steps),
        S::Downloading => (Step::Downloading, ProgressUnit::Files),
        S::Verifying => (Step::Verifying, ProgressUnit::Files),
        S::Extracting => (Step::ExtractingJava, ProgressUnit::Steps),
        S::ValidatingJava => (Step::CheckingJava, ProgressUnit::Steps),
        S::StartingGame => (Step::StartingJava, ProgressUnit::Steps),
    }
}

/// Typed codes stay typed; the backend's safe message is kept as the detail.
pub fn map_error(error: &lc::LauncherError, during: Option<OperationKind>) -> LauncherError {
    use lc::ErrorCode as C;
    let kind = match error.code {
        C::UnsupportedPlatform => ErrorKind::UnsupportedPlatform,
        // During sign-in the only configuration input is the owned client ID.
        C::Configuration if during == Some(OperationKind::SignIn) => ErrorKind::AuthNotConfigured,
        C::Configuration => ErrorKind::Configuration,
        C::Busy => ErrorKind::Busy,
        C::Cancelled => ErrorKind::Busy,
        C::Io => ErrorKind::Io,
        C::Network => ErrorKind::Network,
        // API 1 codes are generic: an Authentication failure at Play may follow a successful
        // Microsoft refresh (for example an Xbox denial), and a Minecraft HTTP 403 does not
        // establish why. Neutral kinds only; the backend's safe stage/status stays visible.
        // Approval- or expiry-specific claims wait for explicit backend subtypes (Sol, 2026-11-16).
        C::Authentication => ErrorKind::Authentication,
        C::ServiceAccess => ErrorKind::ServiceDenied,
        C::Entitlement => ErrorKind::NotEntitled,
        C::Integrity => ErrorKind::ChecksumMismatch,
        C::Metadata => ErrorKind::Metadata,
        C::JavaRuntime => ErrorKind::JavaRuntime,
        C::Process => ErrorKind::ProcessStartFailed,
        C::CredentialStore => ErrorKind::CredentialStore,
        C::Internal => ErrorKind::Internal,
    };
    LauncherError {
        kind,
        detail: error.message.clone(),
        retryable: error.retryable,
    }
}

/// Profile versions from the release manifest the backend installs (display only).
pub fn read_profile(release_manifest: &Path) -> ProfileSummary {
    let value: serde_json::Value = std::fs::read(release_manifest)
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or(serde_json::Value::Null);
    let text = |k: &str| {
        value
            .get(k)
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_owned()
    };
    let fabric_api = value
        .pointer("/fabric_api/url")
        .and_then(|v| v.as_str())
        .and_then(|u| u.rsplit('/').nth(1))
        .unwrap_or_default()
        .to_owned();
    ProfileSummary {
        minecraft: Some(text("minecraft"))
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "1.21.11".into()),
        fabric_loader: text("loader"),
        fabric_api,
        ooze: String::new(),
        java_major: 21,
    }
}

/// The live backend.
pub struct CoreBackend {
    launcher: lc::Launcher,
    mapper: Mapper,
    prefs: UiPrefs,
    prefs_path: PathBuf,
    data_dir: PathBuf,
    profile: ProfileSummary,
    queue: VecDeque<Event>,
    scratch: Vec<Event>,
    next_local: u64,
}

impl CoreBackend {
    /// Open Sol's backend. On failure the interface shows a startup error page instead.
    pub fn open(config: lc::LauncherConfig) -> Result<Self, LauncherError> {
        let data_dir = config.data_dir.clone();
        let profile = read_profile(&config.release_manifest);
        let launcher = lc::Launcher::open(config).map_err(|e| map_error(&e, None))?;
        let prefs_path = data_dir.join("ui-preferences.json");
        let prefs = UiPrefs::load(&prefs_path);
        let mut this = Self {
            launcher,
            mapper: Mapper::default(),
            prefs,
            prefs_path,
            data_dir,
            profile,
            queue: VecDeque::new(),
            scratch: Vec::new(),
            next_local: 1 << 62,
        };
        let snapshot = this.snapshot();
        this.queue.push_back(Event::Snapshot(Box::new(snapshot)));
        Ok(this)
    }

    fn settings(&self) -> Settings {
        let s = self.launcher.snapshot().settings;
        Settings {
            install_root: self.prefs.setup_complete.then(|| s.instance_dir.clone()),
            java: s.java_path.map_or(JavaChoice::Managed, JavaChoice::Custom),
            memory_mib: s.memory_mib,
            while_playing: self.prefs.while_playing(),
            motion: self.prefs.motion(),
            ui_scale_percent: self.prefs.ui_scale_percent,
        }
    }

    fn snapshot(&self) -> Snapshot {
        let s = self.launcher.snapshot();
        Snapshot {
            system: SystemInfo {
                capabilities: capabilities(),
                launcher_version: env!("CARGO_PKG_VERSION").to_owned(),
                api_version: (u16::try_from(s.api_version).unwrap_or(u16::MAX), 0),
                default_install_root: s.settings.instance_dir.clone(),
                memory: MEMORY,
                system_reduced_motion: None,
            },
            profile: self.profile.clone(),
            settings: self.settings(),
            account: s.account.map(|a| AccountSummary {
                username: a.username,
                uuid: a.uuid,
            }),
            install: InstallStatus::Unknown,
            game: None,
            running: Vec::new(),
        }
    }

    fn crash_reports(&self) -> Option<PathBuf> {
        let dir = self
            .launcher
            .snapshot()
            .settings
            .instance_dir
            .join("crash-reports");
        dir.is_dir().then_some(dir)
    }

    fn local_op(&mut self, kind: OperationKind, outcome: Outcome) -> OperationId {
        let op = OperationId(self.next_local);
        self.next_local += 1;
        self.queue.push_back(Event::Started { op, kind });
        self.queue.push_back(Event::Finished { op, outcome });
        op
    }

    fn submit_core(
        &mut self,
        command: lc::LauncherCommand,
        kind: OperationKind,
    ) -> Result<lc::OperationId, LauncherError> {
        let id = self
            .launcher
            .submit(command)
            .map_err(|e| map_error(&e, Some(kind)))?;
        self.mapper.track(id, kind);
        Ok(id)
    }

    fn save_prefs(&mut self) -> Result<(), LauncherError> {
        self.prefs
            .save(&self.prefs_path)
            .map_err(|_| LauncherError {
                kind: ErrorKind::Io,
                detail: "Couldn't save the interface preferences file.".into(),
                retryable: true,
            })
    }

    fn open_folder(&mut self, folder: Folder) -> Outcome {
        let instance = self.launcher.snapshot().settings.instance_dir;
        let path = match folder {
            Folder::InstallRoot => instance,
            Folder::CrashReports => instance.join("crash-reports"),
            Folder::Logs => self.data_dir.join("logs"),
            Folder::Diagnostics => self.data_dir.clone(),
        };
        if !path.is_dir() {
            return Outcome::Failed(LauncherError {
                kind: ErrorKind::Io,
                detail:
                    "That folder doesn't exist yet. It is created the first time Ooze needs it."
                        .into(),
                retryable: false,
            });
        }
        #[cfg(windows)]
        let opened = std::process::Command::new("explorer.exe")
            .arg(&path)
            .spawn()
            .is_ok();
        #[cfg(not(windows))]
        let opened = false;
        if opened {
            Outcome::Succeeded(Payload::None)
        } else {
            Outcome::Failed(LauncherError {
                kind: ErrorKind::Io,
                detail: "Windows couldn't open the folder.".into(),
                retryable: false,
            })
        }
    }
}

impl Backend for CoreBackend {
    fn api_version(&self) -> (u16, u16) {
        (
            u16::try_from(lc::LAUNCHER_API_VERSION).unwrap_or(u16::MAX),
            0,
        )
    }

    fn set_waker(&mut self, _waker: Waker) {
        // API 1 has no notification hook; see `wants_polling`.
    }

    fn wants_polling(&self) -> bool {
        self.mapper.active()
    }

    fn submit(&mut self, command: Command) -> Result<OperationId, LauncherError> {
        use lc::LauncherCommand as C;
        let id = match command {
            Command::SignIn { .. } => self.submit_core(C::SignIn, OperationKind::SignIn)?,
            Command::SignOut => self.submit_core(C::SignOut, OperationKind::SignOut)?,
            Command::Prepare { repair: false } => {
                self.submit_core(C::Install, OperationKind::Prepare { repair: false })?
            }
            Command::Prepare { repair: true } => {
                self.submit_core(C::Repair, OperationKind::Prepare { repair: true })?
            }
            Command::Play => self.submit_core(C::Launch, OperationKind::Launch)?,
            Command::Cancel { op } => {
                return if self.launcher.cancel(lc::OperationId(op.0)) {
                    Ok(op)
                } else {
                    Err(LauncherError {
                        kind: ErrorKind::Busy,
                        detail: "This step can no longer be cancelled.".into(),
                        retryable: false,
                    })
                };
            }
            Command::SetInstallRoot { path } => {
                let mut s = self.launcher.snapshot().settings;
                s.instance_dir = path;
                let id = self.submit_core(C::SaveSettings(s), OperationKind::SetInstallRoot)?;
                let mut ui = self.settings();
                ui.install_root = Some(PathBuf::new());
                self.mapper.track_settings(id, ui);
                id
            }
            Command::UpdateSettings(wanted) => {
                let current = self.launcher.snapshot().settings;
                let core_settings = lc::Settings {
                    instance_dir: wanted
                        .install_root
                        .clone()
                        .unwrap_or_else(|| current.instance_dir.clone()),
                    java_path: match &wanted.java {
                        JavaChoice::Managed => None,
                        JavaChoice::Custom(p) => Some(p.clone()),
                    },
                    memory_mib: wanted.memory_mib,
                };
                self.prefs.apply(&wanted);
                self.save_prefs()?;
                if core_settings == current {
                    let op = self.local_op(
                        OperationKind::UpdateSettings,
                        Outcome::Succeeded(Payload::None),
                    );
                    let settings = self.settings();
                    self.queue.push_back(Event::SettingsChanged(settings));
                    return Ok(op);
                }
                let id = self.submit_core(
                    C::SaveSettings(core_settings),
                    OperationKind::UpdateSettings,
                )?;
                self.mapper.track_settings(id, wanted);
                id
            }
            Command::OpenFolder(folder) => {
                let outcome = self.open_folder(folder);
                return Ok(self.local_op(OperationKind::OpenFolder, outcome));
            }
            Command::ReopenSignInBrowser { .. }
            | Command::CheckJava { .. }
            | Command::ExportDiagnostics => {
                return Err(LauncherError {
                    kind: ErrorKind::Internal,
                    detail: "Not offered by this backend.".into(),
                    retryable: false,
                });
            }
        };
        Ok(OperationId(id.0))
    }

    fn drain_events(&mut self, out: &mut Vec<Event>) {
        out.extend(self.queue.drain(..));
        while let Some(event) = self.launcher.try_event() {
            let crash = matches!(event, lc::LauncherEvent::GameExited { .. })
                .then(|| self.crash_reports())
                .flatten();
            self.scratch.clear();
            let saved = self.mapper.map(event, crash, &mut self.scratch);
            out.append(&mut self.scratch);
            if let Some(settings) = saved {
                if settings.install_root.is_some() && !self.prefs.setup_complete {
                    self.prefs.setup_complete = true;
                    if let Err(e) = self.save_prefs() {
                        out.push(Event::Log(vec![LogLine {
                            level: LogLevel::Warn,
                            source: LogSource::Launcher,
                            text: e.detail,
                        }]));
                    }
                }
                out.push(Event::SettingsChanged(self.settings()));
            }
        }
    }
}

/// Stands in when the backend could not start; the interface shows only the error.
pub struct FailedBackend {
    pub error: LauncherError,
}

impl Backend for FailedBackend {
    fn api_version(&self) -> (u16, u16) {
        (API_MAJOR, 0)
    }
    fn startup_error(&self) -> Option<LauncherError> {
        Some(self.error.clone())
    }
    fn set_waker(&mut self, _waker: Waker) {}
    fn submit(&mut self, _command: Command) -> Result<OperationId, LauncherError> {
        Err(self.error.clone())
    }
    fn drain_events(&mut self, _out: &mut Vec<Event>) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use lc::{ErrorCode, LauncherEvent as E, OperationId as Id, Stage};

    fn err(code: ErrorCode) -> lc::LauncherError {
        lc::LauncherError {
            code,
            message: "safe message".into(),
            retryable: false,
        }
    }

    /// Sol's review (L0): saving must replace an existing file on Windows too, where a rename
    /// onto an existing name has different semantics. CI runs this on windows-latest.
    #[test]
    fn prefs_save_replaces_an_existing_file_and_leaves_no_temp_file() {
        let dir = std::env::temp_dir().join(format!(
            "ooze-ui-prefs-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_nanos())
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("launcher-ui.json");
        let mut prefs = UiPrefs::default();
        prefs.save(&path).unwrap();
        prefs.setup_complete = true;
        prefs.ui_scale_percent = 150;
        prefs.save(&path).unwrap();
        let again = UiPrefs::load(&path);
        assert!(again.setup_complete);
        assert_eq!(again.ui_scale_percent, 150);
        assert!(!path.with_extension("json.tmp").exists());
        // A leftover temp file from an interrupted save does not block the next save.
        std::fs::write(path.with_extension("json.tmp"), b"partial").unwrap();
        prefs.ui_scale_percent = 175;
        prefs.save(&path).unwrap();
        assert_eq!(UiPrefs::load(&path).ui_scale_percent, 175);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn readiness_requires_the_mods_own_render_thread_line() {
        for ok in [
            "[stdout] [12:00:02] [Render thread/INFO]: OOZE_READY v1 screen=title engine=LOADED",
            "[12:00:02] [Render thread/INFO]: OOZE_READY v1 screen=title engine=UNSUPPORTED_PLATFORM",
            "[12:00:02] [Render thread/INFO] (oozeclient) OOZE_READY v1 screen=title engine=MISSING_LIBRARY",
            "[stdout] [12:00:02] [Render thread/INFO]: OOZE_READY v1 screen=title engine=LOADED\r",
        ] {
            assert!(is_ready_marker(ok), "{ok}");
        }
        for bad in [
            // A server or player chat message (also how a Vanilla instance could be fooled).
            "[12:01:00] [Render thread/INFO]: [CHAT] <Steve> OOZE_READY v1 screen=title engine=LOADED",
            "[12:01:00] [Render thread/INFO]: [System] [CHAT] OOZE_READY v1 screen=title engine=LOADED",
            // Another thread or level, or the marker inside other text.
            "[12:00:02] [Worker-Main-1/INFO]: OOZE_READY v1 screen=title engine=LOADED",
            "[12:00:02] [Render thread/WARN]: OOZE_READY v1 screen=title engine=LOADED",
            "[12:00:02] [Render thread/INFO]: mod says OOZE_READY v1 screen=title engine=LOADED",
            "[12:00:02] [Render thread/INFO]: OOZE_READY v1 screen=title engine=loaded; rm",
            "[12:00:02] [Render thread/INFO]: OOZE_READY v1 screen=title engine=",
            "OOZE_READY v1 screen=title engine=LOADED",
        ] {
            assert!(!is_ready_marker(bad), "{bad}");
        }
    }

    fn run(m: &mut Mapper, events: Vec<E>) -> Vec<Event> {
        let mut out = Vec::new();
        for e in events {
            m.map(e, None, &mut out);
        }
        out
    }

    /// Gate L review: a chat message (or another mod) printing the marker must not make the
    /// launcher claim the title screen, for example in a Vanilla instance that has no Ooze mod.
    #[test]
    fn chat_text_never_marks_the_title_screen() {
        let mut m = Mapper::default();
        m.track(Id(5), OperationKind::Launch);
        let out = run(
            &mut m,
            vec![
                E::Started { id: Id(5) },
                E::GameStarted { id: Id(5), pid: 7 },
                E::LogLine {
                    id: Id(5),
                    line: "[stdout] [12:05:00] [Render thread/INFO]: [CHAT] <Steve> OOZE_READY v1 screen=title engine=LOADED".into(),
                },
                E::LogLine {
                    id: Id(5),
                    line: "[stdout] [12:05:01] [Render thread/INFO]: [System] [CHAT] OOZE_READY v1 screen=title engine=LOADED".into(),
                },
            ],
        );
        assert!(
            !out.iter().any(|e| matches!(
                e,
                Event::Game {
                    state: GameState::Running { title_screen: true },
                    ..
                }
            )),
            "{out:?}"
        );
    }

    #[test]
    fn launch_ends_at_spawn_and_the_game_is_followed_as_a_session() {
        let mut m = Mapper::default();
        m.track(Id(3), OperationKind::Launch);
        let out = run(&mut m, vec![
            E::Started { id: Id(3) },
            E::Progress { id: Id(3), stage: Stage::Authorizing, completed: 0, total: None },
            E::Progress { id: Id(3), stage: Stage::Verifying, completed: 5, total: Some(10) },
            E::GameStarted { id: Id(3), pid: 4242 },
            E::LogLine { id: Id(3), line: "[stdout] [12:00:01] [Render thread/WARN]: something".into() },
            E::LogLine { id: Id(3), line: "[stdout] [12:00:02] [Render thread/INFO]: OOZE_READY v1 screen=title engine=LOADED".into() },
            E::GameExited { id: Id(3), code: Some(-1) },
            E::Failed { id: Id(3), error: err(ErrorCode::Process) },
        ]);
        assert!(matches!(
            out[1],
            Event::Progress {
                progress: Progress {
                    step: Step::RefreshingSession,
                    ..
                },
                ..
            }
        ));
        assert!(matches!(
            out[2],
            Event::Progress {
                progress: Progress {
                    step: Step::Verifying,
                    done: 5,
                    total: Some(10),
                    unit: ProgressUnit::Files
                },
                ..
            }
        ));
        assert!(out.contains(&Event::Finished {
            op: OperationId(3),
            outcome: Outcome::Succeeded(Payload::None)
        }));
        assert!(out.contains(&Event::Game {
            session: GameSessionId(1),
            state: GameState::Running {
                title_screen: false
            }
        }));
        assert!(out.contains(&Event::Game {
            session: GameSessionId(1),
            state: GameState::Running { title_screen: true }
        }));
        assert!(out.iter().any(|e| matches!(e, Event::Log(l) if l[0].level == LogLevel::Warn && l[0].source == LogSource::Game)));
        assert!(out.contains(&Event::Game {
            session: GameSessionId(1),
            state: GameState::Crashed {
                code: Some(-1),
                crash_report: None
            }
        }));
        let finished = out
            .iter()
            .filter(|e| matches!(e, Event::Finished { .. }))
            .count();
        assert_eq!(
            finished, 1,
            "the backend's Failed{{Process}} after the exit is absorbed: {out:?}"
        );
        assert!(!m.active());
    }

    #[test]
    fn clean_exit_then_completed() {
        let mut m = Mapper::default();
        m.track(Id(1), OperationKind::Launch);
        let out = run(
            &mut m,
            vec![
                E::Started { id: Id(1) },
                E::GameStarted { id: Id(1), pid: 1 },
                E::GameExited {
                    id: Id(1),
                    code: Some(0),
                },
                E::Completed { id: Id(1) },
            ],
        );
        assert!(out.contains(&Event::Game {
            session: GameSessionId(1),
            state: GameState::Exited { code: 0 }
        }));
        assert_eq!(
            out.iter()
                .filter(|e| matches!(e, Event::Finished { .. }))
                .count(),
            1
        );
    }

    #[test]
    fn launch_failing_before_spawn_is_reported_with_typed_kind() {
        let mut m = Mapper::default();
        m.track(Id(2), OperationKind::Launch);
        let out = run(
            &mut m,
            vec![
                E::Started { id: Id(2) },
                E::Failed {
                    id: Id(2),
                    error: err(ErrorCode::Authentication),
                },
            ],
        );
        assert!(matches!(
            &out[1],
            Event::Finished {
                outcome: Outcome::Failed(LauncherError {
                    kind: ErrorKind::Authentication,
                    ..
                }),
                ..
            }
        ));
    }

    #[test]
    fn error_codes_map_by_operation() {
        let e = |c, k| map_error(&err(c), Some(k)).kind;
        assert_eq!(
            e(ErrorCode::Configuration, OperationKind::SignIn),
            ErrorKind::AuthNotConfigured
        );
        assert_eq!(
            e(ErrorCode::Configuration, OperationKind::UpdateSettings),
            ErrorKind::Configuration
        );
        assert_eq!(
            e(ErrorCode::Authentication, OperationKind::SignIn),
            ErrorKind::Authentication
        );
        assert_eq!(
            e(ErrorCode::ServiceAccess, OperationKind::SignIn),
            ErrorKind::ServiceDenied
        );
        assert_eq!(
            e(ErrorCode::ServiceAccess, OperationKind::Launch),
            ErrorKind::ServiceDenied
        );
        assert_eq!(
            e(ErrorCode::Authentication, OperationKind::Launch),
            ErrorKind::Authentication,
            "no session-expiry claim from a generic code"
        );
        assert_eq!(
            e(ErrorCode::Entitlement, OperationKind::SignIn),
            ErrorKind::NotEntitled
        );
        assert_eq!(
            e(
                ErrorCode::Integrity,
                OperationKind::Prepare { repair: false }
            ),
            ErrorKind::ChecksumMismatch
        );
        assert_eq!(
            map_error(&err(ErrorCode::Network), None).detail,
            "safe message"
        );
    }

    #[test]
    fn install_completion_reports_ready_and_integrity_failure_needs_repair() {
        let mut m = Mapper::default();
        m.track(Id(4), OperationKind::Prepare { repair: false });
        let out = run(
            &mut m,
            vec![
                E::Started { id: Id(4) },
                E::Progress {
                    id: Id(4),
                    stage: Stage::Downloading,
                    completed: 1,
                    total: Some(9),
                },
                E::Completed { id: Id(4) },
            ],
        );
        assert!(out.contains(&Event::InstallChanged(InstallStatus::Ready)));
        m.track(Id(5), OperationKind::Prepare { repair: true });
        let out = run(
            &mut m,
            vec![E::Failed {
                id: Id(5),
                error: err(ErrorCode::Integrity),
            }],
        );
        assert!(out.contains(&Event::InstallChanged(InstallStatus::NeedsRepair)));
    }

    #[test]
    fn untracked_and_repeated_terminal_events_are_ignored() {
        let mut m = Mapper::default();
        m.track(Id(6), OperationKind::SignIn);
        let out = run(
            &mut m,
            vec![
                E::Cancelled { id: Id(6) },
                E::Cancelled { id: Id(6) },
                E::Completed { id: Id(99) },
                E::Progress {
                    id: Id(99),
                    stage: Stage::Resolving,
                    completed: 0,
                    total: None,
                },
            ],
        );
        assert_eq!(
            out,
            vec![Event::Finished {
                op: OperationId(6),
                outcome: Outcome::Cancelled
            }]
        );
    }

    #[test]
    fn the_title_marker_counts_only_for_game_output_and_once() {
        let mut m = Mapper::default();
        m.track(Id(7), OperationKind::Prepare { repair: false });
        let out = run(
            &mut m,
            vec![E::LogLine {
                id: Id(7),
                line: "OOZE_READY v1 (not from a game)".into(),
            }],
        );
        assert!(!out.iter().any(|e| matches!(e, Event::Game { .. })));
    }

    #[test]
    fn settings_saves_report_back_only_on_success() {
        let mut m = Mapper::default();
        m.track(Id(8), OperationKind::UpdateSettings);
        let s = crate::fixtures::snapshot().settings;
        m.track_settings(Id(8), s.clone());
        let mut out = Vec::new();
        assert_eq!(
            m.map(E::Completed { id: Id(8) }, None, &mut out),
            Some(s.clone())
        );
        m.track(Id(9), OperationKind::UpdateSettings);
        m.track_settings(Id(9), s);
        assert_eq!(
            m.map(
                E::Failed {
                    id: Id(9),
                    error: err(ErrorCode::Configuration)
                },
                None,
                &mut out
            ),
            None
        );
    }

    #[test]
    fn ui_prefs_round_trip_and_reject_unknown_schema() {
        let dir = std::env::temp_dir().join(format!("ooze-ui-prefs-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("ui-preferences.json");
        let p = UiPrefs {
            setup_complete: true,
            motion: "off".into(),
            ..UiPrefs::default()
        };
        p.save(&path).unwrap();
        assert_eq!(UiPrefs::load(&path), p);
        std::fs::write(&path, br#"{"schema":2}"#).unwrap();
        assert_eq!(UiPrefs::load(&path), UiPrefs::default());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn profile_is_read_from_the_release_manifest() {
        let dir = std::env::temp_dir().join(format!("ooze-ui-release-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("release.json");
        std::fs::write(&path, br#"{"minecraft":"1.21.11","loader":"0.20.1","fabric_api":{"url":"https://maven.fabricmc.net/net/fabricmc/fabric-api/fabric-api/0.141.6+1.21.11/fabric-api-0.141.6+1.21.11.jar"}}"#).unwrap();
        let p = read_profile(&path);
        assert_eq!(
            (
                p.minecraft.as_str(),
                p.fabric_loader.as_str(),
                p.fabric_api.as_str()
            ),
            ("1.21.11", "0.20.1", "0.141.6+1.21.11")
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The real core refuses to open off Windows; the interface must show that, not crash.
    #[test]
    #[cfg(not(windows))]
    fn real_backend_refuses_non_windows_with_a_typed_error() {
        let dir = std::env::temp_dir();
        let r = CoreBackend::open(lc::LauncherConfig {
            data_dir: dir.clone(),
            release_manifest: dir.join("release.json"),
            microsoft_client_id: None,
        });
        assert_eq!(
            r.err().map(|e| e.kind),
            Some(ErrorKind::UnsupportedPlatform)
        );
    }
}
