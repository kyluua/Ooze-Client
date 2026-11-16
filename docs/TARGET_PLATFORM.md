# Ooze Client — Target & Platform Strategy

> Deliverable 12. This replaces the earlier multi-version strategy: per the owner's decision on 2026-11-09, 26.1/26.2 builds are no longer requirements.
>
> It covers what Ooze targets, how the build is pinned, where native code runs, how authentication works, and what must be verified before claiming support.
>
> Updated 2026-11-16: Ooze also ships the **Ooze Launcher**, an independent Windows x64 desktop launcher with its own Microsoft sign-in (§4, ARCHITECTURE §19).

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
| Launcher | Ooze Launcher, Rust (egui/eframe), Windows x64; installs the profile above from `build/dist/ooze-launcher-profile.json` | ARCHITECTURE §19 |

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

**Primary path: the Ooze Launcher** (owner decision 2026-11-16).

```
Ooze Launcher backend (Sol) ──(system browser + PKCE, owned public client ID)──► Microsoft
        │  Xbox Live → XSTS → Minecraft Services → entitlement → profile
        │  tokens stay in the backend (Windows DPAPI); never in the UI, logs or exports
        ├──► launcher UI (Claude): AccountSummary { username, uuid } + typed states only
        └──(launch interface; command never logged raw)──► Minecraft 1.21.11 + Fabric Loader
                                                              │
                                     Ooze Fabric adapter (Java) reads only:
                                     · username (display)  · UUID (display / skin lookup by vanilla)
                                                              │  bounded snapshot
                                                              ▼
                                                         in-game Rust UI (display only)
```

- **Developer configuration, not per-player setup.** The launcher needs the project's own registered Microsoft **public client** application. It is a desktop app with no client secret, using the system browser and PKCE. The registration is made once by the developer; see Microsoft's [app registration quickstart](https://learn.microsoft.com/en-us/entra/identity-platform/quickstart-register-app) and [public vs confidential clients](https://learn.microsoft.com/en-us/entra/identity-platform/msal-client-applications). Sol documents the exact steps in `docs/launcher/AUTH_SETUP.md`.
- **Minecraft Services approval.** New third-party applications must be approved for Minecraft Services. Until then the services reject sign-in with "Invalid app registration" (observed by other developers in 2026; verify the current process in `AUTH_SETUP.md`). The "no app registration / no approval" wish of 2026-11-09 **cannot be promised** for an independent launcher. The launcher shows `AuthNotConfigured` or `AuthAppNotApproved` honestly and never falls back to offline play.
- **No paid hosting.** There is no Azure hosting, no Ooze login server and no Ooze account.
- **Passwords and other launchers.** Ooze never collects Microsoft passwords and never reads another launcher's credentials.
- **Account switching** happens in the Ooze Launcher (Settings → Account → Switch account / Sign out).

**Inside the mod** (unchanged): the mod never reads, logs, stores or transmits tokens or session IDs. The in-game Rust UI receives only username and UUID.

**Secondary path: manual install** into the official Minecraft Launcher (INSTALL.md §B). It still works because Ooze is a normal Fabric mod; sign-in is then the official launcher's.

**Development** runs (`runClient`, launch tests) use Loom's offline development account. That is a development convenience, not a product feature.

## 5. Native availability states (Fabric side)

The Fabric adapter maps Sol's typed `NativeLoader.LoadState` **one-to-one** (`BridgeStatusMapper`; no text matching). A unit test fails if the two state sets ever diverge.

Sol's loader checks the platform **first**, so on every non-Windows-x64 host the real state is `UNSUPPORTED_PLATFORM`, whatever the JAR contains. On Windows x64:
- a JAR built without `-PoozeNativeResources`, or one missing the DLL, is `MISSING_LIBRARY`;
- a manifest, version, ABI, capability or SHA-256 mismatch is `INCOMPATIBLE`;
- a `System.load` or I/O failure is `LOAD_FAILED`.

A controlled bridge failure at runtime (in the developer probe or the diagnostic screen) switches the session to `LOAD_FAILED` and closes the gate.

| State | Meaning | Behaviour |
|---|---|---|
| `LOADED` | Bridge and native library loaded, versions compatible | Ooze features may register (milestone permitting) |
| `MISSING_LIBRARY` | No native manifest or library bundled in this build | Vanilla-only mode, diagnostic naming the platform |
| `UNSUPPORTED_PLATFORM` | OS or architecture not in §2's supported list | Vanilla-only mode, diagnostic |
| `INCOMPATIBLE` | Manifest, version, ABI, capabilities or SHA-256 don't match this mod | Vanilla-only mode, diagnostic with the loader's reason |
| `LOAD_FAILED` | Loading threw (I/O, security software, linkage), **or** the engine was disabled after a runtime bridge failure | Vanilla-only mode, diagnostic with the error class and message |

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
| T5b | 1.21.11 `GuiGraphics.guiRenderState` is package-private; custom elements go through `GuiRenderState#submitGuiElement` (accessor mixin); vanilla `RenderPipelines.GUI` draws QUADS | ✓ verified C0 (javap and accessor-applied launch check); fixture drawing **not yet Game-launched** (needs Windows) |
| T6 | Since 1.21.6, GUI drawing is deferred through a GUI render state; since 1.21.9, input callbacks use record types | Verify in C1/C2 against the 1.21.11 jar |
| T7 | Ordering of GUI elements with different pipelines within one stratum | Unknown; C1 must test it (ARCHITECTURE §5.9) |
| T8 | LWJGL `tinyfd` is on the 1.21.11 runtime classpath (Java-side file dialog) | Verify in C7; a fallback is specified |
| T9 | Fabric API `0.141.6+1.21.11` on maven.fabricmc.net has SHA-256 `bdff7fd7…b9ed0f` (2,426,039 bytes); `meta.fabricmc.net/v2/versions/loader/1.21.11/0.20.1/profile/json` resolves | ✓ verified 2026-11-16 (downloaded, matches Fabric's `.sha256`) |
| T10 | New third-party apps need Minecraft Services approval; unapproved apps get HTTP 403 "Invalid app registration" | Reported by developers (Microsoft Q&A, Feb 2026); **Sol must verify** the current process in AUTH_SETUP.md |
| T12 | The owner's public client ID `359e0deb-…e273d2` is accepted by Microsoft's personal-account (consumers) sign-in | ✗ **not yet**: 2026-11-16, Microsoft answered "unauthorized_client: The client does not exist or is not enabled for consumers" (CLAUDE-L0 §11). Recheck after the registration's account types are fixed |
| T11 | egui/eframe 0.37 exposes AccessKit 0.25 → `accesskit_windows` 0.35 (UI Automation, including LiveRegionChanged) | ✓ verified 2026-11-16 (crate sources); Narrator run pending |
