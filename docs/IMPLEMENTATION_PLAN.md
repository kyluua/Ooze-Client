# Ooze Client — Milestone Implementation Plan (for Codex)

> Deliverable 15. A sequence of realistic, testable milestones from an empty repository to the v0.1.0 "Gel" release candidate. Read [AGENTS.md](../AGENTS.md) first.

## 0. Working rules

1. **One milestone per pull request** (large milestones may be split into numbered parts, e.g. `M5a`, `M5b`). Each PR description lists the acceptance criteria as a checklist with evidence: test names, screenshots, profiler tables.
2. **All three targets must compile at every milestone from M0 onwards.** Runtime verification happens on the reference target (26.1.x), with smoke checks on 1.21.11 and 26.2.x (TESTING §10).
3. **The spec is the source of truth.** If something in the spec is impossible or clearly wrong, implement the closest faithful alternative, then document the deviation in the PR **and** update the relevant doc in the same PR.
4. **No placeholders presented as features.** If a UI element's behaviour comes in a later milestone, the element does not appear yet. Developer tools are hidden behind the dev flag.
5. **No gameplay modules.** Only the sample modules and widgets listed in M9 and M12.
6. **Tokens, not literals.** Colours, sizes, radii, durations, easings and springs come from tokens.
7. **Tests with the code.** Every milestone adds the unit and golden tests listed for it.

## 1. Milestone overview

| # | Milestone | Main outcome | Depends on |
|---|---|---|---|
| M0 | Toolchain and skeleton | Three target JARs build and load; CI; facts register verified | — |
| M1 | Core foundations | Maths, colour, clock, animation, events, config, i18n, with tests | M0 |
| M2 | Rendering foundation | DrawList, tessellator with AA, shapes, shadows, scale model, backends for all targets, Render Lab | M1 |
| M3 | Typography engine | Font registry, stb rasterizer, atlases, layout, fallbacks, Type Lab | M2 |
| M4 | UI toolkit core | Nodes, layout, input, focus, layers, scroll, screen host, router | M3 |
| M5 | Material and components I | Ooze material painter; C01–C09, C14, C19, C22, C25, C28 with full motion; Component Gallery | M4 |
| M6 | Components II | C10–C13, C15–C18, C20–C21, C23–C24, C26–C27, C29–C31, C33–C34 | M5 |
| M7 | Theme system and Theme Studio (S05) | Themes, live switching, validation, studio | M6 |
| M8 | Background system and Background Studio (S06) | All background types, image pipeline, wallpapers, legibility guard | M7 |
| M9 | Modules, settings, keybinds, shell and search | Registry, samples, ClickGUI shell (S03), inspector, command palette (S11) | M8 |
| M10 | Main menu, intro, settings, about | S01, S02, S04, S12, welcome flow, profiles, import/export | M9 |
| M11 | Typography page, motion page, keybinds page | S07, S08, S10, adaptive governor | M10 |
| M12 | HUD framework and HUD editor | HUD system, sample widgets, S09 | M11 |
| M13 | Liquid polish and cross-version hardening | Drip lifecycles, breathing edges, surface tension, particles, intro polish, 26.2 Vulkan verification, performance pass | M12 |
| M14 | Release candidate | Accessibility audit, robustness tests, packaging, licences, docs, CI release | M13 |

---

## M0 — Toolchain and skeleton

**Goal:** an empty Ooze mod builds and loads on all three targets, and the platform facts are verified.

**Tasks**
- [ ] Gradle wrapper, `settings.gradle.kts`, version catalog and the `build-logic` convention plugins (REPOSITORY_STRUCTURE §1).
- [ ] Projects `:core`, `:platform-lwjgl`, `:versions:mc1_21_11`, `:versions:mc26_1`, `:versions:mc26_2`, plus the `mc-shared` source directory wired into each version project.
- [ ] Loom configuration per MULTI_VERSION §2:
  - 1.21.11: remapping plugin + `officialMojangMappings()`, Java 21.
  - 26.x: non-remapping plugin, Java 25.
- [ ] A `fabric.mod.json` per target (id `oozeclient`, ranges, Java), and an empty `oozeclient.mixins.json`.
- [ ] Client entrypoint logs `Ooze Client <ver> for <target> (backend: <name>)`.
- [ ] Jar naming `ooze-client-<ver>+mc<target>.jar`; `buildAll` collects the JARs into `build/dist/`.
- [ ] Spotless and an ArchUnit test skeleton (`core` must not depend on `net.minecraft..`, `com.mojang..`, `net.fabricmc..`, `org.lwjgl..`).
- [ ] CI workflow `build.yml`: build and unit tests on push and PR.
- [ ] **Verify the facts register** (MULTI_VERSION §7). Mark each item ✓ or corrected in the doc.
- [ ] Copy `docs/tokens/ooze_classic.theme.json` to `core/src/main/resources/assets/oozeclient/themes/`.

**Acceptance criteria**
- `./gradlew build` produces three JARs.
- Each target's `runClient` reaches the vanilla title screen with the log line.
- CI is green.
- The facts register is updated.

**Out of scope:** any UI.

---

## M1 — Core foundations

**Goal:** the Minecraft-independent building blocks, fully unit-tested.

**Tasks**
- [ ] `core.math`, `core.color` (OKLab, contrast, mix) and `core.anim`:
  - `FrameClock`, `CubicBezier`;
  - closed-form `Spring` and `SpringSpec` tokens; `Tween`;
  - `AnimatedFloat/Vec2/Color/Rect`, `Animator` (awake list), `Motion` resolution with user settings (MOTION_SPEC §3, §8, §10).
- [ ] `core.event` `EventBus` (ARCHITECTURE §13).
- [ ] `core.config`: `Codec`, `ConfigFile`, `AtomicFiles`, `MigrationChain`, `SaveScheduler` (fake-clock testable), `ConfigStore` and the directory layout (ARCHITECTURE §9).
- [ ] `core.i18n` `Text` + `Translator` SPI; `core.platform` interfaces as stubs (§11.1); `Ooze.bootstrap` skeleton with readiness gates.
- [ ] `core.diag` `FrameProfiler` and `Counters`.

**Acceptance criteria**
- All TESTING §2 rows for springs, easing, colour, config and events are implemented and pass.
- Coverage of `core.anim`, `core.config` and `core.event` is ≥ 85% lines.

---

## M2 — Rendering foundation

**Goal:** crisp, anti-aliased vector shapes on screen on every target, with the Ooze scale model.

**Tasks**
- [ ] `DrawList`, `Batch`, `Canvas` (ou → px, transform, opacity and clip stacks, snapping), `Paint`, `Path`/`PathBuilder` (superellipse rounded rects, capsules, circles, blobs, drips), `Tessellator` (fan, monotone and ear-clipping fills; strokes; 1 px AA fringe; flattening tolerance 0.15 px), `MeshCache` with `ScaleEpoch` (ARCHITECTURE §5.2–5.4).
- [ ] `ShadowPainter` (5-ring approximation of a Gaussian) and the elevation tokens.
- [ ] `RenderBackend` implementations for all three targets (§5.8): derived TRIANGLES pipelines with the degenerate-quad fallback, scissor, px → GUI scaling, textures (create, upload, destroy).
- [ ] The **ordering investigation** (§5.9): implement `layerBreak` handling according to the findings, and record the findings in ARCHITECTURE §5.9.
- [ ] Generic `OozeScreenHost` in `mc-shared` (render and resize only at this stage), plus a dev-only **Render Lab** screen showing the primitives, outlines at all widths, shadows e1–e4, drips, blobs, gradient meshes and the ordering torture scene.
- [ ] The reference rasterizer and golden tests (TESTING §2).

**Acceptance criteria**
- Golden tests pass for the primitives.
- Render Lab passes the crispness checklist (TESTING §5) at 1080p, 1440p and 4K on the reference target.
- Render Lab renders identically on 1.21.11 and 26.2 (OpenGL and Vulkan) within golden tolerance.
- Render Lab "Stress" (2,000 rounded rects with outlines and shadows) runs ≤ 2 ms CPU on reference hardware.

---

## M3 — Typography engine

**Goal:** pixel-exact text in four bundled families, with fallbacks.

**Tasks**
- [ ] Download and bundle the static TTFs and `OFL.txt` files **unmodified** (TYPOGRAPHY §2). Write `fonts.json` with the verbatim copyright lines. Copy the licences to `licenses/fonts/`.
- [ ] `platform-lwjgl`: `StbGlyphRasterizer` (subpixel buckets, synthetic italic via outline shear, synthetic bold dilation, light-on-dark gamma variant).
- [ ] `FontRegistry` (bundled plus a user folder scan), `FaceMatcher`, size normalisation, `GlyphAtlas` (skyline packer, LRU pages, budgeted uploads), pre-warming.
- [ ] `TextEngine`: layout, line breaking, ellipsis, tracking, baseline snapping, layout cache, measurement; `ScriptDetector` and the vanilla fallback via `PlatformText`.
- [ ] `TypeRole` table and `TextStyle`; Canvas `text()`.
- [ ] Dev-only **Type Lab**: every role in every bundled family and style, synthesized markers, a scale slider, and a glyph atlas viewer.

**Acceptance criteria**
- TYPOGRAPHY §9 checklist passes.
- Unit tests for layout and matching pass.
- Changing the window size re-keys glyphs with no visible blur at rest.
- Arabic and Devanagari sample strings render via the vanilla fallback.
- Licence files are present in the JAR.

---

## M4 — UI toolkit core

**Goal:** a retained UI framework that can host interactive screens.

**Tasks**
- [ ] `Node`, `Root`, layout containers (`Row`, `Column`, `Stack`, `Grid`, `Absolute`), constraints, measuring cache and dirty flags (ARCHITECTURE §6.1).
- [ ] `Property`, `Computed`, subscriptions tied to node lifetime (§6.2).
- [ ] Pointer and keyboard dispatch, capture and bubble, pointer capture, modality, `FocusManager` (scopes, roving focus, `F6`), the Esc chain plumbing (INTERACTION_FLOWS §3).
- [ ] `LayerStack` (popover, modal, toast and tooltip layers with anchors and flip/shift placement); `Router` with history and anchors.
- [ ] `Scroll` container (smoothing, rubber band, edge fades, scrollbar, virtualisation) (MOTION_SPEC §4.7).
- [ ] `OozeScreenHost` complete: all input callbacks, cursor shapes, file drop, narration bridge (`PlatformNarrator`), and the `isPauseScreen` / `renderBackground` overrides.

**Acceptance criteria**
- A dev test screen with nested scroll containers, a focusable grid and a popover passes keyboard navigation and the Esc chain.
- Narration speaks focus changes when the narrator is on.
- No allocation is observed in steady state with the pointer idle.

---

## M5 — Ooze material and components I

**Goal:** the signature look and feel on the core controls.

**Tasks**
- [ ] `OozeMaterialPainter`: every layer in DESIGN_SYSTEM §3 (body gradient, inner shade, gloss band, rim light, glint, outline, static drips), surface classes, quality tiers.
- [ ] Focus ring (COMPONENTS §1.3) and `InteractionState`.
- [ ] Components with all states, motion and keyboard behaviour:
  - C01 Button (all variants and sizes, loading, hover drip), C02 Icon button;
  - C03 Toggle (stretch, landing), C04 Checkbox, C05 Radio, C06 Segmented (liquid morph);
  - C07 Slider (bubble, meniscus, ripple), C08 Number field, C09 Text field (full editing);
  - C14 Card, C19 Scroll (styled), C22 Tooltip (drip tail), C25 Badge/chip (Sample badge), C28 Divider.
- [ ] The icon pipeline: `iconCompile` task, Lucide subset plus brand icons, `IconRenderer`.
- [ ] Dev-only **Component Gallery**: every component × every state, live and interactive.

**Acceptance criteria**
- COMPONENTS §13 cells for these components are verified in the Gallery in light and dark themes.
- Motion matches MOTION_SPEC §4.1–4.4 and §4.9 (reviewed in a screen recording attached to the PR at 60 and 144 Hz).
- Reduced and Off modes are correct.
- Golden screenshots of the Gallery exist at 3 resolutions.

---

## M6 — Components II

**Goal:** the complete component library.

**Tasks**
- [ ] Pickers: C10 Dropdown (type-ahead, search > 8), C11 Color picker (SV mesh, strips, inputs, swatches, contrast readout and Fix), C12 Font selector, C13 Keybind chip (listening, conflict badge).
- [ ] Navigation: C15 Panel/window (drips, title bar, drag), C16 Sidebar (rail, overlay in compact), C17 Nav item (shared morph indicator), C18 Tabs.
- [ ] Overlays: C20 Modal (focus trap, kinds), C21 Popover, C23 Toast (stack, timers, in-world compact variant), C24 Loaders, C26 Accordion, C27 Context menu, C29 Empty state, C30 Preview frame, C31 Setting row (mapping per setting type, reset, search pulse), C33 Decorations, C34 Logo (all variants with the morph parameter).

**Acceptance criteria**
- The Gallery covers all of COMPONENTS §13.
- Each overlay passes focus trap and return tests.
- Golden screenshots are updated.

---

## M7 — Theme system and Theme Studio (S05)

**Tasks**
- [ ] `ThemeSpec`, `ThemeResolver` (inheritance, references, alpha), `ResolvedTheme` arrays, `ThemeValidator`, `ThemeManager` with the OKLab transition (DESIGN_SYSTEM §8, ARCHITECTURE §7).
- [ ] The five built-in themes (DESIGN_SYSTEM §7.6), each validating with zero contrast failures.
- [ ] Component `TOKENS` descriptors.
- [ ] S05 Theme Studio, complete per SCREENS S05: duplicate-on-edit, token editor, live preview, token highlight pulse, contrast panel with Fix, compare, undo/redo, import and export.

**Acceptance criteria**
- Theme switch animation is smooth (no frame > budget).
- An exported theme re-imports identically.
- An invalid theme file shows warnings and still loads with fallbacks.

---

## M8 — Background system and Background Studio (S06)

**Tasks**
- [ ] `BackgroundSpec` and its codec; `BackgroundManager` (contexts, inheritance, crossfades); singleton `BackgroundRenderer` (BACKGROUNDS §3, §6).
- [ ] Base layers: solid, gradient (OKLab mesh), image (fit, focus, zoom, mips), wallpaper, animated gradient, world.
- [ ] Effects (blur cache, dim, tint, vignette, grain) and ambient layers (blobs, particles, parallax).
- [ ] `ImageLibrary`: tinyfd dialog (with hide-if-unsupported), drag-drop, inbox folder, worker pipeline (hash, copy, decode, normalise, mips, thumbnail, stats, palette), memory policy.
- [ ] The six procedural wallpapers.
- [ ] `LegibilityGuard`, the background-text treatment and the plate.
- [ ] S06 in docked mode with the window morph, peek, context mini-previews, all editors (stop editor, angle dial, focal picker), readability badge, undo/redo and revert.

**Acceptance criteria**
- Flow F5 passes.
- Importing a 4K PNG keeps the frame budget.
- GPU memory stays within budget.
- Busy-image text gets a plate.
- No banding on gradient wallpapers.

---

## M9 — Modules, settings, keybinds, shell and search

**Tasks**
- [ ] `core.module`, `core.setting` and `core.input` per ARCHITECTURE §8: registry, contexts, fair-play metadata, capabilities, setting types and codecs, `KeybindManager` (toggle and hold), core vanilla key mappings via the adapter, optional key mixins with the polling fallback.
- [ ] Profiles: `modules.json` per profile; favourites and recents.
- [ ] **Sample modules** (all `sample = true`, category `showcase`):
  - **Settings Showcase:** one of every setting type, with groups and visibility predicates. It has no gameplay effect and its description says so.
  - **Notification Playground:** action settings that post each toast type (real notifications).
- [ ] S03 shell: window, sidebar, header controls, card grid and list view, inspector (push, overlay or sheet), context menu, empty states, the `Right Shift` open key, world blur and dim, pause setting, remember last page.
- [ ] `core.search` and the S11 command palette (field morph, scopes, deep links with the row pulse, recents, Ctrl+Enter).
- [ ] Mod Menu config screen factory (optional dependency).

**Acceptance criteria**
- Flows F2, F3, F8 and F9 pass (gametests where scripted).
- Module auto-disable works.
- Search ranking fixtures pass.
- Cross-version gate (TESTING §4).

---

## M10 — Main menu, intro, settings, about

**Tasks**
- [ ] Title-screen interception (mixin, with the Fabric screen-event fallback that adds an Ooze button), preserving the vanilla accessibility onboarding.
- [ ] S01 Intro (MOTION_SPEC §6 timeline, loading fallback, skip) and the Welcome sheet (3 steps, live apply).
- [ ] S02 Main menu: every region per SCREENS S02, including the Continue card (async worlds), Recent servers (ping off by default), What's new (`changelog.json` generated from `CHANGELOG.md`), the profile chip with the nearest-filtered face, the compatibility tray, footer links and the compact layout.
- [ ] S04 Settings, complete (all sections, filter, profiles management, import/export zip with preview, reset all).
- [ ] S12 About (real versions, renderer and GPU, copy debug info, changelog, credits, licences with the viewer modal, links via confirm-link).

**Acceptance criteria**
- The S02 checklist in TESTING §8 passes, with every vanilla function reachable.
- F1 and F10 pass.
- The About licences are complete.

---

## M11 — Typography page, motion page, keybinds page

**Tasks**
- [ ] S07 Typography (global and per-role settings, preview modes, contrast, synthesized chips, licence panel, user fonts folder and rescan).
- [ ] S08 Motion & effects (all settings, Auto reduced motion from Distortion Effects, quality tiers, Custom effects, Motion Lab with the spring plot and live frame stats).
- [ ] The adaptive quality governor (PERFORMANCE_AND_ACCESSIBILITY §3).
- [ ] S10 Keybinds (filters, conflicts with vanilla, hold mode, fixed shortcuts list, Open controls).

**Acceptance criteria**
- F6 passes.
- Every motion setting is observably applied in the Motion Lab.
- The governor demotes and promotes in the benchmark scene with hysteresis.

---

## M12 — HUD framework and HUD editor

**Tasks**
- [ ] `core.hud` (ARCHITECTURE §10): widgets, placements, anchors, clamping, HUD renderer via the platform HUD hook, F1 and editor suppression.
- [ ] **Sample HUD modules and widgets** (category `hud`, `sample = true`): FPS, Clock and Ooze Watermark, each with 2–4 settings (style Compact/Gel, colour, show label).
- [ ] S09 HUD editor: in-world and preview modes, vanilla safe zones per GUI scale, snapping and guides, Alt suspend, marquee and multi-select, scale handles, settings popover, widget library drawer, toolbar auto-hide, keyboard nudge and scale, undo/redo, save on exit.
- [ ] The in-world compact toast for module keybind toggles.

**Acceptance criteria**
- F7 passes.
- HUD budget: 0 allocations in steady state, ≤ 0.25 ms.
- Layouts survive resolution changes (anchor-relative).

---

## M13 — Liquid polish and cross-version hardening

**Tasks**
- [ ] The drip lifecycle state machine (grow, hold, retract or detach with a falling droplet; MOTION_SPEC §4.11) on windows, modals, the logo plate and the Dripline wallpaper.
- [ ] Breathing edges (Ultra), surface tension (Ultra, opt-in), ambient attraction, specular parallax, final shimmer tuning.
- [ ] Intro polish: the liquid-wipe reveal and the shared-element move to the menu logo slot.
- [ ] **26.2 verification** on OpenGL and Vulkan (MULTI_VERSION §4), including texture recreation on a backend or resource reload.
- [ ] The full performance pass (TESTING §6) on all scenes and targets. Fix any budget violations.

**Acceptance criteria**
- The cross-version matrix is all green.
- Performance tables meet the budgets.
- A motion review recording at 60, 144 and 240 Hz is attached.

---

## M14 — Release candidate (v0.1.0 "Gel")

**Tasks**
- [ ] Accessibility audit (PERFORMANCE_AND_ACCESSIBILITY §6, every checkbox) and a narration pass.
- [ ] Robustness suite (TESTING §7).
- [ ] Complete the translations audit (no raw keys); `en_us.json` reviewed for voice (PRODUCT_VISION §8).
- [ ] Licences: `licenses/` folder complete; About lists everything; `assets-src/LICENSES.md` is accurate.
- [ ] `release.yml`: on a tag, build all targets and attach the JARs and checksums.
- [ ] Update README (install instructions per target, screenshots), `CHANGELOG.md` and the docs (resolve every recorded deviation).

**Acceptance criteria**
- TESTING §10 Definition of Done holds for the whole codebase.
- All checklists in TESTING §5–§9 are completed and attached to the PR.
- A tagged pre-release build exists with three JARs.

---

## 2. Effort guidance (relative)

| Size | Milestones |
|---|---|
| Large | M2, M3, M5, M8, M9 |
| Medium | M4, M6, M7, M10, M12, M13 |
| Small | M0, M1, M11, M14 |

## 3. Risk register

| Risk | Impact | Mitigation |
|---|---|---|
| GUI render-state ordering reorders mixed batches | Visual layering bugs | M2 investigation, `layerBreak` and stratum advances, the torture-scene golden |
| Derived pipelines unavailable or failing on Vulkan | No triangle mode | Degenerate-quad fallback with vanilla pipelines (built in M2, tested on all targets) |
| tinyfd missing on a target or OS | No native file picker | Drag-drop and inbox folder fallbacks; the button is hidden |
| Font licence constraints | Legal | Unmodified files, verbatim OFL, attribution UI (TYPOGRAPHY §10) |
| Vertex volume at 4K | Frame time | Mesh caching, virtualisation, quality tiers, adaptive governor |
| API churn in hotfixes | Broken JAR | Narrow ranges per target, gametests before widening |
| Bubblegum Sans legibility at small sizes | Readability | 11 ou minimum, size normalisation, Body font override, Bolder text |
