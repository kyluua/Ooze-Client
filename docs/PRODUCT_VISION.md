# Ooze Client — Product Vision & Design Philosophy

> Deliverable 1 of 15. Read this first: every other document assumes these principles.

## 1. Vision

**Ooze Client** is a Minecraft Java Edition client for Fabric. Its interface looks and moves like a premium, living material: a smooth, glossy, elastic white substance with a confident ink outline.

People should recognise an Ooze screenshot at a glance and enjoy every interaction. Under the visuals sits an engineering foundation that can grow into a full-featured client without the UI being rewritten.

**One-line identity:** *White gel, ink outline, glossy light.*

## 2. The material metaphor

Every surface in Ooze is made of one of two substances:

| Substance | Look | Used for |
|---|---|---|
| **White Ooze** | Pearl-to-frost white gel with a soft vertical gradient, a gloss band, a cool inner shade and an ink outline | Panels, cards, buttons, fields, the logo plate |
| **Ink Gel** | Deep ink-coloured gel with a faint gloss (or the user's accent colour) | Things that are active or selected: primary buttons, toggle tracks when on, slider fills, active indicators, tooltips |

The two substances obey the same physics. They inflate when hovered, squash when pressed, wobble back into shape when released, flow between positions instead of jumping, and drip slowly from the bottom edges of large surfaces. Users learn the rules once and every control in the client follows them.

## 3. Design pillars

1. **Material honesty.** Every surface is the same gel and behaves the same way. There are no flat rectangles, no generic gradients and no mixing of unrelated styles.
2. **Calm by default, playful on contact.** Ambient motion is slow and quiet. The fun comes from responding to the user (inflation, squash, flow, ripple), never from things moving on their own and competing for attention.
3. **Crisp at every density.** Curves, outlines, icons and text are generated for the current physical pixel grid. They are equally sharp at 1080p, 1440p and 4K. 4K quality comes from vector rendering, not from 4K images.
4. **Everything customisable, nothing fragile.** The defaults are excellent. Colours, fonts, backgrounds and motion can all be changed, but guard rails such as contrast checks, the legibility guard and minimum sizes stop customisation from making the UI unreadable.
5. **Real or absent.** Every visible control works. Sample content is labelled **Sample**. If a feature is not built yet, it does not appear in the UI.
6. **Performance is a feature.** Effects degrade gracefully. The UI never meaningfully reduces game FPS, and every expensive effect has a cheaper tier.
7. **Keyboard, mouse and reduced motion are all first-class.** Everything works without a mouse, and everything works with motion turned off.

## 4. Personality

| Is | Is not |
|---|---|
| Soft, glossy, confident | Childish, cartoon-cluttered |
| Playful in motion, precise in layout | Chaotic, bouncy for its own sake |
| Futuristic through material and light | Neon "gamer" RGB, sharp sci-fi chrome |
| Premium and calm | Generic flat Material/Bootstrap look |
| Original | An imitation of any existing Minecraft client's visual language |

Personality sliders, used when making judgement calls:

```
Playful  ●●●○○○○○○○  Serious      (leans playful, but restrained)
Organic  ●●●●●●●○○○  Geometric    (organic silhouettes, geometric layout grid)
Dense    ○○○○●●●●●●  Airy         (airy: generous spacing)
Loud     ○○○○○○○●●●  Quiet        (quiet surfaces, expressive interactions)
```

## 5. Who it is for

| Persona | Needs | What Ooze gives them |
|---|---|---|
| **The Aesthete** | A beautiful, cohesive client for everyday play and screenshots | A distinctive default look, a custom main menu and backgrounds |
| **The Tinkerer** | Deep customisation without breaking things | Theme Studio, Typography editor, Background studio, Motion settings, profiles, import/export |
| **The Creator** (streams, videos) | A clean, legible HUD that looks good on camera | HUD editor with snapping and safe zones, crisp text at 4K |
| **The Future Power User** | Many modules, fast access | Searchable ClickGUI, command palette, keybinds, profiles (modules arrive after v0.1) |

## 6. First-release scope: v0.1 "Gel"

**In scope.** The complete visual client framework:

- Rendering, typography, motion, theme, background, configuration, module and HUD frameworks.
- All 12 screens in [SCREENS.md](SCREENS.md), fully interactive.
- The full component library in [COMPONENTS.md](COMPONENTS.md).
- Sample modules and HUD widgets, clearly labelled **Sample**, that exercise every framework capability.
- Three distribution builds: Minecraft 1.21.11, 26.1.x and 26.2.x (OpenGL and Vulkan).

**Out of scope for v0.1.** These are deliberate non-goals:

- Gameplay modules beyond the labelled samples.
- Online services: accounts, cosmetics servers, friends, telemetry, analytics, auto-updates, remote news feeds.
- Custom UI sound assets. v0.1 uses the vanilla UI click with Ooze pitch and volume tuning.
- Features that give an unfair advantage on servers. When modules arrive, the module metadata includes fair-play classification ([ARCHITECTURE.md §8](ARCHITECTURE.md)).
- A single JAR spanning incompatible Minecraft versions.

## 7. Success criteria

| Area | Target |
|---|---|
| Recognisability | Shown cropped screenshots of three screens, testers identify them as "the same client" and describe the look with words like *glossy, gel, soft, liquid* |
| Crispness | Passes the crispness checklist ([TESTING.md §5](TESTING.md)) at 1920×1080, 2560×1440 and 3840×2160 |
| Smoothness | No dropped animation frames attributable to Ooze at 60 / 144 / 240 Hz on reference hardware. Ooze UI CPU time stays within the budgets in [PERFORMANCE_AND_ACCESSIBILITY.md](PERFORMANCE_AND_ACCESSIBILITY.md) |
| Learnability | A first-time user changes the background to a local image in under 30 seconds and finds any setting through search in under 10 seconds |
| Robustness | No settings loss under crash, kill or corrupt-file scenarios ([TESTING.md §7](TESTING.md)) |
| Accessibility | All screens are fully operable by keyboard. Reduced motion and Off modes have no broken states |

## 8. Voice and microcopy

- **Sentence case** for everything except the Section Heading role, which is uppercase by design.
- Use short, plain, friendly words: "Pick a background", not "Configure background parameters".
- Ooze flavour is a seasoning. At most one playful phrase per screen (for example the tagline "Soft on the outside."). Never put flavour text on errors, warnings or destructive confirmations.
- Name controls by outcome, for example "Pause the game while open", not "Pause flag".
- Errors say what happened and what to do next: "Couldn't read *sunset.webp*. Ooze supports PNG, JPEG, BMP, TGA and GIF."
- Destructive actions name the thing being destroyed: "Delete theme 'Mint Copy'?" with the buttons **Delete theme** and **Cancel**.
- Every user-facing string is a translation key under `assets/oozeclient/lang/`. `en_us.json` is the source of truth.

## 9. Document map

| # | Deliverable | Document |
|---|---|---|
| 1 | Product vision and design philosophy | `docs/PRODUCT_VISION.md` (this file) |
| 2–3 | Visual identity, colour and theme tokens | [DESIGN_SYSTEM.md](DESIGN_SYSTEM.md), [tokens/ooze_classic.theme.json](tokens/ooze_classic.theme.json) |
| 4 | Typography system | [TYPOGRAPHY.md](TYPOGRAPHY.md) |
| 5 | Animation and motion | [MOTION_SPEC.md](MOTION_SPEC.md) |
| 6 | Background customisation | [BACKGROUNDS.md](BACKGROUNDS.md) |
| 7 | Component inventory | [COMPONENTS.md](COMPONENTS.md) |
| 8 | Screen-by-screen layouts | [SCREENS.md](SCREENS.md) |
| 9 | Interaction flows | [INTERACTION_FLOWS.md](INTERACTION_FLOWS.md) |
| 10 | Java/Fabric architecture | [ARCHITECTURE.md](ARCHITECTURE.md) |
| 11 | Repository structure | [REPOSITORY_STRUCTURE.md](REPOSITORY_STRUCTURE.md) |
| 12 | Multi-version strategy | [MULTI_VERSION.md](MULTI_VERSION.md) |
| 13 | Performance (and accessibility) | [PERFORMANCE_AND_ACCESSIBILITY.md](PERFORMANCE_AND_ACCESSIBILITY.md) |
| 14 | Acceptance criteria and testing | [TESTING.md](TESTING.md) |
| 15 | Milestone plan | [IMPLEMENTATION_PLAN.md](IMPLEMENTATION_PLAN.md) |
