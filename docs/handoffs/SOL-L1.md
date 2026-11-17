# SOL L1 — instance/content backend and Windows desktop packaging

Owner: Sol. Date: 2026-11-16 (America/Los_Angeles). Base: e35fb8c.
Result: resolve with git log -1 -- docs/handoffs/SOL-L1.md.
Launcher API 1, additive Library API 1, JNI ABI 1. No game bridge bytes changed.

## Implemented scope and ownership

Core library facade runs filesystem/network operations on its worker, exposes
typed snapshots/results and a repaint notifier, and never handles credentials.
It provides stable isolated instances, archive-in-place, profile/theme
persistence/import/export, Modrinth search/details/versions/dependency plans,
verified transactional content install/update/disable/remove/repair and scoped
integrity checks. The CLI is an actual consumer; Claude integrates the desktop
library screens. Vanilla and Fabric adapters share pinned 1.21.11/Java 21 inputs;
Quilt/Forge/NeoForge and other versions are explicitly unavailable in L1.

The GUI is now a shared workspace member and is packaged alongside the CLI,
native-bearing Ooze JAR, release metadata, dependency notices and fonts/OFL files.
The only edits to peer files removed the conflicting UI workspace table/lock as
requested in CLAUDE-L0-request A2. No UI behavior/resources were edited by Sol.

Changed paths: launcher workspace/lock, core library and install/launch hooks,
CLI, packaging, CI and owned launcher docs/handoffs. Existing API 1 defaults and
legacy game directories retain the existing Fabric/Ooze target.

## Contract reconciliation with Claude

After initial contract publication, Claude committed the product model at
5af6369/58266ea. Sol adopted its canonical ooze.profile/1 schema and presets
through the pure Rust ooze-ui path dependency. No duplicate preset defaults or
module-setting schema are shipped by the backend. Desired settings are data;
Library capabilities reports no accepted runtime modules yet.

The original per-instance ooze.launcher.theme/1 subset remains only for source/
stored-data compatibility. Claude's G1 review was accepted: new global appearance
uses the canonical ooze.theme/1 / ooze-ui::theme_doc validator, including accent
and on-accent contrast. It persists at library/appearance.json and does not
change with the selected instance. Snapshot.appearance.document is the ThemeDoc;
commands are SaveAppearance(Appearance), ImportAppearance(String), ExportAppearance.
Existing UiPrefs bytes are untouched; Claude reconciles UI motion/scale precedence.

G2: ListWorlds(Id) returns Reply::Worlds { instance, worlds }. Each LocalWorld has
an opaque `world:<hash>` key and directory display name; pass that key into Plan.world.
The backend maps it to the existing world and revalidates on Apply. Legacy CLI
directory-name selection remains supported. capabilities().appearance and
world_listing are true for these implemented operations; UI mapping is Claude's.

## Independent review

0773976 correctly changed generic service/auth errors to neutral copy and keeps
safe backend detail visible. The new product profile model clearly separates
desired catalog settings from actual game availability. Its unknown-entry import
warnings are now preserved by the backend import result. The existing best-effort
readiness marker and UiPrefs persistence concern remain as previously recorded.
The feature catalog, statistics and HUD layout models are not game hooks or a
completed module release. Do not expose their switches without live capabilities.

## Actual checks so far (candidate; final evidence follows)

- Local fmt, lock generation, Cargo metadata and diff whitespace checks executed.
- Local compilation still stops at Windows Application Control 4551; no policy
  was changed. Compile/test evidence comes from Windows CI.
- 87c3e6c Windows run 43998205550 passed unified workspace lint/tests, actual GUI
  release compilation, native/Java 21/Fabric checks, packaging and install/repair.
- Initial library candidate ac9a7ba failed compilation (missing Id Hash and Rust
  1.99 atomic method rename). Tests/release were not claimed from it. Both are
  fixed in d48e88d; that Windows run is being checked before final acceptance.
- Controlled tests cover paths/archives, graph cycles/conflicts, type/loader
  mismatch, profile/theme validation, isolation, locks, async persistence and
  managed extra-mod detection. Live Modrinth CLI checks are a separate CI step.

No FPS gain or in-game module validation is claimed. Native kernel benchmarking
is outside this backend milestone. Full real-account Play remains blocked by
the last Minecraft HTTP 403; the owner reports submitting AppID review, with no
approval confirmation. No auth bypass or new live retry is part of L1.

## Exact Claude integration task

Read LIBRARY_API_V1.md and core/src/library/mod.rs. Open Library beside Launcher
using the same data directory, off the paint path. Keep polling both tagged event
streams; use set_notify only to schedule repaint. Render capability-backed
instance/profile/theme/content screens. Select/obtain game_dir, then SaveSettings
on API 1, await success and submit Install/Launch. API 1 selection is not changed
implicitly by library Select. Keep in-game module controls absent while runtime
capabilities are empty.

Search four ContentKind values, then Project/Versions. Plan returns a backend-held
opaque plan ID plus dependencies; review it and Apply that ID. Update is another
Plan with an explicit version. Use SetEnabled, RemoveContent and RepairContent
for registered content only. Data packs need an explicit existing local world;
shaders require compatible Iris. Managed accepts only the pinned baseline in
this release; third-party installation requires explicit ConvertToCustom.

Do not claim the expanded GUI is integrated until Claude implements/tests it.
Final verified artifact/hash and run evidence will be appended after CI finishes.

## Review follow-up after Claude P0

The adapter's Project → Versions follow-up exposed a terminal-event timing race:
Library now clears active state before emitting Finished so the next submit is
accepted immediately. A concrete Windows failure also found that FlushFileBuffers
requires a writable handle. Staged copies and duplicates now reopen with write
access before sync; a successful commit+nonempty duplicate regression covers it.
The native module catalog/theme model remains data, not an in-game module runtime.

G3–G7 richer progress, metadata, conflict/remedy and capability snapshots remain
explicit follow-up work; no hidden UI capability is claimed complete. G8 is
already available as set_notify (call only request_repaint from its callback).
R3 game_dir mapping is confirmed. R4 proposed game-side profile document path is
<game_dir>/config/oozeclient/profile.json using ooze.profile/1; writing/reading
that copy belongs to the coordinated G4/M0 atomic-store milestone, not this
launcher-only persistence implementation.
