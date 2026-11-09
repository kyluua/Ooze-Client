# Ooze Client

A premium, customisable **Minecraft Java Edition 1.21.11** client delivered as a **Fabric** mod and loaded by the **official Minecraft Launcher**.

Its interface is a glossy, elastic white "ooze" with a crisp ink outline, rendered as sharp vector geometry from 1080p to 4K. It moves like a soft, viscous liquid.

> **Status: C0. A Fabric build with no native engine yet.**
> - The Fabric 1.21.11 build launches to the vanilla title screen.
> - It reports a clear "Ooze Client is off" diagnostic, because the Rust engine and bridge (Sol's G0) are not integrated yet.
> - Vanilla Minecraft stays fully usable.
>
> See [`docs/handoffs/CLAUDE-C0.md`](docs/handoffs/CLAUDE-C0.md) for what is built and tested, and [`docs/IMPLEMENTATION_PLAN.md`](docs/IMPLEMENTATION_PLAN.md) for what comes next. Coding agents: start with [`AGENTS.md`](AGENTS.md).

## Target

| Item | Value |
|---|---|
| Minecraft | Java Edition **1.21.11** |
| Loader | Fabric Loader ≥ 0.20.1, Fabric API 0.141.6+1.21.11 |
| Java | 21 (Fabric adapter) |
| Engine and UI | **Rust**, through one batched JNI bridge |
| Native platforms | **Windows x64 first**. Other platforms run vanilla with a diagnostic until supported |
| Sign-in | Official Minecraft Launcher only. Ooze has **no account system** |
| Distribution | `ooze-client-<ver>+mc1.21.11.jar` |

## Build and test

```bash
./gradlew build                     # remapped JAR + Java unit tests
./gradlew buildAll                  # → build/dist/
tools/launch-tests.sh               # real client launches (Linux: under xvfb-run) + screenshots
```

Installation for players: [`docs/INSTALL.md`](docs/INSTALL.md).

## Specification

| # | Deliverable | Document |
|---|---|---|
| 1 | Product vision and design philosophy | [docs/PRODUCT_VISION.md](docs/PRODUCT_VISION.md) |
| 2–3 | Visual identity; colour and theme tokens | [docs/DESIGN_SYSTEM.md](docs/DESIGN_SYSTEM.md) · [default theme JSON](docs/tokens/ooze_classic.theme.json) |
| 4 | Typography | [docs/TYPOGRAPHY.md](docs/TYPOGRAPHY.md) |
| 5 | Motion | [docs/MOTION_SPEC.md](docs/MOTION_SPEC.md) |
| 6 | Backgrounds | [docs/BACKGROUNDS.md](docs/BACKGROUNDS.md) |
| 7 | Components | [docs/COMPONENTS.md](docs/COMPONENTS.md) |
| 8 | Screens | [docs/SCREENS.md](docs/SCREENS.md) |
| 9 | Interaction flows | [docs/INTERACTION_FLOWS.md](docs/INTERACTION_FLOWS.md) |
| 10 | Architecture (Fabric 1.21.11 + Rust) | [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) |
| 11 | Repository structure | [docs/REPOSITORY_STRUCTURE.md](docs/REPOSITORY_STRUCTURE.md) |
| 12 | Target and platform | [docs/TARGET_PLATFORM.md](docs/TARGET_PLATFORM.md) |
| 13 | Performance and accessibility | [docs/PERFORMANCE_AND_ACCESSIBILITY.md](docs/PERFORMANCE_AND_ACCESSIBILITY.md) |
| 14 | Testing and acceptance | [docs/TESTING.md](docs/TESTING.md) |
| 15 | Milestones | [docs/IMPLEMENTATION_PLAN.md](docs/IMPLEMENTATION_PLAN.md) |
| — | Real-module backlog | [docs/FEATURE_BACKLOG.md](docs/FEATURE_BACKLOG.md) |
| — | Handoffs | [docs/handoffs/](docs/handoffs/) |

## Licence

Ooze Client code: [MIT](LICENSE). Bundled fonts (Bubblegum Sans, Inter, Caveat, Cormorant Garamond): SIL Open Font License 1.1, shipped unmodified with their licence texts. Icons: Lucide (ISC) plus original Ooze artwork. See [docs/TYPOGRAPHY.md §10](docs/TYPOGRAPHY.md).
