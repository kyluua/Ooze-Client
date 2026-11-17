# Ooze modules: sourced matrix, fair-play policy and profiles

> Owner: Claude (product). Created 2026-11-17 after the owner authorised real client modules, multiple instances, Modrinth content, themes and integrity modes.
> This file replaces the "framework samples only" stance of v0.1. `FEATURE_BACKLOG.md` keeps the per-module engineering notes; this file is the product source of truth for **what** ships and **under which rules**.

## 1. Research record (reference catalogs)

The reference clients are **product references only**. Ooze implements every module originally, with its own visuals. No code, assets, text or layouts are copied. Third-party code or assets are reused only under a compatible licence, with attribution.

| Source (official URL) | Fetched | What could actually be read | What could **not** be read |
|---|---|---|---|
| Lunar Client features: <https://www.lunarclient.com/features> | 2026-11-17, HTTP 200, complete static HTML | Four feature cards ("Single Install", "Countless Mods", "Boosted Frames", "MultiOS Support"). Named in body text: **Keystrokes, Coordinates, Freelook, Toggle Sprint, Full Bright, Optimized Chunk Loading, Entity Rendering**, optional third-party **Sodium / OptiFine**. Claims "over 65" mods and frame-rate gains "often doubling" | The 65+ mods are not enumerated on this page. Performance claims are marketing, not verified |
| Bunny Client modules: <https://bunnyclient.lol/modules> | 2026-11-17, HTTP 200 | Heading "Modules" with the claim "121 modules, 20 visual effects", a search box, "Show all modules" | **The module list itself is rendered by script and was not present in the fetched HTML. It was not inspected.** A browser-rendered fetch was not used (refused as circumventing the site's delivery). No Bunny module names are recorded here |
| Badlion Client: <https://www.badlion.net/minecraft-client> (and `/free-minecraft-mods`) | 2026-11-17, **HTTP 403** to automated fetches | Nothing from the pages themselves | **Not inspected.** Search-engine snippets (not page inspection) mention **Armor Status**, **ToggleSprint** (Badlion wiki), **Keystrokes** (about page), and "100+ mods" |

**Consequences:**
- None of the three catalogs was fully inspected. This matrix makes no claim of parity with any of them.
- Ooze's priority list comes from the owner's brief (2026-11-17). The "Reference" column shows only what a source above actually confirmed.
- Server rules differ. For example, Hypixel's "Allowed Modifications" page restricts some of these features. A feature existing in another client never implies it is allowed everywhere (§3).

## 2. Module matrix

**Status vocabulary:**
- **Planned:** specified, not built.
- **Model built:** settings, presets and layout logic exist in `ooze-ui` and are Headless-tested; the in-game hook and rendering are not built.
  - The launcher's Profile page edits these settings (enable, typed editors, presets, import/export). It is Headless-tested over the fixture library and hidden until Sol's library reports `profiles` (P0, 2026-11-17).
- **Built**, **Headless-tested**, **Game-launched (OS)**: the AGENTS.md evidence words.

**Compatibility:** "Fabric 1.21.11 Ooze instance" means the module exists only where the Ooze mod is installed. Other instances show "Ooze modules unavailable".

| Id | Ooze module | Reference (§1) | Research date | Ooze behaviour (original) | Fair-play class | Compatibility | Dependencies | Status |
|---|---|---|---|---|---|---|---|---|
| M-01 | FPS and frame time | Not named on the fetched pages; owner priority | 2026-11-17 | FPS plus a frame-time line (average and 1% low over a rolling window); optional compact mode | INFORMATIONAL | Fabric 1.21.11 Ooze instance | Java: frame time per frame (snapshot). Rust: stats, layout. Bridge: HUD frame output (G1) | Model built |
| M-02 | Coordinates and facing | Lunar ("Coordinates") | 2026-11-17 | Block X/Y/Z, cardinal facing and yaw; optional biome off by default; hidden when the vanilla Reduced Debug Info option is on | INFORMATIONAL | Fabric 1.21.11 Ooze instance | Java: position/facing snapshot; respects reduced-debug-info. Rust: format, layout | Model built |
| M-03 | Armour and durability | Badlion ("Armor Status", search snippet only) | 2026-11-17 | Equipped armour and main/off-hand item with durability as value or percent; warning tone below a threshold | INFORMATIONAL | Fabric 1.21.11 Ooze instance | Java: equipment snapshot plus the **vanilla item-icon draw command** (bridge request). Rust: layout, thresholds | Model built |
| M-04 | Status effects | Not named on fetched pages; owner priority (F-04) | 2026-11-17 | Active effects with remaining time and amplifier; vanilla effect icons | INFORMATIONAL | Fabric 1.21.11 Ooze instance | Java: effects snapshot plus vanilla icon command. Rust: layout | Model built |
| M-05 | Keystrokes | Lunar ("Keystrokes"); Badlion (about page, search snippet) | 2026-11-17 | W/A/S/D, jump, sneak and mouse buttons lit while held. **Display only, never input injection** | INFORMATIONAL | Fabric 1.21.11 Ooze instance | Java: key state from the input batch. Rust: layout, press animation (honours reduced motion) | Model built |
| M-06 | CPS display | Not named on fetched pages; owner priority | 2026-11-17 | Left/right clicks per second over a 1 s sliding window; can sit inside Keystrokes | INFORMATIONAL | Fabric 1.21.11 Ooze instance | Java: click events with timestamps. Rust: sliding-window counter | Model built |
| M-07 | Ping | Not named on fetched pages; owner priority (F-06) | 2026-11-17 | Own latency from the player list; hidden in singleplayer | INFORMATIONAL | Fabric 1.21.11 Ooze instance | Java: latency snapshot. Rust: layout, thresholds | Model built |
| M-08 | Hold-to-zoom | Not named on fetched pages; owner priority (F-01) | 2026-11-17 | Hold a key to zoom; optional scroll to adjust; smoothing honours reduced motion and Off; respects the FOV Effects slider | QUALITY_OF_LIFE (server-sensitive) | Fabric 1.21.11 Ooze instance | Java: FOV hook (`require = 0` mixin), keybind. Rust: zoom state, spring | Model built |
| M-09 | Custom crosshair | Not named on fetched pages; owner priority (F-08) | 2026-11-17 | Shape (cross, dot, circle), size, gap, thickness, colour, outline. Off restores the vanilla crosshair. Never changes hit detection | COSMETIC | Fabric 1.21.11 Ooze instance | Java: replace the crosshair HUD element via Fabric's HUD registry. Rust: shape tessellation (G2) | Model built |
| M-10 | HUD editor | Owner priority | 2026-11-17 | Drag or arrow-key widgets; 9 anchors; grid and edge snapping; per-widget and global scale; reset per widget or all; keyboard-only path | (tool) | Fabric 1.21.11 Ooze instance | Rust: layout engine (built). Java: editor screen host (C2). Bridge: G1 | Layout model built |
| M-11 | Toggle sprint | Lunar, Badlion (wiki) | 2026-11-17 | Links to **vanilla's own** Sprint: Toggle option (Controls); no new hook | QUALITY_OF_LIFE (server-sensitive) | Any vanilla-based instance | None (vanilla option) | Planned (link only) |
| M-12 | Performance preset content | Lunar ("Optimized Chunk Loading", "Entity Rendering", optional Sodium/OptiFine) | 2026-11-17 | The FPS profile **suggests** compatible performance mods from Modrinth (for example Sodium) for the instance's exact version and loader. The user installs them; nothing is auto-installed. Ooze makes no frame-rate claims | n/a (content) | Per instance; verified against the version/loader from Modrinth metadata | Backend content API (L1/L2). Windows game test for compatibility with the Ooze mod | Planned |
| M-13 | Freelook | Lunar ("Freelook") | 2026-11-17 | — | GAMEPLAY | — | — | **Blocked** until the server-policy design exists (§3) |
| M-14 | Full bright | Lunar ("Full Bright") | 2026-11-17 | — | GAMEPLAY | — | — | **Blocked** (visibility advantage) |

**Permanently excluded (unchanged):** anything that automates or assists combat, movement, building or mining. That includes:
- input automation (auto-clickers, macros), aim assistance, reach modification;
- ESP, X-ray, entity or ore highlighting, radar of hidden players;
- flight, speed or other movement exploits;
- anything that reads information the vanilla client does not show.

## 3. Fair-play policy

| Class | Meaning | Managed Fair Play mode | Custom mode |
|---|---|---|---|
| COSMETIC | Changes appearance only | Allowed if on the pinned list | Allowed |
| INFORMATIONAL | Shows information the vanilla client already exposes | Allowed if on the pinned list | Allowed |
| QUALITY_OF_LIFE | Convenience without information or input advantage; some servers restrict items such as zoom or toggle sprint | Allowed if on the pinned list; **server-sensitive** items carry a notice | Allowed, with notice |
| GAMEPLAY | Could change what a player can see or do | **Never** | Blocked until a server-policy design exists (servers able to disable it, plus a visible indicator) |

- Ooze never claims a mode is "cheat-proof", "anticheat-safe" or "server-approved". The copy says what Ooze checked and nothing more.
- Every module shows its class and, when server-sensitive, the notice "Some servers don't allow this. Check the server's rules."
- **Managed Fair Play** = the pinned, verified set of Ooze modules plus pinned content, with backend integrity checks (`docs/launcher/CONTENT_AND_INTEGRITY.md`).
- **Custom** = user-managed compatible mods and packs, with no fair-play verification claim.

## 4. Profiles (presets)

The three terms are distinct everywhere in the product:

| Term | What it is | Stored by |
|---|---|---|
| **Instance** | One isolated game directory: Minecraft version, mod loader, Java runtime, files, installed content | Backend (Sol) |
| **Profile** (preset) | Module enable states and settings, HUD arrangement, visual settings, and optional suggested content | Backend persistence, per instance; data schema `ooze.profile/1` (Claude) |
| **Theme** | Launcher (and later in-game) interface appearance | Backend persistence; data schema `oozeclient.theme/1` (existing) |

**Built-in presets** (data: `native/crates/ooze-ui/presets/*.json`; logic: `ooze-ui::profile`):

| Preset | Intent | Enabled modules | Visual settings | Suggested content (never automatic) |
|---|---|---|---|---|
| **FPS** | Restrained effects for frame rate | FPS and frame time (compact) | Effects quality Low, ambient motion off, drips off | Compatible performance mods for the instance's version/loader, if the backend verifies them |
| **PvP** | Clear combat information | FPS, Keystrokes with CPS, Armour, Status effects, Ping, Custom crosshair (simple cross). Zoom available on a key | Effects quality Medium | None |
| **Minimal** | Quiet interface | None (everything optional off) | Effects quality Medium, ambient motion off | None |

**Rules:**
- **User overrides survive.** A profile is the preset plus a layer of user overrides. Switching presets or updating Ooze never discards overrides. Reset clears them (per setting, per module, or all).
- **What it changes.** Before applying, the UI lists every setting that would differ from the instance's current values. The list comes from `ooze-ui::profile::diff`.
- **Duplicate, export and import.** Profiles are exported and imported as data only (`ooze.profile/1` JSON, validated, no code). Unknown modules or settings in an import are reported and skipped.
- **Ooze modules exist only in Fabric 1.21.11 Ooze instances.** Selecting a profile for any other instance shows that its modules are unavailable there.
