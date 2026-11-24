// Ooze Client — Fabric adapter for Minecraft 1.21.11 (Java 21, Mojang official names, remapped).
// Owner: Claude (Fabric integration). Engine/UI logic belongs in Rust (native/), reached only
// through Sol's bridge-java API. See docs/ARCHITECTURE.md §2 and AGENTS.md.

import groovy.json.JsonOutput
import java.security.MessageDigest

plugins {
    id("net.fabricmc.fabric-loom-remap")
}

// Project properties: versions/mc1_21_11/gradle.properties (pins) + root gradle.properties.
val minecraftVersion = project.property("minecraft_version") as String
val loaderVersion = project.property("loader_version") as String
val fabricApiVersion = project.property("fabric_api_version") as String
val modVersion = project.property("mod_version") as String

group = project.property("maven_group") as String
version = "$modVersion+mc$minecraftVersion"

base {
    archivesName = "ooze-client"
}

java {
    toolchain {
        languageVersion = JavaLanguageVersion.of(21)
    }
}

tasks.withType<JavaCompile>().configureEach {
    options.release = 21
    options.encoding = "UTF-8"
}

loom {
    // The mod is client-only; no split source sets needed.
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
        // C0 tests stop at the title screen and never start a world, so no EULA is accepted here.
        eula = false
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
    minecraft("com.mojang:minecraft:$minecraftVersion")
    mappings(loom.officialMojangMappings())
    modImplementation("net.fabricmc:fabric-loader:$loaderVersion")
    modImplementation("net.fabricmc.fabric-api:fabric-api:$fabricApiVersion")
    compileOnly("org.jspecify:jspecify:1.0.0")

    // Sol's bridge (G0): compile + dev-runtime dependency. Its classes and resources are merged
    // into this mod's JAR exactly once (see tasks.jar). Never `include` it: Loom would wrap it as
    // a separate nested mod with its own generated mod ID, which Sol's contract forbids.
    implementation(project(":bridge-java"))

    testCompileOnly("org.jspecify:jspecify:1.0.0")
    testImplementation(platform("org.junit:junit-bom:5.11.4"))
    testImplementation("org.junit.jupiter:junit-jupiter")
    testRuntimeOnly("org.junit.platform:junit-platform-launcher")
}

tasks.test {
    useJUnitPlatform()
}

tasks.processResources {
    val props = mapOf("version" to project.version.toString(), "minecraft_version" to minecraftVersion, "loader_version" to loaderVersion)
    inputs.properties(props)
    filesMatching("fabric.mod.json") {
        expand(props)
    }
}

val bridgeJar = project(":bridge-java").tasks.named<Jar>("jar")

tasks.jar {
    from(rootProject.file("LICENSE")) {
        rename { "LICENSE_ooze-client" }
    }
    from(rootProject.file("licenses")) {
        into("licenses")
    }
    // Merge bridge-java (classes + META-INF/ooze/** + natives/**) once; NativeBindings is not
    // relocated, so its JNI symbol names stay valid.
    dependsOn(bridgeJar)
    from(bridgeJar.map { zipTree(it.archiveFile) }) {
        exclude("META-INF/MANIFEST.MF")
    }
    duplicatesStrategy = DuplicatesStrategy.FAIL
}

val collectDist by tasks.registering(Copy::class) {
    group = "build"
    description = "Copies the remapped distribution JAR into build/dist/."
    from(tasks.named("remapJar"))
    into(rootProject.layout.buildDirectory.dir("dist"))
}

// ---------------------------------------------------------------------------------------------
// Production-mode launch test: runs the *shipped* remapped JAR (intermediary names, not the dev
// environment) together with a remapped copy of the gametest mod. Dev-only switches are inert here.
// ---------------------------------------------------------------------------------------------
val gametestDevJar by tasks.registering(Jar::class) {
    from(sourceSets["gametest"].output)
    archiveBaseName = "ooze-client-gametest"
    archiveClassifier = "dev"
    destinationDirectory = layout.buildDirectory.dir("devlibs")
}

val remapGametestJar by tasks.registering(net.fabricmc.loom.task.RemapJarTask::class) {
    inputFile = gametestDevJar.flatMap { it.archiveFile }
    archiveBaseName = "ooze-client-gametest"
    archiveClassifier = ""
    destinationDirectory = layout.buildDirectory.dir("gametest-libs")
}

// Loom provides the `productionRuntimeMods` configuration (mods placed on the production run).
dependencies {
    "productionRuntimeMods"("net.fabricmc.fabric-api:fabric-api:$fabricApiVersion")
}
val productionRuntimeMods = configurations.named("productionRuntimeMods")

tasks.register<net.fabricmc.loom.task.prod.ClientProductionRunTask>("runProductionClientGameTest") {
    group = "verification"
    description = "Launches the remapped distribution JAR in a production Fabric environment and runs the launch test."
    jvmArgs.add("-Dfabric.client.gametest")
    providers.gradleProperty("oozeExpectEngineState").orNull?.let { jvmArgs.add("-Doozeclient.test.expectEngineState=$it") }
    // Passing a fault here proves release builds IGNORE developer fault injection (expect NOT_BUNDLED).
    providers.gradleProperty("oozeEngineFault").orNull?.let { jvmArgs.add("-Doozeclient.dev.engineFault=$it") }
    // -PoozeProductionJar=<abs path> runs an already-packaged mod JAR (for example the one inside a
    // verified launcher package) instead of this checkout's build, so acceptance can name the exact
    // artifact that was tested.
    val packagedJar = providers.gradleProperty("oozeProductionJar").orNull
    if (packagedJar != null) {
        val jar = file(packagedJar)
        require(jar.isFile) { "oozeProductionJar does not exist: $packagedJar" }
        inputs.file(jar)
        mods.from(jar, remapGametestJar, productionRuntimeMods)
        doFirst {
            val digest = MessageDigest.getInstance("SHA-256").digest(jar.readBytes())
                .joinToString("") { "%02x".format(it) }
            logger.lifecycle("Production run uses packaged mod JAR {} (sha256 {})", jar, digest)
        }
    } else {
        mods.from(tasks.named("remapJar"), remapGametestJar, productionRuntimeMods)
    }
    runDir = layout.buildDirectory.dir("run/productionClientGameTest")
}

// ---------------------------------------------------------------------------------------------
// Launcher profile manifest (L0): the exact Fabric profile the Ooze Launcher installs. Codex's
// installer consumes build/dist/ooze-launcher-profile.json together with the mod JAR it lists
// (docs/handoffs/CLAUDE-L0-request.md). Fabric API is downloaded by the installer from Fabric's
// Maven and checked against the SHA-256 recorded here; Minecraft itself is never bundled.
// ---------------------------------------------------------------------------------------------
val fabricApiArtifact = configurations.detachedConfiguration(
    dependencies.create("net.fabricmc.fabric-api:fabric-api:$fabricApiVersion")
).apply { isTransitive = false }

fun sha256(file: File): String =
    MessageDigest.getInstance("SHA-256").digest(file.readBytes()).joinToString("") { "%02x".format(it) }

val launcherProfile = tasks.register("launcherProfile") {
    group = "build"
    description = "Writes build/dist/ooze-launcher-profile.json (pinned profile + checksums) for the Ooze Launcher installer."
    val remapJar = tasks.named<net.fabricmc.loom.task.RemapJarTask>("remapJar")
    dependsOn(remapJar)
    inputs.files(fabricApiArtifact)
    inputs.file(remapJar.flatMap { it.archiveFile })
    val out = rootProject.layout.buildDirectory.file("dist/ooze-launcher-profile.json")
    outputs.file(out)
    doLast {
        val modJar = remapJar.get().archiveFile.get().asFile
        val apiJar = fabricApiArtifact.singleFile
        val apiPath = "net/fabricmc/fabric-api/fabric-api/$fabricApiVersion/fabric-api-$fabricApiVersion.jar"
        val profile = linkedMapOf(
            "schema" to "oozelauncher.profile/1",
            "minecraft" to minecraftVersion,
            "java" to mapOf("major" to 21),
            "fabric" to linkedMapOf(
                "loader" to loaderVersion,
                "profileJson" to "https://meta.fabricmc.net/v2/versions/loader/$minecraftVersion/$loaderVersion/profile/json",
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
        val file = out.get().asFile
        file.parentFile.mkdirs()
        file.writeText(JsonOutput.prettyPrint(JsonOutput.toJson(profile)) + "\n")
        logger.lifecycle("Ooze: wrote ${file.relativeTo(rootProject.projectDir)}")
    }
}

collectDist.configure { finalizedBy(launcherProfile) }
