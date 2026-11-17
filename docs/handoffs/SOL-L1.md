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
Final verified artifact/hash and run evidence is recorded below.

## Review follow-up after Claude P0

The adapter's Project → Versions follow-up exposed a terminal-event timing race:
Library now clears active state before emitting Finished so the next submit is
accepted immediately. A concrete Windows failure also found that FlushFileBuffers
requires a writable handle. Staged copies and duplicates now reopen with write
access before sync; a successful commit+nonempty duplicate regression covers it.
The native module catalog/theme model remains data, not an in-game module runtime.

G3 backend progress is implemented as Event::Progress { id, stage, done, total,
item }. Stage is Contacting/Downloading/Verifying/Committing. Downloads are
throttled to first/last plus 50 ms; Committing is reliable and closes cancellation
atomically. Claude: map the event and hide Cancel once Committing arrives.
G4–G7 richer metadata, conflict/remedy and capability snapshots remain
explicit follow-up work; no hidden UI capability is claimed complete. G8 is
already available as set_notify (call only request_repaint from its callback).
R3 game_dir mapping is confirmed. R4 proposed game-side profile document path is
<game_dir>/config/oozeclient/profile.json using ooze.profile/1; writing/reading
that copy belongs to the coordinated G4/M0 atomic-store milestone, not this
launcher-only persistence implementation.

## Final code and actual verification

**Code result:** main `43c83d2025b204b3d9e337570ed598f73fcba775`.
Base: `e35fb8c2e7050852ce38f55daaf9b963dfe18e95`. This handoff's commit is
discoverable with git log -1 -- docs/handoffs/SOL-L1.md. The backend component is
Built and Headless-tested; joint L1/product release acceptance remains open.

[Windows run 44012177547](https://github.com/kyluua/Ooze-Client/actions/runs/44012177547)
passed both jobs (133801543599, 133803443268): fmt/clippy, 46 core backend tests,
86 UI tests (two intentionally ignored network tests), release CLI and GUI,
4 native core + 1 JNI + 24 product-model Rust tests, and the real Java
21.0.13+9-LTS amd64 bridge harness with 20,323 assertions (script and Gradle).
Fabric build/tests passed. The pinned package repeat matched; with developer
tools/JDK removed from PATH, actual managed-Java install/repair and baseline
verification passed for both Vanilla and Fabric 1.21.11, preserving options.txt.

The real CLI/API step passed searches for all four content types, exact Iris
ka9PDlsN + required Sodium rkdTcxoT installation, hash verification, disable/
enable, modified-file detection and explicit repair, canonical profile/global
appearance checks, an empty local-world listing and extra-mod detection without
deletion. These are not Game-launched checks. Separate actual owner-Windows CLI
runs of verified 166d88a and ac2f273 candidates repeated the content suite; the
ac2f273 run also queried real Iris versions and checked game/loader compatibility.
The final packaged CLI's --version and managed-Java preflight ran locally:
an unvalidated custom Java path was refused before installation, then the test
settings were restored to managed Java. No sign-in or game spawn was attempted.

The last policy correction checks managed Java before installer/runtime or
authentication work, while preserving startup recovery for an archived prior
selection. It does not change API 1 or JNI ABI 1.

## Final review artifact

GitHub artifact **14297318859**, `ooze-launcher-windows-x64`, source 43c83d2,
13,929,553 bytes. Both release executables, real native-bearing Ooze JAR, pinned
release metadata, 332 dependency-notice entries and unmodified fonts/OFL files
are included. All **598 files** in SHA256SUMS verified locally after checking
the downloaded archive digest, before the CLI was executed.

| Artifact | SHA-256 |
|---|---|
| ZIP | `00e458dda0974b0c70271f5f13ff75c820ca5170b96a818ca845c873b4087a7b` |
| ooze-launcher.exe | `ad1cc2040740f25a78d9cf13ca86f32e8903b7514e00e9852ecb3f55212a0493` |
| ooze-launcher-cli.exe | `c600187c70f2a95f94672cfe23bdc7eab45f32f55da745cdc55969f76687b798` |

Ignored local delivery: launcher/dist/ooze-launcher-43c83d2.zip and extracted
launcher/dist/ooze-launcher-43c83d2/. This is an unsigned review package, not a
player release or proof of a working signed-in Minecraft session. The CLI still
reports the existing workspace version 0.1.0-l0; commit/checksum identifies this
L1 candidate, and release branding is not silently treated as a new release.

## Measurements and limitations

The committed benchmark-166d88a.json records the actual candidate CLI hash,
Windows 10.0.26300 x64, three new-process samples each, an existing two-mod test
library and unflushed OS file caches. Median startup/list was 37.428 ms;
startup plus registered-content verification was 60.682 ms. Observed peak process
working-set samples were about 3.2–10.3 MB at 5 ms sampling; they are not native
allocator retained/peak measurements. GUI frame time, Java allocation, JNI/copy/
serialization cost, GPU residency and Java kernel comparisons remain unmeasured.
There is no FPS-improvement claim. See packaging/measure-library.ps1 for commands.

Local Rust compilation remains blocked by Windows Application Control 4551 on
dependency build scripts. No policy was changed. Automatic approval separately
rejected the attempted downloaded GUI launch with only “blocked by policy” as
the reason. The package was subsequently downloaded/hash-verified without
opening its GUI. No local window or Narrator pass is claimed.

The original main checkout's staged LICENSE/README deletions remain untouched;
Sol used the existing clean checkout and published to main with checked leases,
fetch/merge and no force-push. No accounts, credentials or Minecraft player data
were imported into the library or evidence.

## Joint acceptance and next task

Read SOL-L1-CLAUDE-P0-REVIEW.md: fix selection/SaveSettings sequencing before Play,
consume global appearance, world listing, progress and the notifier, and retain
capability gating. L1's profile UI gate depends on actual runtime modules; do not
call the joint milestone complete while runtime_modules is empty. Reconcile that
dependency with G1/G4/M0 while preserving Real or absent. The current release GUI
does consume real instance/content operations, but remaining mappings are explicit.

L2 still needs per-content-type Game-launched evidence, richer catalog/conflict
and integrity-remedy states, world-scoped management and remaining G4–G7 requests.
L3 other versions/Quilt/Forge/NeoForge remain unavailable. G1/G4/M0–M3 own actual
game snapshots, frame/text/icon commands, atomic in-game configuration and module
hooks; this milestone did not change the game bridge. Full authenticated Play
and title-screen readiness remain unverified pending confirmation of app access
and an entitled-account Gate L run. No new live authentication retry was made.
