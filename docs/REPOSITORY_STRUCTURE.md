# Ooze Client — Repository Structure

> Deliverable 11. Rewritten 2026-11-09 for the 1.21.11 / Rust architecture.
> - **Owner:** C = Claude, S = Sol.
> - **State:** ✓ = exists, ○ = planned.

## 1. Tree

```
Ooze-Client/
├── README.md                     C ✓  overview, targets, docs index
├── AGENTS.md                     C ✓  rules for both agents, ownership, evidence vocabulary
├── LICENSE                       – ✓  MIT
├── .gitignore                    C ✓
├── settings.gradle.kts           C ✓  includes :versions:mc1_21_11 (+ :bridge-java when it exists)
├── build.gradle.kts              C ✓  pins Loom (remap plugin) once; buildAll → build/dist/
├── gradle.properties             C ✓  mod_version, maven_group, JVM args
├── gradlew, gradlew.bat, gradle/wrapper/   C ✓  Gradle 9.8.2, distribution SHA-256 pinned
├── versions/mc1_21_11/           C ✓  THE Fabric adapter (Java 21, Mojang names, remapped)
│   ├── build.gradle.kts, gradle.properties (pinned MC/Loader/Fabric API)
│   └── src/
│       ├── main/java/io/github/kyluua/ooze/mc/v1_21_11/   entrypoint, engine/, diag/, dev/ (C0); render/, input/, screen/, … (later)
│       ├── main/resources/fabric.mod.json, assets/oozeclient/lang/*.json
│       ├── test/java/…           headless JUnit tests (rules, gating, diagnostics)
│       └── gametest/…            Fabric client gametests (dev-only mod "oozeclient-gametest"; never shipped)
├── bridge-java/                  S ○  Java side of the bridge: native loading, status, protocol buffers (only `native` decls)
├── native/                       S ○  Cargo workspace (toolchain, lints, CI scripts)
│   ├── crates/<engine crates>    S ○  render primitives, text, animation, theme, config, settings, events, search, images
│   ├── crates/<bridge crate>     S ○  JNI exports, protocol encode/decode, bounds checks
│   └── crates/ooze-ui/           C ○  components, screens, navigation, focus, interactions, product UI state
├── assets-src/                   C ○  brand, theme tokens, fonts + OFL texts, icons (SVG), LICENSES.md
├── licenses/                     C ○  third-party licence texts copied into the JAR root
├── docs/                         C ✓  product specification (Sol owns docs/engine/ and the bridge protocol doc)
│   ├── handoffs/                 C+S ✓  CLAUDE-<milestone>.md, SOL-<milestone>.md, requests
│   ├── engine/                   S ○  engine and bridge protocol documentation
│   └── tokens/ooze_classic.theme.json   C ✓  canonical default theme
└── .github/workflows/            S ○  CI (Sol)
```

## 2. How the pieces combine

| Gradle/Cargo unit | Language | Produces | Packaged into the JAR as |
|---|---|---|---|
| `:versions:mc1_21_11` | Java 21 | The remapped Fabric mod | The JAR itself: `ooze-client-<ver>+mc1.21.11.jar` |
| `:bridge-java` (Sol) | Java 21 | Bridge classes | Jar-in-jar (`include`) or merged classes, as agreed in G0 |
| `native/` (Sol) | Rust | `ooze_native` per platform | A resource inside the JAR (path agreed in G0); extracted and loaded by bridge-java |
| `native/crates/ooze-ui` (Claude) | Rust | Linked into the native library | Part of `ooze_native` |

**Native artifacts:**
- **Windows x64** is first.
- Others are added only with a launch test (TARGET_PLATFORM §2).
- Building the native library is Sol's CI and toolchain. The Gradle build consumes the published artifact through the path defined in Sol's handoff; it does not run Cargo itself unless the G0 handoff says it should.

## 3. Conventions

| Topic | Convention |
|---|---|
| Java packages | `io.github.kyluua.ooze.mc.v1_21_11.*`; bridge-java packages are defined by Sol |
| Rust | Crate names `ooze-*`; workspace lints are Sol's |
| Commits | Conventional commits; one milestone per PR or branch; a handoff document per milestone |
| Generated assets | Produced by build tasks; committed only if listed in `assets-src/LICENSES.md` |
| Versioning | SemVer for the mod; JAR suffix `+mc1.21.11`; the bridge version is reported in the handoff and logs |

## 4. Gradle tasks (contract)

| Task | Purpose |
|---|---|
| `./gradlew build` | Compiles, runs Java unit tests, builds the remapped JAR |
| `./gradlew buildAll` | Copies the distribution JAR to `build/dist/` |
| `./gradlew :versions:mc1_21_11:runClient` | Development client (offline development account) |
| `xvfb-run -a ./gradlew :versions:mc1_21_11:runClientGameTest` | Launch tests with screenshots in `versions/mc1_21_11/build/run/clientGameTest/screenshots/` (Linux; on Windows run without `xvfb-run`) |
| `-PoozeEngineFault=<state>` on either run task | Development-only fault injection of an engine state (TARGET_PLATFORM §5) |
