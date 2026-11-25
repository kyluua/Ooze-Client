use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Independent of the in-game JNI ABI (currently 1).
pub const LAUNCHER_API_VERSION: u32 = 1;
pub const LAUNCHER_API_MINOR_VERSION: u16 = 1;

#[derive(Clone, Debug)]
pub struct LauncherConfig {
    pub data_dir: PathBuf,
    pub release_manifest: PathBuf,
    /// Public application ID, never a client secret. None gives an actionable auth error.
    pub microsoft_client_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    pub instance_dir: PathBuf,
    /// None selects the release's managed Java 21 runtime.
    pub java_path: Option<PathBuf>,
    pub memory_mib: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AccountSummary {
    pub username: String,
    pub uuid: String,
}

#[derive(Clone, Debug)]
pub struct StateSnapshot {
    pub api_version: u32,
    pub settings: Settings,
    /// A remembered profile is display metadata, not proof of a currently valid session.
    pub account: Option<AccountSummary>,
    pub active_operation: Option<OperationId>,
    pub game_pid: Option<u32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct OperationId(pub u64);

/// Names and semantics correspond to Fabric's EngineStatus.State.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EngineState {
    Loaded,
    MissingLibrary,
    UnsupportedPlatform,
    Incompatible,
    LoadFailed,
}

#[derive(Clone, Debug)]
pub enum LauncherCommand {
    SignIn,
    SignOut,
    Install,
    Repair,
    Launch,
    /// Export the latest launcher-owned redacted game log and release metadata.
    ExportDiagnostics(PathBuf),
    SaveSettings(Settings),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    AwaitingBrowser,
    Authorizing,
    Resolving,
    Downloading,
    Verifying,
    Extracting,
    ValidatingJava,
    StartingGame,
}

#[derive(Clone, Debug)]
pub enum LauncherEvent {
    Started {
        id: OperationId,
    },
    Progress {
        id: OperationId,
        stage: Stage,
        completed: u64,
        total: Option<u64>,
    },
    AccountChanged {
        id: OperationId,
        account: Option<AccountSummary>,
    },
    GameStarted {
        id: OperationId,
        pid: u32,
    },
    /// The mod's title marker was observed. Delivered once, before GameExited.
    GameReady {
        id: OperationId,
        engine_state: EngineState,
    },
    GameExited {
        id: OperationId,
        code: Option<i32>,
    },
    /// Best effort, bounded and redacted. Complete redacted output is saved under data_dir/logs.
    LogLine {
        id: OperationId,
        line: String,
    },
    Completed {
        id: OperationId,
    },
    Cancelled {
        id: OperationId,
    },
    Failed {
        id: OperationId,
        error: LauncherError,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorCode {
    UnsupportedPlatform,
    Configuration,
    Busy,
    Cancelled,
    Io,
    Network,
    Authentication,
    ServiceAccess,
    Entitlement,
    Integrity,
    Metadata,
    JavaRuntime,
    Process,
    CredentialStore,
    Internal,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LauncherError {
    pub code: ErrorCode,
    /// Safe diagnostic; never a provider response body, raw HTTP error, token or argv.
    pub message: String,
    pub retryable: bool,
}

impl std::fmt::Display for LauncherError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}: {}", self.code, self.message)
    }
}
impl std::error::Error for LauncherError {}
