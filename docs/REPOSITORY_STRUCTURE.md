# Ooze Client — Repository Structure

> Deliverable 11. The target layout of the repository once implementation begins (milestone M0 creates the skeleton).

## 1. Tree

```
Ooze-Client/
├── README.md                         Project overview + docs index
├── AGENTS.md                         Rules for coding agents (Codex etc.)
├── LICENSE                           MIT (Ooze code)
├── CHANGELOG.md                      Human changelog (source for assets/…/changelog.json via build task)
├── settings.gradle.kts               Includes subprojects; declares plugin versions once (apply false)
├── build.gradle.kts                  Root: shared config, aggregate tasks (buildAll, testAll, runClient<Target>)
├── gradle.properties                 mod_version, maven_group, org.gradle.jvmargs …
├── gradle/
│   ├── libs.versions.toml            Version catalog: loom, junit, assertj, archunit, gson, slf4j, fastutil, lwjgl (compileOnly)
│   └── wrapper/                      Gradle wrapper (pinned)
├── build-logic/                      Included build with convention plugins
│   └── src/main/kotlin/
│       ├── ooze.java-library.gradle.kts      Java 21 release, javadoc lint, Spotless, tests
│       ├── ooze.fabric-target.gradle.kts     Shared Loom config for versions/* (run configs, jar naming, resources)
│       └── ooze.assets.gradle.kts            iconCompile, changelogJson, blueNoise, license copy tasks
├── core/                             Pure Java 21 — no Minecraft (ARCHITECTURE §2)
│   ├── build.gradle.kts
│   └── src/
│       ├── main/java/io/github/kyluua/ooze/core/…      packages per ARCHITECTURE §2.1
│       ├── main/resources/assets/oozeclient/
│       │   ├── themes/        ooze_classic.theme.json, midnight_gel…, mint_melt…, bubblegum_pop…, high_contrast…
│       │   ├── fonts/         fonts.json + family folders with TTF + OFL.txt (TYPOGRAPHY §2.1)
│       │   ├── icons/         icons.json (generated)
│       │   ├── lang/          en_us.json (source of truth)
│       │   ├── textures/      blue_noise_64.png (generated)
│       │   ├── changelog.json, credits.json
│       └── test/java/…        unit + golden tests; test/resources/golden/*.png
├── platform-lwjgl/                   stb_truetype rasterizer, STBImage decoder, TinyFileDialogs
│   └── src/main/java/io/github/kyluua/ooze/platform/lwjgl/…
├── mc-shared/                        Integration source compiled into EVERY target (not a standalone project)
│   └── src/main/java/io/github/kyluua/ooze/mc/…          OozeScreenHost, shared adapters, HUD hook glue
├── versions/
│   ├── mc1_21_11/
│   │   ├── build.gradle.kts          Loom (remapping plugin id), officialMojangMappings, Java 21
│   │   ├── gradle.properties         minecraft_version=1.21.11, loader_version, fabric_api_version, java=21
│   │   └── src/main/
│   │       ├── java/io/github/kyluua/ooze/mc/v1_21_11/…   entrypoint, render backend, input, mixins, platform impls
│   │       └── resources/
│   │           ├── fabric.mod.json   depends: minecraft "~1.21.11", java ">=21", fabricloader, fabric-api
│   │           └── oozeclient.mixins.json
│   ├── mc26_1/                       Loom (non-remapping plugin id), Java 25; minecraft "~26.1"
│   └── mc26_2/                       Loom (non-remapping), Java 25; minecraft "~26.2"; Vulkan + OpenGL verification
├── assets-src/                       Design sources compiled by build-logic tasks
│   ├── icons/*.svg                   Lucide subset (ISC) + Ooze brand icons
│   └── LICENSES.md                   Source/licence of every asset
├── licenses/                         Third-party licence texts copied into every JAR root
│   ├── fonts/*-OFL.txt
│   └── lucide-ISC.txt
├── docs/                             The specification (this folder)
│   └── tokens/ooze_classic.theme.json
├── tools/
│   └── screenshots/                  Golden-image compare script for client gametests (TESTING §3)
└── .github/
    ├── workflows/build.yml           Matrix: build + unit tests for all targets; client gametests (headless) where supported
    ├── workflows/release.yml         Tag → build all targets → attach JARs to the GitHub release
    └── pull_request_template.md
```

## 2. How the sources combine

| Gradle project | Compiles | Packaged as |
|---|---|---|
| `:core` | `core/src/main/java` (Java 21 `--release 21`) | Classes included in every target JAR |
| `:platform-lwjgl` | `platform-lwjgl/src` (Java 21) | Classes included in every target JAR |
| `:versions:mc1_21_11` | its own `src` + `mc-shared/src/main/java` (as an extra source dir) | `ooze-client-<ver>+mc1.21.11.jar` |
| `:versions:mc26_1` | its own `src` + `mc-shared` | `ooze-client-<ver>+mc26.1.jar` |
| `:versions:mc26_2` | its own `src` + `mc-shared` | `ooze-client-<ver>+mc26.2.jar` |

- `core` and `platform-lwjgl` classes are bundled into each target JAR **directly**: `jar { from(project(":core").sourceSets.main.output) }` plus resources. Jar-in-jar is not used. There is exactly one Ooze mod id per JAR and no nested-mod metadata.
- `mc-shared` is a **source directory, not a compiled project**. Each target compiles it against its own Minecraft version. If shared code does not compile on one target, move the differing call behind an adapter interface in `versions/*`. Never add version branches inside `mc-shared`.

## 3. Conventions

| Topic | Convention |
|---|---|
| Packages | `io.github.kyluua.ooze.core.*`, `….platform.lwjgl.*`, `….mc.*` (shared), `….mc.v1_21_11.*`, `….mc.v26_1.*`, `….mc.v26_2.*` |
| Formatting | Spotless with palantir-java-format; `./gradlew spotlessApply` before commit. CI checks it |
| Nullness | JSpecify annotations (`@Nullable`); everything else is non-null by default |
| Naming | Components `PascalCase` with no `Ooze` prefix (`Button`, `Toggle`); screens `…View` in core and `…Screen` in versions; tokens via enums (`ThemeToken.SURFACE_BASE`) |
| Tests | JUnit 5 + AssertJ in `core`; ArchUnit rule `core_must_not_depend_on_minecraft`; golden images under `core/src/test/resources/golden/` |
| Commits | Conventional commits (`feat(ui): …`, `fix(render): …`); one milestone per PR (IMPLEMENTATION_PLAN) |
| Generated assets | Produced only by build-logic tasks. Generated files are committed only when listed in `assets-src/LICENSES.md` |
| Versioning | SemVer for the mod (`0.1.0`). Jar suffix `+mc<target>` |

## 4. Gradle tasks (contract)

| Task | Purpose |
|---|---|
| `./gradlew build` | Compiles all projects; runs unit tests; builds all three JARs |
| `./gradlew :core:test` | Core unit + golden tests (no Minecraft) |
| `./gradlew :versions:mc26_1:runClient` | Dev client for a target (likewise `mc1_21_11`, `mc26_2`) |
| `./gradlew :versions:<t>:runClientGametest` | Fabric client gametests incl. screenshot goldens (TESTING §3) |
| `./gradlew iconCompile` | SVG → `icons.json` |
| `./gradlew spotlessCheck` / `spotlessApply` | Format |
| `./gradlew buildAll` | Alias that builds every distribution JAR into `build/dist/` |
