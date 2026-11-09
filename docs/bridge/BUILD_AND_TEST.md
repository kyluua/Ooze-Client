# Building and testing G0

Run commands from the repository root. G0 targets **Windows x64 only**; Linux, macOS and ARM support are not claimed. Developers need rustup, Visual Studio C++ Build Tools + Windows SDK, and an actual JDK 21. End users receive prebuilt native code and need none of the Rust/MSVC build tools.

```powershell
./native/tools/build-g0.ps1 -JavaHome 'C:\Program Files\Eclipse Adoptium\jdk-21.0.12.101-hotspot' -IncludeFaultInjection
```

Replace JavaHome with your installed JDK 21 path. Both javac and the running harness enforce Java 21; `--release 21` under a newer JVM alone is not accepted as execution evidence. The root rust-toolchain.toml downloads/uses the pinned Rust 1.99.1 toolchain through rustup.

The script fails on any formatting, Clippy, Rust test, javac warning, packaging or harness failure. It executes:

- `cargo fmt --manifest-path native/Cargo.toml --all -- --check`
- `cargo clippy --manifest-path native/Cargo.toml --locked --workspace --all-targets --all-features -- -D warnings`
- `cargo test --manifest-path native/Cargo.toml --locked --workspace --all-features`
- Optimized Windows builds, Java `--release 21 -Xlint:all -Werror`, and the packaged-library harness with `-Xcheck:jni`.

The default produces/tests only the normal artifact. `-IncludeFaultInjection` additionally builds/tests a separate DLL and JAR with a deliberately injected panic. Its stderr panic-hook message is expected; a successful run continues to print the caught-panic check and a final PASS. Do not ship the fault artifact. JVM native aborts/access violations are not tested or recoverable through this mechanism.

Outputs are ignored build artifacts:

```text
native/dist/g0/release/
  ooze-bridge-java-0.1.0-g0-windows-x86_64-release.jar
  native-artifact.properties
  SHA256SUMS
  resources/META-INF/ooze/{native.properties,resources.json,LICENSE,THIRD_PARTY.txt,...}
  resources/natives/windows-x86_64/ooze_jni.dll
native/dist/g0/fault-injection/  # TEST ONLY, never distribute in the client
```

Production JAR resources include dependency license notices and Rust's license/copyright files. Those upstream Rust files are copied verbatim from the official `rust-lang/rust` 1.99.1 tag into native/licenses; build tools gather Cargo dependency license texts from the locked sources and COPYRIGHT-library.html from the pinned toolchain. Copyright: https://github.com/rust-lang/rust/tree/1.99.1.

The harness verifies manifest/hash/capability checks, missing/corrupt-library fallback, strict input/output validation, transactional batches, invalid/stale handles, thread confinement, 64-session limits, 10,000 create/destroy cycles, deterministic geometry, lease reuse protection, frame validity after session shutdown and panic conversion. Cache directories are retained under the OS temp directory and printed for inspection because Windows keeps loaded DLLs mapped until process exit.

## Standalone Gradle module

Claude can include `:bridge-java` in the root build. A module-local settings file also permits standalone checking before that root exists. First build/provide the native artifact, then run Gradle 9.x with JDK 21 available:

```powershell
gradle -p bridge-java build
```

`check` runs `bridgeContractTest`, a dependency-free main harness under the Java 21 toolchain, on Windows x64. It is explicitly SKIPPED on other hosts, not reported as native-platform support. The ordinary empty JUnit `test` task is disabled explicitly; it is not being reported as a passing test suite. By default module resources come from `native/dist/g0/release/resources`; override with `-PoozeNativeResources=<absolute-prebuilt-resources-directory>`. Java-only contributors on any host can supply the prebuilt resources without Cargo; other hosts can compile/package the Java module but cannot execute this Windows DLL. No developer-local path is embedded into the module.

The native production artifact is assembled by the PowerShell script. The Gradle module consumes prebuilt resources and does not compile native code on an end user's first launch. Root Fabric build integration remains Claude's C0 work.

## CI and limits

`.github/workflows/native-g0.yml` defines the Windows build/harness job and uploads normal and fault artifacts separately. The workflow has not run on GitHub until the branch is pushed and CI reports a result. The local script is the executed equivalent, not evidence of a remote green check.

No Minecraft process, vanilla rendering, game FPS, authentication flow or actual in-game fallback is validated by this headless harness. Those are C0/in-game integration checks. Current package/resource manifests are contracts for that next task, not a finished client JAR.
