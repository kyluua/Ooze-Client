# Ooze Client

A premium, fully customisable **Minecraft Java Edition client for Fabric**. Its interface is made of a smooth, glossy, elastic white "ooze" with a crisp ink outline. It is rendered as sharp vector geometry at 1080p, 1440p and 4K, and it moves like a soft, viscous liquid.

> **Status: design and architecture phase.** This repository contains the complete product, design and engineering specification. Implementation proceeds milestone by milestone per [`docs/IMPLEMENTATION_PLAN.md`](docs/IMPLEMENTATION_PLAN.md). Coding agents: start with [`AGENTS.md`](AGENTS.md).

## Targets

| Minecraft | Java | Notes | Distribution JAR |
|---|---|---|---|
| 1.21.11 | 21 | Obfuscated runtime; Ooze sources use Mojang names | `ooze-client-<ver>+mc1.21.11.jar` |
| 26.1.x | 25 | Unobfuscated; reference target | `ooze-client-<ver>+mc26.1.jar` |
| 26.2.x | 25 | OpenGL and Vulkan backends | `ooze-client-<ver>+mc26.2.jar` |

There is one JAR per target. A single JAR does not span incompatible versions ([MULTI_VERSION.md](docs/MULTI_VERSION.md)).

## Specification

| # | Deliverable | Document |
|---|---|---|
| 1 | Product vision and design philosophy | [docs/PRODUCT_VISION.md](docs/PRODUCT_VISION.md) |
| 2–3 | Visual identity; colour and theme tokens | [docs/DESIGN_SYSTEM.md](docs/DESIGN_SYSTEM.md) · [default theme JSON](docs/tokens/ooze_classic.theme.json) |
| 4 | Typography system | [docs/TYPOGRAPHY.md](docs/TYPOGRAPHY.md) |
| 5 | Animation and motion | [docs/MOTION_SPEC.md](docs/MOTION_SPEC.md) |
| 6 | Background customisation architecture | [docs/BACKGROUNDS.md](docs/BACKGROUNDS.md) |
| 7 | Component inventory | [docs/COMPONENTS.md](docs/COMPONENTS.md) |
| 8 | Screen-by-screen layouts | [docs/SCREENS.md](docs/SCREENS.md) |
| 9 | User interaction flows | [docs/INTERACTION_FLOWS.md](docs/INTERACTION_FLOWS.md) |
| 10 | Java/Fabric architecture | [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) |
| 11 | Repository structure | [docs/REPOSITORY_STRUCTURE.md](docs/REPOSITORY_STRUCTURE.md) |
| 12 | Multi-version compatibility | [docs/MULTI_VERSION.md](docs/MULTI_VERSION.md) |
| 13 | Performance and accessibility | [docs/PERFORMANCE_AND_ACCESSIBILITY.md](docs/PERFORMANCE_AND_ACCESSIBILITY.md) |
| 14 | Acceptance criteria and testing | [docs/TESTING.md](docs/TESTING.md) |
| 15 | Milestone implementation plan | [docs/IMPLEMENTATION_PLAN.md](docs/IMPLEMENTATION_PLAN.md) |

## The look in one paragraph

Every surface is **White Ooze**: a pearl-to-frost gel with an asymmetric gloss band, a tiny glint, a cool inner shade, soft two-layer shadows and a continuous ink outline around organic superellipse corners. Active things are **Ink Gel** (or the user's accent). Controls inflate on hover, squash when pressed and wobble back. Toggle knobs stretch as they travel, and indicators flow between items like liquid. Large panels drip slowly from their bottom edge. Everything can be customised, including theme, fonts, background and motion, while guard rails keep it readable.

## Licence

Ooze Client code: [MIT](LICENSE). Bundled fonts (Bubblegum Sans, Inter, Caveat, Cormorant Garamond): SIL Open Font License 1.1, shipped unmodified with their licence texts. Icons: Lucide (ISC) plus original Ooze artwork. See [docs/TYPOGRAPHY.md §10](docs/TYPOGRAPHY.md).
