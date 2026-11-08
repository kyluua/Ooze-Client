# Ooze Client — Multi-Version Compatibility Strategy

> Deliverable 12. How Ooze supports Minecraft Fabric **1.21.11 through 26.2**: what is shared, what is version-specific, how each target is built, and what must be verified.

## 1. Target matrix

| Target | Java | Obfuscation at runtime | Dev names in Ooze source | Loom mode | Rendering | Distribution |
|---|---|---|---|---|---|---|
| **1.21.11** | 21 | Obfuscated (Fabric Intermediary at runtime) | Mojang official mappings | Remapping plugin | OpenGL (Blaze3D abstraction; deferred GUI render state) | `+mc1.21.11.jar` |
| **26.1.x** | 25 | **Unobfuscated** (Mojang names at runtime) | Mojang names (native) | Non-remapping plugin | OpenGL (Blaze3D) | `+mc26.1.jar` |
| **26.2.x** | 25 | Unobfuscated | Mojang names | Non-remapping plugin | **OpenGL and Vulkan** backends behind Blaze3D | `+mc26.2.jar` |

- **Reference target** (deep verification at every milestone): **26.1.x**. It shares names with 26.2, and its API is closest to both neighbours.
- **Hotfix policy:** each JAR declares a range in `fabric.mod.json`:
  - 1.21.11 target: `"minecraft": "~1.21.11"`;
  - 26.1 target: `"~26.1"` (≥ 26.1.0, < 26.2);
  - 26.2 target: `"~26.2"`.
  
  When a hotfix breaks something, ship a new JAR for that target. Never widen a range without running the target's gametests.

**A single JAR will not work across these targets**, because of:
- obfuscated vs. unobfuscated runtime names;
- Java 21 vs. Java 25 class files (core is compiled for 21, but the 26.x adapters use 25);
- GUI, render and input API differences.

Ooze ships **one JAR per target** (ADR-005).

## 2. What the 26.1 mapping change means for Ooze

Starting with 26.1, Mojang ships Java Edition **without obfuscation**, and Fabric's toolchain uses Mojang's names directly. There is no Intermediary layer and no Yarn for 26.1+. Consequences, and how Ooze handles each:

| Consequence | Ooze approach |
|---|---|
| Yarn names are not available for 26.1+ | **All Ooze sources use Mojang names in every target.** The 1.21.11 project uses `loom.officialMojangMappings()`, so `mc-shared` compiles unchanged across all three (ADR-004) |
| No remapping step for 26.x JARs | 26.x projects apply Loom's **non-remapping** plugin. 1.21.11 applies the **remapping** plugin, and Loom remaps Mojang → Intermediary for the published JAR |
| Mixin targets are Mojang names at runtime on 26.x | Mixin source is identical (Mojang names in both). On 1.21.11 the refmap produced by Loom handles remapping; on 26.x no refmap is needed |
| Access widening | Avoid it. Use Mixin `@Accessor` / `@Invoker`, which work the same on all targets. If widening becomes unavoidable, use the format the target's Loom version requires (access widener, or its successor format) |
| A single Gradle build hosts both Loom modes | Declare **one Loom version** in `settings.gradle.kts` (`apply false`) and apply the remapping plugin id in `mc1_21_11` and the non-remapping plugin id in `mc26_*`. If a single Loom version cannot serve both modes, split `mc1_21_11` into its own included build (documented fallback) |

## 3. API areas that differ, and their adapters

Each area lists the stable core interface that hides the difference. "Verify" items are checked in M0 and M2 against the target's actual API (§7).

| Area | What varies | Core abstraction | Notes |
|---|---|---|---|
| GUI rendering | GUI render-state element submission, pipeline registration, scissor, layer advancing | `RenderBackend` (ARCHITECTURE §5.8) | The biggest adapter. Uses vanilla GUI pipeline snippets only |
| Textures | Texture creation, upload, sampler and filter configuration | `RenderBackend.createTexture/uploadRegion` | Linear vs. nearest per texture |
| World blur | How the menu blur is requested | `RenderBackend.requestWorldBlur` | Falls back to the vanilla option |
| Input | Record-based input event types (since 1.21.9) vs. raw ints; key-mapping categories | `OozeScreenHost` conversion → core `PointerEvent`/`KeyEvent`; `PlatformInput` | Core never sees vanilla input types |
| Key mappings | Registration API, category objects | `PlatformInput.registerCoreKeyMappings` | — |
| HUD | Fabric HUD element registry vs. legacy callback | `PlatformHudHook` (internal) | Registered after the vanilla HUD layers |
| Screens | `Screen` method signatures, `renderBackground` behaviour, title-screen class, file-drop callback | `OozeScreenHost` (mc-shared, with small per-target subclasses if signatures differ) | — |
| Text | Vanilla text component API, font renderer draw calls | `PlatformText` | Fallback runs only |
| Game data | World list loading, server list access, connect and join helpers | `PlatformGame` | All async |
| Resource identifiers | Identifier class naming in Mojang mappings | Hidden behind `OozeId` ↔ adapter conversion | Core never references it |
| Window | Framebuffer size, GUI scale, cursor API | `PlatformWindow` | GLFW cursors are used directly if there is no vanilla API |
| Sound | Sound event and manager APIs | `PlatformAudio` | — |

## 4. 26.2: OpenGL and Vulkan

Ooze's rendering rules guarantee backend independence:

1. **No direct graphics API calls**: no `GL*`, no `GlStateManager`, no Vulkan calls, no NanoVG anywhere (ADR-002). This is enforced by an ArchUnit rule over `versions/mc26_2`, which bans `org.lwjgl.opengl..` and `org.lwjgl.vulkan..` imports.
2. **Only vanilla pipelines and shaders** (or pipelines derived from vanilla snippets that keep vanilla shaders). Mojang maintains these for both backends.
3. **Textures** are created only through the Blaze3D device abstraction via the adapter.
4. **Capability detection:** at start-up `BackendCapabilities` records the backend name (`OpenGL` or `Vulkan`), the GPU string and whether derived triangle pipelines work. If they do not, Ooze uses the degenerate-quad path (ARCHITECTURE §5.8).
5. **Test matrix:** every 26.2 milestone gate runs the client gametests on **both** backends (TESTING §4). Golden screenshots must match within tolerance on both.
6. **Backend switching at runtime** (if the game allows it, for example a settings change that triggers a reload): Ooze listens for the resource or renderer reload signal through the adapter, **recreates all GPU textures** (glyph atlases, backgrounds, thumbnails, noise) from CPU-side sources, and bumps `ScaleEpoch`. CPU-side copies of glyph bitmaps are kept for the current page set so the rebuild needs no re-rasterization.

## 5. Shared vs. specific code

```
core (≈ 85% of code)  ── 100% shared, Java 21
platform-lwjgl        ── 100% shared (LWJGL APIs are stable across targets)
mc-shared             ── shared source, compiled per target (≈ 10%)
versions/<target>     ── ≈ 5% per target: entrypoint, mixins, RenderBackend impl, input conversion, HUD hook, small API shims
```

**Rule of thumb:** if a change touches `versions/*` for more than one target, consider whether the code belongs in `mc-shared` behind an interface.

## 6. Adding a future target (e.g. 26.3)

1. Copy the closest `versions/<target>` to `versions/mc26_3`. Update its `gradle.properties` (Minecraft, Loader, Fabric API and Java versions) and the `fabric.mod.json` range.
2. Build. Fix compile errors **only** in `versions/mc26_3`, or move code behind a new adapter method (and implement that method in all targets).
3. Run `runClientGametest` for the new target. Update goldens only for intended visual differences, and record why in the PR.
4. Update §1 of this document and the README target table.
5. Run the full cross-version checklist (TESTING §4).

## 7. Facts and assumptions register

The implementer **must verify** these items against official sources at M0, before writing adapter code, and record the outcome (✓ or a correction) in this table in the M0 PR.

Sources:
- Fabric's developer docs and blog (docs.fabricmc.net, fabricmc.net/develop, fabricmc.net/blog)
- Mojang's Java Edition changelogs and snapshot posts (minecraft.net)
- The Fabric Loom documentation and changelog

| # | Statement | Confidence | Used by |
|---|---|---|---|
| V1 | 1.21.11 is the final 1.21.x release; the next release line is 26.1 (year-based versioning) | High | Target matrix |
| V2 | 26.1+ Java Edition ships without obfuscation; Fabric uses Mojang names for 26.1+ and does not publish Yarn for it | High | §2 |
| V3 | 26.1+ requires Java 25 | High | Toolchains |
| V4 | Loom provides separate plugin ids for remapping (obfuscated targets) and non-remapping (unobfuscated targets) modes | Medium – verify exact ids and the minimum Loom version | Build |
| V5 | Since 1.21.6, GUI drawing is deferred via a GUI render state with element objects, and 2D poses use a `Matrix3x2f` stack | High | RenderBackend |
| V6 | Since 1.21.9, screen input callbacks use record-based event types, and key-mapping categories are objects | High – verify exact names | Input adapter |
| V7 | Fabric API offers a HUD element registry (1.21.6+) | High – verify the 26.x package names | HUD hook |
| V8 | Fabric API offers client gametests with screenshot support (1.21.4+) | High – verify availability on 26.2 | TESTING §3 |
| V9 | 26.2 introduces a Vulkan rendering backend alongside OpenGL behind Blaze3D | Per the product brief – verify how the backend is selected, and whether derived pipelines with vanilla shaders work on Vulkan | §4 |
| V10 | LWJGL `stb` (STBImage, STBTruetype) and `tinyfd` are on the Minecraft runtime classpath for all targets | High for stb; **medium for tinyfd** (the fallback is already designed) | Text, images, dialogs |
| V11 | Ordering of GUI elements of different pipelines within one stratum | Unknown – must be tested (ARCHITECTURE §5.9) | RenderBackend |
| V12 | Mojang-mapped resource identifier class name per target | Verify | Adapter only |
| V13 | Vanilla menu blur radius can or cannot be set per call | Verify per target | World blur |

## 8. Distribution

- **Release artefacts per version tag:** three JARs, each named with its target suffix, plus SHA-256 checksums.
- **Store metadata** (Modrinth or CurseForge, when used): each JAR is uploaded with exactly the game versions in its `fabric.mod.json` range and loader `fabric`. Never mark a JAR compatible with other targets.
- **Runtime guard:** Fabric Loader enforces the dependency ranges. Ooze also logs its target at start-up, and the About page shows it.
