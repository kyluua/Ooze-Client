# Handoff: CLAUDE · C0

| Field | Value |
|---|---|
| Owner | Claude (Fabric integration and product lead) |
| Milestone | **C0**: spec reconciliation, pinned Fabric 1.21.11 build, vanilla-only gate |
| Branch | `claude/blissful-euler-g55yql` |
| Base commit | `d56186573ffcd002fccbe902b877a8f7315e994d` |
| Result commit | `3064b19e0fe2c8e5cb0df2e6766633c700b7eb16` (this handoff is committed on top of it) |
| Date | 2026-11-09 |
| **Bridge version** | **None.** Sol's G0 handoff was not available (see §6). The build reports `NOT_BUNDLED` |
| Status | **C0 complete except item 4** ("integrate Sol's bridge and Windows x64 artifact"). That item moved to **C0b** and is blocked on G0 |

## 1. What was delivered

| C0 item | Result |
|---|---|
| 1. Reconcile AGENTS, architecture, repository structure, milestones and tests with the new target, language and authentication decisions | Done. `AGENTS.md` rewritten (ownership split, auth rules, evidence vocabulary, handoff protocol). `ARCHITECTURE.md` rewritten (Java adapter / bridge-java / Rust layers, frame pipeline, command vocabulary, auth boundary, ADR-012/013/014). `MULTI_VERSION.md` → `TARGET_PLATFORM.md`. `REPOSITORY_STRUCTURE.md`, `IMPLEMENTATION_PLAN.md` (C/G series), `TESTING.md`, `PRODUCT_VISION.md`, `README.md`, and the Rust ownership notes in TYPOGRAPHY, BACKGROUNDS, MOTION_SPEC, COMPONENTS and SCREENS updated. New: `INSTALL.md`, `FEATURE_BACKLOG.md` |
| 2. Repair milestone dependencies, stage early gates, exclude backups from backups, byte-based texture budgets | Done; see §5 |
| 3. Pinned Fabric 1.21.11 / Java 21 Gradle build | Done (§2) |
| 4. Integrate Sol's bridge and Windows x64 artifact | **Blocked: no G0.** Prepared: `settings.gradle.kts` auto-includes `:bridge-java` when `bridge-java/build.gradle.kts` exists, and `versions/mc1_21_11/build.gradle.kts` then adds it as `implementation` + `include`. A single integration point is marked in `EngineConnectors.select` |
| 5. Fabric launches to the vanilla title screen; a developer action exercises the bridge | Title screen: **Game-launched (Linux x64, Xvfb + Mesa)**. Developer action: implemented (`DeveloperProbe`, F9, registered only in development **and** with a loaded engine), **not exercised**: no bridge exists to probe |
| 6. A missing or incompatible native library gives a clear diagnostic, leaves vanilla usable and shows no non-working controls | **Game-launched (Linux x64)** for all five non-loaded states, via development fault injection and the real `NOT_BUNDLED` state. Real-DLL variants are in C0b (Windows) |

## 2. Pinned build

| Item | Version | How it was verified |
|---|---|---|
| Minecraft | 1.21.11 | Owner requirement |
| Fabric Loader | 0.20.1 | meta.fabricmc.net stable for 1.21.11 |
| Fabric API | 0.141.6+1.21.11 | maven.fabricmc.net latest for 1.21.11 |
| Loom | `net.fabricmc.fabric-loom-remap` **1.17.21** | Gradle module metadata: 1.18+ requires JVM 25 to *run Gradle*; 1.17.21 runs on JDK 21. **Deviation from the official example's `1.19-SNAPSHOT`**, so the whole toolchain stays on Java 21. Bump only when JDK 25 for Gradle is acceptable |
| Gradle | 9.8.2, wrapper `distributionSha256Sum=21bd5da2…9607` | Matches fabric-example-mod `1.21.11` |
| Mappings | `loom.officialMojangMappings()` | Distribution JAR verified remapped to intermediary (`class_310`, `class_2561`, …) |
| Java | 21 (`--release 21`, toolchain 21) | Local build on OpenJDK 21.0.12 |

## 3. Test results (evidence vocabulary)

| What | Evidence | Result |
|---|---|---|
| `./gradlew build buildAll` | **Built** (Linux x64, OpenJDK 21.0.12) | PASS |
| JUnit: 21 tests in `EngineGateTest`, `EngineConnectorsTest`, `PlatformInfoTest`, `EngineDiagnosticsTest`, `SourceRulesTest` | **Headless-tested** (Linux x64) | 21/21 pass |
| `tools/launch-tests.sh`: 7 real Minecraft 1.21.11 client launches under Xvfb + Mesa llvmpipe | **Game-launched (Linux x64)** | 7/7 PASS (table below) |
| Windows x64 launch | Not run (no Windows host in this environment) | **Not tested** |
| Native bridge round trip | Not possible (no G0) | **Not tested** |
| Manual verification | — | **None** (screenshots reviewed by the agent, not by a person) |

**Launch matrix** (each case asserts: title screen reached, expected engine state, no `key.oozeclient.*` key mappings, the "Ooze Client is off" notice queued, and a vanilla Options → Done round trip):

```
dev-not_bundled            PASS   state=NOT_BUNDLED
dev-missing_library        PASS   state=MISSING_LIBRARY        (fault-injected)
dev-unsupported_platform   PASS   state=UNSUPPORTED_PLATFORM   (fault-injected)
dev-incompatible           PASS   state=INCOMPATIBLE           (fault-injected)
dev-load_failed            PASS   state=LOAD_FAILED            (fault-injected)
prod-not_bundled           PASS   state=NOT_BUNDLED            (shipped remapped JAR, production Fabric env, dev=false)
prod-fault-ignored         PASS   state=NOT_BUNDLED            (release run ignores -Doozeclient.dev.engineFault, logs a warning)
```

**Evidence files:** `docs/handoffs/evidence/C0/`
- `SUMMARY.txt`;
- a `*.excerpt.txt` per case;
- screenshots `dev-not_bundled.png`, `dev-unsupported_platform.png`, `prod-not_bundled.png`.

The screenshots show the vanilla 1.21.11 title screen ("Minecraft 1.21.11/Fabric (Modded)") with the Ooze notice toast.

**Measured:** Fabric-side bootstrap 4–11 ms; engine connect ≤ 0.5 ms (the stub connector). Release and dev runs both well under the 50 ms budget.

**Environment notes:**
- The container has no audio device, so Minecraft logs "Failed to open OpenAL device".
- Realms authorisation fails with Loom's offline development account.
- Both are expected and don't affect the assertions.

## 4. Artifacts

| Artifact | Location | Size | SHA-256 |
|---|---|---|---|
| Distribution JAR | `build/dist/ooze-client-0.1.0-alpha.0+mc1.21.11.jar` (rebuild with `./gradlew buildAll`; not committed) | 27,194 bytes | `3cca60b29017d46c23c85b130f77187cf796dd45d5c020944acce2b9cbe2f2c6` |
| Gametest mod (development only, never shipped) | `versions/mc1_21_11/build/gametest-libs/` | — | — |

The JAR is reproducible only up to timestamps: Gradle reproducible-archive settings aren't enabled yet; that is a follow-up for C10.

## 5. Design-risk review (`OOZE_REPOSITORY_REVIEW.md` was not available, so I checked the listed risks myself)

| Risk | Finding | Fix (where) |
|---|---|---|
| Milestone dependencies | Components (old M5) used theme tokens before the theme system (old M7). HUD (M12) waited needlessly on typography and motion pages (M11). Those pages (M11) waited on the main menu (M10) | C3 introduces the theme runtime before components. C9 depends on C5 + C1. C8 depends on C5 (+G3/G4) and runs in parallel (IMPLEMENTATION_PLAN §1, §4) |
| Late acceptance gates | Performance, cross-version, Vulkan and accessibility checks came only at M13/M14 | Gates A–E plus per-milestone Perf / A11y / Launch gates (IMPLEMENTATION_PLAN §3) |
| Backups copied into backups | The old ARCHITECTURE §9.5 copied the whole config tree (excluding only `library/`), so each snapshot contained `backups/`, which grows recursively | Exclude `backups/`, `backgrounds/library|thumbs|inbox/`, `*.tmp`, `*.bak`, `*.corrupt-*`; skip snapshots > 8 MiB (ARCHITECTURE §9.5; G4 test in TESTING §7) |
| Texture budgets not enforceable | Glyph "≤ 24 MB" contradicted "2048² pages at high scale" (6 × 16 MiB = 96 MiB). The background "≤ 64 MB" example left out the mip chain it requires, and a crossfade would hold two 4K images (≈ 88 MiB) | Exact byte budgets per category plus a 100 MiB total. Enforced by Rust accountants **and** independently by the Java texture executor. Crossfade rule: the outgoing image keeps levels ≥ 1 (PERFORMANCE_AND_ACCESSIBILITY §2.2, BACKGROUNDS §4.5, TYPOGRAPHY §5.3) |
| Unbounded bridge traffic (new architecture) | — | JNI budgets: ≤ 2 calls per frame, 0 when idle, size caps, never per vertex or glyph (PERFORMANCE_AND_ACCESSIBILITY §2.4) |
| Identity leakage (new architecture) | — | Only username and UUID cross the bridge. `SourceRulesTest` bans token and session accessors in the adapter (TARGET_PLATFORM §4, ARCHITECTURE §17) |

## 6. Review of Sol's diff

**No Sol diff was available to review.** Checked at the start of C0:
- `git ls-remote origin` showed only `main` and `claude/blissful-euler-g55yql`;
- no `docs/handoffs/SOL-*` files;
- no reachable peer session.

The review is due in C0b, before integration.

## 7. Requests to Sol for G0 (the Fabric side needs these; method names are Sol's to choose)

1. **Gradle project** `bridge-java/` with `build.gradle.kts`, Java 21, **no Minecraft or Fabric dependencies**. `settings.gradle.kts` includes it automatically. Tell me whether it should be jar-in-jar (`include`) or merged classes; Loom's `include` on a non-mod JAR needs verifying.
2. **Load and status API** that never throws and reports every TARGET_PLATFORM §5 state:
   - `LOADED`, `MISSING_LIBRARY`, `UNSUPPORTED_PLATFORM`, `INCOMPATIBLE`, `LOAD_FAILED`;
   - the **bridge semantic version** and **protocol version**;
   - a human reason and technical detail.
   
   `EngineConnectors.select` will wrap it in one adapter class.
3. **Developer probe**: one round trip (for example echo plus timing) that C0b wires to the developer-only F9 action.
4. **Windows x64 artifact:**
   - its path inside the JAR (proposal: `natives/windows-x86_64/<lib>.dll`);
   - checksum verification;
   - the extraction directory (proposal: under `<gameDir>/.ooze/natives/<bridge-version>/`, never the temp dir root);
   - how the Gradle build obtains it (a prebuilt path or task). Gradle should not require Cargo for Java-only contributors.
5. **Failure behaviour:** no panic crosses FFI; a native fault inside a frame disables the engine and reports it; the JVM stays alive.
6. **Threading:** calls only on Minecraft's render thread; Rust worker threads never call back into the JVM.
7. **Protocol v1 (G1) requirements**, already written into the product spec:
   - frame input and output contents (ARCHITECTURE §5.2);
   - the platform command vocabulary (§11.1);
   - the snapshot schema with **username and UUID only** for identity;
   - budgets (PERFORMANCE_AND_ACCESSIBILITY §2.4);
   - texture work entries carrying byte sizes.
8. **Resource contract:** which assets (fonts with OFL, theme JSON, icons) are embedded in the native library and which are passed from the JAR at `init`. Translations: Java sends the resolved `oozeclient.*` table at `init` and on language change (ARCHITECTURE §16).
9. **Byte-budget accountants** with the §2.2 numbers (glyph 32 MiB, backgrounds 64 MiB, miscellaneous 4 MiB). **Config backups** excluding the paths in ARCHITECTURE §9.5.
10. **A `docs/handoffs/SOL-G0.md`** with the same fields as this file, so the C0b review can start from it.

## 8. Limitations

- **No Windows evidence.** Every launch result above is Linux x64; Windows x64 is the first supported platform and needs C0b.
- Failure states other than `NOT_BUNDLED` were **fault-injected**, not produced by real libraries.
- The developer probe has **never run**.
- `-Doozeclient.dev=true` deliberately enables developer behaviour (fault injection, probe) in release runs for diagnosis.
- Gradle prints "deprecated features … incompatible with Gradle 10" warnings, probably from Loom 1.17. Revisit when bumping Loom.
- No product UI exists yet. That is intended: product UI starts in C1–C3 (AGENTS "Real or absent").
- The earlier HTML design preview is a separate artifact and is **not** part of the repository or the product.

## 9. Exact next task

**C0b (Claude): integrate Sol's G0 bridge**, as soon as `docs/handoffs/SOL-G0.md` and Sol's branch exist:
1. Review Sol's G0 diff and record findings in `docs/handoffs/CLAUDE-C0b.md`.
2. Add `BridgeEngineConnector` in `versions/mc1_21_11/.../engine/`, delegating to bridge-java's load, status, probe and close calls, and select it in `EngineConnectors.select`.
3. Package bridge-java and the Windows x64 library. Verify the JAR contents and size.
4. Add Windows launch cases to `tools/launch-tests.sh`:
   - `LOADED` plus probe;
   - DLL removed → `MISSING_LIBRARY`;
   - mismatched DLL → `INCOMPATIBLE`.
   
   Then run them on Windows x64.

**If G0 is still unavailable**, the next independent work, which does not touch Sol's files, is **assets-src preparation** for C3:
- download the four font families' upstream static TTFs and their `OFL.txt` unmodified, recording copyright lines and SHA-256 sums;
- move the canonical theme JSON into `assets-src/themes/`;
- add the icon SVG sources with `assets-src/LICENSES.md`.

It gets its own handoff.
