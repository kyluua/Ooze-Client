# Ooze Client — Architecture (Fabric 1.21.11 + Rust)

> Deliverable 10. Rewritten 2026-11-09 for the owner's decisions:
> - one target, Minecraft 1.21.11 on Fabric;
> - a Java 21 Fabric adapter;
> - most Ooze engine and UI logic in Rust behind one batched JNI bridge;
> - authentication only through the official Minecraft Launcher.
>
> Section numbers match the previous version so cross-references in other documents stay valid.

**Identifiers**

| Item | Value |
|---|---|
| Mod id and asset namespace | `oozeclient` |
| Java packages | `io.github.kyluua.ooze.mc.v1_21_11.*` (Fabric adapter, Claude); `bridge-java` packages are defined by Sol |
| Rust crates | `native/crates/ooze-ui` (Claude); engine and bridge crates are defined by Sol |
| Config directory | `<gameDir>/config/oozeclient/` |

---

## 1. Principles

1. **Rust owns Ooze logic; Java owns Minecraft.**
   - Rust owns UI state, components, layout, animation, theming, typography, configuration, settings, search and HUD composition.
   - Java owns everything that touches Minecraft or Fabric objects, the GPU and the vanilla renderer.
2. **One bridge, batched.** Each frame Java sends one bounded input batch plus snapshot deltas, and receives one **versioned frame output**: geometry, texture work and platform commands. There is never JNI per vertex, glyph, entity, widget or setting.
3. **No raw graphics APIs anywhere.** Rust never calls OpenGL, Vulkan or NanoVG. Java submits geometry only through vanilla GUI pipelines (ADR-001, ADR-002).
4. **Native code is Minecraft-agnostic.** Only Ooze-defined data crosses the bridge. Obfuscated or mapped Minecraft symbols never do.
5. **Vanilla is preserved and the engine fails soft.** If the native engine is missing or incompatible, Minecraft runs as vanilla with a clear diagnostic and no non-working Ooze controls (TARGET_PLATFORM §5).
6. **Identity stays with the launcher.** Only username and UUID reach Rust, for display (§17).
7. **Data-driven appearance.** Themes, fonts, wallpapers and motion are tokens and data.

## 2. Layers

```
┌────────────────────────────────────────────────────────────────────────────────────┐
│ ooze-client-<ver>+mc1.21.11.jar   (remapped Fabric mod; bundles bridge-java + natives)│
├────────────────────────────────────────────────────────────────────────────────────┤
│ versions/mc1_21_11  (Java 21, Claude)                                              │
│   entrypoint · lifecycle · minimal mixins · input capture · screen hosts · HUD hook │
│   render adapter (frame output → vanilla GUI render state) · texture uploads       │
│   game snapshot producers · platform command executor · narration · compat adapters│
├────────────────────────────────────────────────────────────────────────────────────┤
│ bridge-java  (Java 21, Sol)                                                        │
│   native load + verification · status/diagnostics · versioned protocol buffers     │
│   the only `native` declarations in the codebase                                   │
├──────────────────────────────── JNI (bounded batches) ─────────────────────────────┤
│ native/  (Rust, Sol's Cargo workspace)                                             │
│   engine crates (Sol): render primitives · text · animation · theme resolution ·   │
│     config I/O · settings/state algorithms · events · search · images · budgets    │
│   ooze-ui (Claude): components · screens · navigation · focus · interactions ·     │
│     product UI state · material painter · HUD composition                          │
└────────────────────────────────────────────────────────────────────────────────────┘
```

**Allowed dependencies**

| Layer | May depend on |
|---|---|
| `versions/mc1_21_11` | Minecraft, Fabric API, `bridge-java`. It must not declare `native` methods |
| `bridge-java` | JDK only. It must not import Minecraft or Fabric classes, so it can be tested without the game |
| `ooze-ui` | Sol's engine crates through their public APIs. It never touches JNI types directly; the bridge crate adapts |
| All Rust crates | No OpenGL, Vulkan or NanoVG crates, and no Minecraft names |

Rules that will be enforced by tests:
- Java: no `native` methods and no `org.lwjgl.opengl`/`org.lwjgl.vulkan` imports in `versions/mc1_21_11` (a source scan test, added in C1).
- Rust: dependency deny-lists in Sol's workspace lints.

### 2.1 Java package map (`versions/mc1_21_11`)

| Package (`io.github.kyluua.ooze.mc.v1_21_11.…`) | Contents |
|---|---|
| (root) | `OozeClient` entrypoint |
| `engine` | `EngineConnector` (Fabric-side port), `BridgeEngineConnector` (the only adapter over Sol's `NativeLoader`/`EngineSession`), `BridgeStatusMapper` (G0 diagnostics → states), `EngineStatus`, `EngineGate`, `PlatformInfo` |
| `diag` | Diagnostics formatting and logging, the vanilla-toast notice |
| `dev` | Developer-only actions (registered only in the development environment and only with a loaded engine): F9 bridge probe, `BridgeDiagnosticScreen` + `FixtureRenderState` (G0 fixture through vanilla's GUI render state) |
| `render` (C1) | Frame output validation, GUI element render states, texture manager and budget checks |
| `input` (C2) | Input capture and batching |
| `screen` (C2) | `OozeScreenHost` and the hosts |
| `snapshot` (C2) | Immutable game snapshots |
| `command` (C2) | Platform command executor |
| `hud` (C9) | HUD hook |
| `narration` (C2) | Narrator bridge |
| `compat` (C6) | Mod Menu and title-screen tray |
| `mixin` | Minimal mixins, only where Fabric API has no event. C0: `GuiGraphicsAccessor` (read access to the frame's `GuiRenderState`; accessor only) |

## 3. Runtime lifecycle

```mermaid
sequenceDiagram
    participant F as Fabric adapter (Java)
    participant B as bridge-java (Sol)
    participant R as Rust engine + ooze-ui
    F->>B: load() at ClientModInitializer
    B-->>F: status (LOADED | MISSING_LIBRARY | INCOMPATIBLE | LOAD_FAILED | UNSUPPORTED_PLATFORM)
    alt LOADED
      F->>R: init(config dir, platform info, resource bundle, translations)
      Note over F,R: async engine start-up (fonts, theme, config) reported via readiness flags in frame output
      F->>R: frame(input batch, snapshot deltas) per rendered frame while an Ooze surface is visible
      R-->>F: frame output (versioned): geometry, texture work, platform commands
      F->>R: shutdown() at CLIENT_STOPPING (flushes config)
    else not loaded
      F->>F: vanilla-only mode: diagnostic log + one vanilla toast; register nothing else
    end
```

- Fabric-side bootstrap must take **≤ 50 ms** on the main thread, excluding native library loading, which is measured and reported separately.
- When no Ooze surface is visible (no Ooze screen, HUD widgets off), Java makes **no per-frame JNI calls**.

## 4. Coordinate spaces and the screen host

| Space | Unit | Used by |
|---|---|---|
| Physical | px (framebuffer pixels) | Frame output vertices, scissor, glyph rasterization, snapping |
| Ooze | ou = px / uiScale | All layout, components, HUD positions (computed in Rust) |
| Minecraft GUI | px / mcGuiScale | Vanilla APIs only; the Java adapter converts |

**`OozeScreenHost`** (Java, from C2) is a vanilla `Screen` subclass:
- It captures input in GUI coordinates, converts it to px, and appends it to the frame's input batch.
- It sends a frame header with framebuffer size, Minecraft GUI scale, time and focus.
- It submits the returned frame output.
- The scale model (DESIGN_SYSTEM §9.1) is computed in Rust from the header.

## 5. Rendering architecture

### 5.1 Frame pipeline

```
Java (render thread)                         Rust (same thread, inside one JNI call)
──────────────────────────────────────       ──────────────────────────────────────────
build FrameInput:                            update(dt) → layout(dirty) → paint
  header (time, fb size, gui scale, focus)   → DrawList in px (triangles, two vertex kinds)
  input events since last frame (bounded)    → texture work (atlas/image regions, byte-budgeted)
  snapshot deltas (bounded)                  → platform commands (validated vocabulary)
            ──── one JNI call ────►          ◄── FrameOutput (versioned binary, direct buffers)
validate header, version, bounds, counts
apply texture work (create/upload/destroy) within budgets
submit batches → vanilla GUI render state
execute platform commands on the render thread
```

**Bridge budgets** (enforced and measured; see PERFORMANCE_AND_ACCESSIBILITY §2):
- ≤ 2 JNI calls per rendered frame per visible surface.
- Bounded input and snapshot sizes.
- Frame output size limits declared in the protocol.

The binary format, buffer ownership and version negotiation belong to Sol's contract, [`docs/bridge/ABI_V1.md`](bridge/ABI_V1.md). G0 implements the subset `OZIN` input batches plus a 104-byte `OZFR` diagnostic fixture. **Lease rule:** close a `FrameLease` only after vanilla consumed the vertices, or eagerly copy them into Minecraft-owned data first (C0 copies eagerly). This section defines the product requirements the full format must meet.

### 5.2 Frame output contents (product requirements)

- **Vertex streams.** A **colour** stream (`x, y, argb`) and a **textured** stream (`x, y, u, v, argb`). Positions are in px; primitives are triangles.
- **Batches.** Each batch is `{kind, textureId, scissorRect, firstVertex, vertexCount, layerBreak}`.
- **Texture work.** Create, upload-region and destroy, keyed by an Ooze texture id, with RGBA8 pixel payloads and filter and wrap modes. Every entry carries its byte size so Java can enforce budgets (§5.7).
- **Platform commands.** A closed, versioned vocabulary (§11.1). Unknown commands are rejected and logged once.
- **Readiness and diagnostics.** Flags for fonts, theme and background; frame timings measured on the Rust side.

### 5.3 Paths, paints and tessellation (Rust engine, Sol)

These are requirements on Sol's engine; the implementation is Sol's:
- Analytic **1 px AA fringe** on every edge.
- Superellipse corners and drips (DESIGN_SYSTEM §4).
- Flattening tolerance ≤ 0.15 px.
- Fan, monotone and ear-clipping triangulation.
- Strokes with round joins.
- Gaussian-approximating shadow rings (DESIGN_SYSTEM §6).
- Paint evaluated per vertex: solid, two-stop linear, vertical alpha mask, radial alpha.

### 5.4 Mesh caching (Rust engine)

- A mesh cache keyed by shape parameters and **scale epoch**. Colour and opacity are evaluated at emission, so animating them never re-tessellates.
- Transient meshes for shape-changing animations.
- Limits are byte-based (PERFORMANCE_AND_ACCESSIBILITY §2.2).

### 5.5 Ooze material painter (Rust, `ooze-ui`)

- The White Ooze and Ink Gel layer stack of DESIGN_SYSTEM §3 is product design.
- It lives in `ooze-ui` and composes the engine's primitives.
- It reads only resolved theme tokens and the effects quality tier.

### 5.6 Clipping

- Rectangular scissor only, in px, mapped to the vanilla GUI scissor.
- Components must be designed so that a rectangular scissor plus edge-fade masks is enough (COMPONENTS C19).

### 5.7 Textures (Java executes, Rust plans, both enforce budgets)

- Rust decides **what** textures exist (glyph atlas pages, background images and mips, blurred copies, thumbnails, the noise tile) and keeps a **byte-accurate texture budget accountant**.
- Java creates, uploads and destroys the GPU textures through Minecraft's texture APIs on the render thread. It **independently re-checks** the declared byte sizes against the same budgets and rejects any operation that would exceed them, with a diagnostic.
- Byte formula: `bytes = Σ levels (w_l × h_l × 4)`. A full mip chain adds about ⅓.
- Linear filtering for atlases and images; nearest filtering for pixel art (the player face) and the noise tile.

### 5.8 Java render adapter (vanilla submission)

1. Each batch becomes one custom GUI element render state in the frame's vanilla GUI render state, carrying its pipeline, texture binding, scissor and bounds.
2. **Pipelines.** Preferred: pipelines derived from the vanilla GUI snippets with `TRIANGLES` mode, keeping the vanilla shaders. Fallback: the vanilla `GUI` / `GUI_TEXTURED` (QUADS) pipelines with degenerate quads `(v0, v1, v2, v2)`.
3. **Coordinates.** px vertices are scaled by `1 / mcGuiScale`, so the vanilla projection maps them back to exact pixels.
4. **Verification.** Verify the exact 1.21.11 class and method names in C1 (TARGET_PLATFORM T6).

### 5.9 Ordering

- The frame output is in painter's order, and `layerBreak` marks where order must be preserved across kinds.
- C1 must test whether the vanilla GUI render state reorders mixed pipelines within a stratum. If it does, the adapter advances the stratum at every `layerBreak`.
- This is verified by the ordering torture-scene golden (TESTING §3).

---

## 6. UI toolkit (Rust, `ooze-ui`)

### 6.1 Node model

- A retained node tree with layout (`Row`, `Column`, `Stack`, `Grid`, `Absolute`, `Scroll`), cached measuring and dirty flags.
- Per-frame update (animations), paint into the engine's DrawList, input handlers, focusability and narration info.
- Visual transforms for animation don't affect layout.

### 6.2 Reactive state

- Product UI state uses observable properties. Settings from the engine's settings store bind two-way.
- Subscriptions are owned by nodes and released on detach.

### 6.3 Input and focus

Implemented per INTERACTION_FLOWS §3:
- capture and bubble dispatch, pointer capture, focus-visible modality, focus scopes, roving focus and the Esc chain;
- cursor shapes are requested through a platform command.

### 6.4 Layers and router

- A fixed layer stack (DESIGN_SYSTEM §9.5).
- A router with history and anchors, for example `shell/modules/{category}#setting:<path>`.

## 7. Theme runtime (Rust)

- Theme JSON (DESIGN_SYSTEM §8) is parsed and resolved by the engine into resolved token arrays, with OKLab transitions.
- The token set and built-in themes are product assets (`assets-src/themes/`), embedded per the resource contract (§16).

## 8. Module system

### 8.1 Model

The model is split across the bridge:

| Part | Owner and language | Responsibility |
|---|---|---|
| Module metadata, settings and state | Rust (engine algorithms; product definitions in `ooze-ui` or a product crate agreed with Sol) | id, name key, category, tags, sample flag, `FairPlay`, required capabilities, settings, enabled state, persistence |
| **Game-facing hooks and actions** | **Java** (Claude) | Anything that reads or changes the game: snapshot providers (for example FPS and time for HUD widgets), vanilla actions, mixin-backed hooks |

- Rust asks Java to act through **platform commands**, and Java feeds data back through **snapshots**.
- `FairPlay` is mandatory metadata: `COSMETIC`, `INFORMATIONAL`, `QUALITY_OF_LIFE` or `GAMEPLAY`.
- Real modules come only from `docs/FEATURE_BACKLOG.md`, one milestone at a time.

### 8.2 Registry and lifecycle

- Registration is static at start-up. Enabling a module checks its environment (menu or world) and capabilities.
- Hook failures in Java are caught per module. After 3 failures within 10 s, Java reports them and Rust disables the module with a toast.

### 8.3 Settings

The same typed settings as before:
- Boolean, int and float (range, step, format), enum, colour, keybind, string, string list, font, action, group.
- Each setting has a default, validation, a visibility predicate, search tags and a stable `SettingPath`.

Persistence is Rust's job (§9).

### 8.4 Keybinds

- **Core actions** are registered as vanilla key mappings in the "Ooze Client" category, so they also appear in vanilla Controls. Example: Open ClickGUI, default Right Shift.
- **Module binds** are evaluated from the input batch, only when no screen is open. Toggle and hold semantics are unchanged.

---

## 9. Configuration persistence (Rust)

### 9.1 Files

```
config/oozeclient/
├── client.json            global preferences (schema oozeclient.client/N)
├── keybinds.json
├── themes/<id>.theme.json
├── backgrounds/backgrounds.json, library/, thumbs/, inbox/
├── profiles/index.json, profiles/<id>/modules.json, profiles/<id>/hud.json
└── backups/<yyyyMMdd-HHmmss>/        start-up snapshots (keep 5)
```

### 9.2 Format and migrations

- JSON with `"schema": "<name>/<int>"`. Unknown fields are kept.
- Migration chains run per file type, and files from newer versions open read-only (INTERACTION_FLOWS §4).

### 9.3 Atomic writes

The write sequence:
1. Write a temporary file and fsync it.
2. Copy the current file to `.bak`.
3. Rename the temporary file over the current file atomically.

Loading falls back to `.bak`, then to defaults. A corrupt file is renamed and quarantined.

### 9.4 Threads

- Saves are debounced (750 ms, at most 5 s) on a Rust I/O thread.
- Bounded synchronous flush (≤ 500 ms) at screen close, world leave, profile switch and `CLIENT_STOPPING`.

### 9.5 Backups (corrected)

At start-up, if any config file changed since the last snapshot and that snapshot is more than 24 h old, the config tree is copied to `backups/<timestamp>/`. The 5 newest snapshots are kept.

**Excluded from every backup copy:**
- `backups/` itself, so backups never contain backups and never grow recursively;
- `backgrounds/library/`, `backgrounds/thumbs/` and `backgrounds/inbox/` (large, user-recoverable media);
- `*.tmp`, `*.bak`, `*.corrupt-*`.

A snapshot larger than **8 MiB (8,388,608 bytes)** is skipped with a diagnostic.

---

## 10. HUD system

- **Rust** composes HUD widgets (anchors, offsets in ou, clamping, snapping) into a small separate frame output.
- **Java** registers one HUD element through Fabric API's HUD element registry (verify the name in C9), supplies HUD snapshots and submits the output.
- The HUD is skipped when the GUI is hidden (F1) or the HUD editor is open.
- Budget: ≤ 0.25 ms in total (Java + Rust) for 3 widgets.
- Sample widgets (FPS, Clock, Watermark) carry the Sample badge.

## 11. Platform commands, snapshots and adapters (Java)

### 11.1 Platform command vocabulary (product requirement for the protocol)

| Area | Commands (Rust → Java) |
|---|---|
| Navigation | Open vanilla screen (singleplayer, multiplayer, Realms, options, language, accessibility, credits, controls), close the Ooze screen, return to parent |
| Game | Join saved server *n*, load world *id*, quit (with the vanilla confirmation) |
| System | Open URL through the vanilla confirm-link screen, open folder, set cursor shape, clipboard set, play UI click (pitch, volume) |
| Accessibility | Narrate text (interrupt or polite) |
| Files | Open file dialog (tinyfd, off-thread) → result as a later snapshot |
| Keys | Read vanilla key mappings (snapshot) |

**Snapshots** (Java → Rust; immutable and bounded):
- window and framebuffer size, GUI scale, focus;
- options: language, Distortion Effects (for Auto reduced motion), narrator state;
- player display metadata: username and UUID only;
- world list and server list (names and ids only);
- FPS, clock, in-world flag, screen-open flag.

### 11.2 Mixins

Minimal and per need:
- title-screen replacement (`Minecraft#setScreen`), from C6;
- key and mouse capture for module binds when Fabric events are insufficient.

They use `require = 0` plus a runtime check, and never crash.

### 11.3 World blur

The vanilla menu-background blur is used behind in-world Ooze screens. If 1.21.11 has no per-call radius, Ooze uses Minecraft's option unchanged and the UI says so. Ooze never writes the user's vanilla options.

### 11.4 Compatibility

| Mod | Integration |
|---|---|
| Mod Menu | Config screen factory, from C6 |
| Other title-screen mods | Compatibility tray, from C6 |
| Sodium and Iris | Smoke-tested only |

## 12. Search (Rust)

The fuzzy index and scoring are unchanged from the previous specification:
- exact 1000, prefix 800, word-prefix 600, substring 400, fuzzy subsequence 100+;
- recency bonus;
- ≤ 1 ms per keystroke for 2,000 entries.

## 13. Events

- Ooze's internal event bus runs in Rust.
- Game events arrive as typed entries in the input or snapshot batch, for example world join and leave, screen opened and resolution changed.
- Java never posts per-entity events.

## 14. Threading

| Thread | Work |
|---|---|
| Minecraft render thread (Java) | Input capture, the JNI frame call, frame output validation and submission, texture operations, platform commands |
| Rust worker and I/O threads (Sol) | Decode, rasterization, blur, config I/O. They **never call back into the JVM**; results surface in the next frame output |
| Java short-lived thread | Blocking tinyfd dialogs, with the result delivered in the next snapshot |

## 15. Diagnostics

- Java logs the engine status at start-up (TARGET_PLATFORM §5).
- Frame statistics combine Java time (capture, validate, submit) with the Rust time reported in the frame output.
- A developer overlay is available only in the development environment.

## 16. Resources and localisation

- **Source assets** live in `assets-src/`: theme tokens, fonts with their OFL texts, icons and the brand.
- **Java-side resources** live in `versions/mc1_21_11/src/main/resources/assets/oozeclient/`, including `lang/*.json`.
- **Resource contract** (agreed with Sol in G0 or G1): which assets are embedded in the native library and which are passed from the JAR at `init`.
- **Translations:** Java resolves the `oozeclient.*` keys for the current language through Minecraft's language system, and sends the table to Rust at `init` and on language change. Rust never parses Minecraft language files itself.

## 17. Authentication and identity boundary

See TARGET_PLATFORM §4.

| Item | Rule |
|---|---|
| Sign-in, entitlement and session | The official launcher and Minecraft |
| Ooze account system | **None.** No Microsoft login, no Azure application registration, no app-approval dependency |
| Crosses into Rust | Username and UUID, display only |
| Never crosses, logs or persists | Access tokens, session IDs, refresh tokens, Microsoft account data |
| Account switching | In the launcher; no Ooze control |

---

## 18. Architecture decision records

| ADR | Decision | Status |
|---|---|---|
| 001 | Vector UI tessellated on the CPU with analytic AA. Java submits it through vanilla GUI pipelines; no custom shaders in v1 | Kept (tessellation now in Rust) |
| 002 | No NanoVG and no direct OpenGL or Vulkan, in Java or Rust | Kept, extended to Rust |
| 003 | Text rasterized at physical pixel size and cached in atlases | Kept; the rasterizer is Sol's choice in the Rust engine |
| 004 | Mojang official names in Java sources; the Loom remap plugin for 1.21.11 | Updated: 1.21.11 only |
| 005 | One distribution JAR for 1.21.11 that bundles per-platform native libraries; Windows x64 first | **Replaces** "one JAR per Minecraft target" |
| 006 | JSON config with schema versions, migrations, atomic writes and backups; backups exclude `backups/` and media | Updated (implemented in Rust) |
| 007 | Retained UI tree with reactive state | Kept (Rust) |
| 008 | Closed-form springs; ambient motion as pure functions of time | Kept (Rust) |
| 009 | Ooze scale model independent of Minecraft's GUI scale | Kept |
| 010 | Exact-type event bus | Kept (Rust) |
| 011 | ~~Gradle subprojects plus shared source dirs for several Minecraft targets~~ | **Superseded** by ADR-013 |
| 012 | **Authentication only through the official Minecraft Launcher**; Ooze has no account system | New |
| 013 | **Rust engine and UI with a Java 21 Fabric adapter, joined by one batched, versioned JNI bridge owned by Sol** | New (owner decision 2026-11-09) |
| 014 | **Single target Minecraft 1.21.11**; 26.1/26.2 removed | New (owner decision 2026-11-09) |
