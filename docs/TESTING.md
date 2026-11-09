# Ooze Client — Acceptance Criteria & Testing

> Deliverable 14. Updated 2026-11-09 for Fabric 1.21.11 + Rust.
> Milestone-specific gates are in [IMPLEMENTATION_PLAN.md](IMPLEMENTATION_PLAN.md) §3.

## 0. Evidence vocabulary

Every result reported in a handoff uses exactly one of these words, and states the OS it ran on.

| Word | Meaning |
|---|---|
| **Built** | Compiles and packages |
| **Headless-tested** | Automated tests passed without a game window: JUnit, `cargo test`, contract tests, goldens |
| **Game-launched** | A real Minecraft 1.21.11 client started with the mod, and the stated automated check passed |
| **Manually verified** | A person looked at or used it, on stated hardware |

A platform is **supported** only with a native artifact **and** a Game-launched pass on that platform (TARGET_PLATFORM §2).

## 1. Test layers

| Layer | Tooling | Owner | Covers |
|---|---|---|---|
| Java unit | JUnit 5 (`./gradlew build`) | Claude | Engine gate, diagnostics, translations, source rules (no `native` declarations, raw GL/Vulkan or session secrets), and from C1 frame-output validation and texture budget checks |
| Rust unit | `cargo test` (Sol's workspace) | Sol (engine), Claude (`ooze-ui`) | Springs, easing, colour, tessellation, text layout, search, config, events, settings; UI state, focus and layout in `ooze-ui` |
| Bridge contract | Sol's harness | Sol | Version handshake, bounds and validation, fault isolation (no panic crosses FFI), buffer limits |
| Golden render | Sol's reference rasterizer + PNG goldens | Sol (primitives), Claude (components) | Shapes, AA, material layers, ordering |
| Client launch tests | Fabric client gametests (`runClientGameTest`, `runProductionClientGameTest`) | Claude | Real launches: title screen, engine states, vanilla usability, screens and flows (later) |
| Manual QA | §8–§9 checklists | Claude | Look and feel, motion, accessibility |
| Performance | Frame profiler scenes (§6) | Claude (product scenes), Sol (benchmarks) | Budgets |

## 2. Required unit tests

**Java (implemented in C0, extended per milestone)**

| Test | Asserts |
|---|---|
| `EngineGateTest` | No feature, key mapping or developer probe without a loaded engine. The probe appears only in development with a loaded engine. The notice appears whenever the engine isn't loaded |
| `EngineConnectorsTest` | Fault injection works only in the development environment; `LOADED` can't be injected; a throwing connector becomes `LOAD_FAILED` |
| `PlatformInfoTest` | OS and architecture normalisation; only `windows-x86_64` is in the supported list |
| `EngineDiagnosticsTest` | Every failure state has a full log block and an `en_us` translation with matching placeholders; no account or secret wording |
| `SourceRulesTest` | The adapter declares no `native` methods, imports no raw graphics APIs and references no session secrets |

**Rust:** the engine-side tests listed in the previous specification (springs against RK4, cubic-bezier, OKLab and contrast, tessellation watertightness, text layout, search ranking, config atomicity and migrations, events, settings) are Sol's. `ooze-ui` tests are Claude's from C2.

## 3. Client launch tests

Run the whole matrix and collect evidence (screenshots, log excerpts, summary):

```bash
OOZE_NATIVE_RESOURCES=<abs dir> tools/launch-tests.sh build/launch-evidence   # Linux uses xvfb-run; Windows: Git Bash
```

The expected state defaults inside the test to **`LOADED` on Windows x64** and **`UNSUPPORTED_PLATFORM` elsewhere**. Sol's loader checks the platform first. `-PoozeExpectEngineState=<STATE>` overrides it.

Every case also asserts that the `GuiGraphicsAccessor` mixin applied (dev and production-remapped).

| Case | Task | Asserts |
|---|---|---|
| `dev-default` | `runClientGameTest` | Title screen reached; **real** state from Sol's loader; when not loaded: no `key.oozeclient.*` mappings and the notice was queued; vanilla Options round trip |
| `dev-<state>` for `missing_library`, `incompatible`, `load_failed`, `not_bundled` | `runClientGameTest -PoozeEngineFault=<state>` | The same, for each fault-injected state |
| `prod-default` | `runProductionClientGameTest` | The **shipped remapped JAR** (bridge classes and DLL merged) in a production Fabric environment (`dev=false`); real loader state |
| `prod-fault-ignored` | `runProductionClientGameTest -PoozeEngineFault=incompatible` | A release build ignores developer fault injection (real state, with a warning) |
| `win-real-missing_library`, `win-real-corrupt`, `win-real-abi-mismatch` (**Windows x64 only**) | Generated from the provided resources: DLL removed, one DLL byte flipped, manifest `bridge.abi=2` | Real `MISSING_LIBRARY` / `LOAD_FAILED` / `INCOMPATIBLE` from Sol's loader; vanilla usable |
| `LOADED` branch (**Windows x64 only**) | `dev-default` with resources | The F9 developer key exists; the probe round trip passes; `BridgeDiagnosticScreen` renders fixture frames; the colour switches `F8F9FF` → `171923` → `F8F9FF` with simulated left-button hold and release; two screenshots |

**Pending (Gate B, Windows x64):** the `win-real-*` cases and the `LOADED` branch are implemented, but they have not been run, because no Windows host has been available to Claude.

**From C1 onwards:** screen screenshots at window sizes 1920×1080, 2560×1440 and 3840×2160; an ordering torture-scene golden; scripted flows.

Notes for this environment:
- Minecraft cannot open an OpenAL device in a headless container; the client runs without sound.
- Realms authorisation fails with Loom's offline development account; that is expected.
- Neither affects the assertions.

## 4. Platform matrix (replaces the cross-version matrix)

| Platform | Native | Required at each Launch gate |
|---|---|---|
| Windows x64 | **Yes** (first) | Game-launched with `LOADED` from C0b; every gate's product checks |
| Linux x64 | No (until an artifact exists) | Game-launched in vanilla-only mode: the diagnostic plus vanilla usable |
| macOS, Windows arm64 | No | Vanilla-only behaviour, Manually verified when hardware is available |

Supported GPUs are tested on Windows across NVIDIA, AMD and Intel before release (C10).

## 5. Crispness checklist (DESIGN_SYSTEM §11)

Unchanged. Run from C1 at 1920×1080, 2560×1440, 3840×2160 and a 1280×720 window, at UI scale 100% and 125%:
- 1 px AA transitions;
- whole-pixel straight outlines;
- glyph texels drawn 1 : 1 at rest;
- sharp icons;
- no banding;
- upscaled images show the quality notice;
- Minecraft's GUI scale does not change Ooze sizes.

## 6. Performance procedure

- Scenes come from PERFORMANCE_AND_ACCESSIBILITY §2.1 and are scripted in the dev-only Render Lab benchmarks.
- Measure **Java time** (capture, validate, submit), **Rust time** (reported in the frame output) and **JNI calls and bytes per frame**.
- Each run: 10 s warm-up, then a 60 s capture; report p50, p95 and p99 in the handoff.

## 7. Robustness

| Test | Pass condition | Owner and milestone |
|---|---|---|
| Missing, incompatible or failing native library | Vanilla usable; a clear diagnostic; no Ooze controls | Claude: C0 (simulated), C0b (real, Windows) |
| Release build with developer switches | The switches are ignored | Claude: C0 |
| Kill the process during repeated saves | Every restart loads a valid config; ≤ 5 s of changes lost | Sol G4 (store), Claude C5 (in game) |
| Corrupt or missing or future-schema config | `.bak` restore, quarantine, read-only mode | Sol G4 |
| Backups | A snapshot never contains `backups/` or media; snapshots over 8 MiB are skipped | Sol G4 |
| Texture budget pressure | No GPU allocation exceeds the byte budget; the diagnostic fires; the UI degrades as specified | Claude C1 (Java check) + Sol G3/G5 (accountants) |
| Native panic or fault in a frame | Caught; the engine disables itself; vanilla usable | Sol G1 contract + Claude C1 launch test |

## 8. Manual QA, per screen

Unchanged from the previous specification (S01–S12 checklists). Applied as each screen's milestone lands.

## 9. Manual QA, component states

Unchanged. Verify the COMPONENTS §13 matrix in the dev-only Component Gallery (C3/C4).

## 10. Definition of Done

1. `./gradlew build` passes. Sol's `cargo` checks pass for any touched crate.
2. Launch tests (§3) pass for every case the milestone touches. Windows x64 evidence is required from C0b onwards.
3. No `native` declarations outside bridge-java; no raw GL, Vulkan or NanoVG; no session secrets (`SourceRulesTest` plus Rust lints).
4. Tokens, not literals. Every user-facing string has an `en_us` key.
5. Keyboard access, focus-visible styling, narration, reduced motion and Off for all new interactive elements.
6. No visible non-working control; samples are labelled; developer tools are dev-only.
7. Performance budgets met for the scenes the milestone touches, with tables in the handoff.
8. The handoff is written with evidence words and lists any spec deviations, with the docs updated.
