# Ooze Client — Java/Fabric Architecture

> Deliverable 10. How the client is built: layers, lifecycle, rendering, UI toolkit, themes, modules, settings, keybinds, events, configuration, HUD, search, platform adapters, threading and the key architecture decisions.
> Repository layout: [REPOSITORY_STRUCTURE.md](REPOSITORY_STRUCTURE.md). Version strategy: [MULTI_VERSION.md](MULTI_VERSION.md).

**Identifiers used throughout**

| Item | Value |
|---|---|
| Mod id and asset namespace | `oozeclient` |
| Display name | Ooze Client |
| Base Java package | `io.github.kyluua.ooze` |
| Config directory | `<gameDir>/config/oozeclient/` |

---

## 1. Principles

1. **The core knows nothing about Minecraft.** All UI, rendering logic, animation, theming, typography layout, configuration, modules, events, HUD and search live in `core`. Core is pure Java 21 with no Minecraft imports and is unit-testable without a game.
2. **Thin, explicit adapters.** Each Minecraft target implements a small set of platform interfaces (§11). Version differences are absorbed there, never in core.
3. **One rendering path that works everywhere.** CPU-tessellated, analytically anti-aliased geometry is submitted through **vanilla GUI pipelines**. There is no custom GLSL and no direct OpenGL in v1, so OpenGL and Vulkan behave identically (ADR-001, ADR-002).
4. **Data-driven appearance.** Themes, fonts, wallpapers and motion are tokens and data, not code.
5. **Fail soft.** Optional features turn off with a diagnostic. Nothing optional may crash the game.

## 2. Layers

```
┌────────────────────────────────────────────────────────────────────────────────────┐
│ Distribution JARs   ooze-client-<ver>+mc1.21.11.jar · +mc26.1.jar · +mc26.2.jar    │
├────────────────────────────────────────────────────────────────────────────────────┤
│ versions/<target>   Fabric entrypoints · mixins · platform adapters                │  per Minecraft target
│                     (render backend, input, screens, HUD hook, text fallback)      │
├────────────────────────────────────────────────────────────────────────────────────┤
│ mc-shared (source)  integration code that compiles unchanged against every         │  compiled separately
│                     target (Mojang names); shared by the version projects          │  per target
├────────────────────────────────────────────────────────────────────────────────────┤
│ platform-lwjgl      stb_truetype rasterizer · STBImage decoder · TinyFileDialogs   │  version-agnostic
├────────────────────────────────────────────────────────────────────────────────────┤
│ core (Java 21)      anim · render · text · ui · component · theme · background     │  no Minecraft
│                     config · event · module · setting · input · hud · search ·     │
│                     i18n · platform (SPI) · diag                                   │
└────────────────────────────────────────────────────────────────────────────────────┘
```

**Allowed dependencies (enforced by Gradle and ArchUnit tests):**
- `core` → JDK, Gson (compileOnly; present at runtime in Minecraft), SLF4J API, fastutil (compileOnly; present in Minecraft).
- `platform-lwjgl` → `core`, LWJGL `lwjgl`, `lwjgl-stb` and `lwjgl-tinyfd` (compileOnly).
- `versions/*` → everything, plus Minecraft and Fabric.
- `core` must never import `net.minecraft.*`, `com.mojang.*`, `net.fabricmc.*` or `org.lwjgl.*`.

### 2.1 Core package map

| Package (`io.github.kyluua.ooze.…`) | Contents |
|---|---|
| `core.math` | `Vec2`, `Rect`, `Insets`, `Affine2D`, `MathUtil` (clamp, lerp, snap) |
| `core.color` | `Argb` (packed-int utilities), `OkLab`, `ColorMix`, `Contrast` (WCAG ratio) |
| `core.anim` | `FrameClock`, `SpringSpec`, `TweenSpec`, `Easing`/`CubicBezier`, `AnimatedFloat`/`Vec2`/`Color`/`Rect`, `Animator`, `Motion`, `MotionToken`, `MotionSettings` |
| `core.render` | `DrawList`, `Batch`, `Canvas`, `Mesh`, `MeshCache`, `Paint`, `Path`, `PathBuilder`, `Tessellator`, `StrokeStyle`, `ClipStack`, `TextureHandle`, `RenderBackend` (SPI), `FrameInfo` |
| `core.render.shape` | `RoundedRect` (superellipse corners), `Capsule`, `Circle`, `Blob`, `DripSet`, `ShadowPainter`, `OozeMaterialPainter`, `MaterialSpec`, `MaterialState` |
| `core.render.icon` | `IconSet`, `Icon` (path commands), `IconRenderer` |
| `core.text` | `FontRegistry`, `FontFamily`, `FontFace`, `FaceMatcher`, `GlyphRasterizer` (SPI), `GlyphAtlas`, `GlyphCache`, `TypeRole`, `TextStyle`, `TextEngine`, `TextLayout`, `LineBreaker`, `ScriptDetector` |
| `core.ui` | `Node`, `Root`, `Layer`, `LayerStack`, `Constraints`, `LayoutParams`, `Row`/`Column`/`Stack`/`Grid`/`Absolute`/`Scroll`, `PointerEvent`, `KeyEvent`, `CharEvent`, `FocusManager`, `InteractionState`, `Router`, `Route`, `Page`, `NarrationInfo` |
| `core.ui.component` | All components C01–C34 |
| `core.ui.screen` | Screen view trees: `IntroView`, `MainMenuView`, `ShellView` plus pages, `HudEditorView` |
| `core.theme` | `ThemeSpec` (JSON model), `ThemeResolver`, `ResolvedTheme`, `ThemeToken` (enum), `ThemeManager`, `ThemeValidator` |
| `core.background` | See BACKGROUNDS §9 |
| `core.config` | `ConfigStore`, `ConfigFile<T>`, `Codec<T>`, `Codecs`, `Migration`, `MigrationChain`, `AtomicFiles`, `Profiles`, `SaveScheduler` |
| `core.event` | `EventBus`, `Event`, `Cancellable`, `Priority`, `Subscription`, and the event catalogue (§13) |
| `core.module` | `Module`, `ModuleInfo`, `Category`, `Categories`, `ModuleRegistry`, `ModuleContext`, `ModuleProvider` (SPI), `FairPlay`, `Capability` |
| `core.setting` | `Setting<T>` and its subtypes, `SettingGroup`, `Visibility`, `SettingPath` |
| `core.input` | `KeyCombo`, `KeyCodes` (GLFW-compatible ints), `Keybind`, `KeybindAction`, `KeybindManager`, `InputModality` |
| `core.hud` | `HudWidget`, `HudWidgetInfo`, `HudContext`, `HudLayout`, `Anchor`, `HudManager`, `Snapper` |
| `core.search` | `SearchIndex`, `SearchEntry`, `SearchProvider`, `FuzzyMatcher`, `SearchResult` |
| `core.i18n` | `Text` (literal or translatable), `Translator` (SPI) |
| `core.platform` | `Platform` and its sub-interfaces (§11.1) |
| `core.diag` | `FrameProfiler`, `Counters`, `DebugOverlay`, `OozeLog` |
| `core` (root) | `Ooze` (service locator and bootstrap), `OozeServices`, `OozeId` (a `namespace:path` record; core never uses Minecraft's identifier class) |

---

## 3. Runtime lifecycle

```mermaid
sequenceDiagram
    participant F as Fabric (version module)
    participant O as Ooze (core)
    participant W as Workers
    F->>O: ClientModInitializer.onInitializeClient → Ooze.bootstrap(platform)
    O->>O: ConfigStore.load(client.json, keybinds.json) (sync, small, < 20 ms)
    O->>O: Registries: categories, built-in settings pages, search providers
    O->>F: collect "oozeclient:modules" entrypoints → ModuleProvider.register(...)
    O->>O: ModuleRegistry.freeze(); load profile modules.json; restore enabled states (deferred until client started)
    O->>W: async: FontRegistry.scan + load faces; ThemeManager.load; BackgroundManager.prepare(main_menu)
    F->>O: ClientLifecycle STARTED → Ooze.onClientStarted (render thread ready: create atlases, upload pending textures)
    F->>O: setScreen(TitleScreen) intercepted → IntroScreen / MainMenuScreen
    F->>O: CLIENT_STOPPING → SaveScheduler.flushAll(); disable modules; free native buffers
```

- `Ooze.bootstrap` must finish in **≤ 150 ms** on the main thread. Everything else is asynchronous.
- **Readiness gates:** `Ooze.ready(Feature.FONTS | THEME | BACKGROUND)`. Screens wait on gates using the S01 loading fallback, and never block the thread.

---

## 4. Coordinate spaces and the screen host

| Space | Unit | Used by |
|---|---|---|
| Physical | px (framebuffer pixels) | DrawList vertices, scissor rects, glyph rasterization, snapping |
| Ooze | ou = px / uiScale | All layout, components, HUD positions |
| Minecraft GUI | px / mcGuiScale | Vanilla APIs only; the adapter converts |

**`OozeScreenHost`** (a version module; it extends the vanilla `Screen`) bridges Minecraft to a core `Root`:

| Minecraft callback | Host action |
|---|---|
| `init()` / resize | Compute `uiScale` (DESIGN_SYSTEM §9.1) → `root.setViewport(fbW / uiScale, fbH / uiScale, uiScale)`. If the scale changed, bump `ScaleEpoch` (invalidates meshes and glyphs) |
| `render(graphics, mx, my, partialTick)` | `clock.tick()` → `root.update(frame)` (animations, hover from the pointer position) → `root.layoutIfDirty()` → `root.paint(canvas)` → `backend.submit(drawList, graphics)` |
| Mouse callbacks | Convert GUI coordinates to ou (`gui × mcGuiScale / uiScale`) and dispatch a `PointerEvent` (press, release, drag, move, scroll with fractional deltas) |
| Key and char callbacks | Convert to `KeyEvent` (GLFW key, scancode, modifiers, repeat) and `CharEvent` (code point) |
| `onFilesDrop(paths)` | Dispatch a `FileDropEvent` |
| `onClose()` / `shouldCloseOnEsc()` | Return `false`. Esc goes through the precedence chain (INTERACTION_FLOWS §3.4) |
| `isPauseScreen()` | From settings (shell) / `true` (HUD editor, in singleplayer) |
| `renderBackground(...)` | Overridden. The Ooze `BackgroundRenderer` paints the context's background. World contexts request the vanilla blur through the adapter |

The four hosts (`IntroScreen`, `MainMenuScreen`, `ShellScreen`, `HudEditorScreen`) each construct their core view and pass it to the generic host.

---

## 5. Rendering architecture

### 5.1 Frame pipeline

```
update(dt) ─► layout (dirty nodes only) ─► paint: nodes emit to Canvas (ou)
           ─► Canvas converts to px, applies transform/opacity/clip, snaps ─► DrawList (batches of px-space vertices)
           ─► RenderBackend.submit(DrawList) ─► vanilla GUI render state (one element per batch)
```

### 5.2 DrawList

- Two reusable vertex streams:
  - **Color:** `x, y, argb` → vanilla `POSITION_COLOR`.
  - **Textured:** `x, y, u, v, argb` → vanilla `POSITION_TEX_COLOR`.
- Primitives are **triangles**. Streams are backed by growable `float[]`/`int[]` arrays that are reset (not reallocated) every frame.
- A **batch** is `{kind, texture, scissorRect, firstVertex, vertexCount, layerBreak}`. Consecutive emissions with the same kind, texture and scissor are merged.
- `layerBreak` marks where painter's order must be preserved across kinds (§5.9).

### 5.3 Paths, paints and tessellation

```java
// Geometry is built in local px space (after ou→px conversion) so the AA fringe is exactly 1 px.
Path path = PathBuilder.roundedRect(w, h, r, exponent).withDrips(dripSet).build();
Mesh fill   = Tessellator.fill(path, FillRule.SIMPLE);                       // interior + 1 px AA fringe
Mesh stroke = Tessellator.stroke(path, widthPx, Join.ROUND, Cap.ROUND);      // band + fringe on both sides
```

**Mesh vertex layout:** `x, y, u, v, coverage`. Here `(u, v)` are paint coordinates (normalised position in the shape's bounds) and `coverage` is the AA alpha (1 inside, 0 at the outer fringe).

Colours are computed **at emission time**: `argb = paint.eval(u, v) × coverage × opacity`. This lets colours and opacity animate without re-tessellating.

**`Paint` variants**

| Variant | Meaning |
|---|---|
| `Solid` | One colour |
| `Linear2` | Two stops along an axis in (u, v) |
| `VerticalAlphaMask` | Solid with α as a function of v (used for gloss, rim and inner shade) |
| `RadialAlpha` | Blobs and vignette |

Multi-stop gradients are only used by background meshes, which split their grid at stop positions (BACKGROUNDS §3.1).

**Triangulation**

| Shape | Method |
|---|---|
| Convex (rounded rects, capsules, circles) | Fan |
| Star-shaped (blobs) | Fan from the centre |
| Y-monotone (drips) | Strip |
| General simple polygons (icons with concavities, the stops editor) | Ear clipping, O(n²), n ≤ 512 |

**AA fringe:** each fill-boundary vertex is offset ±0.5 px along its miter normal (miter length clamped to 4). Inner ring coverage is 1, outer ring coverage is 0. Strokes have four rings. Strokes thinner than 1 px are drawn 1 px wide with coverage = width.

**Curve flattening:** adaptive subdivision with a tolerance of 0.15 px (DESIGN_SYSTEM R5).

### 5.4 Mesh caching

- `MeshCache` is an LRU of 512 meshes. The key is `(shapeKind, w_px, h_px, r_px, exponent, strokePx, dripsHash, scaleEpoch)`. Sizes are quantised to 1/4 px.
- **Static surfaces** (most of the UI) reuse meshes every frame. Emission is just transform + colour evaluation + copy.
- **Uniform and non-uniform scale animations** (inflate, squash) are applied as a transform at emission and do not invalidate the mesh.
- **Shape-changing animations** (radius settle, drips, the morphing indicator, blobs, breathing edges) re-tessellate while animating. They are bypassed in the cache via a `transient` flag.

### 5.5 OozeMaterialPainter

```java
public final class OozeMaterialPainter {
    /** Paints the full layer stack of DESIGN_SYSTEM §3 for one surface. */
    public void paint(Canvas c, ShapeSpec shape, MaterialSpec material, MaterialState state);
}
public record MaterialSpec(SurfaceClass surfaceClass, Elevation restingElevation, boolean drips, long stableId) {}
public final class MaterialState { // animated, owned by the component
    float elevation; float glossBoost; float pressMix; float shimmerPhase; Vec2 glossOffset; float opacity; float dripProgress; ...
}
```

The painter reads only resolved theme tokens and the effects quality tier. For example, Low quality uses 1 shadow layer with 3 rings, no shimmer and static drips.

### 5.6 Clipping

- **Rectangular scissor only** (in px, intersected on a stack). This maps directly to the vanilla GUI scissor.
- **Rounded clipping is not supported** in the Tessellated v1 path. Components must be designed so that a rectangular scissor plus **edge-fade masks** is enough. The masks are overlays drawn in the container's surface colour (COMPONENTS C19).

### 5.7 Textures

- `TextureHandle` is an opaque handle created through `RenderBackend.createTexture(w, h, TextureOptions{linear, clamp})`.
- Uploads go through `uploadRegion(handle, x, y, w, h, ByteBuffer rgba)` and run on the render thread only.
- Users of textures:

| User | Notes |
|---|---|
| Glyph atlas pages | Linear filtering |
| Background images (with mip levels as separate textures) | Linear |
| Blurred background copies | Linear |
| Thumbnails | Linear |
| Blue-noise tile | Nearest, repeat |
| Player face | Nearest |

### 5.8 Render backend SPI and vanilla submission

```java
public interface RenderBackend {
    BackendCapabilities capabilities();          // backendName ("OpenGL"/"Vulkan"), gpuName, trianglePipeline, maxTextureSize
    TextureHandle createTexture(int w, int h, TextureOptions o);
    void uploadRegion(TextureHandle t, int x, int y, int w, int h, java.nio.ByteBuffer rgba);
    void destroyTexture(TextureHandle t);
    void submit(DrawList list, Object nativeFrameContext); // nativeFrameContext = vanilla GuiGraphics
    void requestWorldBlur(float strength);        // §11.3
}
```

**Version implementation (1.21.6+ style deferred GUI rendering, which all targets use):**
1. For each batch, submit one custom **GUI element render state** into the frame's vanilla GUI render state. The element carries its pipeline, its texture binding, its scissor area and bounds (for culling), and builds its vertices from the DrawList arrays.
2. **Pipelines:**
   - **Preferred:** two pipelines **derived from the vanilla GUI pipeline snippets** with `TRIANGLES` primitive mode. They are vanilla shaders (`gui`, `gui_textured`) in Ooze-registered pipelines, so no custom shader code is needed.
   - **Fallback:** if pipeline registration is unavailable on a target, use the vanilla `GUI` / `GUI_TEXTURED` (QUADS) pipelines and emit each triangle as a degenerate quad `(v0, v1, v2, v2)`. This costs +33% vertices.
3. **Coordinates:** DrawList vertices are in px. The element applies scale `1 / mcGuiScale` so the vanilla projection maps them back to exact pixels.

### 5.9 Ordering

The DrawList records in **painter's order**.

> **Verify per target (M2):** whether the vanilla GUI render state may reorder elements with different pipelines or textures within one layer (stratum).

If it can reorder them, the backend inserts an explicit layer advance (the vanilla "next stratum" call) at every `layerBreak`. The Canvas sets `layerBreak` when:
- a textured batch follows a colour batch (or vice versa) **and** their bounds overlap; or
- a node is marked `isolatesLayer` (popovers, modals, tooltips, the drag ghost).

M2's acceptance test (TESTING §3) renders an overlap torture scene and compares it to a golden image.

---

## 6. UI toolkit

### 6.1 Node model

A retained tree:

```java
public abstract class Node {
    // structure
    Node parent; final ArrayList<Node> children;
    // layout (ou)
    LayoutParams params; Rect bounds; int dirty;            // DIRTY_LAYOUT | DIRTY_PAINT
    protected Size measure(Constraints c);                  // default: delegate to layout container
    protected void arrange(Rect finalBounds);
    // per-frame
    protected void update(FrameContext f);                  // animation targets from state
    protected void paint(Canvas c);                         // emit visuals (ou); children painted after by default
    // input (return true = consumed)
    protected boolean onPointer(PointerEvent e);  protected boolean onKey(KeyEvent e);  protected boolean onChar(CharEvent e);
    // focus & a11y
    protected boolean focusable(); protected NarrationInfo narration();
    // visual transforms for animation (applied around pivot at paint time, not affecting layout)
    final Affine2D visualTransform; float opacity = 1f;
}
```

**Layout containers:** `Row` and `Column` (flex: gap, padding, align, justify, grow, shrink, basis, min, max), `Stack` (overlay), `Grid` (auto-fill by minimum column width), `Absolute` (HUD and overlays), `Scroll` (C19 with virtualisation).

**Measuring** is cached per `(constraints, epoch)`. Layout runs only for dirty subtrees. Most frames run no layout at all.

### 6.2 Reactive properties

```java
public interface Property<T> { T get(); void set(T v); Subscription subscribe(Consumer<T> listener); }
```

- `Setting<T>` implements `Property<T>`. Components bind two-way: `toggle.bind(setting)`.
- `Computed<T>` derives a value from other properties (for example, a filtered module list).
- Subscriptions are owned by nodes and released on detach. There are no leaks across screen reopenings.

### 6.3 Input and focus

- Implemented per INTERACTION_FLOWS §3.
- `FocusManager` keeps focus scopes, roving-focus groups and the modality.
- **Cursor shapes** (arrow, hand, I-beam, resize, grab) are requested through `PlatformWindow.setCursor`, and the host applies them once per frame.

### 6.4 Layers and router

- `LayerStack` holds the fixed z-ordered layers (DESIGN_SYSTEM §9.5). Popovers, modals, toasts and tooltips are added to their layer with an anchor reference.
- `Router` keeps the shell's history and creates `Page`s from `Route`s:
  - `shell/modules/{category}`, `shell/settings`, `shell/theme`, `shell/background`, `shell/typography`, `shell/motion`, `shell/keybinds`, `shell/about`.
  - Each route has an optional anchor (for example `#setting:oozeclient:settings_showcase/accent_strength`) for deep links from search.

---

## 7. Theme runtime

- `ThemeManager` loads built-in and user `ThemeSpec`s (DESIGN_SYSTEM §8). It resolves inheritance and references into a `ResolvedTheme`: `int[] colors` and `float[] numbers` indexed by `ThemeToken` ordinals.
- **Transitions:** while switching, `ThemeManager.current()` returns an interpolated theme computed once per frame (OKLab lerp over `colors[]`).
- `ThemeValidator` produces contrast and range warnings for the Theme Studio.
- **Persistence:** the active theme id is stored in `client.json`. User themes live in `config/oozeclient/themes/<id>.theme.json`, written atomically.

---

## 8. Module system

### 8.1 Model

```java
public abstract class Module {
    protected Module(ModuleInfo info) { ... }
    public final ModuleInfo info();
    public final SettingGroup settings();                 // declared via fields, see 8.3
    public final boolean isEnabled();
    protected void onEnable(ModuleContext ctx) {}         // subscribe to events, register HUD widgets, etc.
    protected void onDisable() {}                         // ModuleContext auto-releases everything registered through it
}

public record ModuleInfo(
    OozeId id,                    // "oozeclient:settings_showcase" (namespace = providing mod id)
    Text name, Text description,
    Category category, IconRef icon, Set<String> tags,
    boolean sample,                   // shows the SAMPLE badge; excluded from "real" counts in About
    boolean defaultEnabled,
    KeyCombo defaultKey, Set<KeybindMode> keyModes,       // TOGGLE, HOLD
    Environment environment,          // ANY, IN_WORLD, MENU
    FairPlay fairPlay,                // COSMETIC, INFORMATIONAL, QUALITY_OF_LIFE, GAMEPLAY
    Set<Capability> requires,         // platform capabilities (e.g. HUD, WORLD_RENDER_HOOK); unmet → unavailable with reason
    String sinceVersion) {}
```

**Categories** are registered data `(id, name, icon, order)`.
- Built-in: `showcase` (samples), `hud`, `visual`, `utility`, `social`, `performance`.
- Categories with no available modules are hidden in release builds.

**Fair play:**
- `FairPlay` is mandatory metadata. A future `ServerPolicy` service (not in v0.1) may disable modules by classification when a server requests it.
- v0.1 contains only `COSMETIC` and `INFORMATIONAL` sample modules.

### 8.2 Registry and lifecycle

```java
public interface ModuleRegistrar { void register(Module m); void registerCategory(Category c); }
public interface ModuleProvider { void registerModules(ModuleRegistrar r); }   // "oozeclient:modules" Fabric entrypoint

public final class ModuleRegistry {
    Module get(OozeId id);  List<Module> byCategory(Category c);  List<Module> all();
    void setEnabled(Module m, boolean enabled, ToggleSource source);  // USER, KEYBIND, PROFILE_LOAD, ERROR, API
    Subscription onToggle(Consumer<ModuleToggledEvent> l);
}
```

- **Registration** is only allowed during bootstrap. Then the registry is frozen (it throws if modified later).
- **Enabling:** check `Environment` and `requires`. Create a `ModuleContext` (a scoped handle to the event bus, HUD manager, notifications and scheduler), call `onEnable`, and post `ModuleToggledEvent`.
- **Disabling:** call `onDisable`, close the context (unsubscribes everything), and post the event.
- **Exceptions** in `onEnable` or listeners: log them and auto-disable after the threshold (INTERACTION_FLOWS §4).

### 8.3 Settings

```java
public final class SettingsShowcaseModule extends Module {           // SAMPLE
    private final BooleanSetting sparkle   = setting(BooleanSetting.builder("sparkle").defaultValue(true));
    private final FloatSetting   strength  = setting(FloatSetting.builder("accent_strength").defaultValue(0.6f)
                                               .range(0f, 1f).step(0.05f).format(NumberFormat.PERCENT).group("appearance"));
    private final ColorSetting   highlight = setting(ColorSetting.builder("highlight").defaultValue(0xFFDCE8F7).alpha(false));
    private final EnumSetting<Style> style = setting(EnumSetting.builder("style", Style.class).defaultValue(Style.SOFT));
    private final KeybindSetting extraKey  = setting(KeybindSetting.builder("extra_key"));
    ...
}
```

| Type | Value | Extra metadata |
|---|---|---|
| `BooleanSetting` | boolean | — |
| `IntSetting` / `FloatSetting` | int / float | min, max, step, format, unit, logarithmic, precise |
| `EnumSetting<E>` | E (implements `Labeled`) | option icons, descriptions |
| `ColorSetting` | packed ARGB | alpha allowed |
| `KeybindSetting` | `KeyCombo` | — |
| `StringSetting` | String | max length, pattern |
| `StringListSetting` | `List<String>` | max items |
| `FontSetting` | family id | — |
| `ActionSetting` | — | callback, confirm text |
| `SettingGroup` | — | nesting (one level in the UI; deeper levels flatten into an accordion) |

**Common setting features**
- id (snake_case, unique in its module); name and description keys auto-derived as `module.<ns>.<module>.setting.<id>.name|desc`.
- Default value, a validator (clamp or reject), `Visibility` predicate (`visibleWhen(sparkle::get)`), search tags, `restartRequired` flag.
- `Codec<T>` for JSON. Invalid values fall back to the default and log a warning.
- A **SettingPath** (`oozeclient:settings_showcase/appearance/accent_strength`) is used by search, deep links and config.

### 8.4 Keybinds

- `KeyCombo(device: KEYBOARD | MOUSE, code: int (GLFW), modifiers: bitmask CTRL|SHIFT|ALT|SUPER)`.
- `KeybindManager` holds actions: the Ooze core actions and each module's toggle bind.
- **Core actions** (Open ClickGUI, Open HUD editor) are **also registered as vanilla key mappings** (category "Ooze Client") through the adapter. The vanilla mapping is the source of truth for those actions, and S10 edits it through the adapter.
- **Module binds** are Ooze-managed:
  - Evaluated on `KeyInputEvent` / `MouseButtonEvent` only when no screen is open.
  - A combo matches when its key is pressed and the required modifiers are held exactly. Extra modifiers do not match, so `Ctrl+K` ≠ `K`.
  - `TOGGLE` flips the module on press. `HOLD` enables it on press and disables it on release (or when focus is lost).
- Conflicts are computed across Ooze actions and vanilla mappings (SCREENS S10).

---

## 9. Configuration persistence

### 9.1 Files

```
config/oozeclient/
├── client.json              schema oozeclient.client/N   – global prefs: ui, typography, motion, effects, theme id, main menu, notifications, firstRun
├── keybinds.json            schema oozeclient.keybinds/N – core action binds mirror + per-profile module binds are in the profile
├── backgrounds/backgrounds.json + library/ + thumbs/ + inbox/
├── themes/<id>.theme.json
├── fonts/                   user fonts (read-only for Ooze)
├── profiles/<profileId>/modules.json  (enabled, settings, binds, favourites, recent)
│                        └── hud.json  (widget layouts)
├── profiles/index.json      profile list + active profile
└── backups/<yyyyMMdd-HHmmss>/…        rotating start-up snapshots (keep 5)
```

### 9.2 Format and codecs

- JSON via Gson with pretty printing (2 spaces) and stable key order (as declared), so diffs are readable.
- Every file starts with `"schema": "<name>/<int>"`.
- A `Codec<T>` per model type:
  - **Unknown fields are kept** in an `extras` `JsonObject` and written back, so newer configs survive older clients.
  - Missing fields take their defaults.

### 9.3 Migrations

- A `MigrationChain` per file type: `Migration(fromVersion, toVersion, UnaryOperator<JsonObject>)`, applied in sequence.
- If the version is newer than known, the file opens in **read-only** mode (INTERACTION_FLOWS §4).
- Migrations are pure functions with golden-file tests (TESTING §2).

### 9.4 Atomic and reliable writes

```
save(file, json):
  bytes ← serialize(snapshot)                     // snapshot taken on render thread; serialization on ooze-io thread
  write bytes → file.tmp ; FileChannel.force(true)
  if file exists: copy file → file.bak (REPLACE_EXISTING)
  move file.tmp → file (ATOMIC_MOVE, REPLACE_EXISTING)   // fallback: REPLACE_EXISTING without ATOMIC_MOVE if unsupported
load(file):
  try parse(file) → migrate → decode
  else try parse(file.bak) → … ; rename broken file to file.corrupt-<ts>.json
  else defaults
```

### 9.5 Save scheduling

- Any change marks its file dirty. `SaveScheduler` debounces for **750 ms** after the last change, with a **5 s maximum delay** under continuous edits.
- **Flushes** happen synchronously (bounded at 500 ms) when an Ooze screen closes, on world leave, on profile switch, and on `CLIENT_STOPPING`.
- **Backups:** at start-up, if any file changed since the last snapshot and the last snapshot is more than 24 h old, the client copies the config tree (excluding `library/`) to `backups/<timestamp>/` and keeps the last 5.

---

## 10. HUD system

```java
public abstract class HudWidget {
    protected HudWidget(HudWidgetInfo info) {}                   // id, name, sample, defaultAnchor, defaultOffset, resizable
    public abstract Size measure(HudContext ctx);                 // ou at scale 1.0
    public abstract void paint(HudContext ctx, Canvas canvas);    // origin at widget top-left
    public boolean visible(HudContext ctx) { return true; }       // e.g. hide while debug screen open (setting)
    public SettingGroup settings();
}
public record HudPlacement(OozeId widget, boolean enabled, Anchor anchor, Vec2 offsetOu, float scale, JsonObject settings) {}
```

- **`Anchor`:** 9 points. Position = `anchorPoint(viewport) + offset − anchorPivot(size × scale)`. Offsets are stored in ou, so a layout keeps its look across resolutions. Positions are clamped to safe bounds (8 ou margin) on load and on resize.
- **`HudManager`:** keeps the placements of the active profile and the widgets registered by modules (`ModuleContext.hud().register(widget)`).
- **`HudRenderer`:** called from the platform HUD hook after the vanilla HUD. It is skipped when the GUI is hidden (F1), when the HUD editor is open, or when Ooze HUD is toggled off.
- **Budget:** ≤ 0.25 ms CPU for 3 widgets (PERFORMANCE_AND_ACCESSIBILITY §2). Widgets cache their text layouts and re-lay out only when their content changes. FPS text, for example, changes at most 4 times per second (`updateHz`).
- **Sample widgets (v0.1):** `FPS` (real frames per second, updated 4×/s), `Clock` (system time, 12h/24h), `Ooze Watermark` (logo + "ooze"). Each is backed by a module in the `hud` category so the toggle → widget link is exercised.

---

## 11. Platform SPI and version adapters

### 11.1 Interfaces (core.platform)

| Interface | Responsibilities |
|---|---|
| `PlatformInfo` | Minecraft version, loader and API versions, Java, OS, backend name, GPU string, installed mod count |
| `PlatformWindow` | Framebuffer size, Minecraft GUI scale, focused/minimised, `setCursor(CursorShape)`, clipboard get/set |
| `RenderBackend` | §5.8 |
| `GlyphRasterizer` | Provided by `platform-lwjgl` (stb) |
| `PlatformImages` | Decode (STBImage, `platform-lwjgl`), plus texture creation via `RenderBackend` |
| `PlatformDialogs` | `openFile(filters) → CompletableFuture<Optional<Path>>` (tinyfd), `isSupported()` |
| `PlatformFiles` | Config, game and logs directories; `openFolder(path)`; `openUrl(url)` via the vanilla confirm-link screen |
| `PlatformText` | Vanilla text fallback (measure and draw runs), and conversion from the vanilla `Component` |
| `PlatformInput` | Key names (localised), current modifier state, vanilla key-mapping list (for conflicts), register core key mappings |
| `PlatformGame` | In world?; current FPS; open vanilla screens (singleplayer, multiplayer, Realms, options, language, accessibility, credits, controls); recent worlds (async); saved servers; join server; load world; quit; Distortion Effects value (for Auto reduced motion); Mod Menu present? |
| `PlatformAudio` | `playUiClick(pitch, volume)` |
| `PlatformNarrator` | `isActive()`, `narrate(String, interrupt)` |
| `Translator` | `translate(key, args)`, `languageCode()`, change listener |

### 11.2 Mixins (minimal and per version)

| Mixin target (Mojang names) | Purpose | Required? |
|---|---|---|
| `Minecraft#setScreen` (HEAD) | Replace `TitleScreen` with the Intro or `MainMenuScreen` when enabled | Optional. Without it, the Ooze button is added to the vanilla title screen via the Fabric screen events API |
| `KeyboardHandler#keyPress` (TAIL) | Post `KeyInputEvent` for module binds | Optional. Fallback: poll key state per client tick (can miss presses under 50 ms; documented) |
| `MouseHandler#onPress` (TAIL) | Post `MouseButtonEvent` for module binds | Optional |

- Mixins use `require = 0` for optional features, plus a runtime check that disables the feature and records a diagnostic.
- Prefer Fabric API events wherever they exist: client lifecycle, ticks, the HUD element registry, screen events, key binding helper.

### 11.3 World blur

`RenderBackend.requestWorldBlur(strength)` asks the adapter to apply the **vanilla menu-background blur** behind Ooze world screens. Where the target lets Ooze set the radius, `strength` maps to it. Otherwise Minecraft's *Menu Background Blur* option is used unchanged, and the UI says so (BACKGROUNDS §4.4). Ooze never writes to the user's vanilla options.

### 11.4 Compatibility adapters

| Integration | Behaviour |
|---|---|
| **Mod Menu** (optional compile-only dependency) | Provides a config screen factory → opens the shell's Settings page |
| **Other title-screen mods** | The "More from your mods" tray (SCREENS S02) |
| **Sodium, Iris** | No integration needed; only smoke-tested |

---

## 12. Search

- `SearchIndex` collects entries from `SearchProvider`s: modules, settings, pages, actions, themes, wallpapers and keybinds.
- **Entry fields:** `id`, `type`, `title`, `subtitle` (path), `keywords`, `icon`, `action` (a route with an anchor, or a runnable).
- The index is rebuilt incrementally on registry and theme changes and on language change.
- **Normalisation:** lowercase, NFD with combining marks stripped, and punctuation folded to spaces. The title, keywords and subtitle are precomputed as `char[]`s.

**Scoring** (the highest-scoring field wins; title ×1.0, keywords ×0.8, subtitle ×0.6):

| Match | Score |
|---|---|
| Exact title | 1000 |
| Title prefix | 800 |
| Every query token prefixes some word | 600 |
| Substring | 400 |
| Fuzzy subsequence | 100 + 15 per consecutive-run character + 30 per word-start character − 1 per gap character (no match if the subsequence fails) |

- **Bonuses:** +50 if picked in the last 10 selections; +25 if the type matches the current scope.
- **Ties:** broken by type order (Page > Module > Setting > Action > other), then alphabetically.
- **Performance:** ≤ 1 ms per keystroke for 2,000 entries on reference hardware. Queries run synchronously on the render thread.

---

## 13. Event system

```java
public final class EventBus {
    public <E extends Event> Subscription subscribe(Class<E> type, Consumer<? super E> listener);
    public <E extends Event> Subscription subscribe(Class<E> type, Priority priority, boolean receiveCancelled, Consumer<? super E> listener);
    public <E extends Event> E post(E event);   // client thread only (asserted in dev)
}
```

- **Exact-type dispatch** (no hierarchy walk) through a `Map<Class<?>, Listener[]>`.
- **Copy-on-write arrays** are sorted by priority (`HIGHEST … LOWEST`, then insertion order). Posting iterates a snapshot with no allocation.
- **`Cancellable` events:** listeners after cancellation are skipped unless `receiveCancelled`.
- **Failure isolation:** each listener call is wrapped. Exceptions are attributed to the owning module (subscription metadata) for auto-disable.
- No reflection or annotation scanning, which keeps start-up fast and makes ownership explicit.

**Event catalogue (v0.1):**
- Lifecycle: `ClientStartedEvent`, `ClientStoppingEvent`, `ClientTickEvent.Pre/Post`
- Screens: `ScreenOpenedEvent`, `ScreenClosedEvent`
- World: `WorldJoinEvent`, `WorldLeaveEvent`
- Input: `KeyInputEvent`, `MouseButtonEvent`
- Display: `ResolutionChangedEvent`
- Appearance: `ThemeChangedEvent`, `FontSettingsChangedEvent`, `BackgroundChangedEvent`
- State: `SettingChangedEvent(path)`, `ModuleToggledEvent`, `ProfileChangedEvent`
- Internal: `HudRenderEvent`

New gameplay events are added by version adapters when modules need them.

---

## 14. Threading model

| Thread | Work |
|---|---|
| **Render / client thread** | All UI, Minecraft calls, GPU uploads, event posting, and the module lifecycle |
| `ooze-io` (one, daemon) | Config serialisation and writes, file copies, backups |
| `ooze-worker` (two, daemon) | Image decode, mips, blur, palette and stats, glyph rasterization, font scanning |
| `ooze-dialog` (short-lived) | Blocking tinyfd calls |

- Results return to the render thread through `RenderThreadQueue`, which is drained at the start of each frame with a **2 ms budget**. Leftovers continue next frame.
- Workers never touch Minecraft objects or GPU resources.
- Shared state crossing threads is passed as immutable snapshots.

---

## 15. Diagnostics

- `FrameProfiler` measures `update`, `layout`, `paint` and `submit` (in ns) per frame. It keeps a 120-frame ring buffer and exposes avg, p95 and p99.
- `Counters` track vertices, batches, mesh-cache hits and misses, glyph uploads, atlas occupancy and allocated bytes per frame (`com.sun.management.ThreadMXBean#getThreadAllocatedBytes` when available).
- **Debug overlay** (Settings › Advanced): a compact Ink Gel panel at the top-left with the stats above.
- **Developer tools** (dev flag `-Doozeclient.dev=true`, or Settings › Advanced): Render Lab, Component Gallery, Type Lab.
- **Logging:** SLF4J logger `OozeClient`. One-time warnings are deduplicated by key.

## 16. Internationalisation

- `Text.translatable(key, args…)` / `Text.literal(s)`. These are resolved lazily by `Translator` and cached per language epoch.
- Language files live in `assets/oozeclient/lang/*.json` and are loaded by Minecraft's resource system, so language switching just works.
- **Key conventions:** `oozeclient.<area>.<thing>`, `module.<ns>.<id>.name`, `setting…` (§8.3).

---

## 17. Architecture decision records

| ADR | Decision | Rationale | Revisit when |
|---|---|---|---|
| 001 | **CPU-tessellated vector UI with analytic AA, submitted through vanilla GUI pipelines; no custom shaders in v1** | Works unchanged on OpenGL and Vulkan (26.2), and fits the deferred GUI renderer. 1 px analytic fringes give crisp edges at any scale. Colours can animate without re-tessellation | Custom pipelines are proven portable on all targets, enabling an optional SDF tier for smooth-union effects |
| 002 | **No NanoVG, no direct OpenGL calls** | NanoVG issues its own GL state changes. That conflicts with Blaze3D state tracking and is incompatible with a Vulkan backend | Never, for the shared path |
| 003 | **stb_truetype runtime rasterization at physical size, cached in atlases** | Pixel-exact text at rest at every resolution. stb is available in every target through LWJGL. Simpler than MSDF and sharper at small sizes | MSDF may be added for very large animated display text |
| 004 | **Mojang official names in every target's sources** (including 1.21.11 via Loom's official mappings) | 26.1+ is unobfuscated with Mojang names, so shared integration source compiles across all targets | — |
| 005 | **Separate distribution JAR per target with a shared core** | Mapping, Java and API breaks make a universal JAR unsafe | — |
| 006 | **Gson JSON, schema versions, migrations, atomic writes, backups** | Human-readable and diffable; Gson ships with Minecraft; crash-safe | Config size grows beyond about 1 MB |
| 007 | **Retained-mode UI tree with reactive properties** | Needed for animation state, focus, accessibility, layout caching and two-way settings binding | — |
| 008 | **Closed-form springs; ambient motion as pure functions of time** | Frame-rate independent, stable, deterministic in tests | — |
| 009 | **Ooze scale model independent of Minecraft's GUI scale** | Minecraft's integer GUI scale cannot give consistent sizes at 1440p and 4K | — |
| 010 | **Exact-type event bus with lambda subscriptions** | Fast, allocation-free, explicit ownership; no reflection | — |
| 011 | **Plain Gradle subprojects plus shared source directories, not a source preprocessor** | Three targets are few enough that adapter interfaces keep sources readable, with no comment-directive code | More than about 5 targets → evaluate Stonecutter |
