# CLAUDE P0 — Product reconciliation, module model and library interface

- **Owner:** Claude (product lead: Fabric integration, in-game UI, launcher interface).
- **Milestone:** P0 (`IMPLEMENTATION_PLAN.md`, owner scope expansion 2026-11-17).
- **Base:** `0773976` (main, after Sol's `87c3e6c` library contract).
- **Result:** `6141efd` (P0 code), then the L1 start: Sol's library merged (`ac9a7ba`, `d48e88d`) and the real adapter added. Merged with `origin/main` before the push. Hashes: `git log`.

## 1. What was delivered

**1. Specs reconciled** with the owner's expansion:
- real modules;
- FPS/PvP/Minimal profiles;
- isolated instances;
- Modrinth content;
- themes;
- Managed Fair Play / Custom integrity.

Files and changes:
- `PRODUCT_VISION.md`: scope.
- `IMPLEMENTATION_PLAN.md`: new milestones P0, L1–L3 and M0–M3. C5 now keeps only the shell, and C9 is superseded by M1.
- `ARCHITECTURE.md`: §8.1a profiles, §10 HUD, §19.1 library, ADR-017 (instance/profile/theme separation), ADR-018 (one module catalog plus fair-play policy).
- `TARGET_PLATFORM.md`: the instance table.
- `FEATURE_BACKLOG.md`: F→M mapping.
- `AGENTS.md`: module and instance rules.

**2. Sourced module matrix and fair-play policy** (`docs/MODULES.md`):
- Bunny, Lunar and Badlion are recorded honestly. Pages that were not inspected are marked so; there is no claim of a full catalog.
- Modules M-01…M-14 each carry a class, compatibility, dependencies and status.
- The permanently excluded list is kept.

**3. `native/crates/ooze-ui`** (new crate, standard library only, `#![forbid(unsafe_code)]`). It is the one catalog for the game, the library and the launcher:
- `catalog`: modules, fair-play classes, typed settings;
- `settings`: validation;
- `profile`: `ooze.profile/1` documents and `ooze.preset/1` FPS/PvP/Minimal; layering defaults → preset → overrides; diff, reset, duplicate, strict import with warnings;
- `hud`: anchors, clamping, snapping, scale;
- `stats`: CPS ring and frame statistics, both allocation-free;
- `json`: a bounded parser;
- `theme_doc`: new, see item 5.

**4. Launcher library interface** (`launcher/crates/ooze-launcher-ui`), built to `LIBRARY_UI.md`:
- `library.rs`: the interface's typed mirror of Library API 1.
- `library_model.rs`: the reducer. Stale results are ignored; there is one operation at a time; Committing marks the commit boundary.
- `app/library_pages.rs`: the Instances, Profile, Content and Appearance pages.
  - **Instances:** create (only reported combinations); Select for Play (API 1 `UpdateSettings` with the instance's game directory); rename/duplicate/archive with honest confirmations; per-instance integrity report with Repair / Quarantine / Open folder, and **no delete**; mode switch with confirmation.
  - **Profile:** preset cards with "What this changes" computed from `ooze_ui::profile::diff`, and the FPS card's "no performance mods, no promised gain". A preset switch keeps overrides (optional reset) and lists every change. There are module switches with class tags and a server-sensitive notice; GAMEPLAY modules are not listed. Typed editors (switch, stepper, segmented, hex colour, key capture) all go through `SettingDef::validate`. Also: save/discard/reset, export to the clipboard, and import with review and warnings.
  - **Content:**
    - kinds offered per instance (Vanilla: resource and data packs only);
    - search with sort, category filter and paging; results "compatible … as reported by Modrinth";
    - details: plain-text description, links show their host, incompatible versions show the backend's reason and can't be chosen;
    - dependency review: required, optional (not installed), embedded and conflicts. A conflict blocks Install, and so does a missing shader loader;
    - install: progress, Cancel until the commit boundary, then "Finishing — can't cancel now"; the result reads **"Installed and verified"** with a per-kind hint, never "working";
    - data packs need an existing local world and are refused while the game runs; shader packs explain their loader;
    - installed list with update, enable/disable and remove (recorded files only).
  - **Appearance:** palette accents and custom hex behind the shared contrast guard (the reason and measured ratio are shown); contrast Standard/High; text size 90–130 %; interface scale; background Drips/Gradient/Plain; motion. Live preview across the whole window, plus Apply (library `SaveTheme`, with API 1 `UpdateSettings` for motion and scale), Revert, Reset to Ooze Classic, and import/export of `ooze.theme/1`.
- **Gating:** every page, tab and action appears only with its library capability. **The release binary attaches no library today, so it is exactly the L0 launcher** (test `without_a_library_the_launcher_has_no_library_pages`).
- **Fixture library** (`library_fixture.rs`): only under `cfg(test)` / `dev-fixtures`. Every name says "fixture". The developer preview attaches it with `ooze-launcher-preview -- ready library`.

**5. `ooze.theme/1`** (`ooze_ui::theme_doc`):
- strict data-only document: unknown fields are refused, no URLs, paths or fonts;
- the contrast guard reads the canonical token file: text on the accent ≥ 4.5:1, accent vs canvas ≥ 3:1;
- shared with Sol's library (request R1).

**6. Fixes:**
- Selected segmented-control labels were painted ink on the Ink Gel and were unreadable. This was an L0 bug in my widget: `painter.galley`'s colour is only a fallback. It is now fixed and affects every radio group, including Settings.
- Added the `UiPrefs::save` replace regression test Sol requested.

**7. Strings:** 352 new `en_us` keys. Tests check:
- every key used exists;
- every key is used, including keys built from the catalog;
- **no banned claim** appears in any string: cheat-proof, undetectable, anticheat, server-approved, safe on all servers, guaranteed, ready to use.

**8. L1 started: the real library is attached** (Sol pushed `ac9a7ba`/`d48e88d` during P0).
- `core_library.rs` adapts `ooze_launcher_core::library` to the interface types. A project's details take two library operations and are reported as one; profile load and the installed list are answered from Sol's snapshot.
- `bin/ooze-launcher.rs` opens the library with the launcher's data directory. If opening fails, a `FailedLibrary` shows the error on the Instances page.
- **Capabilities come from Sol's `Library::capabilities()` and report only what is real today:**
  - instances: create Fabric 0.20.1 / Vanilla on 1.21.11, select, rename, duplicate, archive, convert Managed → Custom;
  - **profiles are hidden** until `runtime_modules` is non-empty. The game can't run any module before M0, so module switches would be decorative (Real or absent; this agrees with Sol's SOL-L1). The adapter still maps load and save, with a revision check; that is tested;
  - content: mods, resource packs and shader packs; search, details and versions; dependency plan; apply; installed list; enable/disable/remove;
  - integrity: verify.

  Hidden until Sol adds them (see the updated request):
  - Appearance: the library's theme is per instance and uses a different schema;
  - data packs: no world list;
  - sort and categories, switching to Managed, and progress steps.
- The live tests found that real Modrinth data includes CJK titles that the shipped fonts can't draw. The app now sets egui's Tofu policy explicitly; AccessKit still reads the real text. Bundling a wider-coverage font (for example Noto Sans CJK, OFL, around 16 MB) is an **owner decision**.

## 2. Changed paths (mine)

- `AGENTS.md`
- `docs/PRODUCT_VISION.md`, `docs/IMPLEMENTATION_PLAN.md`, `docs/ARCHITECTURE.md`, `docs/TARGET_PLATFORM.md`, `docs/FEATURE_BACKLOG.md`
- `docs/MODULES.md` (new) and `docs/launcher/LIBRARY_UI.md` (new)
- `docs/handoffs/CLAUDE-P0-request.md`, `docs/handoffs/CLAUDE-P0.md` and `docs/handoffs/evidence/CLAUDE-P0/*.png`
- `native/crates/ooze-ui/**` (new)
- `launcher/crates/ooze-launcher-ui/**`: `Cargo.toml` (adds the `ooze-ui` path dependency), `core_library.rs` and `bin/ooze-launcher.rs` (real library attached), plus `app.rs`, `app/library_pages.rs`, `appearance.rs`, `library.rs`, `library_model.rs`, `library_fixture.rs`, `library_ui_tests.rs`, `widgets.rs`, `i18n.rs`, `core_backend.rs` (test only), `lib.rs` and `bin/ooze-launcher-preview.rs`
- `launcher/resources/lang/en_us.json`

**Mechanical changes to Sol's files** (needed to keep the build valid; no behaviour changed):
- `native/Cargo.lock`: `ooze-ui` entry. The crate joins Sol's `members = ["crates/*"]` automatically.
- `launcher/Cargo.lock`: `ooze-ui` entry (path dependency of the UI crate).

## 3. Contract versions

| Contract | Version |
|---|---|
| JNI ABI | 1 (unchanged) |
| Launcher API | 1 (unchanged) |
| Library API | 1, Sol's `ooze_launcher_core::library` (`LIBRARY_API_VERSION = 1`), consumed through `core_library.rs` |
| Schemas | `ooze.profile/1`, `ooze.preset/1`, `ooze.theme/1` |

## 4. Evidence

| Check | Result |
|---|---|
| `native`: `cargo fmt --all --check`; `cargo clippy -p ooze-ui --all-targets -D warnings` (Linux **and** `--target x86_64-pc-windows-msvc`); `cargo clippy --workspace --all-targets -D warnings`; `cargo test --workspace` | **Built**, **Headless-tested** (Linux): 29 tests passed (24 in `ooze-ui`) |
| `launcher/crates/ooze-launcher-ui`: `cargo fmt --check`; `cargo clippy --all-targets --features dev-fixtures -D warnings`; `cargo test --features dev-fixtures` | **Built**, **Headless-tested** (Linux, lavapipe): **87 passed, 2 ignored** (the live tests; see below). That is 56 L0 tests plus 31 new: 20 library page, 3 model, 3 appearance, 3 adapter over Sol's real library, 1 prefs, 1 banned-claims scan |
| `launcher` workspace: `cargo fmt --all --check`; `cargo test --workspace` (without `dev-fixtures`) | **Headless-tested** (Linux): 100 passed, 0 failed |
| `launcher` workspace: `cargo clippy --workspace --all-targets -D warnings` on Linux | **Fails in Sol's `ooze-launcher-core`** (`game_guard.rs:63`, unused test import on non-Windows). This is pre-existing and not from this change (R5) |
| `ooze-launcher-ui` over **Sol's real library**, Linux, no network: `the_real_library_round_trips_through_the_interface_types` (create → select → profile save with revision check → installed list → rename → archive); `unsupported_combinations_are_refused_by_the_library_with_its_reason` | **Headless-tested** (Linux) |
| **Live Modrinth**, Linux, `--ignored`: `live_modrinth_search_details_plan_install_and_verify` | **Headless-tested** (Linux, real network). Real search returned results for mods (20 of 13,242), resource packs (20 of 18,947) and shader packs (20 of 766). Iris details and versions were read. The plan was `sodium-fabric-0.8.14+mc1.21.11.jar` (required) plus `iris-fabric-1.10.8+mc1.21.11.jar` (selected). Apply downloaded and verified both (SHA-512), the installed list shows 2 items, and the integrity check reports 0 issues in 2 files. **This is not Game-launched: no Minecraft started** |
| Live window, `--ignored`: `live_window_shows_real_modrinth_results_for_a_fabric_instance` | **Headless-tested** (Linux, real network, real library; API 1 side is the fixture, hence "FixturePlayer"). Screenshots: `lib-live-content-results.png` (`ed561e193fe20421b0fa29c541cb89397b47ad9f65b88c1173f47a7e4748dd91`) and `lib-live-instances.png` (`b933307a4f7efb3679a968cfd8e888b37e0dde19f66c75137caefcb878400e54`) |
| `launcher` workspace after merging Sol's library (through `e36cbb2`): `cargo test --workspace` | **Headless-tested** (Linux): 122 passed, 0 failed, 2 ignored (the live tests) |
| Windows: launcher type-check for `x86_64-pc-windows-msvc` | **Not run locally:** `ring` (Sol's TLS dependency) needs the MSVC C toolchain. Sol's Windows CI (`launcher-l0.yml`, `cargo test --workspace` on windows-latest, dx12) is the Windows evidence after this push |
| Gradle / Fabric | Not run: no Java, Gradle or game changes in P0 |
| Game-launched | **None.** Nothing Minecraft-facing changed; no module runs in game yet (M0+) |
| Manually verified | **None** |

**Screenshots** (`docs/handoffs/evidence/CLAUDE-P0/`, fixture library, Linux lavapipe). They show appearance only, not backend behaviour:

| File | SHA-256 |
|---|---|
| `lib-instances.png` | `c9735709da6022639e43c4186cab5bf0ed61ad0f9b0bf8ad6beaa809a6148856` |
| `lib-integrity.png` | `e88e16190b1779044a20d9be1096e717e28e6d95091e42b603a5f23953ded921` |
| `lib-profile.png` | `695f338a2ff53dde4b1f59cfee5d4deaced1bc4dc05ac4c94bfebcdd6b1beb6d` |
| `lib-content-review.png` | `f654cb4445aa2caa01f176fdf6b5a2c899933e301d4b62c28bec9411944bda8b` |
| `lib-content-installed.png` | `5b9d875a887b51a76e7a3cc28a962cd9f660dfd363c474f727ac55da67abcbca` |
| `lib-appearance-preview.png` | `9642332e29d0224afe107086a72f942899cfdf444616052adad6c9c3b3bb9221` |

## 5. Limitations

- **L1 is not passed.** The real library is attached and Headless-tested on Linux, including live Modrinth. Still open:
  - Windows CI on this push;
  - a persisted theme (Appearance stays hidden until request G1);
  - **Play of an installed instance to `OOZE_READY` on Windows x64**, needed for Game-launched (R3 decides the directory mapping);
  - a Narrator pass.

  Nothing in P0 is Game-launched.
- **Non-Latin text** in Modrinth data draws as boxes (owner decision on a wider font).
- **Authentication:**
  - The owner's AppID review is submitted, and approval is unconfirmed.
  - Microsoft and Xbox passed; Minecraft `login_with_xbox` previously returned 403.
  - Nothing here claims sign-in or Play is verified.
- **Modules in game:** not started. M0 needs G1 (frame protocol), G4 (config store) and request R4.
- **Key capture** covers letters, digits, F1–F12, arrows, space, tab, enter and backspace. Mouse buttons can't be bound from the launcher yet.
- **HUD placement** is edited only in game (M1). The launcher offers only "Reset HUD layout".
- **Contrast at High** is token-level: outlines and secondary text. A full high-contrast palette is not designed yet.
- The 130 % text size was checked only in unit tests (token sizes and control heights). No screenshot review at that size has been done yet.

## 6. Review of Sol's latest changes

**`e36cbb2` / SOL-L1:**
- Answers **R3**: on Select, the interface submits API 1 settings with `install_root = game_dir`, then Play. This matches Sol's "SaveSettings, await, then Install/Launch". The UI already does it on Select and never changes API 1 implicitly otherwise.
- Adds `Library::capabilities()`, which the adapter now uses.
- On the theme, Sol names its subset `ooze.launcher.theme/1` and asks me to "reconcile product copy". I keep **Appearance hidden** rather than ship a per-instance theme, for two reasons:
  - ADR-017 (the appearance must not change with the selected instance);
  - Sol's validator lacks the accent/on-accent contrast check.

  G1 stands: one global `ooze.theme/1` through `ooze_ui::theme_doc`, which Sol already links.


**`87c3e6c` (library contract, packaging):**
- Correct: registering `ooze-launcher-ui` in the workspace, and removing my crate's standalone `[workspace]` table and lock.
- Packaging now requires `-GuiExe`, and Windows CI runs the whole workspace's tests on windows-latest with dx12. That means my kittest suite (including the new library tests and the prefs regression) runs on Windows.
- `LIBRARY_API_V1.md` is clear on the cancellation boundary, opaque plans, CDN-only downloads, no-delete integrity and honest limits. My only contract requests are those in `CLAUDE-P0-request.md` (R1–R4).

**`ca0698d` / `f254af5` (service denial diagnostics):** consumed in `0773976` (neutral copy). No further findings.

**`ac9a7ba` / `d48e88d` (library implementation)**, reviewed before integrating:
- Good:
  - profiles validated with `ooze-ui` (R1 for profiles);
  - typed, ID-validated commands;
  - CDN-only downloads with SHA-512;
  - data-pack world confined to `saves/<name>` with `level.dat`;
  - one operation at a time, with cancellation;
  - honest refusal text for unsupported targets.
- **Finding 1 (design):** the theme is stored **per instance** (`Instance.theme`, `SaveTheme{instance}`). That contradicts ADR-017: the launcher's appearance would change with the selected instance. Its validation also checks only background vs foreground, not text on the accent. Please adopt `ooze_ui::theme_doc` as one global theme (G1).
- **Finding 2:** `versions()` filters by game version only, so Fabric-incompatible versions are listed and fail later at Plan. Please also pass Modrinth's `loaders` filter for the instance's loader.
- **Finding 3:** search hits drop Modrinth's `author`, `downloads` and `date_modified`, and versions drop `version_type`, `date_published` and file size. The interface shows them when present (G4).

**Finding:** `cargo clippy --workspace --all-targets -- -D warnings` fails on Linux at `crates/ooze-launcher-core/src/game_guard.rs:63` (unused `use super::*`). It is low severity because Windows CI is unaffected. Left for Sol.

## 7. Exact next task

**Sol:** close the gaps in `CLAUDE-P0-request.md` (G1–G8) and answer R3.
- G1, the global `ooze.theme/1`, unblocks Appearance and the L1 theme item.
- G2, world listing, unblocks data packs.
- Then say in SOL-L1.md which of them landed.

**Claude (L1), next:**
1. As each gap lands, map it in `core_library.rs` and turn on its capability.
2. On Windows x64 (owner machine or CI artifact):
   - install the packaged launcher;
   - create a Fabric 1.21.11 instance;
   - choose PvP;
   - install Iris + Sodium through the window;
   - Play to `OOZE_READY` in that instance, then **Game-launched**;
   - Narrator pass, then **Manually verified** if the owner can.
3. Owner decision: bundle a CJK-capable font or not.

**Claude (M0), once Sol's engine milestones G1 (frame protocol) and G4 (config store) land:** the in-game module runtime from the `ooze-ui` catalog.
