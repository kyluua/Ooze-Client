# AGENTS.md: Instructions for coding agents

This repository is **Ooze Client**, a Fabric client for Minecraft Java Edition (targets 1.21.11, 26.1.x and 26.2.x). The specification in `docs/` is the source of truth. Implement it milestone by milestone.

## Read first (in this order)

1. `docs/PRODUCT_VISION.md`: principles (especially *Real or absent* and *Tokens, not literals*).
2. `docs/IMPLEMENTATION_PLAN.md`: find the milestone you are working on. Do **only** that milestone.
3. `docs/ARCHITECTURE.md` and `docs/REPOSITORY_STRUCTURE.md`: where the code goes.
4. The topic docs your milestone references: `DESIGN_SYSTEM.md`, `TYPOGRAPHY.md`, `MOTION_SPEC.md`, `BACKGROUNDS.md`, `COMPONENTS.md`, `SCREENS.md`, `INTERACTION_FLOWS.md`, `MULTI_VERSION.md`, `PERFORMANCE_AND_ACCESSIBILITY.md`, `TESTING.md`.

## Hard rules

- **`core` never imports Minecraft, Mojang, Fabric or LWJGL classes.** An ArchUnit test enforces this.
- **No direct graphics API calls**: no `GL*`, `GlStateManager`, Vulkan or NanoVG. Render only through `RenderBackend` and vanilla GUI pipelines (ARCHITECTURE §5.8, ADR-001/002).
- **Mojang official names in all sources**, including the 1.21.11 target (ADR-004).
- **Every target must compile** at the end of every change: `./gradlew build`.
- **No hard-coded colours, sizes, radii, durations or easings** in components. Use `ThemeToken`, `SpaceToken`, `SizeToken` and `MotionToken`.
- **Animations are time-based** through `FrameClock` / `Animator` / `Motion.resolve`. Never use per-frame increments. Always honour Reduced motion and Off.
- **Every user-facing string** goes in `assets/oozeclient/lang/en_us.json`.
- **No fake controls.** Anything visible must work. Sample modules, widgets and data must carry the **Sample** badge.
- **No gameplay modules** beyond the samples named in IMPLEMENTATION_PLAN M9 and M12.
- **Fonts:** ship upstream static TTFs **unmodified** together with their `OFL.txt`. Never subset or rename them.
- **Config writes** go through `ConfigStore` (atomic, debounced). Never write files from the render thread outside the bounded flushes.
- **Optional features must fail soft.** Use `require = 0` mixins plus a runtime check, never a crash.

## Commands

```bash
./gradlew build                                  # all targets + unit/golden/ArchUnit tests
./gradlew :core:test                             # fast core tests
./gradlew :versions:mc26_1:runClient             # reference target dev client
./gradlew :versions:mc26_1:runClientGametest     # client gametests + screenshot goldens
./gradlew spotlessApply                          # format before committing
./gradlew :core:test -Pgolden.update=true        # regenerate goldens (explain every change in the PR)
```

## Definition of Done

See `docs/TESTING.md §10`. In the PR description:
- list the milestone's acceptance criteria as a checklist, with evidence (tests, screenshots at 1080p, 1440p and 4K, profiler tables);
- list any spec deviations, and update the affected doc in the same PR.

## Style

- Java 21 language level in `core` and `platform-lwjgl`; Java 25 is allowed in the 26.x adapters.
- palantir-java-format via Spotless; JSpecify `@Nullable`.
- Prefer small, final classes and records.
- No reflection in hot paths. No allocation in per-frame paint paths (use reusable arrays and indexed loops).
- Conventional commit messages (`feat(render): …`).
