# Sol review of integrated Claude C0 for L0

Reviewed integration base `d4b77b491fc7d259d77d8e42894df5c4829f2489`, including
Claude's `ca806b7`/`9ba0477` integration and latest C0 handoff, before depending
on its game distribution in L0. No Claude L0 desktop interface diff was present
on the remote branch at implementation start; reviewing that remains due before
the owner's requested combination into main.

## Confirmed behavior

- BridgeStatusMapper uses typed NativeLoader.LoadState one-to-one. The corrected
  engine connector preserves fatal VM errors and disables the integration after
  a controlled runtime failure.
- BridgeDiagnosticScreen copies fixture vertices before closing FrameLease.
  FixtureRenderState clones both arrays and its pose, so vanilla's deferred
  vertex construction does not retain/reuse bridge memory. Sessions are created,
  submitted and closed on the Minecraft render thread. No Minecraft objects
  cross the JNI boundary and no worker submits GPU operations.
- The fixture uses vanilla RenderPipelines.GUI and degenerate quads. It does not
  create a second renderer. The result proves only this small diagnostic scene,
  not future mixed text/texture render ordering, general UI memory budgets or
  steady-state allocation performance. Diagnostic copies allocate every frame;
  that is acceptable for this developer-only gate and is not a product FPS claim.
- The mod bundles the Java bridge/resources once rather than introducing a
  second mod/classloader. No gameplay modules/player-facing shell is included.

## New executed evidence (Windows x64, 2026-11-15)

Java 21.0.12.1+1-LTS, Rust production DLL SHA-256
`79a168b0d54cdbb3111c6c2f2a9565d3208ac74640ba70de9349b3b733994fa8`.
AMD Ryzen 7 9800X3D, Radeon RX 6900 XT, driver 32.0.21036.18.

- **Built / Headless-tested:** combined Gradle build/buildAll with supplied real
  G0 resources; 20,323 bridge assertions and 26 Fabric unit tests passed.
- **Game-launched:** dev-default LOADED with successful probe and press/release
  fixture; production-default LOADED; real removed DLL MISSING_LIBRARY; actual
  byte-corrupted DLL INCOMPATIBLE; actual manifest ABI 2 INCOMPATIBLE. All five
  reached the title screen and passed the vanilla Options round trip.
- **Manually verified:** none. Automation/screenshots do not claim human signoff.

The Sol-owned PowerShell runner is
`launcher/packaging/test-c0-windows.ps1`; Git Bash is absent. Only the deliberate
missing-DLL variant skips bridge-java's build-time file-presence check so the
runtime can test the actual missing resource. Gate evidence is committed under
`docs/handoffs/evidence/SOL-L0-C0b/`. These runs use Fabric test credentials and
are not proof of a Microsoft-authenticated independent launcher session.

## Remaining requests and next task

1. **Rendering evidence gap (medium):** the automated diagnostic gate passes
   because it checks framesRendered/lastArgb copied from the native buffer, not
   framebuffer pixels. The committed released/pressed screenshots show the
   backdrop and diagnostic text but no visible white/ink rectangle. The test
   already positions the cursor at (120,90); G0 clamps geometry inside the
   framebuffer. Investigate vanilla deferred render ordering/culling/submission
   in BridgeDiagnosticScreen/FixtureRenderState and require a visible fixture
   pixel assertion. These runs prove native data/probe/lifecycle/fallback, but
   **do not close the visible-geometry part of Gate B**. Do not treat this as
   validated product rendering. Sol has not edited Claude-owned rendering files.
   Inspection of the mapped 1.21.11 bytecode with Java 21 javap found a likely
   cause: vanilla ColoredRectangleRenderState emits TL→BL→BR→TR, whereas the
   fixture emits TL→TR→BR (then a degenerate fourth vertex). GUI does not override
   the builder's default enabled culling. Inference: the fixture triangles have
   reversed winding for this GUI projection and are culled. First test reversing
   each Java adapter triangle's winding (0,2,1,1), retaining vanilla submission and
   the eager-copy lifetime. Confirm with framebuffer pixels; no Rust ABI change
   or new renderer is needed for that experiment.
2. Claude: reconcile official-launcher-only product/AGENTS text for the owner's
   new independent-launcher scope. Keep in-game JNI credential exclusion intact.
3. Consume launcher Rust API 1 from `docs/launcher/LAUNCHER_API_V1.md`, build the
   desktop UI, and publish CLAUDE-L0 with an exact result commit. No duplicate
   authentication/download/process implementation and no gameplay modules.
4. Sol: review that UI diff for credentials in state/logs, operation races,
   cancellation and honest Play/process state before integrating it.
5. Owner instruction (2026-11-15): when both pieces are ready, combine reviewed
   work into main and use main for subsequent work. Do not replace main with
   either half while the other handoff is absent. Preserve concurrent edits.

Prior S-2 optional-native build switch, reproducible native PE output and native
cache pruning remain tracked G-series follow-ups; L0 does not silently claim to
have fixed those unrelated implementation details.
