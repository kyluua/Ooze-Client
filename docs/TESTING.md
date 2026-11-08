# Ooze Client — Acceptance Criteria & Testing Checklist

> Deliverable 14. The test strategy, automated suites, manual checklists and the global Definition of Done. Milestone-specific acceptance criteria are in [IMPLEMENTATION_PLAN.md](IMPLEMENTATION_PLAN.md).

## 1. Test pyramid

| Layer | Tooling | Runs | Covers |
|---|---|---|---|
| Unit | JUnit 5 + AssertJ (`:core:test`) | Every build (CI) | Math, colour, animation, tessellation invariants, text layout, search, config, events, modules, settings, HUD layout |
| Golden render (headless) | Core **reference rasterizer** + PNG goldens | Every build | Shapes, AA fringe, material layers, drips, icons, ordering logic |
| Architecture | ArchUnit | Every build | Layer rules (core has no Minecraft imports; no GL or Vulkan imports in `versions/*`) |
| Client gametests | Fabric client gametest API, per target | CI where a GPU or software renderer is available, otherwise at milestone gates | Screens open, real rendering, screenshot goldens at 3 resolutions, input flows |
| Manual QA | Checklists in §8–§9 | Milestone gates | Look and feel, motion quality, accessibility |
| Performance | FrameProfiler scenes (§6) | Milestone gates | Budgets |

## 2. Unit test requirements (minimums)

| Area | Required tests |
|---|---|
| Springs | The closed form matches a high-resolution numerical integration (RK4, dt = 1e-5) within 1e-4 for each token. Frame-rate independence: 30 / 60 / 144 / 240 FPS step sequences reach the same value at t = 0.5 s (±1e-4). Overshoot matches the tables in MOTION_SPEC §3.2 (±0.5%). Sleep thresholds |
| Easing | Cubic-bezier endpoints, monotonicity for monotone curves, tokens against reference samples |
| Colour | OKLab round-trip (≤ 1/255 error), contrast ratios for the DESIGN_SYSTEM §7.3 table (exact to 0.01), auto `text.onAccent` choice |
| Theme | Reference resolution, inheritance, alpha syntax, cycles reported, unknown keys preserved, built-ins validate with no contrast failures |
| Tessellator | Watertight fills (the area of the triangulated interior plus half the fringe equals the analytic area ± 0.5%). No degenerate or inverted triangles. Fringe width exactly 1 px. Superellipse segment counts per formula. Drip paths are y-monotone. Ear clipping on concave icons |
| Text | Line breaking (spaces, hyphens, CJK, long words), ellipsis, letter spacing, baseline snapping, face matching (all §4.2 rules), size normalisation, script detection routes to fallback |
| Search | Ranking fixtures (exact > prefix > word-prefix > substring > fuzzy), diacritics, scopes, recency boost, ≤ 1 ms for 2,000 entries (JMH-style microbenchmark, reported but not gating in CI) |
| Config | Atomic write sequence (with fault injection between steps), `.bak` fallback, corrupt quarantine, migrations (golden JSON in → golden JSON out), newer-schema read-only, unknown-field round-trip, debounce timing with a fake clock |
| Events | Priority order, cancellation, `receiveCancelled`, unsubscribe during dispatch, exception isolation and attribution |
| Modules | Lifecycle (enable and disable release all context subscriptions), environment and capability gating, auto-disable after 3 errors in 10 s, registry freeze |
| Settings | Codecs for every type, validation and clamping, visibility predicates, paths |
| Keybinds | Exact-modifier matching, toggle vs. hold, hold release on focus loss, conflict detection |
| HUD | Anchor maths for all 9 anchors, clamping on resize, snapping candidates and thresholds |
| Allocation | HUD paint path allocates 0 bytes per frame after warm-up (`getThreadAllocatedBytes`, skipped if unavailable) |

### Reference rasterizer

`core/src/test/java/…/render/RefRaster` is a CPU triangle rasterizer for DrawList colour and textured batches.
- It works at 4× supersampling, uses straight-alpha `SRC_ALPHA, ONE_MINUS_SRC_ALPHA` blending in sRGB (matching vanilla), honours scissor, and writes PNG.
- Goldens live in `core/src/test/resources/golden/`. The comparison tolerance is a per-channel absolute difference ≤ 2 on 99.5% of pixels, with none > 16.
- Update goldens with `-Pgolden.update=true`. The PR must explain each change.

## 3. Client gametests (per target)

Using the Fabric client gametest API (MULTI_VERSION V8):

1. **Boot:** reach the Ooze main menu. The intro is skipped via the test setting. No errors in the log.
2. **Screens:** for each of S01–S12, at window sizes **1920×1080, 2560×1440 and 3840×2160** and Minecraft GUI scale Auto, open the screen, wait for animations to settle (`Animator.allSettled()` hook), and screenshot. Compare to goldens (same tolerance as §2; separate golden sets per target are allowed when vanilla content differs).
3. **Overlap ordering torture scene** (Developer › Render Lab › "Ordering"): text, shapes and images interleaved and overlapping. The golden must match exactly in its ordering regions (validates ARCHITECTURE §5.9).
4. **Flows:** scripted F2 (toggle a module and change a setting via simulated input), F3 (search navigation), F7 (HUD drag with keyboard nudges). Assert the config files are written with the expected values.
5. **World:** create a flat singleplayer world, open the shell with `Right Shift`, check the world blur and dim, close it, and check the HUD widgets render in the expected rects.
6. **Resize:** change the window size during the shell; assert no exceptions, the scale epoch is bumped, the glyph cache is re-keyed and the layout is valid.

## 4. Cross-version matrix (milestone gates M2, M9, M13, M14)

| Target | Backend | Windows | macOS | Linux |
|---|---|---|---|---|
| 1.21.11 | OpenGL | ✓ | ✓ | ✓ |
| 26.1.x | OpenGL | ✓ | ✓ | ✓ |
| 26.2.x | OpenGL | ✓ | ✓ | ✓ |
| 26.2.x | Vulkan | ✓ | (if supported by the game) | ✓ |

Each cell: gametests §3 pass, manual smoke (open every screen, toggle a module, change theme and background, HUD drag), and no new warnings in the log. GPU vendors across the matrix: at least NVIDIA, AMD, Intel and Apple Silicon.

## 5. Crispness checklist (DESIGN_SYSTEM §11)

Run at 1920×1080, 2560×1440, 3840×2160 and a 1280×720 window, with UI scale 100% and 125%:

- [ ] Zoomed 800% crops of a button edge, toggle knob, card corner and drip show an AA transition exactly 1 px wide, with no stair-stepping.
- [ ] Straight outline segments are whole-pixel aligned. A 1.5 ou outline renders as 2 px at 1440p (1.333 scale) and 3 px at 4K, with no half-transparent double lines.
- [ ] At rest, text glyph texels map 1 : 1 to screen pixels. Dev assertion: no text quads with scale ≠ 1 outside animation.
- [ ] Icons are sharp at 16/20/24/32 ou on all scales. A 1.75-stroke icon at 24 ou on 4K renders 3.5 px strokes with smooth AA.
- [ ] No background shows visible banding (gradient test wallpapers with grain at the default level).
- [ ] An image smaller than the screen shows the quality notice in S06 and is never silently upscaled without it.
- [ ] Changing Minecraft's GUI scale does not change Ooze UI sizes.

## 6. Performance test procedure

1. Scenes: those listed in PERFORMANCE_AND_ACCESSIBILITY §2.1, scripted via Developer › Render Lab › "Benchmarks", which drives deterministic input such as hovering cards in sequence and dragging a slider.
2. For each target on the reference tier: 10 s warm-up, then a 60 s capture. Export the FrameProfiler CSV (Settings › Advanced › Export frame stats).
3. Pass when p95 ≤ budget, the allocation counter is within budget, and the adaptive governor did **not** demote at the default tier on reference hardware.
4. Record the results in the milestone PR (a table per scene).

## 7. Robustness tests

| Test | Pass condition |
|---|---|
| Kill the process (SIGKILL / Task Manager) during repeated saves (a scripted setting change loop, 200 iterations) | Every subsequent start loads a valid config. At most the last 5 s of changes are lost |
| Corrupt `client.json` (truncate it) | The client starts, restores from `.bak`, quarantines the file and shows the toast |
| Delete every config file | Defaults load. The welcome sheet is shown once |
| Config from a future schema | Read-only mode with the warning. The file is unchanged after exit |
| Missing background image | Falls back to the default wallpaper with a toast |
| Missing or corrupted user font | The font is listed as unavailable and falls back. No crash |
| Disable optional mixins (test config) | Features degrade as documented. No crash |
| Module throws in a listener | Auto-disabled after the threshold with a toast. The game continues |
| 4K PNG import while dragging a slider | Frame p95 within the Theme Studio budget throughout |

## 8. Manual QA — per screen (summary checklist)

For **every screen**:
- [ ] Opens and closes with the specified motion.
- [ ] Works fully by keyboard, in tab order.
- [ ] Esc follows the precedence chain.
- [ ] Narration reads the title.
- [ ] Compact, Regular and Wide layouts are correct.
- [ ] Reduced motion and Off modes are correct.
- [ ] All four built-in light and dark themes plus High Contrast look correct.
- [ ] Every control is real (no dead buttons).
- [ ] Sample content is labelled.

Screen-specific:

| Screen | Checks |
|---|---|
| S01 | Skip works on any key or click. The loading fallback shows when start-up is slow (simulated delay). Vanilla onboarding comes first when required. All three welcome steps apply live and persist |
| S02 | Every vanilla function is reachable (Singleplayer, Multiplayer, Realms, Options, Language, Accessibility, Quit, credits via the copyright line). The Mods button appears only with Mod Menu installed. The compat tray appears only with a test mod that adds a title button. Continue loads the last world. Join connects. Ping is off by default |
| S03 | Toggle vs. card-body click vs. right-click. The inspector pushes or overlays per breakpoint. Filters and sort work. Favourites persist. Profile switch works. Badge counts update |
| S04 | Every setting persists and applies live. Reset all requires typing RESET. Import and export round-trip |
| S05 | Built-in themes are read-only (duplicate prompt). Contrast warnings and Fix work. Compare hold works. Undo/redo works. Export and import round-trip. The theme switch animation is smooth |
| S06 | All base types work. Import via dialog, drag-drop and folder. Fit, zoom and focal point behave. Blur buckets never block. Readability badge states are correct. Peek works. Docked morph in and out |
| S07 | Font switch re-lays out in one frame without jumping. Synthesized chips appear correctly. Licence modal shows the full text. Text scale clamps at the 11 ou minimum |
| S08 | Every setting is observable in the Motion Lab. The spring plot is correct. Frame stats show real numbers. The adaptive label reflects demotions (force one via the benchmark) |
| S09 | Snapping and guides. Alt suspends snapping. Anchors are chosen correctly. Scale handles work. Safe zones are drawn at the correct vanilla positions for GUI scales 1–4. Keyboard nudging. Undo/redo. The HUD is not drawn twice |
| S10 | Listening mode, clear, cancel. Conflicts with Ooze and vanilla binds. Hold mode works in-world. Core actions also appear in vanilla Controls |
| S11 | Ranking feels right on the fixtures. Deep links pulse the row. Ctrl+Enter keeps the palette open. Scopes cycle. Recent items persist |
| S12 | Versions are accurate on every target. The renderer shows OpenGL or Vulkan. Copy debug info works. Licences are complete (Ooze MIT, four OFL fonts, Lucide ISC) |

## 9. Manual QA — component states

For every component in the COMPONENTS §13 matrix, verify each ✓ cell visually (normal, hover, pressed, selected, disabled, focus-visible), in light and dark themes, at 1440p. Use the **Component Gallery** (Developer tools), which lays out every component in every state with live interaction.

## 10. Definition of Done (global)

A change is done when **all** of the following hold:

1. It builds for all three targets (`./gradlew build`). Unit, golden and ArchUnit tests pass. Spotless is clean.
2. It works at runtime on the reference target (26.1.x) and passes a smoke check on 1.21.11 and 26.2.x (OpenGL **and** Vulkan for rendering-related changes).
3. No hard-coded colours, sizes or timings in components: everything goes through tokens.
4. Every user-facing string is in `en_us.json`.
5. Keyboard operation, focus-visible styling, Reduced motion and Off modes are implemented for any new interactive element.
6. No visible control is non-functional. Sample content carries the **Sample** badge.
7. Performance budgets are respected for the affected scenes.
8. The docs are updated if behaviour deviates from the spec. The deviation is recorded in the PR description and in the relevant doc.
