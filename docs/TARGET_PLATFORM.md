# Ooze Client — Target & Platform Strategy

> Deliverable 12. This replaces the earlier multi-version strategy: per the owner's decision on 2026-11-09, 26.1/26.2 builds are no longer requirements.
>
> It covers what Ooze targets, how the build is pinned, where native code runs, how authentication works, and what must be verified before claiming support.

## 1. Target

| Item | Value | Source / verification |
|---|---|---|
| Minecraft | **Java Edition 1.21.11** | Owner requirement |
| Loader | **Fabric Loader 0.20.1** (`fabricloader >=0.20.1`) | meta.fabricmc.net stable for 1.21.11, checked 2026-11-09 |
| Fabric API | **0.141.6+1.21.11** | maven.fabricmc.net latest for 1.21.11, checked 2026-11-09 |
| Java (adapter) | **21** (compile `--release 21`; runs on the launcher's Java 21 runtime) | Owner requirement; 1.21.11 runtime |
| Mappings | **Mojang official names** via `loom.officialMojangMappings()` | Owner requirement |
| Build plugin | **`net.fabricmc.fabric-loom-remap` 1.17.21** | The remapping Loom plugin is required for obfuscated 1.21.11. 1.17.21 is the newest release whose Gradle plugin runs on JDK 21 (1.18+ declares JVM 25). The official example uses `1.19-SNAPSHOT`, which needs JDK 25 to run Gradle |
| Gradle | **9.8.2** (wrapper with pinned SHA-256) | Matches the official 1.21.11 example mod |
| Distribution | One remapped Fabric JAR: `ooze-client-<ver>+mc1.21.11.jar` | `./gradlew buildAll` → `build/dist/` |
| Engine | Rust, via Sol's `bridge-java` + per-platform native library | ARCHITECTURE §2 |

**Pinning rules**
- Versions live in `versions/mc1_21_11/gradle.properties` (Minecraft, Loader, Fabric API) and the root `build.gradle.kts` (Loom).
- Bump them only in a dedicated change that re-runs the launch tests (TESTING §3) and records the result in a handoff.
- Never use SNAPSHOT or `+` versions.

**Not targets** (removed from scope): 26.1.x, 26.2.x, multi-JAR builds, any OpenGL/Vulkan backend switch. Code must not copy 26.x APIs or names that differ from 1.21.11.

## 2. Native platforms

Rust code ships as a native library inside the JAR and is loaded by Sol's `bridge-java` at start-up.

| Platform | Status | What may be claimed |
|---|---|---|
| **Windows x64** | **First supported platform** | Supported only when a Windows x64 artifact exists **and** a Game-launched test on Windows x64 passed (TESTING §4) |
| Linux x64, macOS (arm64/x64), Windows arm64 | **Not supported yet** | Vanilla-only mode with a clear diagnostic (§5). Claim support only after an artifact **and** a launch test exist for that platform |

The C0 development container is Linux x64. It can **game-launch** the client in vanilla-only mode under Xvfb with Mesa, but it cannot load a Windows DLL. Windows launch evidence must come from a Windows machine or a Windows CI runner (Sol owns CI).

## 3. Mapping and symbol boundary

| Rule | Why |
|---|---|
| Java sources use Mojang official names; the Loom remap plugin turns them into Intermediary names in the published JAR | 1.21.11 is obfuscated at runtime |
| Mixins keep their Loom refmap | Remapping of injection targets |
| **No Minecraft symbol crosses the bridge**: no class names, method names or obfuscated identifiers | Native code must not depend on obfuscation or mappings |
| Access widening is avoided. Use Mixin `@Accessor` / `@Invoker` where possible | Smaller remap surface |

## 4. Authentication and identity

```
Official Minecraft Launcher ──(Microsoft sign-in, entitlement, session)──► Minecraft 1.21.11 + Fabric Loader
                                                                              │
                                                     Ooze Fabric adapter (Java) reads only:
                                                     · username (display)  · UUID (display / skin lookup by vanilla)
                                                                              │  bounded snapshot
                                                                              ▼
                                                                         Rust UI (display only)
```

- Ooze is a **Fabric client mod**. It is not a launcher and adds **no login**, **no Azure application registration** and **no approval dependency**.
- The launcher and Minecraft keep the session. Ooze never reads, logs, stores or transmits access tokens, session IDs, refresh tokens or Microsoft account data.
- The Rust UI receives only username and UUID, for display: the profile chip and About.
- Account switching happens in the launcher. Ooze shows no "switch account" control.
- A separate launcher with its own authentication is **outside this architecture**.
- Development runs (`runClient`) use Loom's offline development account. That is a development convenience, not a product feature.

## 5. Native availability states (Fabric side)

The Fabric adapter consumes the bridge's load result. Until Sol's G0 handoff lands, the build ships without the bridge and reports `NOT_BUNDLED`.

| State | Meaning | Behaviour |
|---|---|---|
| `LOADED` | Bridge and native library loaded, versions compatible | Ooze features may register (milestone permitting) |
| `NOT_BUNDLED` | This JAR contains no bridge or native library | Vanilla-only mode, diagnostic |
| `MISSING_LIBRARY` | Bridge present, no native library for this OS and architecture, or the file is missing | Vanilla-only mode, diagnostic naming the platform |
| `UNSUPPORTED_PLATFORM` | OS or architecture not in §2's supported list | Vanilla-only mode, diagnostic |
| `INCOMPATIBLE` | Library loaded but its bridge or protocol version doesn't match | Vanilla-only mode, diagnostic with both versions |
| `LOAD_FAILED` | Loading threw (for example a missing VC++ runtime, corrupt file, or security software) | Vanilla-only mode, diagnostic with the error class and message |

**Vanilla-only mode**
- Registers no Ooze key mappings, screens, HUD elements or mixin-driven replacements.
- Logs one clear multi-line diagnostic.
- Shows one vanilla system toast on the first title screen: "Ooze Client is off" plus the reason.
- Vanilla Minecraft stays fully usable.

## 6. Facts register (verify before relying on them)

| # | Statement | Status |
|---|---|---|
| T1 | Fabric Loader 0.20.1 is stable for 1.21.11 | ✓ verified 2026-11-09 (meta.fabricmc.net) |
| T2 | Fabric API 0.141.6+1.21.11 is the latest 1.21.11 build | ✓ verified 2026-11-09 (maven metadata) |
| T3 | `net.fabricmc.fabric-loom-remap` exists from Loom 1.14 onwards; 1.17.21 runs on JDK 21 | ✓ verified 2026-11-09 (plugin markers + Gradle module metadata) |
| T4 | The official 1.21.11 example uses Gradle 9.8.2 + `loom.officialMojangMappings()` | ✓ verified 2026-11-09 (fabric-example-mod branch `1.21.11`) |
| T5 | Fabric API offers client gametests (`fabric-client-gametest` entrypoint, `runClientGameTest`) on 1.21.11 | ✓ verified by the C0 launch test (see `docs/handoffs/CLAUDE-C0.md`) |
| T6 | Since 1.21.6, GUI drawing is deferred through a GUI render state; since 1.21.9, input callbacks use record types | Verify in C1/C2 against the 1.21.11 jar |
| T7 | Ordering of GUI elements with different pipelines within one stratum | Unknown; C1 must test it (ARCHITECTURE §5.9) |
| T8 | LWJGL `tinyfd` is on the 1.21.11 runtime classpath (Java-side file dialog) | Verify in C7; a fallback is specified |
