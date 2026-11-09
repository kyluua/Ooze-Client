plugins {
    `java-library`
}

group = "io.github.kyluua.ooze"
version = "0.1.0-g0"

java {
    toolchain.languageVersion.set(JavaLanguageVersion.of(21))
}

tasks.withType<JavaCompile>().configureEach {
    options.release.set(21)
    options.compilerArgs.addAll(listOf("-Xlint:all", "-Werror"))
}

val nativeResources = providers.gradleProperty("oozeNativeResources")
    .map { file(it) }
    .orElse(layout.projectDirectory.dir("../native/dist/g0/release/resources").asFile)

sourceSets.main {
    resources.srcDir(nativeResources)
}

val verifyNativeArtifact = tasks.register("verifyNativeArtifact") {
    inputs.dir(nativeResources)
    doLast {
        check(nativeResources.get().resolve("META-INF/ooze/native.properties").isFile) {
            "Build G0 with native/tools/build-g0.ps1 or set -PoozeNativeResources to the prebuilt resources directory"
        }
        check(nativeResources.get().resolve("natives/windows-x86_64/ooze_jni.dll").isFile) {
            "The prebuilt Windows x64 DLL is missing"
        }
    }
}

tasks.processResources {
    dependsOn(verifyNativeArtifact)
}

// The Java main harness deliberately has no JUnit/Minecraft dependency.
tasks.test {
    enabled = false
}

val bridgeContractTest = tasks.register<JavaExec>("bridgeContractTest") {
    onlyIf("The G0 native DLL harness runs only on Windows x64") {
        System.getProperty("os.name").startsWith("Windows") &&
            System.getProperty("os.arch") in listOf("amd64", "x86_64")
    }
    dependsOn(tasks.testClasses)
    classpath = sourceSets.test.get().runtimeClasspath
    mainClass.set("io.github.kyluua.ooze.bridge.BridgeHarness")
    javaLauncher.set(javaToolchains.launcherFor {
        languageVersion.set(JavaLanguageVersion.of(21))
    })
    jvmArgs("-Xcheck:jni", "-Djava.awt.headless=true")
}

tasks.check {
    dependsOn(bridgeContractTest)
}
