# Sol RC1 handoff

## Status log

- STARTED 7dd54d73f5897dcd2900241865e8dbac56d271ff
- API-READY 120be4fb35e06e5a0fec38e7828389694fb8fcf7
- Preflight tested through 3a1ad9cb0ba37c611a8caede266322ea9a85b0b6. Waiting for UI-READY.
- Single-instance backend landed at `1beaf85`. The local successor pass used
  ZIP `beff2cd7c1d6f993234850cb41494b4714ed0f9cba098f19c613b843af8d35d0`.
  [Migration and Windows evidence](SOL-INSTANCE-MODE-REMOVAL.md).
  Gate L remains open pending the combined UI package and authenticated Play.
- PACKAGE-READY `b09026a9cfc884f64dc18598befc2936c9fc6441653a8f5996fa0b7d3809edd7`
  `f8ba07aaa7e4604b9edf5d3b0d8e376dfc547b96` `45728004526`.
  The combined Windows package contains the single-instance desktop UI.
  [Windows setup handoff](SOL-WINDOWS-SETUP.md) records its separate installer
  and Prism hashes. Gate L remains open for desktop Play.

Owner Sol. Target Windows x64, Fabric 1.21.11, Java 21 and the bundled Rust
engine. Gate L is open. The previous Windows runs in `SOL-GATE-L.md` cover
earlier packages. They establish a baseline for troubleshooting RC1.

## Review at start

- `ba75d41`, `launcher/crates/ooze-launcher-ui/src/core_backend.rs:111`.
  The readiness matcher searches for the render-thread header anywhere in the
  log line. A header inside message text can still satisfy it. The engine state
  also accepts any uppercase word. C2 will move exact parsing into the core and
  use a control event rather than best-effort log delivery.
- `ba75d41`, `launcher/crates/ooze-launcher-ui/src/app.rs:303`. Play and prepare
  still use the saved directory selection guard. Review found no new wrong
  instance routing or credential handling in this change.
- `fe0d670`, `docs/handoffs/CLAUDE-GATE-L.md`. Documentation-only checkpoint.
  The Linux adapter results and older Windows protocol remain scoped to their
  named artifact and platform. The backend prior to RC1 passed Windows CI run
  45510107357.
- P0 gaps remain scoped to capabilities needed for RC1: Modrinth review/install,
  selected instance and persisted appearance. G5/G6/G7 feature expansion and
  engine/module milestones stay outside this candidate.

## Launcher API 1.1 contract for Claude

Retain `LAUNCHER_API_VERSION: u32 = 1` and add
`LAUNCHER_API_MINOR_VERSION: u16 = 1`. Add
`LauncherEvent::GameReady { id: OperationId, engine_state: EngineState }`.
Rust enum variants are `Loaded`, `MissingLibrary`, `UnsupportedPlatform`,
`Incompatible`, `LoadFailed`. Map only this event tagged with the live launch
operation to the UI title state. Only `Loaded` satisfies the RC1 engine gate.
See [Launcher API](../launcher/LAUNCHER_API_V1.md) for grammar and ordering.

The old UI `LauncherEvent` match at `core_backend.rs:195` was exhaustive.
The mechanical build fix is a single no-op `E::GameReady { .. } => {}` arm
in `launcher/crates/ooze-launcher-ui/src/core_backend.rs`. Claude owns making
that arm consume readiness and reading the minor constant in
`CoreBackend::api_version()`. Keep ordinary log display.

The joint test will wait for UI-READY, API-READY and PACKAGE-VERIFIED. The
owner handles the Microsoft browser and Narrator.

## Browser callback and execution trust

The Windows host resolves localhost to both `::1` and `127.0.0.1`, and an IPv6
loopback socket can bind. The successor accepted only IPv4 before this change.
RC1 now listens on both loopback addresses on the same ephemeral port, while
keeping the registered localhost redirect, the exact callback path and state,
PKCE, the existing decline message and the owned client ID. Local HTTP fixtures
passed on both families. This establishes a reachability gap and its fix. The
cause of the previous three-minute desktop timeout still requires a live attempt
with the owner. Record only browser name, whether it reached the localhost
endpoint and whether a firewall prompt appeared. Omit the URL and query string.

The package is unsigned. The last recorded execution block named the Windows
Smart App Control policy. Windows reports that policy enabled on this host.
Current choices and their setup are in
[Windows execution trust](../launcher/EXECUTION_TRUST.md). Signing setup is
pending the owner's signer or administrator-managed test host.

Claude: please update the UI crate's explicit package version and the Fabric
producer's `mod_version` to `0.1.0-rc1` in your owned files before UI-READY.
Sol owns the launcher workspace version and lockfile reconciliation.

## Packaging and Prism

The launcher workspace and package metadata use `0.1.0-rc1`. The Windows job
uses the exact Gradle profile JAR and verifies its digest before packaging. It
checks the license bundle, original fonts and accompanying texts, managed
install/repair and diagnostics. The separate Prism job output contains the
pack and its hash/size JSON. Only the pinned Fabric API download and the same
official Ooze client JAR are included. Fabric API's CDN bytes match the
launcher pin and Modrinth's SHA-1 and SHA-512.

Local preflight built the release executables, ran workspace Clippy, built a
Java 21 production JAR with the RC1 version supplied on the command line, and
validated the package and modpack scripts. Preflight pack SHA-256
`d00b8917b8bbe606b24d5fc52d308956e7b79bdc7e0a1aa7d2821dd48f55c0c1`.
Its Fabric JAR is 349,022 bytes, SHA-256
`9a8ed200b7f19f2dfc5edb12eebfa600e347c5f5d1a448c1023184c54332feea`.
These are preflight bytes, before the combined UI-READY candidate. The final
pack must use the combined package's JAR.

Windows preflight passed 54 backend tests, 99 ordinary UI tests and both live
Modrinth UI tests. CI now runs the UI renderer tests serially on DX12 and runs
the two live cases separately. Prism Launcher 11.1.1 parsed the preflight pack
and staged the exact Ooze JAR under a new isolated root. Its trust prompt for
the bundled JAR remains pending. Instance creation and Play were not reached.

The local Codex Git runtime contains Bash as `sh.exe`, but lacks helper
executables required by `tools/launch-tests.sh` (including `dd`, `tee` and
`cygpath`). Automatic approval review blocked the Portable Git download and
extractor command with “blocked by policy.” The existing Windows PowerShell
adapter matrix can run separately. Its evidence is distinguishable from the
requested Git Bash invocation.

## Windows native preflight

Source `3a1ad9cb0ba37c611a8caede266322ea9a85b0b6`. **Built** the fresh
Windows x64 G0 DLL and **Headless-tested** 4 core, 1 JNI and 24 product model
tests. The Java 21 JNI harness passed 20,323 assertions. DLL SHA-256
`48fb9e40d4f0252d3650b19b63238b22770ee44240bfdc1399ee73eec3a85c09`.

**Game-launched**, the local PowerShell adapter matrix passed all five
preflight cases. Dev and production reached the title screen with
`OOZE_READY v1 screen=title engine=LOADED`. Real missing DLL reached
`MISSING_LIBRARY`; corrupt and ABI mismatch reached `INCOMPATIBLE`. Each
ran on Windows 11 Pro build 26300, AMD Ryzen 7 9800X3D, Radeon RX 6900 XT and
Temurin Java 21.0.12.1+1 x64. Evidence is in the ignored local directory
`launcher/test-data/rc1-native-preflight-matrix/`. This DLL and matrix are
preflight evidence, separate from the ZIP used for W1–W14.

## Remaining checkpoints

The Windows preflight package is unsigned. The CLI and GUI Authenticode
statuses were both NotSigned. The package bundled 332 dependency notice
records and hash-checked the shipped fonts and their notices. The backend and
packaging source through `3a1ad9c` is on main. Windows CI for that intermediate source is
[run 45558530051](https://github.com/kyluua/Ooze-Client/actions/runs/45558530051).
The original main worktree retains the owner's staged LICENSE and README
deletions. No normal launcher or Prism profile was used.

Sol changed core `api.rs`, `auth.rs`, `backend.rs`, `lib.rs`, `process.rs` and
`readiness.rs` plus the launcher workspace manifest and lock. Packaging
changes are in `launcher/packaging/` and `.github/workflows/launcher-l0.yml`.
The one mechanical UI line is recorded
above. Contract, trust and build docs are in `docs/launcher/`. The final
handoff checkpoint is the latest commit of this file.

Wait for Claude's `UI-READY`, review the incoming diff, then build and verify
the green combined Windows CI package. Record its hashes and the matching
Prism artifact, publish `PACKAGE-READY`, and wait for `PACKAGE-VERIFIED`.
There is no RC1 acceptance hash yet. W1–W14 and the exact RC1 adapter matrix
remain open. W3 and W14 require the owner for browser sign-in/decline and
Narrator. B6 is deferred until its prerequisite checks pass.
