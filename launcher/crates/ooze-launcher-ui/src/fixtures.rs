//! Test and developer-preview fixtures. Compiled only for tests or with the `dev-fixtures`
//! feature, which release builds refuse (see `lib.rs`). Every fixture screen carries the
//! "Developer preview" banner; fixture accounts and progress are never shown in a product build.

use std::collections::{HashSet, VecDeque};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use crate::api::*;

pub fn account() -> AccountSummary {
    AccountSummary {
        username: "FixturePlayer".into(),
        uuid: "00000000-0000-4000-8000-0000000000f1".into(),
    }
}

pub fn snapshot() -> Snapshot {
    Snapshot {
        system: SystemInfo {
            capabilities: Capabilities::ALL,
            launcher_version: "0.1.0-l0 (fixture)".into(),
            api_version: (API_MAJOR, 0),
            default_install_root: PathBuf::from(r"C:\Users\Player\AppData\Roaming\.ooze"),
            memory: MemoryBounds {
                min_mib: 2048,
                max_mib: 12288,
                recommended_mib: 4096,
            },
            system_reduced_motion: Some(false),
        },
        profile: ProfileSummary {
            minecraft: "1.21.11".into(),
            fabric_loader: "0.20.1".into(),
            fabric_api: "0.141.6+1.21.11".into(),
            ooze: "0.1.0-alpha.0".into(),
            java_major: 21,
        },
        settings: Settings {
            install_root: Some(PathBuf::from(r"C:\Users\Player\AppData\Roaming\.ooze")),
            java: JavaChoice::Managed,
            memory_mib: 4096,
            while_playing: WhilePlaying::KeepOpen,
            motion: MotionPreference::FollowSystem,
            ui_scale_percent: 100,
        },
        account: None,
        install: InstallStatus::NotInstalled,
        game: None,
        running: Vec::new(),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scenario {
    FirstRun,
    SignedOut,
    Ready,
    AuthNotConfigured,
    AppNotApproved,
    Offline,
    Crash,
}

impl Scenario {
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "first-run" => Self::FirstRun,
            "signed-out" => Self::SignedOut,
            "ready" => Self::Ready,
            "auth-not-configured" => Self::AuthNotConfigured,
            "app-not-approved" => Self::AppNotApproved,
            "offline" => Self::Offline,
            "crash" => Self::Crash,
            _ => return None,
        })
    }

    pub fn snapshot(self) -> Snapshot {
        let mut s = snapshot();
        match self {
            Self::FirstRun => s.settings.install_root = None,
            Self::SignedOut | Self::AuthNotConfigured | Self::AppNotApproved => {}
            Self::Ready | Self::Crash => {
                s.account = Some(account());
                s.install = InstallStatus::Ready;
            }
            Self::Offline => s.account = Some(account()),
        }
        s
    }
}

#[derive(Default)]
struct Inner {
    queue: VecDeque<Event>,
    submitted: Vec<Command>,
    next_op: u64,
    next_session: u64,
    waker: Option<Arc<Waker>>,
    cancelled: HashSet<OperationId>,
    reject_next: Option<LauncherError>,
    settings: Option<Settings>,
    game_live: bool,
}

/// A scripted backend. `Manual` only records commands (tests push events through the handle);
/// `Simulated` plays a timed, labelled demo of each flow for the developer preview.
pub struct FixtureBackend {
    inner: Arc<Mutex<Inner>>,
    scenario: Option<Scenario>,
}

#[derive(Clone)]
pub struct FixtureHandle {
    inner: Arc<Mutex<Inner>>,
}

fn lock(inner: &Mutex<Inner>) -> std::sync::MutexGuard<'_, Inner> {
    inner.lock().unwrap_or_else(PoisonError::into_inner)
}

impl FixtureHandle {
    pub fn push(&self, event: Event) {
        let mut g = lock(&self.inner);
        g.queue.push_back(event);
        if let Some(w) = g.waker.clone() {
            drop(g);
            w();
        }
    }

    pub fn submitted(&self) -> Vec<Command> {
        lock(&self.inner).submitted.clone()
    }

    pub fn reject_next(&self, error: LauncherError) {
        lock(&self.inner).reject_next = Some(error);
    }

    /// The id the next accepted command will get.
    pub fn next_op(&self) -> OperationId {
        OperationId(lock(&self.inner).next_op + 1)
    }
}

impl FixtureBackend {
    pub fn manual(snapshot: Snapshot) -> (Self, FixtureHandle) {
        let inner = Arc::new(Mutex::new(Inner::default()));
        lock(&inner)
            .queue
            .push_back(Event::Snapshot(Box::new(snapshot)));
        (
            Self {
                inner: inner.clone(),
                scenario: None,
            },
            FixtureHandle { inner },
        )
    }

    pub fn simulated(scenario: Scenario) -> Self {
        let snapshot = scenario.snapshot();
        let inner = Arc::new(Mutex::new(Inner {
            settings: Some(snapshot.settings.clone()),
            ..Inner::default()
        }));
        lock(&inner)
            .queue
            .push_back(Event::Snapshot(Box::new(snapshot)));
        Self {
            inner,
            scenario: Some(scenario),
        }
    }

    /// Queue `events` after `delay_ms`, unless operation `op` was cancelled meanwhile.
    fn later(&self, op: Option<OperationId>, delay_ms: u64, events: Vec<Event>) {
        let inner = self.inner.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(delay_ms));
            let mut g = lock(&inner);
            if op.is_some_and(|o| g.cancelled.contains(&o)) {
                return;
            }
            for e in events {
                if let Event::Game { state, .. } = &e {
                    g.game_live = state.is_live();
                }
                if let Event::SettingsChanged(s) = &e {
                    g.settings = Some(s.clone());
                }
                g.queue.push_back(e);
            }
            if let Some(w) = g.waker.clone() {
                drop(g);
                w();
            }
        });
    }

    fn simulate(&self, scenario: Scenario, op: OperationId, command: &Command) {
        let p = |step, done, total: Option<u64>, unit| Event::Progress {
            op,
            progress: Progress {
                step,
                done,
                total,
                unit,
            },
        };
        let ok = |payload| Event::Finished {
            op,
            outcome: Outcome::Succeeded(payload),
        };
        let fail = |kind, detail: &str, retryable| Event::Finished {
            op,
            outcome: Outcome::Failed(LauncherError {
                kind,
                detail: detail.into(),
                retryable,
            }),
        };
        let log = |text: &str| {
            Event::Log(vec![LogLine {
                level: LogLevel::Info,
                source: LogSource::Launcher,
                text: text.into(),
            }])
        };
        let s = Some(op);
        let kind = crate::state::kind_of(command);
        self.later(None, 0, vec![Event::Started { op, kind }]);
        match command {
            Command::SetInstallRoot { path } => {
                let mut settings = lock(&self.inner).settings.clone().unwrap_or_else(|| snapshot().settings);
                settings.install_root = Some(path.clone());
                self.later(s, 400, vec![Event::SettingsChanged(settings), Event::InstallChanged(InstallStatus::NotInstalled), ok(Payload::None)]);
            }
            Command::SignIn { .. } => {
                self.later(s, 50, vec![p(Step::WaitingForBrowser, 0, None, ProgressUnit::Steps), log("[fixture] opened the system browser (simulated)")]);
                match scenario {
                    Scenario::AuthNotConfigured => self.later(
                        s,
                        700,
                        vec![fail(ErrorKind::AuthNotConfigured, "fixture: this build has no Microsoft client ID", false)],
                    ),
                    Scenario::AppNotApproved => {
                        self.later(s, 2200, vec![p(Step::XboxLive, 1, Some(4), ProgressUnit::Steps)]);
                        self.later(s, 2800, vec![p(Step::MinecraftServices, 2, Some(4), ProgressUnit::Steps)]);
                        self.later(s, 3400, vec![fail(ErrorKind::AuthAppNotApproved, "fixture: HTTP 403 Invalid app registration", false)]);
                    }
                    _ => {
                        let steps = [Step::XboxLive, Step::MinecraftServices, Step::CheckingOwnership, Step::LoadingProfile];
                        for (i, step) in steps.into_iter().enumerate() {
                            self.later(s, 2500 + 500 * i as u64, vec![p(step, i as u64 + 1, Some(4), ProgressUnit::Steps)]);
                        }
                        self.later(s, 4600, vec![Event::AccountChanged(Some(account())), ok(Payload::None)]);
                    }
                }
            }
            Command::SignOut => self.later(s, 200, vec![Event::AccountChanged(None), ok(Payload::None)]),
            Command::Prepare { repair } => {
                let phases: [(Step, u64, ProgressUnit); 7] = [
                    (Step::DownloadingRuntime, 52_428_800, ProgressUnit::Bytes),
                    (Step::DownloadingGame, 29_360_128, ProgressUnit::Bytes),
                    (Step::DownloadingLibraries, 64, ProgressUnit::Files),
                    (Step::DownloadingAssets, 440_401_920, ProgressUnit::Bytes),
                    (Step::InstallingFabric, 3, ProgressUnit::Steps),
                    (Step::InstallingOoze, 2, ProgressUnit::Steps),
                    (Step::Verifying, 3_100, ProgressUnit::Files),
                ];
                let mut t = 100;
                if *repair {
                    self.later(s, t, vec![p(Step::CheckingFiles, 0, Some(3_100), ProgressUnit::Files)]);
                    t += 900;
                }
                for (step, total, unit) in phases {
                    for k in 0..=8u64 {
                        if scenario == Scenario::Offline && step == Step::DownloadingAssets && k == 4 {
                            self.later(s, t, vec![fail(ErrorKind::Offline, "fixture: resources.download.minecraft.net unreachable", true)]);
                            return;
                        }
                        self.later(s, t, vec![p(step, total * k / 8, Some(total), unit)]);
                        t += 140;
                    }
                }
                self.later(s, t, vec![Event::InstallChanged(InstallStatus::Ready), log("[fixture] profile verified"), ok(Payload::None)]);
            }
            Command::Play => {
                let session = {
                    let mut g = lock(&self.inner);
                    g.next_session += 1;
                    GameSessionId(g.next_session)
                };
                self.later(s, 100, vec![p(Step::RefreshingSession, 1, Some(3), ProgressUnit::Steps)]);
                self.later(s, 500, vec![
                    p(Step::StartingJava, 2, Some(3), ProgressUnit::Steps),
                    log("[fixture] starting java --username FixturePlayer --accessToken eyJmaXh0dXJlIjoxfQ.eyJub3QiOiJhLXRva2VuIn0.c2lnbmF0dXJlLW5vdC1yZWFs"),
                ]);
                self.later(s, 900, vec![ok(Payload::None), Event::Game { session, state: GameState::Starting }]);
                self.later(None, 2900, vec![Event::Game { session, state: GameState::Running { title_screen: false } }]);
                self.later(None, 5400, vec![
                    Event::Game { session, state: GameState::Running { title_screen: true } },
                    Event::Log(vec![LogLine { level: LogLevel::Info, source: LogSource::Game, text: "OOZE_READY v1 screen=title engine=LOADED (fixture)".into() }]),
                ]);
                let end = if scenario == Scenario::Crash {
                    GameState::Crashed { code: Some(-1), crash_report: Some(PathBuf::from(r"C:\Users\Player\AppData\Roaming\.ooze\instance\crash-reports\crash-fixture.txt")) }
                } else {
                    GameState::Exited { code: 0 }
                };
                self.later(None, if scenario == Scenario::Crash { 8000 } else { 15000 }, vec![Event::Game { session, state: end }]);
            }
            Command::UpdateSettings(settings) => self.later(s, 150, vec![Event::SettingsChanged(settings.clone()), ok(Payload::None)]),
            Command::CheckJava { path } => {
                let event = if path.to_string_lossy().contains("21") {
                    ok(Payload::Java(JavaInfo { version: "21.0.9".into(), vendor: "Fixture JDK".into(), path: path.clone() }))
                } else {
                    fail(ErrorKind::JavaWrongVersion, "fixture: found Java 17.0.12", false)
                };
                self.later(s, 500, vec![event]);
            }
            Command::ExportDiagnostics => self.later(
                s,
                300,
                vec![ok(Payload::Exported(PathBuf::from(r"C:\Users\Player\AppData\Roaming\.ooze\diagnostics\ooze-diagnostics-fixture.zip")))],
            ),
            Command::OpenFolder(_) => self.later(s, 50, vec![ok(Payload::None)]),
            Command::Cancel { .. } | Command::ReopenSignInBrowser { .. } => {}
        }
    }
}

impl Backend for FixtureBackend {
    fn api_version(&self) -> (u16, u16) {
        (API_MAJOR, 0)
    }

    fn set_waker(&mut self, waker: Waker) {
        lock(&self.inner).waker = Some(Arc::new(waker));
    }

    fn submit(&mut self, command: Command) -> Result<OperationId, LauncherError> {
        let op = {
            let mut g = lock(&self.inner);
            g.submitted.push(command.clone());
            if let Some(e) = g.reject_next.take() {
                return Err(e);
            }
            if command == Command::Play && g.game_live {
                return Err(LauncherError {
                    kind: ErrorKind::AlreadyRunning,
                    detail: "fixture".into(),
                    retryable: false,
                });
            }
            match &command {
                Command::Cancel { op } => {
                    g.cancelled.insert(*op);
                    *op
                }
                Command::ReopenSignInBrowser { op } => *op,
                _ => {
                    g.next_op += 1;
                    OperationId(g.next_op)
                }
            }
        };
        if let Some(scenario) = self.scenario {
            match &command {
                Command::Cancel { op } => {
                    let target = *op;
                    let inner = self.inner.clone();
                    std::thread::spawn(move || {
                        std::thread::sleep(Duration::from_millis(300));
                        let mut g = lock(&inner);
                        g.queue.push_back(Event::Finished {
                            op: target,
                            outcome: Outcome::Cancelled,
                        });
                        if let Some(w) = g.waker.clone() {
                            drop(g);
                            w();
                        }
                    });
                }
                _ => self.simulate(scenario, op, &command),
            }
        }
        Ok(op)
    }

    fn drain_events(&mut self, out: &mut Vec<Event>) {
        out.extend(lock(&self.inner).queue.drain(..));
    }
}
