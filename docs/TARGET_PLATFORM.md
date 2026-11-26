# Ooze Client — Target & Platform Strategy

> Deliverable 12. It covers what Ooze targets, how the build is pinned, where native code runs, how authentication works, and what must be verified before claiming support.
>
> Updated 2026-11-16: Ooze also ships the **Ooze Launcher**, an independent Windows x64 desktop launcher with its own Microsoft sign-in (§4, ARCHITECTURE §19).
>
> **Updated 2026-11-26 (owner brief, multi-version milestone):** Ooze supports Minecraft Java **1.21.11, 26.1, 26.1.1, 26.1.2, 26.2 and 26.3** on Windows x64. This reverses the 2026-11-09 "1.21.11 only" decision for 26.x; the 1.21.11 Java 21 remapped build is kept as it was.

## 1. Target

**Fabric targets.** One distribution JAR per Minecraft API family; each JAR lists exactly the versions it passed the production launch test on (TESTING §3, `tools/launch-tests.sh`).

| Minecraft | Java (adapter compile / runtime) | Fabric Loader | Fabric API | Mapping | JAR (`build/dist/`) |
|---|---|---|---|---|---|
| **1.21.11** | 21 / 21 | 0.20.1 | `0.141.6+1.21.11` | Mojang names, **remapped** (obfuscated runtime) | `ooze-client-<ver>+mc1.21.11.jar` |
| **26.1**, **26.1.1**, **26.1.2** | 25 / 25 | 0.20.1 | `0.155.3+26.1.2` | Mojang names, **not remapped** (unobfuscated) | `ooze-client-<ver>+mc26.1.jar` (shared, compiled against 26.1.2) |
| **26.2** | 25 / 25 | 0.20.1 | `0.161.0+26.2` | not remapped | `ooze-client-<ver>+mc26.2.jar` |
| **26.3** | 25 / 25 | 0.20.1 | `0.166.2+26.3` | not remapped; **SDL3** input and windows | `ooze-client-<ver>+mc26.3.jar` |

| Item | Value | Source / verification |
|---|---|---|
| Build plugins | **Loom 1.19.4**: `net.fabricmc.fabric-loom-remap` (1.21.11) and `net.fabricmc.fabric-loom` (26.x) | Fabric's own example mods use Loom 1.19 for 1.21.11 (remap) and every 26.x branch (non-remap), checked 2026-11-26. Switching 1.21.11 from 1.17.21 changed only the Loom and Tiny Remapper version stamps in its JAR manifest (all classes byte-identical) |
| Gradle | **9.8.2**, daemon on **JDK 25** (`gradle/gradle-daemon-jvm.properties`; Foojay toolchain resolver provisions JDK 21 and 25 when missing) | Loom 1.19 needs JDK 25 to run |
| Shared Java | `versions/common`: Minecraft-independent engine connector, status, gate, diagnostics, readiness marker, runtime state and translations; Java 21 bytecode, merged into every JAR | ARCHITECTURE §2 |
| Engine | Rust, via Sol's `bridge-java` + per-platform native library; **the same native resources in every JAR** | ARCHITECTURE §2 |
| Launcher | Ooze Launcher, Rust (egui/eframe), Windows x64; installs per-version profiles from `build/dist/ooze-launcher-profile-mc<version>.json` (1.21.11 also as `ooze-launcher-profile.json`) | ARCHITECTURE §19, `docs/handoffs/CLAUDE-MV-request.md` |
| Archives | Reproducible (fixed timestamps and entry order) | Same commit + native resources ⇒ same JAR SHA-256 |

**Pinning rules**
- Versions live in each adapter's `gradle.properties` (`versions/mc1_21_11`, `versions/mc26_1`, `mc26_2`, `mc26_3`: Minecraft, compatible versions, Loader, Fabric API, Java) and the root `build.gradle.kts` (Loom).
- A JAR may list several Minecraft versions only if every listed version passes the production launch test with that exact JAR.
- Bump pins only in a dedicated change that re-runs the launch tests (TESTING §3) and records the result in a handoff.
- Never use SNAPSHOT or `+` versions.

**Why one JAR per family, not one for all 26.x.** Compiling against each family catches API drift at build time, and 26.3's SDL3 change makes a shared JAR wrong even where it links: key codes are compile-time constants (`InputConstants.KEY_F9` is 298 under GLFW and 66 under SDL3), so a JAR compiled against 26.1 would bind the wrong keys on 26.3. 26.2 also moved the toast manager and current screen to `Gui`, and 26.3 moved `RenderPipeline` to `com.mojang.renderpearl`. Each family's differences live in two small classes (`compat/McCompat`, `dev/FixtureRenderState`); everything else in `versions/mc26` is shared.

**Instances (owner scope expansion 2026-11-17).** The launcher can manage several isolated instances through Sol's Library API 1. What the launcher may offer:

| Instance kind | Offered | Ooze mod and modules |
|---|---|---|
| Fabric Loader 0.20.1 on Minecraft 1.21.11 (Java 21), 26.1, 26.1.1, 26.1.2, 26.2 or 26.3 (Java 25) | When the library reports the combination (its install adapter and a Windows x64 launch test exist, L3) | **Yes**, with the matching JAR above |
| Vanilla on the same six versions | When the library reports it | No, plays as plain Minecraft |
| Other loaders (Quilt, Forge, NeoForge) or other Minecraft versions | **Only** when the library reports the combination supported, after an install adapter and a Windows x64 launch test exist | No |

**Not targets:** other Minecraft versions, Quilt/Forge/NeoForge builds of the mod, and any Ooze-controlled OpenGL/Vulkan backend switch (26.2+ choose their backend themselves; Ooze draws only through vanilla GUI pipelines). Version-specific Minecraft APIs stay inside their family adapter.

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
| Java sources use Mojang official names. For 1.21.11 the Loom remap plugin turns them into Intermediary names in the published JAR; 26.x JARs keep Mojang names (no remapping) | 1.21.11 is obfuscated at runtime; Mojang ships 26.x unobfuscated |
| 1.21.11 mixins keep their Loom refmap; 26.x mixins need none | Remapping of injection targets |
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
| T13 | Mojang's version manifest lists 26.1, 26.1.1, 26.1.2, 26.2 and 26.3 as releases, each requiring Java 25 (`java-runtime-epsilon`) and shipping no client mappings (unobfuscated); 26.3 replaces `lwjgl-glfw` with `lwjgl-sdl` 3.4.3 | ✓ verified 2026-11-26 (piston-meta.mojang.com) |
| T14 | Fabric Loader 0.20.1 is the newest stable loader for all six versions; Fabric API pins above are the newest releases per family on Modrinth and match fabric-example-mod branches 26.1.2, 26.2 and 26.3 | ✓ verified 2026-11-26 (meta.fabricmc.net, api.modrinth.com, raw.githubusercontent.com) |
| T15 | 26.x renamed `GuiGraphics` to `GuiGraphicsExtractor` (`guiRenderState` field, `addGuiElement`), moved the GUI render state to `net.minecraft.client.renderer.state.gui`, and `Screen.render` to `extractRenderState`; Fabric's key API is `keymapping.v1.KeyMappingHelper` | ✓ verified 2026-11-26 (javap on 26.1.2, 26.2 and 26.3 client JARs; adapters compile) |
| T16 | 26.3: `InputConstants.Type` is `KEYBOARD`/`MOUSE` with SDL scancodes (`KEY_F9` = 66, `KEY_ESCAPE` = 41; GLFW: 298 and 256); under Xvfb with Mesa, 26.3 fails to get a GLX visual and falls back to Vulkan (llvmpipe), and Ooze's GUI element path still renders | ✓ Game-launched on Linux 2026-11-26 (`CLAUDE-MV.md`) |
| T11 | egui/eframe 0.37 exposes AccessKit 0.25 → `accesskit_windows` 0.35 (UI Automation, including LiveRegionChanged) | ✓ verified 2026-11-16 (crate sources); Narrator run pending |
