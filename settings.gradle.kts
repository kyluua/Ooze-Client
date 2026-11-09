// Ooze Client — Gradle settings.
// Target: Minecraft Java Edition 1.21.11 on Fabric, Java 21 (docs/TARGET_PLATFORM.md).
// Sol (GPT-6.1 Sol) owns `bridge-java` and the Cargo workspace under `native/`; they are
// included here only once their handoff lands (see docs/handoffs/).

pluginManagement {
    repositories {
        maven {
            name = "Fabric"
            url = uri("https://maven.fabricmc.net/")
        }
        mavenCentral()
        gradlePluginPortal()
    }
}

rootProject.name = "ooze-client"

include(":versions:mc1_21_11")

// Sol-owned Java bridge. Included automatically once its directory exists (G0 handoff).
if (file("bridge-java/build.gradle.kts").exists()) {
    include(":bridge-java")
}
