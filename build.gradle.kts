// Ooze Client — root build. Plugin versions are declared once here (apply false) and applied
// by the projects that need them. Pinned versions live in gradle.properties files; see
// docs/TARGET_PLATFORM.md for how they were chosen.

plugins {
    // Remapping Loom: 1.21.11 is obfuscated at runtime; sources use Mojang official names.
    // 1.17.21 is the newest Loom release whose Gradle plugin runs on JDK 21 (1.18+ needs JDK 25).
    id("net.fabricmc.fabric-loom-remap") version "1.17.21" apply false
}

tasks.register("buildAll") {
    group = "build"
    description = "Builds every distribution JAR into build/dist/."
    dependsOn(":versions:mc1_21_11:collectDist")
}

// ---------------------------------------------------------------------------------------------
// Integration glue for Sol's :bridge-java (owned by Sol; see docs/bridge/BUILD_AND_TEST.md).
// The prebuilt native resources (Windows x64 DLL + manifest) come from Sol's G0 artifact via
// -PoozeNativeResources=<absolute dir>, defaulting to native/dist/g0/release/resources.
// Without them (e.g. Linux/Java-only contributors) the mod still builds; the bridge then reports
// a missing/unsupported native library at runtime and Ooze stays in vanilla-only mode.
// Sol's bridge-java skips its DLL harness on non-Windows hosts itself (onlyIf). Still requested
// (docs/handoffs/CLAUDE-C0.md §5, S-2): a module switch for building without resources, so this
// remaining glue can be removed.
// ---------------------------------------------------------------------------------------------
val nativeResourcesDir: File = providers.gradleProperty("oozeNativeResources")
    .map { file(it) }
    .getOrElse(file("native/dist/g0/release/resources"))
val nativeResourcesPresent = nativeResourcesDir.resolve("META-INF/ooze/native.properties").isFile
    && nativeResourcesDir.resolve("natives/windows-x86_64/ooze_jni.dll").isFile
val windowsX64Host = System.getProperty("os.name").startsWith("Windows")
    && System.getProperty("os.arch").lowercase() in setOf("amd64", "x86_64")

findProject(":bridge-java")?.let { bridge ->
    bridge.afterEvaluate {
        if (!nativeResourcesPresent) {
            logger.lifecycle(
                "Ooze: no prebuilt native resources at $nativeResourcesDir — building the mod WITHOUT the " +
                    "Windows native library (runtime: vanilla-only mode). Pass -PoozeNativeResources=<dir>.")
            tasks.named("verifyNativeArtifact") { enabled = false }
        }
        if (!nativeResourcesPresent && windowsX64Host) {
            // Sol's harness runs on Windows x64 (its own onlyIf) but needs the DLL resources.
            tasks.named("bridgeContractTest") { enabled = false }
            logger.lifecycle("Ooze: skipping :bridge-java:bridgeContractTest (no native resources supplied).")
        }
    }
}
