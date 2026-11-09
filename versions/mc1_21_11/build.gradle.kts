// Ooze Client — Fabric adapter for Minecraft 1.21.11 (Java 21, Mojang official names, remapped).
// Owner: Claude (Fabric integration). Engine/UI logic belongs in Rust (native/), reached only
// through Sol's bridge-java API. See docs/ARCHITECTURE.md §2 and AGENTS.md.

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
    providers.gradleProperty("oozeEngineFault").orNull?.let { fault ->
        gameTestRun.vmArg("-Doozeclient.dev.engineFault=$fault")
        gameTestRun.vmArg("-Doozeclient.test.expectEngineState=${fault.uppercase().replace('-', '_')}")
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

    // Sol's bridge (G0). Present only after the handoff lands; see settings.gradle.kts.
    if (findProject(":bridge-java") != null) {
        implementation(project(":bridge-java"))
        include(project(":bridge-java"))
    }

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

tasks.jar {
    from(rootProject.file("LICENSE")) {
        rename { "LICENSE_ooze-client" }
    }
    from(rootProject.file("licenses")) {
        into("licenses")
    }
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
    mods.from(tasks.named("remapJar"), remapGametestJar, productionRuntimeMods)
    runDir = layout.buildDirectory.dir("run/productionClientGameTest")
}
