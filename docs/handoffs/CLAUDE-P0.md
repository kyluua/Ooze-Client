# CLAUDE P0 — Product reconciliation, module model and library interface

- **Owner:** Claude (product lead: Fabric integration, in-game UI, launcher interface).
- **Milestone:** P0 (`IMPLEMENTATION_PLAN.md`, owner scope expansion 2026-11-17).
- **Base:** `0773976` (main, after Sol's `87c3e6c` library contract).
- **Result:** `6141efd` (code) plus the commit that adds this file. Merged with `origin/main` before the push.

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

## 2. Changed paths (mine)

- `AGENTS.md`
- `docs/PRODUCT_VISION.md`, `docs/IMPLEMENTATION_PLAN.md`, `docs/ARCHITECTURE.md`, `docs/TARGET_PLATFORM.md`, `docs/FEATURE_BACKLOG.md`
- `docs/MODULES.md` (new) and `docs/launcher/LIBRARY_UI.md` (new)
- `docs/handoffs/CLAUDE-P0-request.md`, `docs/handoffs/CLAUDE-P0.md` and `docs/handoffs/evidence/CLAUDE-P0/*.png`
- `native/crates/ooze-ui/**` (new)
- `launcher/crates/ooze-launcher-ui/**`: `Cargo.toml` (adds the `ooze-ui` path dependency), plus `app.rs`, `app/library_pages.rs`, `appearance.rs`, `library.rs`, `library_model.rs`, `library_fixture.rs`, `library_ui_tests.rs`, `widgets.rs`, `i18n.rs`, `core_backend.rs` (test only), `lib.rs` and `bin/ooze-launcher-preview.rs`
- `launcher/resources/lang/en_us.json`

**Mechanical changes to Sol's files** (needed to keep the build valid; no behaviour changed):
- `native/Cargo.lock`: `ooze-ui` entry. The crate joins Sol's `members = ["crates/*"]` automatically.
- `launcher/Cargo.lock`: `ooze-ui` entry (path dependency of the UI crate).

## 3. Contract versions

| Contract | Version |
|---|---|
| JNI ABI | 1 (unchanged) |
| Launcher API | 1 (unchanged) |
| Library API | 1. Consumed as documented; the interface mirror is v1 and awaits Sol's exact types (request R2) |
| Schemas | `ooze.profile/1`, `ooze.preset/1`, `ooze.theme/1` |

## 4. Evidence

| Check | Result |
|---|---|
| `native`: `cargo fmt --all --check`; `cargo clippy -p ooze-ui --all-targets -D warnings` (Linux **and** `--target x86_64-pc-windows-msvc`); `cargo clippy --workspace --all-targets -D warnings`; `cargo test --workspace` | **Built**, **Headless-tested** (Linux): 29 tests passed (24 in `ooze-ui`) |
| `launcher/crates/ooze-launcher-ui`: `cargo fmt --check`; `cargo clippy --all-targets --features dev-fixtures -D warnings`; `cargo test --features dev-fixtures` | **Built**, **Headless-tested** (Linux, lavapipe): **84 passed** (56 L0 plus 28 new: 20 library page tests, 3 model, 3 appearance, 1 prefs, 1 banned-claims scan) |
| `launcher` workspace: `cargo fmt --all --check`; `cargo test --workspace` (without `dev-fixtures`) | **Headless-tested** (Linux): 100 passed, 0 failed |
| `launcher` workspace: `cargo clippy --workspace --all-targets -D warnings` on Linux | **Fails in Sol's `ooze-launcher-core`** (`game_guard.rs:63`, unused test import on non-Windows). This is pre-existing and not from this change (R5) |
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

- **No real library yet.**
  - Sol's `ooze_launcher_core::library` does not exist in code, so the release launcher shows none of P0's pages.
  - There is no real Modrinth browsing, instance isolation, persisted theme or installed content yet. Those are the **L1 gate**, after Sol's implementation and my adapter.
  - Nothing in P0 is Game-launched.
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

**`87c3e6c` (library contract, packaging):**
- Correct: registering `ooze-launcher-ui` in the workspace, and removing my crate's standalone `[workspace]` table and lock.
- Packaging now requires `-GuiExe`, and Windows CI runs the whole workspace's tests on windows-latest with dx12. That means my kittest suite (including the new library tests and the prefs regression) runs on Windows.
- `LIBRARY_API_V1.md` is clear on the cancellation boundary, opaque plans, CDN-only downloads, no-delete integrity and honest limits. My only contract requests are those in `CLAUDE-P0-request.md` (R1–R4).

**`ca0698d` / `f254af5` (service denial diagnostics):** consumed in `0773976` (neutral copy). No further findings.

**Finding:** `cargo clippy --workspace --all-targets -- -D warnings` fails on Linux at `crates/ooze-launcher-core/src/game_guard.rs:63` (unused `use super::*`). It is low severity because Windows CI is unaffected. Left for Sol.

## 7. Exact next task

**Sol (L1):**
1. Implement Library API 1 per `LIBRARY_API_V1.md` with the shapes in `CLAUDE-P0-request.md` R2. Validate profiles and themes with `ooze-ui` (R1).
2. Answer R3 (the game-directory mapping for Play).
3. Publish the exact types, checks and artifacts in SOL-L1.md.

**Claude (L1), after SOL-L1:**
1. Write `core_library.rs`, an adapter from Sol's types to `library.rs`, with mapping unit tests.
2. Attach the real library in `bin/ooze-launcher.rs`.
3. Run the L1 gate on Windows x64:
   - create a Fabric 1.21.11 instance;
   - choose PvP;
   - persist a theme;
   - real Modrinth search;
   - install one Fabric mod with a required dependency;
   - Play to `OOZE_READY` in that instance, then Game-launched;
   - Narrator pass (Manually verified, if the owner can).

**Claude (M0), in parallel once G1/G4 land:** the in-game module runtime from the `ooze-ui` catalog.
