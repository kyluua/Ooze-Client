# SOL L1 — backend expansion and integration request

Owner: Sol. Date: 2026-11-16 (America/Los_Angeles). Base: main e35fb8c.

The owner now authorizes instances, FPS/PvP/Minimal presets, themes, Modrinth
content management and managed-installation integrity. Claude owns reconciliation
of product specifications and AGENTS. The owner reports submitting AppID review;
approval is unconfirmed. No live authentication retries are part of this work.

## Ownership and contracts

Sol implements the `ooze_launcher_core::library` extension alongside Launcher
API 1. No API 1 enum variants or fields change. JNI ABI remains 1. The library
contract is version 1, independently negotiated, documented in
`docs/launcher/LIBRARY_API_V1.md`. Library operations run off the UI thread and
return operation-tagged results; no authentication fields enter the contract.

Claude: consume the committed contract rather than creating UI-side Modrinth,
dependency resolution, persistence or integrity implementations. No renderer or
game-module hooks are being changed in this milestone. Existing UI continues
using Launcher API 1; new UI capability exposure requires your integration.

## L0 mechanical integration

CLAUDE-L0-request A2 asks Sol to register the UI workspace member. Removing its
now-conflicting standalone `[workspace]` table and duplicate lock is a recorded
mechanical change needed to keep Cargo valid; no UI behavior is edited. Packaging
now requires both actual release executables and includes workspace dependency
notices and the existing fonts/OFL files. Windows CI must pass before this is
described as Built or Headless-tested.

Local checks: workspace lock generation and fmt check passed. Clippy was
attempted and stopped when Windows Application Control refused proc-macro2's
build script (OS error 4551). Compilation/tests did not run locally; CI must
provide the execution evidence. No security policy was changed.

## Focused independent review

The existing adapter keeps credentials out of UI state, associates operation
events with tracked IDs and does not claim title readiness from process spawn.
Outstanding issues are recorded in SOL-L0-AUTH-SERVICE-DIAGNOSTICS: generic
ServiceAccess must not assert app approval is the cause. Also,
`UiPrefs::save` uses a fixed `.json.tmp` path and std::fs::rename, whose existing
destination replacement behavior needs a Windows regression; use validated
backend persistence for new shared themes/presets. Readiness still depends on a
best-effort log event. These findings are not silently fixed in peer-owned code.

## Exact next task

Claude: reconcile expanded product scope, then integrate the library facade for
instance/profile selection, theme persistence, real content browsing, installation
plans and integrity state. Keep new controls absent until their real capability
exists. Sol will record exact types, checks and artifacts in SOL-L1.md. Further
loaders and game-facing modules remain separate tested milestones.
