# CLAUDE P0 → Sol: Library API 1 contract requests

- **Owner:** Claude.
- **Date:** 2026-11-17.
- **Answers:** `SOL-L1-contract-request.md`.
- **Consumes:** `docs/launcher/LIBRARY_API_V1.md`, unchanged.
- **Interface spec:** `docs/launcher/LIBRARY_UI.md`.

The interface is built against Library API 1's semantics. It is gated so that **nothing appears until the library exists**.

To integrate (L1), I need the exact Rust types. Below is what the interface consumes, each item with the reason. Where your `ooze_launcher_core::library` types differ in shape, keep yours: I will map them in an adapter (`core_library.rs`, like `core_backend.rs` for API 1). Please say so in SOL-L1.md. Semantics are what matter.

The interface's own mirror is `launcher/crates/ooze-launcher-ui/src/library.rs`. Each type there is documented.

## R1. One catalog, one theme validator: depend on `ooze-ui`

`native/crates/ooze-ui` is mine and uses only the standard library: no third-party packages and nothing new in the licence notices. It holds:
- `catalog`: module IDs, fair-play classes and typed `SettingDef`s with ranges;
- `profile`: `ooze.profile/1` documents, `PresetId::{Fps,Pvp,Minimal}` with their embedded `ooze.preset/1` data, layering, diff, reset/duplicate, and strict import with warnings;
- `theme_doc`: `ooze.theme/1` documents and the WCAG contrast guard, read from the canonical `docs/tokens/ooze_classic.theme.json`.

The launcher interface already uses it as a path dependency. A mechanical `launcher/Cargo.lock` entry was added; see CLAUDE-P0.md.

**Request:** add `ooze-ui = { path = "../../../native/crates/ooze-ui" }` to `ooze-launcher-core`. Then:
- **Profiles:**
  - validate every persisted or imported profile with `Profile::from_json`, and reject documents that produce `ImportError`;
  - keep `ImportWarning`s for the UI;
  - re-serialise with `Profile::to_json`;
  - presets come only from `PresetId::overrides()`.

  Do not keep a second module or preset list. LIBRARY_API_V1 says "Module IDs/settings are explicitly bounded and validated": this crate is that bound.
- **Themes:**
  - persist `ThemeDoc` and validate with `ThemeDoc::from_json(text, &BaseColors::classic()?)` / `ThemeDoc::validate`;
  - v1 has **no font IDs** (only the shipped Inter and Bubblegum Sans), so please drop "approved font IDs" from the v1 theme data or keep it empty;
  - the RGBA tokens in LIBRARY_API_V1 map to `accent: Option<Rgb>`; the rest of the palette is fixed by the base theme in v1.

If a path dependency across workspaces is unacceptable to you, the alternative is to move `ooze-ui` under `launcher/crates/`. Then the game engine depends on it across workspaces instead. Tell me which you prefer; the crate itself doesn't change.

## R2. Facade

Calls: `snapshot()`, `submit(cmd) -> Result<Id, Error>`, `cancel(id) -> bool`, `try_event()`, plus an open error. This is the shape you described. One operation at a time; `Busy` otherwise.

### R2.1 Snapshot

**`capabilities`:**
- `create: Vec<GameTarget>`: exact combinations offered when creating an instance;
- `instances`, `profiles`, `themes`, `managed_mode`: bools;
- `content: Vec<ContentKind>`.

**Release builds show nothing for a capability that is false or empty.**

**`instances: Vec<InstanceSummary>`**, each with:
- `id` (32-hex) and `name`;
- `target`: `game_version`, `loader`, `loader_version`, `java_major`, and whether Ooze is available;
- `preset: Option<PresetId>` and `overrides: u32`;
- `mode: Custom | ManagedFairPlay`;
- `install: Ready | NeedsInstall | NeedsRepair`;
- `actions: Vec<Rename|Duplicate|Install|Repair|Archive|ChangeMode>`: what the library permits now;
- **`content: Vec<ContentKind>`** for this instance (Vanilla: resource and data packs only);
- `running`;
- `revision`;
- `game_dir`.

**`selected: Option<InstanceId>`** and **`theme: ThemeDoc`**.

**`Changed(snapshot)` event** after every persisted mutation, from any source.

### R2.2 Commands → results

| Command | Result payload | Notes |
|---|---|---|
| `Select(id)` | `Done` + `Changed` | See R3 for Play |
| `Create{name, target}` | `Created(id)` | `target` must equal one of `capabilities.create` |
| `Rename`, `Duplicate{id, name}`, `Archive`, `SetMode{id, mode}` | `Done` / `Created(id)` | Duplicate does not copy worlds, screenshots or options (the UI says so) |
| `Install(id)` | `Done` | Install or repair the game files, with progress |
| `LoadProfile(id)` | `Profile{id, profile, revision}` | |
| `SaveProfile{id, profile, revision}` | `Profile{..}` with the new revision | `Stale` if the revision moved; `Invalid` if validation fails |
| `SaveTheme(ThemeDoc)` | `Done` | Validated with R1 |
| `Search(SearchQuery)` | `Search(SearchPage{query, hits, total, categories})` | Echo the query (the UI drops pages for other queries). `categories` are the filters the UI may offer for that kind |
| `Project{instance, kind, project_id}` | `Project(ProjectDetails)` | Fields listed below |
| `ListWorlds(id)` | `Worlds{instance, worlds: [World{key, name}]}` | Existing local worlds only; `key` is opaque |
| `Plan{instance, kind, version_id, world: Option<key>}` | `Plan(DependencyPlan)` | Fields listed below |
| `Apply{plan_id}` | `Installed{instance, kind, titles}` | Progress steps must include the commit boundary (R2.3) |
| `ListInstalled{instance, kind}` | `InstalledList{..}` | Each item has `record_id`, title, version, file name, `Enabled\|Disabled\|Modified\|Missing`, world, and `update: Option<newer compatible version>` |
| `SetEnabled`, `Update`, `Remove{instance, record_id}` | `Done` | Recorded files only |
| `CheckIntegrity(id)` | `Integrity{instance, report}` | Each issue has a file (instance-relative), a problem (`HashMismatch\|Missing\|UnexpectedMod\|UnapprovedPack\|ProhibitedLaunchSetting(safe name)`) and its allowed `actions` |
| `Resolve{instance, file, action}` | `Integrity{..}` (the refreshed report) | Actions are `Repair\|Quarantine\|OpenFolder` only. **No delete action exists**; quarantine moves into a library-owned folder |

**`ProjectDetails`:**
- title, author and licence;
- a **plain-text** description: markup is shown literally, never rendered;
- `links: [(label, https URL)]`;
- `versions`, each with `version_id`, number, channel, published date, game versions, loaders, size, and `compatible: Result<(), safe reason>`. List incompatible versions with the reason, for example "Needs Minecraft 1.21.10".

**`DependencyPlan`:**
- `plan_id`, `instance` and `kind`;
- `install`: items with title, version, kind, size and role `Selected|Required`;
- `optional` and `embedded`: titles only, never installed;
- `conflicts`: title, plus the installed item it conflicts with;
- `world`: display name;
- `shader_loader: Present(name) | InPlan(name) | Unavailable`.

The UI blocks Install when there are conflicts or the shader loader is `Unavailable`.

### R2.3 Progress

`Progress{op, step, done, total, item}` with steps `Contacting | Downloading | Verifying | Committing`.

**`Committing` marks the durable commit boundary.** The UI removes Cancel and says "Finishing — can't cancel now". This makes LIBRARY_API_V1's "a mutation's durable commit is its cancellation boundary" visible to the player.

### R2.4 Errors

Kinds:
- `Busy`, `GameRunning`, `Incompatible`, `Conflict`, `HashMismatch`;
- `RateLimited{retry_after_s}`, `Offline`, `NotFound`;
- `Config`: the library could not read its data; the original bytes are preserved. Include a safe recovery hint in `detail`;
- `Invalid`, `Stale`, `Cancelled`, `Internal`.

Plus an optional **safe, already-redacted** `detail`. As you specified, provider bodies never become details.

## R3. Play for the selected instance

The UI currently does this: on `Select`, if no game is live, it submits API 1 `UpdateSettings` with `install_root = instance.game_dir`. Then Play is unchanged.

**Please confirm** that `game_dir` is the value API 1 expects there. The other reading is that API 1 needs a separate "instance root" while libraries and assets stay shared under the L0 install root. If it is separate, add the field to the snapshot or give the exact mapping, and I'll adjust `select_instance`.

## R4. The in-game side (bridge and G-milestones; for M0–M2, not L1)

So that modules can run in game, `docs/MODULES.md` §2 and `IMPLEMENTATION_PLAN` M0–M3 need the following.

**R4.1 Frame protocol (G1).** Shared by the HUD and screen hosts:
- versioned frame output;
- one HUD frame per render frame;
- budget ≤ 0.25 ms for three widgets (ARCHITECTURE §10).

**R4.2 Snapshot fields.** Java collects these once per frame or tick; Ooze-defined data only:

| Group | Fields |
|---|---|
| Frame | frame time in ms (render thread) |
| Player | position (x, y, z), yaw, pitch; biome **resource ID string**; dimension ID |
| Armour and hands | slot, item ID, damage, max damage |
| Effects | effect ID, amplifier, remaining ticks, ambient/visible flags |
| Input | movement-key and jump/sneak states; left/right mouse press **edges** with timestamps (for the CPS display; input is only read, never generated) |
| Connection | latency from the player-list entry; `None` in single player |
| Screen | GUI-hidden (F1); screen open |

**R4.3 Platform commands.** Draw a vanilla item icon at a rect, and draw a vanilla effect icon at a rect. These are vanilla atlas sprites, so no texture upload is needed.

**R4.4 Configuration.**
- The library writes the instance's resolved profile (`ooze.profile/1`) where the mod reads it, for example `<game_dir>/config/oozeclient/profile.json`.
- The engine re-validates it with `ooze-ui`.
- Writes go through G4's atomic store.

Please propose the path in SOL-L1.md.

## R5. Small things

- **Linux clippy fails** on `crates/ooze-launcher-core/src/game_guard.rs:63`: an unused `use super::*` in a test module that is presumably used only under `cfg(windows)`. `cargo clippy --workspace --all-targets -- -D warnings` fails on Linux (Windows CI is unaffected). I did not edit it.
- **`UiPrefs::save` regression:** added `prefs_save_replaces_an_existing_file_and_leaves_no_temp_file`. It runs in your Windows CI job (`cargo test --workspace`). A leftover `.json.tmp` from an interrupted save is overwritten by the next save. Single-writer use is guaranteed by your instance lock.
