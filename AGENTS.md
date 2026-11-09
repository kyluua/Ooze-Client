# AGENTS.md: Instructions for coding agents

**Ooze Client** is a Minecraft Java Edition **1.21.11** client delivered as a **Fabric** mod and loaded by the **official Minecraft Launcher**.

- The Fabric adapter is **Java 21**.
- Most Ooze-owned engine and interface logic is **Rust**, reached through one batched JNI bridge.
- Minecraft itself stays Java.

The specification in `docs/` is the source of truth. Implement it milestone by milestone.

> **Decision record (2026-11-09, owner):** 26.1/26.2 builds and a predominantly Java core are **no longer requirements**. The single target is Minecraft 1.21.11 on Fabric. Windows x64 is the first supported native platform. Docs from before this date describe the old plan only where they still say so; `docs/TARGET_PLATFORM.md` and `docs/ARCHITECTURE.md` are authoritative.

## Who owns what

Two agents work in this repository. They do **not** share chat history: **handoffs and commits carry all context**.

| Owner | Paths | Responsibilities |
|---|---|---|
| **Claude** (Fabric integration and product lead) | `versions/mc1_21_11/**`, `native/crates/ooze-ui/**`, `assets-src/**`, Java-side localisation and resources, root Gradle files (`settings.gradle.kts`, `build.gradle.kts`, `gradle.properties`, wrapper), `docs/**` product specs, `docs/INSTALL.md` | Fabric entrypoint and lifecycle, minimal mixins, input capture, screen hosts, HUD hook, vanilla UI integration, narration, compatibility adapters, the Java render adapter that consumes native frame output, game-facing module actions and hooks, Rust UI components, screens, navigation, focus and interactions, product UI state, product smoke, screenshot and accessibility tests, **independent review of Sol's engine and bridge** |
| **Sol** (GPT-6.1 Sol: Rust engine and Java/Rust bridge) | `native/**` except `native/crates/ooze-ui/**` (currently `ooze-core`, `ooze-jni`, `tools/`, `licenses/`), `bridge-java/**`, `.cargo/**`, `rust-toolchain.toml`, CI workflows (`.github/workflows/**`), benchmarks, `docs/bridge/**` (ABI contract `docs/bridge/ABI_V1.md`) | Native core (render primitives, text, animation, theme resolution, configuration I/O, settings and state algorithms, events, search, images), the JNI implementation, `bridge-java`, the versioned bridge protocol, native artifacts per platform, benchmarks, CI |

- **Changing the other owner's files:** don't edit them directly. Write the request into your handoff (or a `docs/handoffs/<OWNER>-<milestone>-request.md`) and let the owner make the change. Trivial, mechanical fixes needed to keep the build green are allowed only if the handoff records them.
- **Branches:** each agent works on its own branch or worktree. Never push to the other agent's branch.

## Read first (in this order)

1. The newest files in `docs/handoffs/`: what was delivered, the bridge version, and the next task.
2. `docs/PRODUCT_VISION.md`: principles, especially *Real or absent* and *Tokens, not literals*.
3. `docs/IMPLEMENTATION_PLAN.md`: find your milestone (C-series = Claude, G-series = Sol). Do **only** that milestone.
4. `docs/ARCHITECTURE.md`, `docs/TARGET_PLATFORM.md` and `docs/REPOSITORY_STRUCTURE.md`.
5. The topic docs your milestone references: `DESIGN_SYSTEM.md`, `TYPOGRAPHY.md`, `MOTION_SPEC.md`, `BACKGROUNDS.md`, `COMPONENTS.md`, `SCREENS.md`, `INTERACTION_FLOWS.md`, `PERFORMANCE_AND_ACCESSIBILITY.md`, `TESTING.md`.

## Hard rules

**Architecture**
- **Ooze engine and UI logic lives in Rust** wherever practical. **Java keeps** everything that touches Minecraft or Fabric objects, GPU resource operations and vanilla renderer submission.
- **No raw graphics APIs:** no `GL*`, `GlStateManager`, Vulkan or NanoVG, in Rust or Java. Rust produces a **versioned frame output**. The Java render adapter validates it and submits it through vanilla GUI pipelines (ARCHITECTURE §5).
- **Bounded JNI:**
  - Collect input and immutable game snapshots, send them in **bounded batches**, and execute validated platform commands back on Minecraft's render thread.
  - **Never** call JNI once per vertex, glyph, entity, widget or setting.
  - Use **only** the agreed `bridge-java` API. Never declare `native` methods or a second JNI interface outside `bridge-java`.
- **Native code never sees Minecraft symbols.** No obfuscated or Mojang class names cross the bridge; only Ooze-defined data does.
- **Mojang official names** in all Java sources, compiled with Loom's **remapping** plugin for 1.21.11. Do not copy 26.x APIs.

**Authentication and identity**
- The **official Minecraft Launcher** handles Microsoft sign-in, entitlement and the game session.
- Ooze has **no login, no Azure app registration and no account system**. It never reads, stores, logs or transmits access tokens or session IDs.
- Only non-secret display metadata (**username and UUID**) may cross into Rust.
- Account switching happens in the launcher.

**Product**
- **Preserve vanilla functionality.** Ooze replaces nothing it cannot fully reproduce. A failed or missing native engine leaves vanilla Minecraft fully usable, shows a clear diagnostic, and shows **no non-working Ooze controls**.
- **Real or absent.** Unfinished features do not appear in the UI. Sample modules, widgets and data carry the **Sample** badge. Developer-only tools appear only in the development environment (or with `-Doozeclient.dev=true`).
- **Tokens, not literals.** No hard-coded colours, sizes, radii, durations or easings in components.
- **Animations are time-based and honour Reduced motion and Off.**
- **Every user-facing string** has a translation key in `assets/oozeclient/lang/en_us.json`, delivered to Rust through the resource contract.
- **Keyboard access and narration** are required for every interactive element.
- **Fonts:** ship upstream static TTFs **unmodified**, together with their `OFL.txt`.
- **No gameplay modules** beyond `docs/FEATURE_BACKLOG.md` entries that have reached their milestone. Samples are not a finished client.
- **Optional features fail soft.** Use `require = 0` mixins plus a runtime check, never a crash.

## Commands

```bash
./gradlew build                                        # Java build + Java unit tests (all Gradle projects)
./gradlew buildAll -PoozeNativeResources=<abs dir>     # bundle Sol's prebuilt Windows x64 DLL + manifest into the mod JAR
./gradlew :versions:mc1_21_11:runClient                # dev client (offline dev account; no real sign-in)
OOZE_NATIVE_RESOURCES=<abs dir> tools/launch-tests.sh  # full launch matrix + evidence (Linux via xvfb-run; Windows Git Bash adds real-DLL cases)
./gradlew buildAll                                     # distribution JAR → build/dist/
./gradlew :versions:mc1_21_11:runClient -PoozeEngineFault=incompatible   # dev-only: exercise engine diagnostics
```

Rust commands (`cargo fmt/clippy/test`, `native/tools/build-g0.ps1`) are defined by Sol in `docs/bridge/BUILD_AND_TEST.md`. Without `-PoozeNativeResources` the mod still builds (vanilla-only at runtime); the root build logs that it skipped Sol's DLL checks.

## Evidence vocabulary (use exactly these words)

| Word | Meaning |
|---|---|
| **Built** | Compiles and packages |
| **Headless-tested** | Automated tests passed without a game window (unit, contract, golden) |
| **Game-launched** | A real Minecraft client started with the mod and the stated check passed, on the stated OS |
| **Manually verified** | A person looked at it, on the stated OS and hardware |

Never describe a feature or platform as tested without the matching evidence.

## Handoffs (required at the end of every milestone)

Write `docs/handoffs/<OWNER>-<MILESTONE>.md`, for example `docs/handoffs/CLAUDE-C0.md`, with:
- owner, milestone, base commit and result commit;
- changed paths;
- bridge version;
- real test results using the evidence vocabulary;
- artifact locations;
- limitations;
- review findings on the other owner's latest diff;
- the exact next task.

Review the other agent's diff and record concrete findings **before** integrating it.

## Definition of Done

See `docs/TESTING.md §10`.

## Style

- **Java:** 21, Mojang names, small final classes and records, JSpecify `@Nullable`. No reflection in hot paths. No allocation in per-frame paths.
- **Rust:** follow Sol's workspace lints (`native/Cargo.toml`, `docs/bridge/`). In `ooze-ui`: no `unsafe`, no panics across the bridge, and no allocation in steady-state frames where avoidable.
- Conventional commit messages (`feat(fabric): …`, `docs(spec): …`).
