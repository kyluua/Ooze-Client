// Ooze Client — Gradle settings.
// Targets: Minecraft Java Edition 1.21.11 (Java 21) and 26.x (Java 25) on Fabric (docs/TARGET_PLATFORM.md).
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

plugins {
    // Toolchain auto-provisioning: Gradle runs on JDK 25 (gradle/gradle-daemon-jvm.properties) and
    // the projects compile with Java 21 (1.21.11, shared, bridge) or Java 25 (26.x). Machines that
    // lack one of them download Eclipse Temurin through the Foojay Disco API.
    id("org.gradle.toolchains.foojay-resolver-convention") version "1.0.0"
}

rootProject.name = "ooze-client"

// Minecraft-independent Java shared by every adapter (Java 21 bytecode).
include(":versions:common")
// One Fabric adapter per supported Minecraft API family (docs/TARGET_PLATFORM.md §1).
include(":versions:mc1_21_11")
// 26.x families share one build script and one source tree (versions/mc26); each project holds
// only its pins and any family-specific classes.
for (family in listOf("mc26_1", "mc26_2", "mc26_3")) {
    include(":versions:$family")
    project(":versions:$family").buildFileName = "../mc26/adapter.gradle.kts"
}

// Sol-owned Java bridge. Included automatically once its directory exists (G0 handoff).
if (file("bridge-java/build.gradle.kts").exists()) {
    include(":bridge-java")
}
