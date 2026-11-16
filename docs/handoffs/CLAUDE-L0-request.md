# Requests from Claude (L0, launcher interface) to Codex and Sol

> Written 2026-11-16 by Claude. Codex had no branch, crate or `docs/launcher/LAUNCHER_API_V1.md` when L0 started, so this file states what the interface needs. It is a proposal: Codex owns the contract and may change names. Please answer in `docs/handoffs/CODEX-L0.md` with the exact contract commit; Claude then adapts `launcher/crates/ooze-launcher-ui/src/api.rs` (the only file that should change).
>
> Branch policy (owner, 2026-11-16): from now on both agents work on **`main`** (AGENTS.md §Branches).

## 1. To Codex: `LAUNCHER_API_V1` as the interface uses it

The exact proposed types are in `launcher/crates/ooze-launcher-ui/src/api.rs`, about 300 lines and documented. A summary follows.

### 1.1 Shape

```rust
pub const API_MAJOR: u16 = 1;               // independent of the in-game JNI ABI
pub trait Backend {
    fn api_version(&self) -> (u16, u16);
    fn set_waker(&mut self, waker: Box<dyn Fn() + Send + Sync>);   // call after queueing events
    fn submit(&mut self, command: Command) -> Result<OperationId, LauncherError>;  // never blocks
    fn drain_events(&mut self, out: &mut Vec<Event>);              // oldest first
}
```

| Concept | Proposal |
|---|---|
| Commands | `SetInstallRoot{path}`, `SignIn{switch_account}`, `ReopenSignInBrowser{op}`, `SignOut`, `Prepare{repair}`, `Play`, `Cancel{op}`, `UpdateSettings(Settings)`, `CheckJava{path}`, `ExportDiagnostics`, `OpenFolder(Folder)` |
| Operations | Every accepted command is one operation with a **monotonic** `OperationId`: `Started` → zero or more `Progress` → exactly one `Finished{Succeeded(payload) / Cancelled / Failed(error)}`. `Cancel` and `ReopenSignInBrowser` return the **target** id and create no operation |
| Progress | `{ step: Step, done, total: Option<u64>, unit: Bytes/Files/Steps }`. Real counters only; `total: None` = indeterminate |
| Steps | Sign-in: `WaitingForBrowser, XboxLive, MinecraftServices, CheckingOwnership, LoadingProfile`. Prepare: `CheckingFiles, DownloadingRuntime, DownloadingGame, DownloadingLibraries, DownloadingAssets, InstallingFabric, InstallingOoze, Verifying`. Launch: `RefreshingSession, StartingJava, WaitingForTitleScreen` |
| Errors | `LauncherError { kind: ErrorKind, detail: String /* redacted */, retryable: bool }`. Kinds in §1.3 |
| Accounts | `AccountSummary { username, uuid }` only. **No token, code, refresh token, xuid or user hash ever reaches the UI** |
| Game | `Event::Game { session: GameSessionId /* monotonic */, state }`, with `state` one of: `Starting` (process spawned), `Running{ title_screen }`, `Exited{code}`, `Crashed{code, crash_report}` |
| Snapshot | `Event::Snapshot` first, and after any resync: system info, profile, settings, account, install status, game, running operations |
| Settings | `install_root: Option<PathBuf>` (None = first run), `java: Managed/Custom(path)`, `memory_mib` (within `SystemInfo.memory` bounds), `while_playing: KeepOpen/Minimize`, `motion: FollowSystem/Full/Reduced/Off`, `ui_scale_percent`. **Persisted by the backend** with atomic writes |
| System info | `launcher_version`, `api_version`, `default_install_root`, `memory {min,max,recommended}`, `system_reduced_motion: Option<bool>` (Windows `SPI_GETCLIENTAREAANIMATION`) |
| Logs | `Event::Log(Vec<LogLine{level, source: Launcher/Game, text}>)`, **already redacted** (the UI redacts again) |

### 1.2 Semantics the UI relies on

1. `submit` never blocks on network, disk, auth or the game. Rejections are typed (`Busy`, `AlreadyRunning`, `NotSignedIn`, `NotInstalled`).
2. **Exactly one `Finished` per operation**, including after a `Cancel`. When a cancel races with completion, report the real outcome.
3. **One Play at a time.** A second `Play` while a launch is pending or a game is live is rejected with `AlreadyRunning`. The UI also guards against it.
4. **Launch finishes when Java is spawned.** Game state then follows the process. Report `Running{title_screen:true}` only after a stdout line **containing** `OOZE_READY v1` appears (§3); the log prefix before it varies.
5. **Minecraft outlives the launcher.** Closing the launcher must not kill the game (no kill-on-close job object). The UI warns that crash reporting stops.
6. **Cancel is safe.** Partially downloaded files are kept and verified next time (UI copy promises this).
7. **Repair and install never touch user data:**
   - `saves/`, `screenshots/`, `resourcepacks/`, `shaderpacks/`, `options.txt`, `servers.dat`, logs;
   - anything outside the managed install root;
   - another launcher's `.minecraft`.

   `SetInstallRoot` never moves or deletes the old root. It rejects system folders, network paths and folders inside another installation (`PathInvalid`, `PathInsideAnotherInstall`).
8. A failed refresh at Play time returns `SessionExpired`, and the UI clears the account. **Never** substitute offline authentication.

### 1.3 Error kinds the UI localises

```
AuthNotConfigured   AuthAppNotApproved   BrowserUnavailable   SignInTimedOut   ConsentDenied
NoXboxProfile       XboxChildAccount     XboxRegionUnavailable XboxBanned      NotEntitled
NoMinecraftProfile  SessionExpired       Offline              ServiceUnavailable RateLimited
PathNotWritable     PathInvalid          PathInsideAnotherInstall DiskFull     FilesInUse
ChecksumMismatch    JavaNotFound         JavaWrongVersion     JavaNot64Bit     NotSignedIn
NotInstalled        AlreadyRunning       Busy                 ProcessStartFailed Internal
```

Suggested service mapping (please verify against current documentation in `AUTH_SETUP.md`):

| Service response | Kind |
|---|---|
| XSTS `XErr` 2148916233 | `NoXboxProfile` |
| XSTS 2148916235 | `XboxRegionUnavailable` |
| XSTS 2148916236 / 2148916237 / 2148916238 | `XboxChildAccount` |
| Minecraft Services 403 "Invalid app registration" | `AuthAppNotApproved` |
| Empty entitlements | `NotEntitled` |
| 404 on the profile | `NoMinecraftProfile` |
| Missing client ID at build time | `AuthNotConfigured` |

### 1.4 Wiring and workspace

- **Register the UI crate** `launcher/crates/ooze-launcher-ui` in your `launcher/Cargo.toml` workspace:
  - adopt or extend its `[lints]`;
  - then delete the crate-local `launcher/crates/ooze-launcher-ui/Cargo.lock`, which exists only so the crate builds standalone;
  - its exact pins are `eframe`/`egui_kittest` `=0.37.0`, `rfd` 0.18, `regex` 1.13, `serde_json` 1.
- **Packaged binary:** `ooze-launcher` calls `ooze_launcher_ui::run(Box::new(core_backend))`. Window options come from `LauncherApp::native_options`. Use `#![windows_subsystem = "windows"]` in release builds.
- **Never enable the `dev-fixtures` feature in packaging.** A release build refuses it with `compile_error!`.
- **The UI keeps adapting to your contract.** When the API lands, `api.rs` becomes `pub use ooze_launcher_core::api::*;` plus small mappings.

### 1.5 Fabric distribution inputs (produced by Claude's Gradle build)

`./gradlew buildAll -PoozeNativeResources=<Sol's release resources>` writes:

```
build/dist/ooze-client-<ver>+mc1.21.11.jar      # the mod; bundle it in the launcher package
build/dist/ooze-launcher-profile.json           # schema oozelauncher.profile/1
```

- **Profile contents:**
  - Minecraft `1.21.11`, Java major 21;
  - Fabric Loader `0.20.1` and its `profileJson` URL on meta.fabricmc.net;
  - Fabric API `0.141.6+1.21.11` with its Maven URL, size and SHA-256 (checked against Fabric's published `.sha256`);
  - the Ooze JAR's name, size and SHA-256 (`"bundled": true`);
  - `readyMarker: "OOZE_READY v1"`.
- **Never bundle Minecraft's client JAR, libraries or assets.** Download them from Mojang's manifest with their published SHA-1 values.
- Verify the bundled Ooze JAR against the manifest before installing it.

### 1.6 Packaging, CI and evidence

| Request | Why |
|---|---|
| A Windows x64 CI job that runs `cargo fmt --check`, `clippy -D warnings` and `cargo test` for the UI crate (WARP or a GPU adapter for wgpu; kittest renders screenshots) | Windows evidence for the interface tests (they were Headless-tested on Linux only) |
| Third-party licence notices for the launcher binary (egui, wgpu, AccessKit, rfd and so on), plus `launcher/resources/fonts/*/OFL.txt` / `LICENSE.txt` and `fonts.json` | Licences |
| An installer or zip that needs no Rust, MSVC or developer tooling, with SHA-256 checksums published beside it | End-user install (`docs/INSTALL.md`) |
| The packaged-app run of §4 together with Claude | L0 acceptance |

## 2. To Sol (unchanged from C0, still open)

- **C0b / Gate B:** a Windows x64 job (or a person) runs `OOZE_NATIVE_RESOURCES=<release resources> tools/launch-tests.sh`. It must pass `LOADED`, the F9 probe, the fixture press colour change, and the real missing, corrupt and ABI-mismatched DLL cases. It now also asserts the `OOZE_READY` marker. The launcher's real-DLL acceptance (§4) depends on the same artifact.
- **S-2:** an in-module switch for building without native resources.
- **S-3:** CI records the PR merge commit instead of the branch head.
- **S-4:** the DLL is not reproducible across machines.
- **S-5:** the native cache is never pruned. With a launcher reinstalling often, pruning old `<gameDir>/.ooze/natives/<ver>/<sha>/` directories matters more.

## 3. The readiness marker

Implemented in `versions/mc1_21_11/.../diag/LaunchReadiness.java` (Claude):

```
OOZE_READY v1 screen=title engine=<STATE>
```

- **When:** logged once, at INFO, when the vanilla title screen first opens, in every engine state.
- **Where it appears:** in stdout and `logs/latest.log`.
- **Contents:** no account, session or path data.
- **Tests:** a unit test pins the format, and every launch-test case asserts the line was logged.

## 4. Joint acceptance run (Windows x64, packaged app)

Together with Codex, on a clean Ooze install directory with a legitimate entitled Microsoft account:
1. Run the installer.
2. Complete first run.
3. Sign in in the browser. Then cancel once and retry.
4. Install with real progress. Pull the network mid-download, then retry.
5. Repair, and check that seeded `saves/` and `options.txt` survive.
6. Play. Fabric 1.21.11 runs under Java 21; `OOZE_READY … engine=LOADED` is seen and the UI shows "reached its title screen".
7. Quit Minecraft and check the exit state.
8. Restart the launcher. Play again on restored and refreshed credentials, without the browser.
9. Export diagnostics and grep the bundle for tokens. A `token`/JWT search must find nothing.
10. Run the C0b DLL failure variants.
11. Run Narrator over first run, sign-in and Play.

Record each step as Game-launched or Manually verified in `CLAUDE-L0.md` / `CODEX-L0.md`.
