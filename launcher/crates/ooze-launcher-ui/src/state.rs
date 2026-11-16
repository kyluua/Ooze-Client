//! Interface state and the event reducer. No egui here: everything is unit-testable.
//!
//! Rules this module enforces:
//! - **Stale events are ignored.** Progress or completion for an operation the interface does not
//!   track (or already saw finish), and game events for an older session, change nothing.
//! - **One Play at a time.** While a launch is pending or the game is live, the primary action is
//!   not `Play`, so a double click cannot start two games (the backend also rejects it).
//! - **Real or absent.** The primary action is derived only from backend state; there is no
//!   optimistic "installed" or "signed in".

use std::collections::VecDeque;
use std::path::PathBuf;

use crate::api::{
    AccountSummary, Capabilities, Command, ErrorKind, Event, GameSessionId, GameState,
    InstallStatus, JavaInfo, LauncherError, LogLine, OperationId, OperationKind, Outcome, Payload,
    ProfileSummary, Progress, Settings, Snapshot, Step, SystemInfo, WhilePlaying,
};
use crate::redact::redact;

#[derive(Clone, Debug, PartialEq)]
pub struct OpView {
    pub id: OperationId,
    pub kind: OperationKind,
    pub progress: Option<Progress>,
    pub cancelling: bool,
}

/// What the big button on the Play page does right now.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Primary {
    /// No snapshot from the backend yet.
    Connecting,
    SignIn,
    Install,
    Repair,
    Play,
    SigningIn(OperationId),
    Preparing(OperationId),
    Launching,
    Playing,
}

impl Primary {
    /// Whether pressing the button submits a command (otherwise it is shown disabled).
    pub fn actionable(self) -> bool {
        matches!(
            self,
            Self::SignIn | Self::Install | Self::Repair | Self::Play
        )
    }
}

/// Persistent notices shown in the banner area. Errors stay until dismissed or superseded.
#[derive(Clone, Debug, PartialEq)]
pub enum Notice {
    Error {
        during: OperationKind,
        error: LauncherError,
    },
    Cancelled(OperationKind),
    SignedOut,
    InstallReady,
    GameExited {
        code: i32,
    },
    GameCrashed {
        code: Option<i32>,
        crash_report: Option<PathBuf>,
    },
    Exported(PathBuf),
}

impl Notice {
    pub fn is_error(&self) -> bool {
        matches!(self, Self::Error { .. } | Self::GameCrashed { .. })
            || matches!(self, Self::GameExited { code } if *code != 0)
    }
}

/// Semantic announcements for the screen-reader live region (localised by the view).
#[derive(Clone, Debug, PartialEq)]
pub enum Announcement {
    Step(Step),
    Notice(Notice),
    SignedIn(String),
    GameStarting,
    GameTitleScreen,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CloseGuard {
    Allow,
    /// Closing would cancel a running sign-in or installation.
    ConfirmOperation(OperationKind),
    /// Minecraft keeps running; the launcher can no longer report a crash.
    ConfirmGameRunning,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowEffect {
    Minimize,
    Restore,
}

#[derive(Clone, Debug, PartialEq)]
pub enum JavaCheckView {
    Checking(OperationId),
    Valid(JavaInfo),
    Invalid(LauncherError),
}

const RECENT_FINISHED: usize = 64;

#[derive(Debug)]
pub struct Model {
    pub system: Option<SystemInfo>,
    pub profile: Option<ProfileSummary>,
    pub settings: Option<Settings>,
    pub account: Option<AccountSummary>,
    pub install: InstallStatus,
    pub game: Option<(GameSessionId, GameState)>,
    pub notice: Option<Notice>,
    pub logs: VecDeque<LogLine>,
    pub java_check: Option<JavaCheckView>,
    ops: Vec<OpView>,
    recently_finished: VecDeque<OperationId>,
    log_cap: usize,
    announcement: Option<(u64, Announcement)>,
    announcement_seq: u64,
    window_effect: Option<WindowEffect>,
    minimized_for: Option<GameSessionId>,
    first_run_pending: Option<OperationId>,
}

impl Model {
    pub fn new(log_cap: usize) -> Self {
        Self {
            system: None,
            profile: None,
            settings: None,
            account: None,
            install: InstallStatus::Unknown,
            game: None,
            notice: None,
            logs: VecDeque::new(),
            java_check: None,
            ops: Vec::new(),
            recently_finished: VecDeque::new(),
            log_cap: log_cap.max(1),
            announcement: None,
            announcement_seq: 0,
            window_effect: None,
            minimized_for: None,
            first_run_pending: None,
        }
    }

    pub fn connected(&self) -> bool {
        self.settings.is_some()
    }

    /// First-run setup is shown until the backend reports an install root.
    pub fn needs_first_run(&self) -> bool {
        self.settings
            .as_ref()
            .is_some_and(|s| s.install_root.is_none())
    }

    pub fn ops(&self) -> &[OpView] {
        &self.ops
    }

    pub fn op(&self, id: OperationId) -> Option<&OpView> {
        self.ops.iter().find(|o| o.id == id)
    }

    pub fn running(&self, pred: impl Fn(OperationKind) -> bool) -> Option<&OpView> {
        self.ops.iter().find(|o| pred(o.kind))
    }

    pub fn sign_in_op(&self) -> Option<&OpView> {
        self.running(|k| k == OperationKind::SignIn)
    }

    pub fn prepare_op(&self) -> Option<&OpView> {
        self.running(|k| matches!(k, OperationKind::Prepare { .. }))
    }

    pub fn launch_op(&self) -> Option<&OpView> {
        self.running(|k| k == OperationKind::Launch)
    }

    pub fn first_run_op(&self) -> Option<OperationId> {
        self.first_run_pending
    }

    pub fn game_live(&self) -> bool {
        self.game.as_ref().is_some_and(|(_, s)| s.is_live())
    }

    pub fn primary(&self) -> Primary {
        if !self.connected() {
            return Primary::Connecting;
        }
        if self.launch_op().is_some() || matches!(self.game, Some((_, GameState::Starting))) {
            return Primary::Launching;
        }
        if self.game_live() {
            return Primary::Playing;
        }
        if let Some(op) = self.prepare_op() {
            return Primary::Preparing(op.id);
        }
        if let Some(op) = self.sign_in_op() {
            return Primary::SigningIn(op.id);
        }
        if self.account.is_none() {
            return Primary::SignIn;
        }
        match self.install {
            InstallStatus::NeedsRepair => Primary::Repair,
            InstallStatus::Ready => Primary::Play,
            // A backend without up-front install status installs and verifies as part of Play.
            InstallStatus::Unknown | InstallStatus::NotInstalled
                if !self.capabilities().install_status =>
            {
                Primary::Play
            }
            InstallStatus::Unknown | InstallStatus::NotInstalled => Primary::Install,
        }
    }

    pub fn capabilities(&self) -> Capabilities {
        self.system
            .as_ref()
            .map_or(Capabilities::ALL, |s| s.capabilities)
    }

    /// True while any tracked operation or the game is active.
    pub fn busy(&self) -> bool {
        !self.ops.is_empty() || self.game_live()
    }

    /// The command the primary button submits, if any.
    pub fn primary_command(&self) -> Option<Command> {
        match self.primary() {
            Primary::SignIn => Some(Command::SignIn {
                switch_account: false,
            }),
            Primary::Install => Some(Command::Prepare { repair: false }),
            Primary::Repair => Some(Command::Prepare { repair: true }),
            Primary::Play => Some(Command::Play),
            _ => None,
        }
    }

    /// Whether a command may be submitted now. Guards against double clicks and races; the
    /// backend enforces the same rules and rejects with `Busy` / `AlreadyRunning`.
    pub fn may_submit(&self, command: &Command) -> bool {
        if !self.connected() {
            return false;
        }
        let busy_install = self.prepare_op().is_some();
        let busy_launch = self.launch_op().is_some() || self.game_live();
        let caps = self.capabilities();
        let local = matches!(
            command,
            Command::Cancel { .. } | Command::ReopenSignInBrowser { .. } | Command::OpenFolder(_)
        );
        if !caps.concurrent_operations && !local && self.busy() {
            return false;
        }
        match command {
            Command::Play => self.primary() == Primary::Play,
            Command::Prepare { .. } => !busy_install && !busy_launch,
            Command::SignIn { .. } => self.sign_in_op().is_none() && !busy_launch,
            Command::SignOut => self.sign_in_op().is_none() && !busy_launch,
            Command::SetInstallRoot { .. } => {
                !busy_install
                    && !busy_launch
                    && self
                        .running(|k| k == OperationKind::SetInstallRoot)
                        .is_none()
            }
            Command::Cancel { op } => self.op(*op).is_some_and(|o| !o.cancelling),
            Command::ReopenSignInBrowser { op } => {
                caps.reopen_browser && self.sign_in_op().is_some_and(|o| o.id == *op)
            }
            Command::CheckJava { .. } => {
                caps.check_java && !matches!(self.java_check, Some(JavaCheckView::Checking(_)))
            }
            Command::UpdateSettings(_) => {
                !busy_launch
                    && self
                        .running(|k| k == OperationKind::UpdateSettings)
                        .is_none()
            }
            Command::ExportDiagnostics => {
                caps.export_diagnostics
                    && self
                        .running(|k| k == OperationKind::ExportDiagnostics)
                        .is_none()
            }
            Command::OpenFolder(_) => caps.open_folders,
        }
    }

    /// Record the backend's immediate answer to a submitted command.
    pub fn on_submitted(&mut self, command: &Command, result: Result<OperationId, LauncherError>) {
        let kind = match command {
            // Cancel acts on an existing operation; report a rejection against that operation.
            Command::Cancel { op } => self
                .op(*op)
                .map_or(OperationKind::UpdateSettings, |o| o.kind),
            _ => kind_of(command),
        };
        match result {
            Ok(id) => {
                if let Command::Cancel { op } = command
                    && let Some(o) = self.ops.iter_mut().find(|o| o.id == *op)
                {
                    o.cancelling = true;
                }
                if let Command::CheckJava { .. } = command {
                    self.java_check = Some(JavaCheckView::Checking(id));
                }
                if let Command::SetInstallRoot { .. } = command {
                    self.first_run_pending = Some(id);
                }
                if !matches!(
                    command,
                    Command::Cancel { .. } | Command::ReopenSignInBrowser { .. }
                ) {
                    self.track(id, kind);
                }
                if matches!(
                    kind,
                    OperationKind::SignIn | OperationKind::Prepare { .. } | OperationKind::Launch
                ) {
                    // A new attempt supersedes an earlier error banner.
                    self.notice = None;
                }
            }
            Err(error) => self.set_notice(Notice::Error {
                during: kind,
                error,
            }),
        }
    }

    pub fn dismiss_notice(&mut self) {
        self.notice = None;
    }

    pub fn close_guard(&self) -> CloseGuard {
        if let Some(op) = self.running(|k| {
            matches!(
                k,
                OperationKind::SignIn | OperationKind::Prepare { .. } | OperationKind::Launch
            )
        }) {
            return CloseGuard::ConfirmOperation(op.kind);
        }
        if self.game_live() {
            return CloseGuard::ConfirmGameRunning;
        }
        CloseGuard::Allow
    }

    /// Operations to cancel when the user confirms closing.
    pub fn cancellable_ops(&self) -> Vec<OperationId> {
        self.ops
            .iter()
            .filter(|o| {
                matches!(
                    o.kind,
                    OperationKind::SignIn | OperationKind::Prepare { .. }
                ) && !o.cancelling
            })
            .map(|o| o.id)
            .collect()
    }

    /// The latest live-region announcement and its sequence number (changes when re-announced).
    pub fn announcement(&self) -> Option<&(u64, Announcement)> {
        self.announcement.as_ref()
    }

    pub fn take_window_effect(&mut self) -> Option<WindowEffect> {
        self.window_effect.take()
    }

    pub fn apply(&mut self, event: Event) {
        match event {
            Event::Snapshot(snapshot) => self.apply_snapshot(*snapshot),
            Event::Started { op, kind } => {
                if !self.recently_finished.contains(&op) {
                    self.track(op, kind);
                }
            }
            Event::Progress { op, progress } => {
                let Some(o) = self.ops.iter_mut().find(|o| o.id == op) else {
                    return; // stale or unknown
                };
                let step_changed = o.progress.map(|p| p.step) != Some(progress.step);
                o.progress = Some(progress);
                if step_changed {
                    self.announce(Announcement::Step(progress.step));
                }
            }
            Event::Finished { op, outcome } => self.finish(op, outcome),
            Event::AccountChanged(account) => {
                let was_signed_in = self.account.is_some();
                if let Some(a) = &account {
                    self.announce(Announcement::SignedIn(a.username.clone()));
                } else if was_signed_in {
                    self.set_notice(Notice::SignedOut);
                }
                self.account = account;
            }
            Event::InstallChanged(status) => self.install = status,
            Event::SettingsChanged(settings) => self.settings = Some(settings),
            Event::Game { session, state } => self.apply_game(session, state),
            Event::Log(lines) => {
                for mut line in lines {
                    if let std::borrow::Cow::Owned(clean) = redact(&line.text) {
                        line.text = clean;
                    }
                    if self.logs.len() == self.log_cap {
                        self.logs.pop_front();
                    }
                    self.logs.push_back(line);
                }
            }
        }
    }

    fn apply_snapshot(&mut self, s: Snapshot) {
        self.system = Some(s.system);
        self.profile = Some(s.profile);
        self.settings = Some(s.settings);
        self.account = s.account;
        self.install = s.install;
        self.game = s.game;
        self.ops
            .retain(|o| s.running.iter().any(|(id, _)| *id == o.id));
        for (id, kind) in s.running {
            self.track(id, kind);
        }
    }

    fn apply_game(&mut self, session: GameSessionId, state: GameState) {
        if let Some((current, _)) = &self.game
            && session < *current
        {
            return; // an older Minecraft process
        }
        let was = self
            .game
            .as_ref()
            .filter(|(s, _)| *s == session)
            .map(|(_, st)| st.clone());
        match &state {
            GameState::Starting if was.is_none() => self.announce(Announcement::GameStarting),
            GameState::Running { title_screen: true }
                if !matches!(was, Some(GameState::Running { title_screen: true })) =>
            {
                self.announce(Announcement::GameTitleScreen);
            }
            GameState::Exited { code } => self.set_notice(Notice::GameExited { code: *code }),
            GameState::Crashed { code, crash_report } => self.set_notice(Notice::GameCrashed {
                code: *code,
                crash_report: crash_report.clone(),
            }),
            _ => {}
        }
        let minimize = self
            .settings
            .as_ref()
            .is_some_and(|s| s.while_playing == WhilePlaying::Minimize);
        if state.is_live() {
            if minimize && self.minimized_for != Some(session) {
                self.minimized_for = Some(session);
                self.window_effect = Some(WindowEffect::Minimize);
            }
        } else if self.minimized_for == Some(session) {
            self.minimized_for = None;
            self.window_effect = Some(WindowEffect::Restore);
        }
        self.game = Some((session, state));
    }

    fn finish(&mut self, op: OperationId, outcome: Outcome) {
        let Some(index) = self.ops.iter().position(|o| o.id == op) else {
            return; // stale or unknown
        };
        let o = self.ops.remove(index);
        self.recently_finished.push_back(op);
        if self.recently_finished.len() > RECENT_FINISHED {
            self.recently_finished.pop_front();
        }
        if self.first_run_pending == Some(op) {
            self.first_run_pending = None;
        }
        match outcome {
            Outcome::Succeeded(payload) => match (o.kind, payload) {
                (OperationKind::CheckJava, Payload::Java(info)) => {
                    self.java_check = Some(JavaCheckView::Valid(info))
                }
                (OperationKind::ExportDiagnostics, Payload::Exported(path)) => {
                    self.set_notice(Notice::Exported(path))
                }
                (OperationKind::Prepare { .. }, _) => {
                    self.install = InstallStatus::Ready;
                    self.set_notice(Notice::InstallReady);
                }
                _ => {}
            },
            Outcome::Cancelled => {
                if o.kind == OperationKind::CheckJava {
                    self.java_check = None;
                }
                self.set_notice(Notice::Cancelled(o.kind));
            }
            Outcome::Failed(error) => {
                if o.kind == OperationKind::CheckJava {
                    self.java_check = Some(JavaCheckView::Invalid(error));
                } else {
                    if error.kind == ErrorKind::SessionExpired {
                        self.account = None;
                    }
                    self.set_notice(Notice::Error {
                        during: o.kind,
                        error,
                    });
                }
            }
        }
    }

    fn track(&mut self, id: OperationId, kind: OperationKind) {
        if !self.ops.iter().any(|o| o.id == id) {
            self.ops.push(OpView {
                id,
                kind,
                progress: None,
                cancelling: false,
            });
        }
    }

    fn set_notice(&mut self, notice: Notice) {
        self.announce(Announcement::Notice(notice.clone()));
        self.notice = Some(notice);
    }

    fn announce(&mut self, a: Announcement) {
        self.announcement_seq += 1;
        self.announcement = Some((self.announcement_seq, a));
    }
}

pub fn kind_of(command: &Command) -> OperationKind {
    match command {
        Command::SetInstallRoot { .. } => OperationKind::SetInstallRoot,
        Command::SignIn { .. } | Command::ReopenSignInBrowser { .. } => OperationKind::SignIn,
        Command::SignOut => OperationKind::SignOut,
        Command::Prepare { repair } => OperationKind::Prepare { repair: *repair },
        Command::Play => OperationKind::Launch,
        Command::Cancel { .. } | Command::UpdateSettings(_) => OperationKind::UpdateSettings,
        Command::CheckJava { .. } => OperationKind::CheckJava,
        Command::ExportDiagnostics => OperationKind::ExportDiagnostics,
        Command::OpenFolder(_) => OperationKind::OpenFolder,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::*;
    use crate::fixtures::{account, snapshot};

    fn ready_model() -> Model {
        let mut m = Model::new(100);
        let mut s = snapshot();
        s.account = Some(account());
        s.install = InstallStatus::Ready;
        m.apply(Event::Snapshot(Box::new(s)));
        m
    }

    fn err(kind: ErrorKind) -> LauncherError {
        LauncherError {
            kind,
            detail: "detail".into(),
            retryable: true,
        }
    }

    #[test]
    fn primary_follows_backend_state_only() {
        let mut m = Model::new(10);
        assert_eq!(m.primary(), Primary::Connecting);
        m.apply(Event::Snapshot(Box::new(snapshot())));
        assert_eq!(m.primary(), Primary::SignIn);
        m.apply(Event::AccountChanged(Some(account())));
        assert_eq!(m.primary(), Primary::Install);
        m.apply(Event::InstallChanged(InstallStatus::NeedsRepair));
        assert_eq!(m.primary(), Primary::Repair);
        m.apply(Event::InstallChanged(InstallStatus::Ready));
        assert_eq!(m.primary(), Primary::Play);
    }

    #[test]
    fn double_play_is_blocked_until_the_launch_finishes() {
        let mut m = ready_model();
        assert!(m.may_submit(&Command::Play));
        m.on_submitted(&Command::Play, Ok(OperationId(7)));
        assert_eq!(m.primary(), Primary::Launching);
        assert!(
            !m.may_submit(&Command::Play),
            "second click must not submit"
        );
        m.apply(Event::Game {
            session: GameSessionId(1),
            state: GameState::Starting,
        });
        m.apply(Event::Finished {
            op: OperationId(7),
            outcome: Outcome::Succeeded(Payload::None),
        });
        assert_eq!(m.primary(), Primary::Launching, "still starting");
        m.apply(Event::Game {
            session: GameSessionId(1),
            state: GameState::Running {
                title_screen: false,
            },
        });
        assert_eq!(m.primary(), Primary::Playing);
        assert!(!m.may_submit(&Command::Play));
        assert!(
            !m.may_submit(&Command::Prepare { repair: true }),
            "no repair under a running game"
        );
        m.apply(Event::Game {
            session: GameSessionId(1),
            state: GameState::Exited { code: 0 },
        });
        assert_eq!(m.primary(), Primary::Play);
        assert_eq!(m.notice, Some(Notice::GameExited { code: 0 }));
    }

    #[test]
    fn stale_progress_and_completion_are_ignored() {
        let mut m = ready_model();
        m.on_submitted(&Command::Prepare { repair: false }, Ok(OperationId(3)));
        m.apply(Event::Finished {
            op: OperationId(3),
            outcome: Outcome::Cancelled,
        });
        // Late events for the finished operation, and for one never seen:
        m.apply(Event::Progress {
            op: OperationId(3),
            progress: Progress {
                step: Step::DownloadingGame,
                done: 1,
                total: Some(2),
                unit: ProgressUnit::Files,
            },
        });
        m.apply(Event::Started {
            op: OperationId(3),
            kind: OperationKind::Prepare { repair: false },
        });
        m.apply(Event::Finished {
            op: OperationId(99),
            outcome: Outcome::Failed(err(ErrorKind::Internal)),
        });
        assert!(m.prepare_op().is_none());
        assert_eq!(
            m.notice,
            Some(Notice::Cancelled(OperationKind::Prepare { repair: false }))
        );
    }

    #[test]
    fn older_game_sessions_cannot_overwrite_newer_ones() {
        let mut m = ready_model();
        m.apply(Event::Game {
            session: GameSessionId(2),
            state: GameState::Running { title_screen: true },
        });
        m.apply(Event::Game {
            session: GameSessionId(1),
            state: GameState::Crashed {
                code: Some(1),
                crash_report: None,
            },
        });
        assert_eq!(
            m.game,
            Some((GameSessionId(2), GameState::Running { title_screen: true }))
        );
        assert!(m.notice.is_none());
    }

    #[test]
    fn cancel_is_marked_once_and_acknowledged_by_the_backend() {
        let mut m = ready_model();
        m.on_submitted(&Command::Prepare { repair: false }, Ok(OperationId(4)));
        let cancel = Command::Cancel { op: OperationId(4) };
        assert!(m.may_submit(&cancel));
        m.on_submitted(&cancel, Ok(OperationId(5)));
        assert!(m.prepare_op().unwrap().cancelling);
        assert!(!m.may_submit(&cancel), "no second cancel while cancelling");
        assert!(
            m.op(OperationId(5)).is_none(),
            "cancel itself is not tracked as an operation"
        );
        m.apply(Event::Finished {
            op: OperationId(4),
            outcome: Outcome::Cancelled,
        });
        assert_eq!(m.primary(), Primary::Play);
    }

    #[test]
    fn close_guard_protects_running_work_and_games() {
        let mut m = ready_model();
        assert_eq!(m.close_guard(), CloseGuard::Allow);
        m.on_submitted(&Command::Prepare { repair: true }, Ok(OperationId(8)));
        assert_eq!(
            m.close_guard(),
            CloseGuard::ConfirmOperation(OperationKind::Prepare { repair: true })
        );
        assert_eq!(m.cancellable_ops(), vec![OperationId(8)]);
        m.apply(Event::Finished {
            op: OperationId(8),
            outcome: Outcome::Succeeded(Payload::None),
        });
        m.apply(Event::Game {
            session: GameSessionId(1),
            state: GameState::Running {
                title_screen: false,
            },
        });
        assert_eq!(m.close_guard(), CloseGuard::ConfirmGameRunning);
    }

    #[test]
    fn rejected_submission_becomes_an_error_notice() {
        let mut m = ready_model();
        m.on_submitted(&Command::Play, Err(err(ErrorKind::AlreadyRunning)));
        assert!(m.launch_op().is_none());
        assert!(matches!(
            m.notice,
            Some(Notice::Error {
                during: OperationKind::Launch,
                ..
            })
        ));
    }

    #[test]
    fn expired_session_signs_the_account_out() {
        let mut m = ready_model();
        m.on_submitted(&Command::Play, Ok(OperationId(9)));
        m.apply(Event::Finished {
            op: OperationId(9),
            outcome: Outcome::Failed(err(ErrorKind::SessionExpired)),
        });
        assert!(m.account.is_none());
        assert_eq!(m.primary(), Primary::SignIn);
    }

    #[test]
    fn minimize_while_playing_restores_after_exit() {
        let mut m = ready_model();
        let mut s = m.settings.clone().unwrap();
        s.while_playing = WhilePlaying::Minimize;
        m.apply(Event::SettingsChanged(s));
        m.apply(Event::Game {
            session: GameSessionId(1),
            state: GameState::Starting,
        });
        assert_eq!(m.take_window_effect(), Some(WindowEffect::Minimize));
        m.apply(Event::Game {
            session: GameSessionId(1),
            state: GameState::Running { title_screen: true },
        });
        assert_eq!(m.take_window_effect(), None, "minimize once per session");
        m.apply(Event::Game {
            session: GameSessionId(1),
            state: GameState::Crashed {
                code: Some(-1),
                crash_report: None,
            },
        });
        assert_eq!(m.take_window_effect(), Some(WindowEffect::Restore));
    }

    #[test]
    fn logs_are_bounded_and_redacted_again() {
        let mut m = Model::new(3);
        let lines = (0..5)
            .map(|i| LogLine {
                level: LogLevel::Info,
                source: LogSource::Game,
                text: format!("line {i} --accessToken secret{i}"),
            })
            .collect();
        m.apply(Event::Log(lines));
        assert_eq!(m.logs.len(), 3);
        assert!(m.logs.iter().all(|l| !l.text.contains("secret")));
        assert!(m.logs.front().unwrap().text.starts_with("line 2"));
    }

    #[test]
    fn progress_step_changes_are_announced_once() {
        let mut m = ready_model();
        m.on_submitted(&Command::Prepare { repair: false }, Ok(OperationId(4)));
        let p = |done| Event::Progress {
            op: OperationId(4),
            progress: Progress {
                step: Step::DownloadingAssets,
                done,
                total: Some(10),
                unit: ProgressUnit::Bytes,
            },
        };
        m.apply(p(1));
        let first = m.announcement().unwrap().0;
        m.apply(p(2));
        assert_eq!(
            m.announcement().unwrap().0,
            first,
            "byte progress does not re-announce"
        );
    }
}
