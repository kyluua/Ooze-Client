# CLAUDE MV — Minecraft 1.21.11 and 26.1–26.3 (Fabric client and launcher interface)

- **Owner:** Claude (Fabric integration, launcher interface, product documentation). **Counterpart:** Sol (`SOL-MULTIVERSION.md`; my acknowledgment and asks in `CLAUDE-MV-request.md`).
- **Milestone:** MV (owner brief 2026-11-26; `IMPLEMENTATION_PLAN.md` MV).
- **Base:** `c4a5236`.
- **Commits:**
  - `c7f9ca2`: Fabric adapters and build.
  - `1d0477e`: launcher interface.
  - `081b805`: specs.
  - `493b008`: merge of Sol's `4b7379c`, `53adaa4`, `0ae6d7e` and `51d09f8`, reviewed in §4.
  - `b6abebc`: integration (release-aware library, JDK 25 detection).
  - `3cd13d8`: this handoff and the evidence (the result commit).
- **Contracts:**
  - Library API 1, with Sol's additive `Library::open_with_release`.
  - Launcher API 1.1 and JNI ABI 1 are unchanged.
  - Readiness marker unchanged: `OOZE_READY v1 screen=title engine=<STATE>`.
  - Catalog schema 2 and validated-client manifest schema 1 (Sol's formats).
  - Launcher profiles keep `oozelauncher.profile/1`, now one file per Minecraft version.
- **Environment:** Linux x86_64 cloud container, Xvfb with Mesa, OpenJDK 21.0.12 and 25.0.5. No Windows machine is attached.

## Status log

- 2026-11-26 **MOD-READY:** every advertised Fabric target is Game-launched on Linux with its shipped JAR (22/22 cases, vanilla-only engine state).
- 2026-11-26 **UI-READY:** the interface is Headless-tested on the merged tree with Sol's release-aware library.
- 2026-11-26 **Windows CI green on `3cd13d8`**, [run 45792158067](https://github.com/kyluua/Ooze-Client/actions/runs/45792158067), `windows-latest`. All on the first Windows build with these Gradle changes:
  - "Assemble real Ooze mod with production Windows engine" (`gradlew.bat build buildAll`, Gradle on JDK 25, the four adapters and the shared tests): **Built** and **Headless-tested**.
  - The launcher workspace tests, including the interface tests, on the real Windows GUI adapter: **Headless-tested**.
  - The live interface and Modrinth tests, the catalog candidate, and Sol's install and repair matrix for all six versions.

  This is not a game launch.
- **Open:** `engine=LOADED` on Windows x64 for every target (Sol, A1) and authenticated Play (A4). Until then the brief's Windows acceptance is not met, and Sol's packaging gets no validated 26.x bundle.

## 1. What changed

**Build (Claude's paths).**
- Loom **1.19.4** for both plugin ids, with the Gradle daemon on **any JDK 25** (`gradle/gradle-daemon-jvm.properties`).
  - The Foojay resolver provisions JDK 21 and 25 when they're missing.
  - CI runners' `JAVA_HOME_21_X64` and `JAVA_HOME_25_X64` are detected (`org.gradle.java.installations.fromEnv`).
  - Before any other change, the 1.21.11 JAR built with Loom 1.19.4 was class-for-class identical to Loom 1.17.21's. Only the Loom and Tiny Remapper version stamps in the manifest changed.
- **`versions/common`** holds the Minecraft-independent Java, as Java 21 bytecode merged into every JAR:
  - engine connector, bridge status mapper, status, gate, fault injection, platform and diagnostics;
  - `ReadinessMarker`;
  - `OozeRuntime` (the engine state formerly in `OozeClient`);
  - the in-game translations.

  It has 27 tests, including one `SourceRulesTest` that scans every adapter's sources.
- **1.21.11 adapter:** still Java 21 and remapped. The entrypoint is thinner; the Minecraft hooks are unchanged.
- **26.x adapters:** one build script (`versions/mc26/adapter.gradle.kts`, the `buildFileName` of `mc26_1`, `mc26_2` and `mc26_3`) and one source tree (`versions/mc26/src`).
  - Ported to 26.x names: `GuiGraphicsExtractor` and its accessor mixin, `extractRenderState`, `addGuiElement`, `text`, and Fabric's `KeyMappingHelper`.
  - Family differences live only in two classes per family:

| Family | `compat/McCompat` | `dev/FixtureRenderState` |
|---|---|---|
| 26.1 | `getToastManager()`, `mc.screen`, `SystemToast.multiline`, `KEYSYM` | blaze3d `RenderPipeline` |
| 26.2 | `gui.toastManager()`, `gui.screen()`, wrapping `SystemToast` constructor, `KEYSYM` | blaze3d `RenderPipeline` |
| 26.3 | as 26.2, but `KEYBOARD` with SDL3 scancodes | `com.mojang.renderpearl` `RenderPipeline` |

- **JARs:** four, reproducible.
  - Each lists exactly its versions: `~1.21.11`; `["26.1","26.1.1","26.1.2"]`; `["26.2"]`; `["26.3"]`.
  - Each declares `java >=21` (1.21.11) or `>=25` (26.x).
  - Per-version profiles `build/dist/ooze-launcher-profile-mc<version>.json`; the legacy `ooze-launcher-profile.json` (1.21.11) is unchanged.
- **`tools/launch-tests.sh`:**
  - It builds once and launches the exact JARs on every listed version.
  - A case passes only on `Loading Minecraft <version>` and the exact `OOZE_READY` line for the expected engine.
  - It writes `jars/validated-ooze-clients.json` (Sol's schema 1) for 26.x production runs that reached `engine=LOADED` (default).

**26.3 SDL3 input and window.** The 26.x launch test checks, in every engine state:
- the family's compiled F9 and Escape codes name those keys on the running backend;
- the window has a handle and a size;
- Options closes with a real Escape key press, and again with Done.

| Minecraft | F9 / Escape codes | Backend (Linux, Xvfb) |
|---|---|---|
| 26.1, 26.1.1, 26.1.2, 26.2 | 298 / 256 (GLFW) | OpenGL |
| 26.3 | 66 / 41 (SDL3 scancodes) | SDL video driver `x11`; OpenGL failed ("Couldn't find matching GLX visual"), so Minecraft fell back to Vulkan on llvmpipe |

On 26.3 the GUI element path still rendered: the fixture pixels were exact, and the wrapped "Ooze Client is off" toast shows on the title screen. A JAR compiled against 26.1 would inline 298 and bind the wrong key on 26.3, so each family is compiled against its own Minecraft (ADR-019).

**Launcher interface.**
- **New instance:** a kind radio group (**Fabric with Ooze**, **Fabric**, **Vanilla**), then a Minecraft-version radio group, newest first.
  - Only `capabilities.create` is offered. It is now Sol's catalog through `CoreLibrary::open_with_release(data_dir, release.json)`, used by the desktop binary.
  - Plain Fabric and Fabric with Ooze for the same version are separate choices, matching Sol's capabilities.
  - The exact target is shown with an "Ooze Client included" or "No Ooze modules" tag. Switching kind keeps the chosen version.
- **Play:** "Minecraft <version>", "<Loader> <version>" or "Vanilla", the Ooze tag, and "Runs on Java <n>".
  - Targets without Ooze never get a title-screen claim, because Sol sends no ready event for them; the page says "Minecraft is running."
  - With a schema 2 catalog there is no single release profile, so nothing version-specific is shown unless an instance is selected.
- **Content:** the page names the instance's target, and the install review says "Resolved for <target>". Search and Plan run for the selected instance; Sol filters by its exact target and the bundled Fabric API of its catalog record.
- **Version-neutral text:** first-run, download steps, Java errors and Java settings no longer say 1.21.11 or Java 21. The exact Java is on Play, and Sol's errors name the required major.
- **Fixture library:** reports all 18 combinations in Sol's order.

**Specs.**
- `AGENTS.md`: decision record 2026-11-26, the names rule and commands.
- `TARGET_PLATFORM.md`: matrix, pins, one JAR per family, facts T13–T16.
- `ARCHITECTURE.md`: ADR-019 and the resources location.
- `IMPLEMENTATION_PLAN.md`: MV milestone.
- Also `REPOSITORY_STRUCTURE.md`, `PRODUCT_VISION.md`, `TESTING.md` §3, `LIBRARY_UI.md` §3, §5 and §9, and `INSTALL.md` (per-version JAR table).

## 2. Changed paths

- Build: `build.gradle.kts`, `settings.gradle.kts`, `gradle.properties`, `gradle/gradle-daemon-jvm.properties`.
- Adapters:
  - `versions/common/**` (new; moved from `versions/mc1_21_11` with history);
  - `versions/mc1_21_11/**`;
  - `versions/mc26/**`, `versions/mc26_1/**`, `versions/mc26_2/**` and `versions/mc26_3/**` (new).
- `tools/launch-tests.sh`.
- Launcher interface:
  - `launcher/crates/ooze-launcher-ui/src/{app.rs, app/library_pages.rs, core_backend.rs, core_library.rs, bin/ooze-launcher.rs, library_fixture.rs, library_ui_tests.rs}`;
  - `launcher/resources/lang/en_us.json`.
- Docs: `AGENTS.md`, `docs/**` specs, `docs/handoffs/CLAUDE-MV*.md`, `docs/handoffs/evidence/CLAUDE-MV/`.

No file owned by Sol was edited. The root `build.gradle.kts` makes every subproject's archives reproducible, which includes the timestamps in `bridge-java`'s JAR.

## 3. Results

**Game-launched on Linux x86_64 (Xvfb, Mesa): 22 of 22 cases.**
- **Inputs:**
  - JARs from `buildAll` with the RC1 native resources: DLL `44d47f19…b3e430`, taken from Sol's Prism artifact 15076307618 after checking its digests (ZIP `04e201b1…`, `.mrpack` `645d6503…`, JAR `088acdc6…`).
  - Each case launched the exact JAR listed below.
- **Engine state:** `UNSUPPORTED_PLATFORM`, Sol's loader's real state on Linux. Every case logged the exact `OOZE_READY v1 screen=title engine=<expected>` and passed the vanilla Options round trip.
- **Evidence:** `docs/handoffs/evidence/CLAUDE-MV/` (summary, every excerpt, a title screenshot per version).

| Case | Minecraft (loaded) | JAR | Result |
|---|---|---|---|
| `prod-1.21.11` | 1.21.11 | `+mc1.21.11` `9a8cb366…` | PASS |
| `prod-26.1`, `prod-26.1.1`, `prod-26.1.2` | 26.1, 26.1.1, 26.1.2 | `+mc26.1` `dcca8370…` | PASS ×3 |
| `prod-26.2` | 26.2 | `+mc26.2` `9fa9ede1…` | PASS |
| `prod-26.3` | 26.3 | `+mc26.3` `8290ad90…` | PASS |
| `dev-<adapter>-missing_library` / `-incompatible` / `-load_failed` (4 adapters) | 1.21.11, 26.1.2, 26.2, 26.3 | dev build | PASS ×12: vanilla Options works with the engine unavailable in each state |
| `prod-<adapter>-fault-ignored` (4) | as above | shipped JAR | PASS ×4: release JARs ignore developer fault injection |

**Full hashes** (reproducible; the same hashes were rebuilt on the merged tree):

| JAR | SHA-256 |
|---|---|
| `ooze-client-0.1.0-rc1+mc1.21.11.jar` | `9a8cb3667213e4b57273c6eb52e5604fda1abd95c99171fb9922523ad6bcb2f8` |
| `ooze-client-0.1.0-rc1+mc26.1.jar` | `dcca8370f46ab5d90e2157e038c5637ea6804686bdade4c9d22376861b0622c5` |
| `ooze-client-0.1.0-rc1+mc26.2.jar` | `9fa9ede12a5c1e671c3ea4262d743754eabf07a0beff0b3ea44892622b1fbab8` |
| `ooze-client-0.1.0-rc1+mc26.3.jar` | `8290ad9060956cdd8a4c994a258c4c97c3ceb682c02b5001032d57f8d93515b4` |

The validated-client manifest was format-checked with `OOZE_VALIDATED_STATE=UNSUPPORTED_PLATFORM` on 26.3. It wrote exactly Sol's schema-1 row. With the default (`LOADED`), a Linux run correctly lists no rows.

**Headless-tested (Linux, merged tree):**

| Check | Result |
|---|---|
| Gradle `build buildAll` | Pass: 27 shared tests, 1 adapter test, and every adapter compiled against its Minecraft |
| Launcher interface | 123 passed, 3 live ignored |
| Launcher core | 48 passed; R1's IPv6 test skipped (it fails in this container; Sol, open since RC1) |
| Lint and format | `cargo fmt --all --check` and workspace clippy `-D warnings` pass |

New interface tests:
- every combination and plain Fabric vs Fabric with Ooze;
- keyboard-only selection;
- Play target (Ooze, Vanilla, plain Fabric);
- content and review target;
- a multi-version catalog shows no single profile;
- **real library with a schema 2 catalog:** exactly its targets, exact creates, and refusal of an unpinned Ooze target.

**Built:** clippy for `x86_64-pc-windows-gnu` passes for the interface library and the desktop binary. It was type-checked, not run.

**Appearance:** screenshots of the create dialog, the minimum window at 200 %, Play and the install review are in `evidence/CLAUDE-MV/ui-*.png`. They show appearance only.

**Not done here:**
- No Windows game launch, so no `engine=LOADED` for any target.
- No launcher run on Windows, so no Narrator pass over the new radio groups.
- No authenticated Play.

## 4. Review of Sol's diffs

| # | Severity | Where | Finding | Status |
|---|---|---|---|---|
| S1 | Info | `049ce3c` `launcher-installer.yml` | Read-only permissions and a pinned checkout action. Only a successful `launcher-l0.yml` run on `main` of this repository is accepted; the artifact digest is checked and the source recorded | Good |
| S2 | Info | `prepare-installer.ps1`, `ooze-launcher.iss` | Per-user install with no elevation. The payload must match `SHA256SUMS` exactly: links, unlisted files, `..` and duplicates are refused. Player data is outside the install folder | Good |
| S3 | Nit | same | Version `0.1.0-rc1` is hard-coded in two places | Derive it from `release.json` next time |
| S4 | Info | `c4a5236` | I re-verified the Prism artifact digests (ZIP, `.mrpack`, JAR and DLL) | Matches |
| S5 | Info | `53adaa4` `catalog.rs` | **Exact targets:**<ul><li>Java is checked against `java_for_minecraft` and Mojang metadata;</li><li>one record per (version, Fabric or not);</li><li>the Fabric profile's id, `inheritsFrom`, main class and every library are pinned;</li><li>Ooze bundles must pin Fabric API and the JAR;</li><li>a legacy schema 1 manifest is still read</li></ul> | Good |
| S6 | Info | `catalog.rs` `targets()` | Fabric records with a bundle are offered both with Ooze and as plain Fabric | Handled: the interface offers three kinds |
| S7 | Info | `backend.rs` | `GameReady` is sent only when the prepared target has Ooze (`engine_state.filter(expect_ooze)`). Vanilla and plain Fabric never claim readiness | Matches the interface ("Minecraft is running.") |
| S8 | Info | `library/mod.rs`, `modrinth.rs` | Create, Search, Versions and Plan check the target against the catalog. The planner's bundled Fabric API comes from the target's record. This was my former V3 | Resolved |
| S9 | Info | `library/mod.rs` `verify_baseline` | A baseline is valid only if its version name and the presence of `mods/ooze-client.jar` match the target | Good |
| S10 | Low | `model.rs` `java_for_minecraft` | The version → Java map is hard-coded beside the catalog's own `java_major` | Acceptable; it must be extended with each new version |
| S11 | Info | `launcher-l0.yml` | CI runs `gradlew.bat build buildAll` with `JAVA_HOME` = 21, reads `ooze-launcher-profile.json`, and uses `build/dist/validated-ooze-clients.json` if present | Compatible: the daemon finds `JAVA_HOME_25_X64` (or provisions one); the legacy profile is unchanged. CI's `build` now also sets up 26.1.2, 26.2 and 26.3 for compiling (more download and disk) |

R1 (CLAUDE-RC1) is still open.

## 5. Limitations and open issues

| # | Issue |
|---|---|
| O1 | **No Windows `engine=LOADED` evidence yet for any target** (brief item 4). It needs Sol's run of `tools/launch-tests.sh` with native resources on Windows x64 (A1). That run also produces the validated manifest that lets Sol's packaging add the 26.x Ooze bundles |
| O2 | Until A1, Sol's candidates offer 26.x as Vanilla and plain Fabric only. Fabric with Ooze stays on 1.21.11, and the interface shows exactly that |
| O3 | 26.2+ may run on Vulkan. Ooze's GUI path rendered on Vulkan (llvmpipe) under Xvfb; native-engine frames on Windows under Vulkan are untested (part of A1) |
| O4 | Narrator on Windows over the kind and version radio groups (owner, with W14) |
| O5 | RC1's 1.21.11 JAR `088acdc6…` is superseded on `main` by the refactored JAR (same behaviour). A base package built from `main` needs its own verification (A2) |

## 6. Next tasks

- **Sol:** A1–A4 in `CLAUDE-MV-request.md`.
- **Claude:**
  - verify Sol's next candidate (catalog rows, JAR hashes against their validated manifest);
  - run live Modrinth tests against a 26.x Fabric instance from the real catalog;
  - re-check the interface on Windows with the owner.
- **Owner:**
  - Narrator and keyboard pass over Instances → New instance and Play on Windows;
  - sign-in for authenticated Play on one 26.x Ooze instance and one Vanilla or plain Fabric instance.
