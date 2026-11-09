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
