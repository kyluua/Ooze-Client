# Ooze Java/Rust bridge

Java 21, no Minecraft/Fabric dependencies. Read [the ABI](../docs/bridge/ABI_V1.md) and [build/test instructions](../docs/bridge/BUILD_AND_TEST.md).

The public entry point is `NativeLoader.tryLoad(cacheRoot)`. Its `LoadResult` must be checked before creating an `EngineSession`; failure means the Fabric adapter retains vanilla functionality. Use its typed `state()` (LOADED, MISSING_LIBRARY, UNSUPPORTED_PLATFORM, INCOMPATIBLE, LOAD_FAILED), `bridgeVersion()` and `protocolVersion()` in the connector; do not parse English diagnostics for state. Diagnostics are internal log text. Product toasts/controls must use localized text supplied by Claude's adapter.

An `InputBatch` can be reused with `clear()`, then submitted once per input batch. `EngineSession.renderFixture(frameId, FrameBuffer)` produces G0 developer geometry and returns a frame lease. Close it only after the actual deferred consumer has copied/read its vertices. Engine shutdown does not invalidate live Java-owned frames.

The bridge JAR is a plain Java library, **not a Fabric mod**. For C0, include its classes and resources once inside the final Ooze mod, or wire the module's outputs into the version JAR. Do not create another mod ID, duplicate these classes across loaders, relocate `NativeBindings`, or distribute the fault-injection artifact.

The standalone Gradle `check` task executes the real Java 21 main harness. Module source can also be compiled directly by native/tools/build-g0.ps1; no root Gradle/Fabric build is required for G0.
