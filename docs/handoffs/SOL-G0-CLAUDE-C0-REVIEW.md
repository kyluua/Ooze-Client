# Sol review of Claude C0

Reviewed Claude implementation `3064b19e0fe2c8e5cb0df2e6766633c700b7eb16` and handoff `0dca252d06e0b6486a35c0dc5322cf4e0e07a7e8`, before integrating them into Sol's working branch. Initial G0 was already headless-tested and published; this review reconciles the concurrent handoffs.

## Findings and requested C0b changes

1. **Packaging decision required:** `versions/mc1_21_11/build.gradle.kts` currently uses both `implementation(project(":bridge-java"))` and Loom `include(project(":bridge-java"))`. A combined Windows build succeeded, and inspection confirmed the final mod contains `META-INF/jars/bridge-java-0.1.0-g0.jar` with synthetic Fabric ID `io_github_kyluua_ooze_bridge-java`. G0 deliberately supplies a plain Java library, not another Fabric mod. Choose direct inclusion of its classes/resources exactly once if retaining the one-mod-ID design (the G0 recommendation). No duplicate-classloader fault was observed; this is a packaging contract decision requiring C0b's final-JAR verification. Claude owns that build-file change.
2. **Native statuses must stay typed:** the C0 handoff asks for LOADED/MISSING_LIBRARY/UNSUPPORTED_PLATFORM/INCOMPATIBLE/LOAD_FAILED. Sol has added these to NativeLoader.LoadResult, plus bridgeVersion/protocolVersion, during this G0 compatibility follow-up. C0b can map them directly; do not infer states by matching diagnostic strings.
3. **Unsupported-host tests must be explicit:** the bridge's original Gradle check would attempt a Windows DLL on Claude's Linux host. Sol has restricted bridgeContractTest to Windows x64 and documented SKIPPED versus supported/tested. Other hosts still require supplied prebuilt resources for packaging. This does not add a Linux native engine.
4. **Do not swallow VM-fatal errors:** `EngineConnectors.connectSafely` currently catches all Throwable, including VM errors and ThreadDeath. C0b should catch anticipated load/bridge/runtime failures and linkage errors, and preserve fatal termination semantics. The C0 request that any native fault leaves the JVM alive must be narrowed: controlled status errors and caught unwinding panics can fail soft; native memory faults and aborts cannot be recovered in-process by Java try/catch.
5. **Update the engine gate after a runtime bridge failure:** DeveloperProbe logs/displays an unsuccessful result, but the static engine status/gate remains loaded. When C0b adds a real connector, a controlled unrecoverable session error should disable further Ooze execution and clean up leases/session state, rather than leaving a loaded gate that continues to submit work. Claude owns that adapter policy.

## Confirmed useful integration points

- Root settings auto-detect the Sol-owned Gradle module; no root-file edit is needed just to include it.
- The typed EngineConnector seam does not invent another native interface, and the unloaded default exposes no dead developer controls.
- Claude's C0 handoff distinguishes its Linux game-launch evidence, fault-injected statuses and missing real-DLL coverage. Windows C0b remains necessary.
- The rewritten product instructions preserve Rust ownership, vanilla rendering, tokenization, accessibility and launcher-owned authentication.

The optional resource/metrics/action protocols, byte-budget accountants, configuration backup algorithms and product rendering requested in the C0 handoff remain future G milestones. G0's ABI document explicitly labels those as reserved. This review does not implement C0b or edit Claude's owned adapter/product files.

Combined verification on Windows x64: `gradlew.bat build buildAll` passed on Java 21/Gradle 9.8.2, including all 21 Claude adapter unit tests and the 20,323-assertion G0 bridge harness. Loom/Claude's root build prints its already documented deprecation warnings. This is Built/Headless-tested evidence, not a Minecraft game launch or completion of the still-unwired native connector. The CI workflow now also runs this combined build after producing native resources.
