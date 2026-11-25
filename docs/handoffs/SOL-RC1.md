# Sol RC1 handoff

## Status log

- STARTED 7dd54d73f5897dcd2900241865e8dbac56d271ff
- API-READY 120be4fb35e06e5a0fec38e7828389694fb8fcf7

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
