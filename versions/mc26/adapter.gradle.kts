// Ooze Client — Fabric adapter for Minecraft 26.x (Java 25, Mojang names, not remapped).
// Owner: Claude (Fabric integration). One script for every 26.x family project
// (versions/mc26_1, mc26_2, mc26_3; settings.gradle.kts points their buildFileName here).
// Each project holds only its pins (gradle.properties) and, where a Minecraft API differs, its own
// classes in <project>/src/main/java. Shared 26.x classes live in versions/mc26/src.
// Engine/UI logic belongs in Rust (native/), reached only through Sol's bridge-java API.

import groovy.json.JsonOutput
import java.security.MessageDigest

plugins {
    id("net.fabricmc.fabric-loom")
}

val shared = rootProject.file("versions/mc26")

val family = project.property("minecraft_family") as String
val compileMinecraft = project.property("minecraft_version") as String
val compatible = (project.property("minecraft_compatible") as String).split(",").map { it.trim() }
val loaderVersion = project.property("loader_version") as String
val fabricApiVersion = project.property("fabric_api_version") as String
val javaVersion = (project.property("java_version") as String).toInt()
val modVersion = project.property("mod_version") as String

// -PoozeMinecraft=<version> reconfigures this project for another version the JAR lists, so the
// production launch test can run the shipped JAR (-PoozeProductionJar) on every listed version.
// The property is build-wide: the other 26.x projects keep their compile version, and this
// project's production run refuses a version it does not list.
val requestedMinecraft: String? = providers.gradleProperty("oozeMinecraft").orNull
val targetMinecraft = requestedMinecraft?.takeIf { it in compatible } ?: compileMinecraft
require(compileMinecraft in compatible) { "minecraft_version must be one of minecraft_compatible" }

group = project.property("maven_group") as String
version = "$modVersion+mc$family"

base {
    archivesName = "ooze-client"
}

java {
    toolchain {
        languageVersion = JavaLanguageVersion.of(javaVersion)
    }
}

tasks.withType<JavaCompile>().configureEach {
    options.release = javaVersion
    options.encoding = "UTF-8"
}

sourceSets {
    named("main") {
        java.srcDir(shared.resolve("src/main/java"))
        resources.srcDir(shared.resolve("src/main/resources"))
    }
}

loom {
    runs {
        named("client") {
            // Developer-only switches. -Doozeclient.dev=true is implied in the dev environment.
            providers.gradleProperty("oozeEngineFault").orNull?.let { vmArg("-Doozeclient.dev.engineFault=$it") }
        }
    }
}

fabricApi {
    configureTests {
        createSourceSet = true
        modId = "oozeclient-gametest"
        enableGameTests = false
        enableClientGameTests = true
        // Launch tests stop at the title screen and never start a world, so no EULA is accepted.
        eula = false
    }
}

sourceSets {
    named("gametest") {
        java.srcDir(shared.resolve("src/gametest/java"))
        resources.srcDir(shared.resolve("src/gametest/resources"))
    }
}

afterEvaluate {
    val gameTestRun = loom.runs.findByName("clientGameTest")
        ?: error("Loom did not create the 'clientGameTest' run; check fabricApi.configureTests")
    val fault = providers.gradleProperty("oozeEngineFault").orNull
    if (fault != null) {
        gameTestRun.vmArg("-Doozeclient.dev.engineFault=$fault")
        gameTestRun.vmArg("-Doozeclient.test.expectEngineState=${fault.uppercase().replace('-', '_')}")
    } else {
        providers.gradleProperty("oozeExpectEngineState").orNull?.let { gameTestRun.vmArg("-Doozeclient.test.expectEngineState=$it") }
    }
}

repositories {
    mavenCentral()
}

dependencies {
    minecraft("com.mojang:minecraft:$targetMinecraft")
    implementation("net.fabricmc:fabric-loader:$loaderVersion")
    implementation("net.fabricmc.fabric-api:fabric-api:$fabricApiVersion")
    compileOnly("org.jspecify:jspecify:1.0.0")

    // Sol's bridge and the shared adapter classes: merged into this JAR exactly once (tasks.jar).
    // Never `include` them: Loom would nest them as separate mods with generated mod IDs.
    implementation(project(":bridge-java"))
    implementation(project(":versions:common"))
}

// The exact Minecraft dependency the JAR declares: every listed version, nothing else.
val minecraftDependency = JsonOutput.toJson(compatible)

tasks.processResources {
    val props = mapOf(
        "version" to project.version.toString(),
        "minecraft_dependency" to minecraftDependency,
        "loader_version" to loaderVersion,
        "java_version" to javaVersion.toString(),
    )
    inputs.properties(props)
    filesMatching("fabric.mod.json") {
        expand(props)
    }
}

val bridgeJar = project(":bridge-java").tasks.named<Jar>("jar")
val commonJar = project(":versions:common").tasks.named<Jar>("jar")

tasks.jar {
    from(rootProject.file("LICENSE")) {
        rename { "LICENSE_ooze-client" }
    }
    from(rootProject.file("licenses")) {
        into("licenses")
    }
    dependsOn(bridgeJar, commonJar)
    from(bridgeJar.map { zipTree(it.archiveFile) }) {
        exclude("META-INF/MANIFEST.MF")
    }
    from(commonJar.map { zipTree(it.archiveFile) }) {
        exclude("META-INF/MANIFEST.MF")
    }
    duplicatesStrategy = DuplicatesStrategy.FAIL
}

val collectDist by tasks.registering(Copy::class) {
    group = "build"
    description = "Copies the distribution JAR into build/dist/."
    from(tasks.named("jar"))
    into(rootProject.layout.buildDirectory.dir("dist"))
}

// ---------------------------------------------------------------------------------------------
// Production-mode launch test: runs the *shipped* JAR outside the dev environment, with the
// launch-test mod and the pinned Fabric API. Dev-only switches are inert here.
// -PoozeProductionJar=<abs path> runs an already-packaged JAR; -PoozeMinecraft runs it on another
// version the JAR lists (both are recorded in the log).
// ---------------------------------------------------------------------------------------------
val gametestJar by tasks.registering(Jar::class) {
    from(sourceSets["gametest"].output)
    archiveBaseName = "ooze-client-gametest"
    archiveClassifier = ""
    destinationDirectory = layout.buildDirectory.dir("gametest-libs")
}

dependencies {
    "productionRuntimeMods"("net.fabricmc.fabric-api:fabric-api:$fabricApiVersion")
}
val productionRuntimeMods = configurations.named("productionRuntimeMods")

fun sha256(file: File): String =
    MessageDigest.getInstance("SHA-256").digest(file.readBytes()).joinToString("") { "%02x".format(it) }

tasks.register<net.fabricmc.loom.task.prod.ClientProductionRunTask>("runProductionClientGameTest") {
    group = "verification"
    description = "Launches the distribution JAR in a production Fabric environment and runs the launch test."
    jvmArgs.add("-Dfabric.client.gametest")
    providers.gradleProperty("oozeExpectEngineState").orNull?.let { jvmArgs.add("-Doozeclient.test.expectEngineState=$it") }
    // Passing a fault here proves release builds IGNORE developer fault injection.
    providers.gradleProperty("oozeEngineFault").orNull?.let { jvmArgs.add("-Doozeclient.dev.engineFault=$it") }
    val packagedJar = providers.gradleProperty("oozeProductionJar").orNull
    val jar = if (packagedJar != null) file(packagedJar) else null
    if (jar != null) {
        require(jar.isFile) { "oozeProductionJar does not exist: $packagedJar" }
        inputs.file(jar)
        mods.from(jar, gametestJar, productionRuntimeMods)
    } else {
        mods.from(tasks.named("jar"), gametestJar, productionRuntimeMods)
    }
    val tested = targetMinecraft
    val shipped = tasks.named<Jar>("jar").flatMap { it.archiveFile }
    val requested = requestedMinecraft
    doFirst {
        require(requested == null || requested == tested) {
            "Minecraft $requested is not listed for the $family adapter (minecraft_compatible=$compatible)"
        }
        val modJar = jar ?: shipped.get().asFile
        logger.lifecycle("Production run: Minecraft {} with mod JAR {} (sha256 {})", tested, modJar.name, sha256(modJar))
    }
    runDir = layout.buildDirectory.dir("run/productionClientGameTest-$targetMinecraft")
}

// ---------------------------------------------------------------------------------------------
// Launcher profile manifest: the exact Fabric profile per listed Minecraft version, for Sol's
// installer (docs/handoffs/CLAUDE-MV-request.md). Minecraft itself is never bundled.
// ---------------------------------------------------------------------------------------------
val fabricApiArtifact = configurations.detachedConfiguration(
    dependencies.create("net.fabricmc.fabric-api:fabric-api:$fabricApiVersion")
).apply { isTransitive = false }

val launcherProfile = tasks.register("launcherProfile") {
    group = "build"
    description = "Writes build/dist/ooze-launcher-profile-mc<version>.json for every listed Minecraft version."
    val jarTask = tasks.named<Jar>("jar")
    dependsOn(jarTask)
    inputs.files(fabricApiArtifact)
    inputs.file(jarTask.flatMap { it.archiveFile })
    val dist = rootProject.layout.buildDirectory.dir("dist")
    outputs.files(compatible.map { v -> dist.map { it.file("ooze-launcher-profile-mc$v.json") } })
    doLast {
        val modJar = jarTask.get().archiveFile.get().asFile
        val apiJar = fabricApiArtifact.singleFile
        val apiPath = "net/fabricmc/fabric-api/fabric-api/$fabricApiVersion/fabric-api-$fabricApiVersion.jar"
        // One profile per Minecraft version, in the same oozelauncher.profile/1 shape as 1.21.11's:
        // the same JAR and Fabric API serve every version of the family.
        for (minecraft in compatible) {
            val profile = linkedMapOf(
                "schema" to "oozelauncher.profile/1",
                "minecraft" to minecraft,
                "java" to mapOf("major" to javaVersion),
                "fabric" to linkedMapOf(
                    "loader" to loaderVersion,
                    "profileJson" to "https://meta.fabricmc.net/v2/versions/loader/$minecraft/$loaderVersion/profile/json",
                ),
                "mods" to listOf(
                    linkedMapOf(
                        "id" to "fabric-api",
                        "version" to fabricApiVersion,
                        "file" to apiJar.name,
                        "url" to "https://maven.fabricmc.net/$apiPath",
                        "size" to apiJar.length(),
                        "sha256" to sha256(apiJar),
                    ),
                    linkedMapOf(
                        "id" to "oozeclient",
                        "version" to project.version.toString(),
                        "file" to modJar.name,
                        "bundled" to true,
                        "size" to modJar.length(),
                        "sha256" to sha256(modJar),
                    ),
                ),
                "readyMarker" to "OOZE_READY v1",
            )
            val file = dist.get().file("ooze-launcher-profile-mc$minecraft.json").asFile
            file.parentFile.mkdirs()
            file.writeText(JsonOutput.prettyPrint(JsonOutput.toJson(profile)) + "\n")
            logger.lifecycle("Ooze: wrote ${file.relativeTo(rootProject.projectDir)}")
        }
    }
}

collectDist.configure { finalizedBy(launcherProfile) }
