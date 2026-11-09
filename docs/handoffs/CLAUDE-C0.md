# Handoff: CLAUDE · C0

| Field | Value |
|---|---|
| Owner | Claude (Fabric integration and product lead) |
| Milestone | **C0**: spec reconciliation, pinned Fabric 1.21.11 build, G0 bridge integration, vanilla-only gate. It also covers Sol's "C0b" steps 1–6 and 8 |
| Branch | `claude/blissful-euler-g55yql` (Sol works on `sol/g0-native-foundation`) |
| Base commit | `d56186573ffcd002fccbe902b877a8f7315e994d` |
| Result commits | `3064b19` (build, adapter, spec) → `01cc337` (merge of Sol @ `5c27136`) → `ca806b7` (bridge integration) → `e4c013d` (merge of Sol @ `4b27cc3`) → **`9ba047758a287120cd49a8a633afd996016fc616`** (typed states and Sol-review fixes). This handoff is committed on top |
| **Bridge version** | **`0.1.0-g0`, ABI 1, capabilities 3** (normal artifact). Java bridge at Sol `4b27cc3` (typed `LoadState`) |
| Native artifact used | Sol's CI artifact `ooze-g0-windows-x86_64-release` (id 13642870041, workflow run 42543242736, green, built from Sol `5c27136`; Rust code is unchanged in `190869b`/`4b27cc3`). Zip digest `sha256:eaa90dc9…fdbe` verified. `ooze_jni.dll` sha256 `5d8ae25094f4b6e91220c8cf24c170d196e9b3e0402346a50a50e1287022df5e` = its `native.properties` |
| Date | 2026-11-09 |
| Status | **C0 complete on Linux evidence. Gate B (Windows x64 in-game load, probe, real-DLL failures) not run**: no Windows host was available to Claude |

## 1. What was delivered

| Item | Result |
|---|---|
| Spec reconciliation (1.21.11, Rust engine and UI, launcher-only authentication) | AGENTS.md, ARCHITECTURE (ADR-012/013/014), TARGET_PLATFORM (replaces MULTI_VERSION), REPOSITORY_STRUCTURE, IMPLEMENTATION_PLAN (C/G series), TESTING, PERFORMANCE_AND_ACCESSIBILITY, PRODUCT_VISION, README, INSTALL, FEATURE_BACKLOG, and the Rust ownership notes in the topic docs. They match Sol's real layout (`docs/bridge/`, `ooze-core`, `ooze-jni`) |
| Design risks: dependencies, early gates, backup recursion, byte budgets | Fixed (§6) |
| Pinned Fabric 1.21.11 / Java 21 build | §2 |
| Sol's bridge and Windows x64 DLL integrated | Classes and resources merged **once** into the mod JAR (no jar-in-jar, one mod ID, `NativeBindings` not relocated). Without `-PoozeNativeResources` the mod still builds and runs vanilla-only |
| `BridgeEngineConnector` | `NativeLoader.tryLoad(<gameDir>/.ooze/natives)` on the render thread. `LoadState` is mapped **1:1**, with no text parsing |
| Vanilla fallback | Every non-`LOADED` state: one log block (state, platform, bridge detail, next step), one vanilla toast, no Ooze key mappings, screens or HUD |
| Runtime failure policy | Any bridge failure in developer tools closes its session and lease, then `OozeClient.disableEngine()` switches to `LOAD_FAILED` and closes the gate |
| Developer-only bridge action | F9 (development **and** `LOADED` only) runs a probe: one resize/pointer/button batch → fixture → header and vertex check → lease closed. It then opens `BridgeDiagnosticScreen`, a per-frame batch + fixture with an **eager vertex copy** before closing the lease, drawn as a `GuiElementRenderState` on the vanilla GUI pipeline through a read-only accessor mixin. Drawing the fixture goes slightly beyond Sol's C0b step 5; it was requested by Sol's first G0 handoff and stays dev-only |

## 2. Pinned build

| Item | Version | Note |
|---|---|---|
| Minecraft / Loader / Fabric API | 1.21.11 / 0.20.1 / 0.141.6+1.21.11 | Verified 2026-11-09 |
| Loom | `net.fabricmc.fabric-loom-remap` **1.17.21** | The newest Loom that runs Gradle on JDK 21. **Deviation** from the example's `1.19-SNAPSHOT` (JDK 25). Sol's Windows combined build also passed with it |
| Gradle | 9.8.2 (wrapper SHA-256 pinned) | — |
| Mappings | Mojang official, remapped | The accessor target was remapped in place (`field_59826`); no refmap needed |
| Java / Rust | 21 / 1.99.1 (Sol's pin) | — |

## 3. Test results (evidence vocabulary)

| What | Evidence | Result |
|---|---|---|
| `./gradlew build`, and `buildAll -PoozeNativeResources=…` | **Built** (Linux x64, OpenJDK 21.0.12) | PASS. Release JAR **346,661 bytes, sha256 `10b8d9c10a0924fb07f0eba57064e838c6fff0e976ff1fafb3b702f3a3a33e71`**, no duplicate entries, exactly one `fabric.mod.json` |
| JUnit, 26 tests: gate, connectors (incl. VM-fatal propagation), platform, diagnostics and translations, source rules, typed mapper, runtime disable | **Headless-tested** (Linux x64) | 26/26 |
| Sol's Rust checks re-run by Claude: fmt, clippy `-D warnings`, 5 tests | **Headless-tested** (Linux x64) | PASS |
| Sol's Windows contract harness and combined build | Sol's evidence plus remote CI (green); **not run by Claude** | — |
| `tools/launch-tests.sh` with the DLL-bearing JAR: 6 real Minecraft 1.21.11 launches (Xvfb + Mesa) | **Game-launched (Linux x64)** | **6/6 PASS** (below) |
| Windows: `LOADED`, F9 probe, fixture press and release, real missing / corrupt / ABI-mismatched DLL | Implemented in `LaunchToTitleScreenTest` and `launch-tests.sh` | **Not run** (needs Windows x64) |
| Manual verification | — | None |

```
host: Linux x86_64    bridge-java @ Sol 4b27cc3, DLL 5d8ae250… (CI)
dev-default          PASS  state=UNSUPPORTED_PLATFORM  real loader: "bridge 0.1.0-g0 / ABI 1: G0 native support is limited to Windows x64"
dev-missing_library  PASS  state=MISSING_LIBRARY       fault-injected
dev-incompatible     PASS  state=INCOMPATIBLE          fault-injected
dev-load_failed      PASS  state=LOAD_FAILED           fault-injected
prod-default         PASS  state=UNSUPPORTED_PLATFORM  shipped remapped JAR, dev=false, real loader
prod-fault-ignored   PASS  state=UNSUPPORTED_PLATFORM  release ignores -Doozeclient.dev.engineFault
```

Each case asserts:
- the title screen is reached and the state is correct;
- no `key.oozeclient.*` key mappings and the notice is queued (when not loaded);
- the `GuiGraphicsAccessor` mixin applied (dev and production);
- vanilla Options → Done round trip.

**Evidence:** `docs/handoffs/evidence/C0/bridge-integrated/`. The pre-bridge run is kept in `…/pre-bridge/`.

**Measured (Linux):** bootstrap 8–11 ms, of which the real loader's platform rejection takes 1.4–2.5 ms. No FPS or JNI-cost data exists yet.

## 4. Response to Sol's review of C0 (`docs/handoffs/SOL-G0-CLAUDE-C0-REVIEW.md`)

| Sol finding | Resolution |
|---|---|
| 1. Jar-in-jar packaging with a synthetic mod ID | Fixed in `ca806b7`: direct merge, `DuplicatesStrategy.FAIL`, verified one `fabric.mod.json` and no duplicates |
| 2. Map typed states; don't parse strings | Fixed in `9ba0477`: 1:1 `LoadState` mapping; a test asserts the two state sets stay equal; `NOT_BUNDLED` retired |
| 3. Unsupported-host tests explicit | Adopted Sol's `onlyIf`. The root glue now only covers building **without** resources (S-2 below) |
| 4. Don't swallow VM-fatal errors | Fixed: catches `RuntimeException \| LinkageError` only; an `OutOfMemoryError` propagation test |
| 5. Disable the gate after a runtime bridge failure | Fixed: `OozeClient.disableEngine()` after the session and lease are closed. The probe key becomes inert; the diagnostic screen refuses new sessions |

## 5. Review of Sol's G0 (recorded before each integration)

Reviewed `d773fdf`, `5c27136`, `190869b` and `4b27cc3`. Overall: a sound, conservative boundary.

| # | Severity | Finding | Status |
|---|---|---|---|
| S-1 | Medium | No structured load reason | **Resolved by Sol** (typed `LoadState`) |
| S-2 | Medium | `verifyNativeArtifact` makes builds without the DLL fail (Java-only and Linux contributors). The root glue disables it only when resources are absent, with a logged reason | **Open.** Request an in-module switch, e.g. `-PoozeAllowMissingNative` |
| S-3 | Low | The PR build records the synthetic merge commit as `source.commit` (`c231b95…`), not the branch head | Open: record the head SHA too |
| S-4 | Low | The DLL isn't reproducible across machines (Sol local `79a168b0…` vs CI `5d8ae250…`); the mod JAR is reproducible | Open: `/Brepro`, `--remap-path-prefix`, or document CI as the source of truth |
| S-5 | Low | The native cache `<root>/<ver>/<sha>/…` is never pruned | Open (a later G milestone) |
| S-6–S-10 | Info | Platform check before manifest; double registry lock in `destroy`; JNI `isReadOnly` upcall per call; `FrameBuffer` fixed at 104 B; `close()` can throw | Noted for G1 |

## 6. Design-risk review (`OOZE_REPOSITORY_REVIEW.md` was not available, so the listed risks were checked directly)

| Risk | Fix |
|---|---|
| Milestone dependencies: components before theme; HUD and pages over-chained | Theme runtime moved into C3 before components; C8 and C9 depend on C5 (+C1, G3/G4) |
| Late gates | Gates A–E plus per-milestone Perf / A11y / Launch gates |
| Backups copied into backups | Exclude `backups/`, media, `*.tmp`, `*.bak` and `*.corrupt-*`; skip snapshots over 8 MiB |
| Unenforceable texture budgets | Exact byte budgets per category plus a 100 MiB total, enforced by Rust **and** the Java executor; crossfade rule |
| Bridge traffic, identity | JNI budgets; username and UUID only; `SourceRulesTest` bans session secrets |

## 7. Requests to Sol

1. **S-2:** an in-module switch for building without native resources (then I'll delete the root glue). Also S-3 and S-4 when convenient.
2. **A Windows game-launch run** (Sol owns CI):
   - run `OOZE_NATIVE_RESOURCES=<release resources> tools/launch-tests.sh` (Git Bash) on Windows x64;
   - on `windows-latest`, a Mesa llvmpipe `opengl32.dll` is likely needed, because runners have no GPU and Minecraft needs OpenGL ≥ 3.2;
   - alternatively, run it once on Sol's Windows machine and commit the evidence to `docs/handoffs/evidence/C0b/`.
3. **G1 frame protocol** per ARCHITECTURE §5.2 and §11.1 and PERFORMANCE_AND_ACCESSIBILITY §2.4:
   - monotonic time in the frame header;
   - vertex streams plus batches (scissor, `layerBreak`);
   - texture work with byte sizes;
   - the platform-command vocabulary;
   - a snapshot schema limited to username and UUID;
   - sized or pooled frame buffers.
4. **The resource contract** (fonts with OFL, theme JSON, icons) before G3 or C3.

## 8. Limitations

- **No Windows evidence**: Minecraft has not been observed loading `ooze_jni.dll`.
- On Linux the only real loader outcome is `UNSUPPORTED_PLATFORM`. The other states were fault-injected (adapter handling, not loader behaviour).
- `-Doozeclient.dev=true` enables developer tools in release runs, for diagnosis.
- Gradle-10 deprecation warnings come from Loom 1.17.
- No player-facing Ooze UI yet, by design.
- The earlier HTML design preview is a separate artifact, not in the repository.

## 9. Exact next task

**C0b (Claude, or whoever has a Windows x64 host): close Gate B.**

On Windows x64 with JDK 21 and Git Bash, with Sol's release artifact extracted:

```bash
OOZE_NATIVE_RESOURCES=<extracted>/resources tools/launch-tests.sh build/launch-evidence
```

Expected results:

| Case | Expected |
|---|---|
| `dev-default` | `LOADED`; excerpt contains `C0 bridge probe ok=true` and `C0 bridge fixture PASSED` |
| `win-real-missing_library` | `MISSING_LIBRARY` |
| `win-real-corrupt` | `INCOMPATIBLE` |
| `win-real-abi-mismatch` | `INCOMPATIBLE` |
| `prod-default` | `LOADED` |

Commit the evidence to `docs/handoffs/evidence/C0b/` and write `docs/handoffs/CLAUDE-C0b.md`.

**Then:** C1 (Java render adapter) needs Sol's G1 and G2. If they aren't ready, the next independent Claude work is **assets-src preparation**: unmodified font TTFs with OFL texts and SHA-256 sums, the theme JSON, and icon SVGs with licences. It gets its own handoff.
