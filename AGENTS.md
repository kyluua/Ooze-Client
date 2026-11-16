# AGENTS.md: Instructions for coding agents

Ooze has two products that ship together:
- **Ooze Client:** a Minecraft Java Edition **1.21.11** client delivered as a **Fabric** mod.
  - The Fabric adapter is **Java 21**.
  - Most Ooze-owned engine and interface logic is **Rust**, reached through one batched JNI bridge.
  - Minecraft itself stays Java.
- **Ooze Launcher:** a standalone **Rust** desktop launcher for Windows x64.
  - It has its **own Microsoft sign-in**.
  - It installs, verifies and **directly launches** Minecraft 1.21.11 with Fabric and Ooze.

The specification in `docs/` is the source of truth. Implement it milestone by milestone.

> **Decision record (2026-11-09, owner):** 26.1/26.2 builds and a predominantly Java core are **no longer requirements**. The single target is Minecraft 1.21.11 on Fabric. Windows x64 is the first supported native platform. `docs/TARGET_PLATFORM.md` and `docs/ARCHITECTURE.md` are authoritative.
>
> **Decision record (2026-11-16, owner):**
> - Ooze ships a **fully independent launcher with its own Microsoft sign-in** (L-series milestones).
> - It supersedes the earlier "official launcher only" authentication for the standalone product.
> - Inside the Fabric mod the old security boundary stays: no tokens in the JNI engine; only non-secret display metadata.
> - From now on **all agents work on `main`**.

## Who owns what

Two agents work in this repository. They do **not** share chat history: **handoffs and commits carry all context**. (The L0 brief called the launcher-backend role "Codex"; in this repository that role is **Sol**, as recorded in `docs/handoffs/SOL-L0-CONTRACT.md`.)

| Owner | Paths | Responsibilities |
|---|---|---|
| **Claude** (product lead: Fabric integration, in-game UI, launcher interface) | `versions/mc1_21_11/**`, `native/crates/ooze-ui/**`, `assets-src/**`, `launcher/crates/ooze-launcher-ui/**`, `launcher/resources/**`, Java-side localisation and resources, root Gradle files (`settings.gradle.kts`, `build.gradle.kts`, `gradle.properties`, wrapper), `tools/launch-tests.sh`, `docs/**` product specs (including `docs/launcher/LAUNCHER_UI.md`), `docs/INSTALL.md` | **In game:** Fabric entrypoint and lifecycle, minimal mixins, input capture, screen hosts, HUD hook, vanilla UI integration, narration, compatibility adapters, the Java render adapter, game-facing module hooks, Rust in-game UI. **Launcher:** screens, components, navigation, focus, accessibility, localisation and product UI state over Sol's API. **Both:** product smoke, screenshot and accessibility tests; the launcher profile manifest from the Fabric build; **independent review of Sol's work** (engine, bridge and launcher backend) |
| **Sol** (GPT-6.1 Sol: Rust engine, Java/Rust bridge, launcher backend) | `native/**` except `native/crates/ooze-ui/**` (currently `ooze-core`, `ooze-jni`, `tools/`, `licenses/`), `bridge-java/**`, `.cargo/**`, `rust-toolchain.toml`, `launcher/Cargo.toml`, `launcher/Cargo.lock`, `launcher/.gitignore`, `launcher/crates/ooze-launcher-core/**`, `launcher/crates/ooze-launcher-cli/**`, `launcher/packaging/**`, CI workflows (`.github/workflows/**`), benchmarks, `docs/bridge/**` (ABI contract `docs/bridge/ABI_V1.md`), `docs/launcher/LAUNCHER_API_V1.md`, `AUTH_SETUP.md`, `BUILD_AND_TEST.md`, `INSTALLATION_V1.md` | **Engine:** native core (render primitives, text, animation, theme resolution, configuration I/O, settings and state algorithms, events, search, images), the JNI implementation, `bridge-java`, the versioned bridge protocol, native artifacts per platform, benchmarks. **Launcher backend:** Microsoft sign-in (system browser + PKCE), Xbox Live / XSTS / Minecraft Services, entitlement, refresh, Windows-protected credential storage, installation and verification, process launching and lifecycle, log redaction, the versioned launcher API (API 1), packaging. **CI** for both |

- **Changing another owner's files:** don't edit them directly. Write the request into your handoff (or `docs/handoffs/<REQUESTER>-<milestone>-request.md`) and let the owner make the change. Trivial, mechanical fixes needed to keep the build green are allowed only if the handoff records them.
- **Shared workspace files:** Sol owns `native/Cargo.toml` and `launcher/Cargo.toml` and registers crates in them (Claude's `ooze-launcher-ui` membership is requested in `CLAUDE-L0.md`).

## Branches

- **Everyone works on `main`** (owner decision 2026-11-16). There are no long-lived agent branches.
- **Before every push:** `git fetch origin main` and merge `origin/main`. Never force-push `main` and never rewrite its history.
- **Keep `main` green:** run your own checks (Commands below) before pushing.
- Commit in small, self-contained steps with conventional messages. A push that only touches another owner's paths needs that owner's recorded agreement (handoff) or must be a recorded mechanical fix.
- Older branches (`claude/*`, `sol/g0-native-foundation`) are historical; their work is merged into `main`.

## Read first (in this order)

1. The newest files in `docs/handoffs/`: what was delivered, the contract versions, and the next task.
2. `docs/PRODUCT_VISION.md`: principles, especially *Real or absent* and *Tokens, not literals*.
3. `docs/IMPLEMENTATION_PLAN.md`: find your milestone (C = Claude in-game, L = launcher (Claude interface + Sol backend), G = Sol engine). Do **only** that milestone.
4. `docs/ARCHITECTURE.md` (§19 for the launcher), `docs/TARGET_PLATFORM.md` and `docs/REPOSITORY_STRUCTURE.md`.
5. The topic docs your milestone references:
   - in game: `DESIGN_SYSTEM.md`, `TYPOGRAPHY.md`, `MOTION_SPEC.md`, `BACKGROUNDS.md`, `COMPONENTS.md`, `SCREENS.md`, `INTERACTION_FLOWS.md`, `PERFORMANCE_AND_ACCESSIBILITY.md`, `TESTING.md`;
   - launcher: `docs/launcher/*.md`.

## Hard rules

**In-game architecture (Fabric mod)**
- **Ooze engine and UI logic lives in Rust** wherever practical. **Java keeps** everything that touches Minecraft or Fabric objects, GPU resource operations and vanilla renderer submission.
- **No raw graphics APIs in game:** no `GL*`, `GlStateManager`, Vulkan or NanoVG, in Rust or Java. Rust produces a **versioned frame output**. The Java render adapter validates it and submits it through vanilla GUI pipelines (ARCHITECTURE §5).
- **Bounded JNI:**
  - Collect input and immutable game snapshots, send them in **bounded batches**, and execute validated platform commands back on Minecraft's render thread.
  - **Never** call JNI once per vertex, glyph, entity, widget or setting.
  - Use **only** the agreed `bridge-java` API. Never declare `native` methods or a second JNI interface outside `bridge-java`.
- **Native code never sees Minecraft symbols.** No obfuscated or Mojang class names cross the bridge; only Ooze-defined data does.
- **Mojang official names** in all Java sources, compiled with Loom's **remapping** plugin for 1.21.11. Do not copy 26.x APIs.

**Launcher architecture**
- **The launcher window** is drawn by a maintained Rust GUI toolkit (egui/eframe with AccessKit; ARCHITECTURE §19, ADR-016). The in-game "no custom renderer" rule does **not** apply to the separate desktop window.
- **The interface talks to the backend only** through the versioned launcher API (`LAUNCHER_API_V1`, independent of the JNI ABI).
  - The interface never reimplements downloads, credential storage, OAuth or launch arguments.
  - The backend never draws UI.
- **Keep the window responsive.** Network, disk, auth and process work run off the UI thread. The interface handles stale events, double activation, cancellation and closing while Minecraft runs.

**Authentication and identity**
- **Launcher:**
  - Microsoft sign-in in the **system browser** with PKCE, using the project's own registered **public client ID** (developer configuration, never per player). The owner-provided ID lives in `launcher/resources/auth/microsoft-public-client-id.txt`; it is public, and no client secret may ever be added.
  - Never collect Microsoft passwords, embed a client secret, borrow another launcher's client ID or read another launcher's credential store.
  - Credentials live only in the backend's Windows-protected storage.
  - The game receives its session only through the backend's launch interface, and **the raw launch command is never logged**.
- **Never fake a sign-in.** If the client ID or Minecraft Services approval is missing, show an honest, actionable configuration error. Never fall back to offline authentication and never claim sign-in passed.
- **Launcher UI:** receives only non-secret account summaries (**username, UUID**) and typed states. Tokens never enter UI state, persisted UI settings, screenshots, logs, exports or the in-game JNI interface.
- **Fabric mod:** never reads, stores, logs or transmits access tokens or session IDs. Only username and UUID may cross into Rust.
- **No Ooze account system and no Ooze login server.**

**Product**
- **Preserve vanilla functionality and player data.**
  - Ooze replaces nothing it cannot fully reproduce.
  - A failed or missing native engine leaves vanilla Minecraft fully usable, shows a clear diagnostic, and shows **no non-working Ooze controls**.
  - Install and repair never touch worlds, screenshots, packs or options.
- **Real or absent.**
  - Unfinished features do not appear in the UI. No mock accounts, fake progress, decorative buttons or unimplemented navigation.
  - Sample modules, widgets and data carry the **Sample** badge.
  - Developer-only tools and test fixtures appear only in development or test builds (`-Doozeclient.dev=true`; the launcher's `dev-fixtures` feature, which release builds refuse).
- **Tokens, not literals.** No hard-coded colours, sizes, radii, durations or easings in components.
- **Animations are time-based and honour Reduced motion and Off.**
- **Every user-facing string** has a translation key:
  - in game: `assets/oozeclient/lang/en_us.json`, delivered to Rust through the resource contract;
  - launcher: `launcher/resources/lang/en_us.json`.
- **Keyboard access, visible focus and narration** (Minecraft's narrator in game; UI Automation via AccessKit in the launcher) are required for every interactive element.
- **Fonts:** ship upstream static TTFs **unmodified**, together with their `OFL.txt`.
- **No gameplay modules** beyond `docs/FEATURE_BACKLOG.md` entries that have reached their milestone. Samples are not a finished client.
- **Optional features fail soft.** Use `require = 0` mixins plus a runtime check, never a crash.
- **Never bundle proprietary Minecraft files** (client JAR, libraries, assets) in any release.

## Commands

```bash
./gradlew build                                        # Java build + Java unit tests (all Gradle projects)
./gradlew buildAll -PoozeNativeResources=<abs dir>     # mod JAR with Sol's Windows x64 DLL + build/dist/ooze-launcher-profile.json
./gradlew :versions:mc1_21_11:runClient                # dev client (offline dev account; no real sign-in)
OOZE_NATIVE_RESOURCES=<abs dir> tools/launch-tests.sh  # full launch matrix + evidence (Linux via xvfb-run; Windows Git Bash adds real-DLL cases)
./gradlew :versions:mc1_21_11:runClient -PoozeEngineFault=incompatible   # dev-only: exercise engine diagnostics

# Launcher interface (Claude); until it is a member of Sol's launcher workspace, run inside the crate:
cd launcher/crates/ooze-launcher-ui && cargo fmt --check && cargo clippy --all-targets --features dev-fixtures -- -D warnings && cargo test
cargo run --features dev-fixtures --bin ooze-launcher-preview -- ready  # developer preview over the simulated backend
cargo run --bin ooze-launcher                                        # the real launcher (Windows x64; elsewhere it shows "needs Windows")
```

- **Rust engine:** Sol defines the commands (`cargo fmt/clippy/test`, `native/tools/build-g0.ps1`) in `docs/bridge/BUILD_AND_TEST.md`.
- **Launcher backend and packaging:** Sol defines them in `docs/launcher/BUILD_AND_TEST.md`.
- **Without `-PoozeNativeResources`** the mod still builds (vanilla-only at runtime); the root build logs that it skipped Sol's DLL checks.

## Evidence vocabulary (use exactly these words)

| Word | Meaning |
|---|---|
| **Built** | Compiles and packages |
| **Headless-tested** | Automated tests passed without a game window (unit, contract, golden, egui_kittest) |
| **Game-launched** | A real Minecraft client started with the mod and the stated check passed, on the stated OS |
| **Manually verified** | A person looked at it, on the stated OS and hardware |

- Never describe a feature or platform as tested without the matching evidence.
- Screenshots establish appearance, not live authentication or backend correctness.
- Installation completing, or a spawned Java process, is **not** a verified launch; that requires the title screen (`OOZE_READY`).

## Handoffs (required at the end of every milestone)

Write `docs/handoffs/<OWNER>-<MILESTONE>.md` (for example `CLAUDE-L0.md`, `SOL-L0.md`) with:
- owner, milestone, base commit and result commit;
- changed paths;
- contract versions (JNI ABI, launcher API);
- real test results using the evidence vocabulary;
- artifact locations and checksums;
- limitations;
- review findings on the other owners' latest diffs;
- the exact next task.

Review the other agents' diffs and record concrete findings **before** integrating them.

## Definition of Done

See `docs/TESTING.md §10`.

## Style

- **Java:** 21, Mojang names, small final classes and records, JSpecify `@Nullable`. No reflection in hot paths. No allocation in per-frame paths.
- **Rust:**
  - Follow the owning workspace's lints (`native/Cargo.toml`, `launcher/Cargo.toml`).
  - In `ooze-ui` and `ooze-launcher-ui`: no `unsafe`, and no panics across the bridge or out of event handlers.
  - In `ooze-ui`: no allocation in steady-state frames where avoidable.
- Conventional commit messages (`feat(fabric): …`, `feat(launcher-ui): …`, `docs(spec): …`).
