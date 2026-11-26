// Ooze Client — Minecraft-independent Java shared by every Fabric version adapter.
// Owner: Claude (Fabric integration). Java 21 bytecode, so the 1.21.11 adapter (Java 21) and the
// 26.x adapters (Java 25) can both merge it. It must never depend on Minecraft or Fabric: engine
// and UI logic live in Rust (native/), reached only through Sol's bridge-java.

plugins {
    `java-library`
}

group = project.property("maven_group") as String
version = project.property("mod_version") as String

java {
    toolchain {
        languageVersion = JavaLanguageVersion.of(21)
    }
}

tasks.withType<JavaCompile>().configureEach {
    options.release = 21
    options.encoding = "UTF-8"
}

repositories {
    mavenCentral()
}

dependencies {
    api(project(":bridge-java"))
    // Provided by every supported Minecraft version at runtime (same versions in 1.21.11 and 26.x).
    compileOnly("org.jspecify:jspecify:1.0.0")
    compileOnly("org.slf4j:slf4j-api:2.0.17")

    testCompileOnly("org.jspecify:jspecify:1.0.0")
    testImplementation("org.slf4j:slf4j-api:2.0.17")
    testImplementation(platform("org.junit:junit-bom:5.11.4"))
    testImplementation("org.junit.jupiter:junit-jupiter")
    testRuntimeOnly("org.junit.platform:junit-platform-launcher")
}

tasks.test {
    useJUnitPlatform()
    // SourceRulesTest scans this module and every adapter under versions/.
    systemProperty("oozeclient.test.versionsDir", rootProject.file("versions").absolutePath)
    inputs.files(fileTree(rootProject.file("versions")) {
        include("*/src/main/java/**/*.java")
    })
}
