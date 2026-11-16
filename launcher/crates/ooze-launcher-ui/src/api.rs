//! The interface's view of the launcher backend: the **proposed** `LAUNCHER_API_V1`.
//!
//! Codex owns the real contract (`docs/launcher/LAUNCHER_API_V1.md`, `ooze-launcher-core`). It
//! had not been published when this crate was written, so this module states exactly what the
//! interface needs (see `docs/handoffs/CLAUDE-L0-request.md`). When Codex's contract lands, this
//! module becomes a thin adapter over it and the screens do not change.
//!
//! Security boundary: nothing in this module can carry a credential. Accounts are display
//! summaries (username and UUID). Errors carry a typed kind plus a backend-redacted detail string.

use std::fmt;
use std::path::PathBuf;

/// The API major version this interface is written against.
pub const API_MAJOR: u16 = 1;

/// Identifies one backend operation (sign-in, prepare, launch, export, ...). Monotonic per process.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct OperationId(pub u64);

/// Identifies one Minecraft process started by the backend. Monotonic per process.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct GameSessionId(pub u64);

/// Non-secret display metadata for the signed-in Microsoft/Minecraft account.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AccountSummary {
    pub username: String,
    pub uuid: String,
}

/// The single supported profile (Minecraft 1.21.11 + Fabric + Ooze), as display strings.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProfileSummary {
    pub minecraft: String,
    pub fabric_loader: String,
    pub fabric_api: String,
    pub ooze: String,
    pub java_major: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum JavaChoice {
    /// The backend downloads and verifies a Java 21 runtime into the install root.
    Managed,
    /// A `java`/`javaw` executable the user chose. The backend validates it is Java 21, 64-bit.
    Custom(PathBuf),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WhilePlaying {
    KeepOpen,
    Minimize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MotionPreference {
    /// Reduced when the OS reports "show animations" off, otherwise Full.
    FollowSystem,
    Full,
    Reduced,
    Off,
}

/// Launcher settings. Persisted by the backend (atomic writes); the interface only edits drafts.
#[derive(Clone, Debug, PartialEq)]
pub struct Settings {
    /// `None` until first-run setup completes.
    pub install_root: Option<PathBuf>,
    pub java: JavaChoice,
    pub memory_mib: u32,
    pub while_playing: WhilePlaying,
    pub motion: MotionPreference,
    /// 100, 125, 150, 175 or 200.
    pub ui_scale_percent: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MemoryBounds {
    pub min_mib: u32,
    pub max_mib: u32,
    pub recommended_mib: u32,
}

/// Facts about this machine and build that the interface displays or uses for defaults.
#[derive(Clone, Debug, PartialEq)]
pub struct SystemInfo {
    pub launcher_version: String,
    /// (major, minor) of the backend API.
    pub api_version: (u16, u16),
    pub default_install_root: PathBuf,
    pub memory: MemoryBounds,
    /// Windows "Show animations in Windows"; `None` when the backend cannot read it.
    pub system_reduced_motion: Option<bool>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InstallStatus {
    /// Not checked yet in this session.
    Unknown,
    NotInstalled,
    Ready,
    /// Verification found missing or damaged managed files.
    NeedsRepair,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Folder {
    InstallRoot,
    Logs,
    CrashReports,
    Diagnostics,
}

/// Everything the interface can ask the backend to do. Every accepted command becomes an
/// operation: the backend reports `Started`, optional `Progress`, then exactly one `Finished`.
/// The exceptions are `Cancel` and `ReopenSignInBrowser`, which act on an existing operation and
/// return that operation's id (cancellation is acknowledged by its `Finished { Cancelled }`).
#[derive(Clone, Debug, PartialEq)]
pub enum Command {
    /// Validate and adopt an install root (first run or Settings). Existing files are kept.
    SetInstallRoot {
        path: PathBuf,
    },
    /// Start the system-browser Microsoft sign-in. `switch_account` forces the account picker.
    SignIn {
        switch_account: bool,
    },
    /// Open the sign-in page in the browser again for a running sign-in operation.
    ReopenSignInBrowser {
        op: OperationId,
    },
    /// Forget the signed-in account and delete its stored credentials.
    SignOut,
    /// Install or verify the profile. `repair` re-verifies every managed file and replaces damaged
    /// ones. Never touches saves, screenshots, resource packs, shader packs or `options.txt`.
    Prepare {
        repair: bool,
    },
    /// Refresh the session if needed, verify the install, and start Minecraft.
    Play,
    /// Cancel a running operation. Cancellation is acknowledged by `Finished { Cancelled }`.
    Cancel {
        op: OperationId,
    },
    UpdateSettings(Settings),
    /// Validate a candidate Java executable without saving it.
    CheckJava {
        path: PathBuf,
    },
    /// Write a redacted diagnostics bundle (logs, versions, settings) into the Diagnostics folder.
    ExportDiagnostics,
    OpenFolder(Folder),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OperationKind {
    SetInstallRoot,
    SignIn,
    SignOut,
    Prepare { repair: bool },
    Launch,
    UpdateSettings,
    CheckJava,
    ExportDiagnostics,
    OpenFolder,
}

/// What an operation is doing right now. Steps are typed so the interface can localise them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    // Sign-in
    WaitingForBrowser,
    XboxLive,
    MinecraftServices,
    CheckingOwnership,
    LoadingProfile,
    // Prepare
    CheckingFiles,
    DownloadingRuntime,
    DownloadingGame,
    DownloadingLibraries,
    DownloadingAssets,
    InstallingFabric,
    InstallingOoze,
    Verifying,
    // Launch
    RefreshingSession,
    StartingJava,
    WaitingForTitleScreen,
    // Other
    Working,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProgressUnit {
    Bytes,
    Files,
    Steps,
}

/// Real progress reported by the backend. `total == None` means indeterminate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Progress {
    pub step: Step,
    pub done: u64,
    pub total: Option<u64>,
    pub unit: ProgressUnit,
}

impl Progress {
    pub fn fraction(&self) -> Option<f32> {
        match self.total {
            Some(total) if total > 0 => Some((self.done.min(total) as f64 / total as f64) as f32),
            _ => None,
        }
    }
}

/// Typed failure reasons. The interface maps each to a localised title, body and actions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorKind {
    // Configuration (developer/build problem, not the player's fault)
    AuthNotConfigured,
    /// Minecraft Services rejected the app registration (new apps need Mojang/Microsoft approval).
    AuthAppNotApproved,
    // Sign-in
    BrowserUnavailable,
    SignInTimedOut,
    ConsentDenied,
    NoXboxProfile,
    XboxChildAccount,
    XboxRegionUnavailable,
    XboxBanned,
    NotEntitled,
    SessionExpired,
    // Network and services
    Offline,
    ServiceUnavailable,
    RateLimited,
    // Install root and disk
    PathNotWritable,
    PathInvalid,
    PathInsideAnotherInstall,
    DiskFull,
    FilesInUse,
    ChecksumMismatch,
    // Java
    JavaNotFound,
    JavaWrongVersion,
    JavaNot64Bit,
    // Launch
    NotSignedIn,
    NotInstalled,
    AlreadyRunning,
    Busy,
    ProcessStartFailed,
    // Anything else
    Internal,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LauncherError {
    pub kind: ErrorKind,
    /// Backend-redacted technical detail for the Details view and diagnostics. Never a credential.
    pub detail: String,
    pub retryable: bool,
}

impl fmt::Display for LauncherError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}: {}", self.kind, self.detail)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JavaInfo {
    pub version: String,
    pub vendor: String,
    pub path: PathBuf,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Payload {
    None,
    Java(JavaInfo),
    Exported(PathBuf),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Outcome {
    Succeeded(Payload),
    Cancelled,
    Failed(LauncherError),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GameState {
    /// The Java process was spawned; Minecraft has not reached its title screen yet.
    Starting,
    /// Running. `title_screen` turns true when the Ooze mod logs its `OOZE_READY` marker.
    Running {
        title_screen: bool,
    },
    Exited {
        code: i32,
    },
    Crashed {
        code: Option<i32>,
        crash_report: Option<PathBuf>,
    },
}

impl GameState {
    pub fn is_live(&self) -> bool {
        matches!(self, Self::Starting | Self::Running { .. })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogSource {
    Launcher,
    Game,
}

/// One log line, already redacted by the backend. The interface redacts again before display.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LogLine {
    pub level: LogLevel,
    pub source: LogSource,
    pub text: String,
}

/// The full state, sent first and after any resynchronisation.
#[derive(Clone, Debug, PartialEq)]
pub struct Snapshot {
    pub system: SystemInfo,
    pub profile: ProfileSummary,
    pub settings: Settings,
    pub account: Option<AccountSummary>,
    pub install: InstallStatus,
    pub game: Option<(GameSessionId, GameState)>,
    pub running: Vec<(OperationId, OperationKind)>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    Snapshot(Box<Snapshot>),
    Started {
        op: OperationId,
        kind: OperationKind,
    },
    Progress {
        op: OperationId,
        progress: Progress,
    },
    Finished {
        op: OperationId,
        outcome: Outcome,
    },
    AccountChanged(Option<AccountSummary>),
    InstallChanged(InstallStatus),
    SettingsChanged(Settings),
    Game {
        session: GameSessionId,
        state: GameState,
    },
    Log(Vec<LogLine>),
}

/// Called by the backend (from any thread) after it queues events, so the window repaints.
pub type Waker = Box<dyn Fn() + Send + Sync>;

/// The backend as the interface uses it. Implementations must never block: work runs on the
/// backend's own threads and results arrive through [`Backend::drain_events`].
pub trait Backend {
    fn api_version(&self) -> (u16, u16);
    fn set_waker(&mut self, waker: Waker);
    /// Accept or reject a command immediately. Rejections are typed (`Busy`, `AlreadyRunning`, ...).
    fn submit(&mut self, command: Command) -> Result<OperationId, LauncherError>;
    /// Move all queued events into `out`, oldest first.
    fn drain_events(&mut self, out: &mut Vec<Event>);
}
