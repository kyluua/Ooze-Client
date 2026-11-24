# Launcher API v1 — L0 contract

Owner: Sol. Initial types: `launcher/crates/ooze-launcher-core/src/api.rs`.
This is a Rust source API, separate from the in-game JNI ABI 1. The new owner
decision permits secrets only inside the standalone launcher's backend. The mod
and its JNI engine continue to receive display identity only. Claude reconciles
the product documents/AGENTS; these older official-launcher-only statements do not
override the owner's new launcher instruction.

## Integration

Claude owns `launcher/crates/ooze-launcher-ui` and `launcher/resources`. Sol owns
the workspace manifest/lock, core, CLI, packaging/CI and this contract. Request
workspace membership rather than editing each other's files. UI depends on
`ooze-launcher-core` by relative path. No separate JNI or web IPC is needed.

The implementation exposes `Launcher::open(LauncherConfig)`, `snapshot()`,
`submit(LauncherCommand) -> Result<OperationId, LauncherError>`,
`cancel(OperationId) -> bool`, and `try_event() -> Option<LauncherEvent>`.
Open performs small local configuration/lock I/O; call it during startup away
from a repaint. Submit returns immediately and runs blocking work on a backend
thread. Poll events from the UI loop. One operation, including a running Java
game, is active at a time; another command returns Busy. Operation IDs increase
within a Launcher lifetime; ignore events for an obsolete operation. Cancel is
cooperative; network requests have bounded timeouts. Cancellation cannot kill a
running game: close/exit Minecraft normally. The game may outlive the launcher.

The backend holds an exclusive data-directory lock and an instance lock while
using an installation. A second process must not modify that installation.
Settings persistence is atomic. Paths must be absolute; instance directories are
explicitly Ooze-managed and never silently adopt a populated unrelated directory.

Started is followed by progress/account/process events and exactly one terminal
Completed/Cancelled/Failed event, excluding process termination. Progress and
LogLine events are best effort and may be dropped under backpressure. Control
events use a bounded queue and can wait for the consumer; keep polling. Snapshot
is authoritative for current account, settings, active operation and game PID.
Progress totals are optional, stage-local counts, not a fabricated global percent.
GameStarted means Java spawned, not that the title screen was reached.
GameExited precedes Completed on success, or Failed on a nonzero exit.

SignIn opens the system browser and validates PKCE/state on a loopback callback;
it completes only after Minecraft entitlement and profile validation. Unrelated
loopback requests leave the callback pending within the original deadline. An
explicit browser denial with matching state returns Authentication with a
declined-sign-in message. SignOut
deletes the backend's stored credential and remembered summary. Switch account by
signing out and signing in (Microsoft account selection is requested). Launch
refreshes/revalidates authorization and verifies the installation before spawning
the production Fabric game. Install/Repair may work before sign-in. SaveSettings
validates memory (1024–16384 MiB), Java/instance paths, and saves without starting
an installation. None for java_path selects the manifest-pinned managed Java 21.

The additive `ExportDiagnostics(PathBuf)` command writes a ZIP to a new absolute
local `.zip` path selected by the caller. It uses the normal worker and terminal
events. On completion, the archive contains `diagnostics.json` (schema 1,
launcher version, platform and package metadata) and, when available, the latest
launcher-owned `game-<timestamp>.log` as `game.log`. The worker bounds the log
read and reapplies credential redaction before publishing the archive. The
archive excludes account settings, credential storage, the launch command and
Minecraft and mod log directories. Game logs can include player names, paths,
and other game output. The command writes locally and preserves existing files.

Only AccountSummary crosses into UI state. A remembered summary does not prove
live authorization. No access/refresh token, Xbox identity token, provider body or
token-bearing launch plan is public. The CLI prints the same safe events. OS
credential storage is required; a plaintext fallback is forbidden. Public OAuth
client ID is developer configuration (environment/build config); a missing ID is
Configuration. HTTP 403 from Minecraft sign-in/entitlement/profile is
ServiceAccess; Xbox user/XSTS 401/403 is Authentication. Safe diagnostics include
the fixed service label and numeric HTTP status, never the response body. A
status alone does not establish the cause or prove missing app approval. Network
429/5xx remains Network. This corrects the original implementation's coarse Xbox
403 classification without adding public types or changing API 1. Access denial
is not bypassed with somebody else's ID or an offline account.

## Artifacts and validation boundary

The packaged release contains the CLI (UI when Claude supplies it), an Ooze JAR
with production native resources, and a pinned release manifest. Minecraft,
Fabric dependencies and managed Java download from documented upstream sources;
Minecraft game files are not redistributed. The installation manifest schema and
sources are specified in BUILD_AND_TEST and the backend installation docs.

Version 1 intentionally excludes gameplay modules, updater, news/backend service,
multiple game versions, offline identity and arbitrary JVM arguments. Changes to
public types or behavior require an explicit handoff and contract update. A
binary-compatible C ABI is not promised between separately compiled Rust crates.
