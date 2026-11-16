# Building and testing the L0 backend

Owner: Sol. Windows x64 only. Rust is pinned by the repository's
`rust-toolchain.toml` (1.99.1); Cargo.lock pins launcher dependencies independently
of the existing native workspace. Java runtime and adapter target remain 21.
The standalone launcher does not call JNI; it installs the real remapped Ooze
mod containing bridge ABI 1 and the production Windows x64 native library.

## Developer checks

From the repository root in PowerShell, with rustup/MSVC build tools and an
actual Java 21 JDK installed:

```powershell
$env:JAVA_HOME_21_X64 = 'C:\path\to\jdk-21'
$env:JAVA_HOME = $env:JAVA_HOME_21_X64
cargo fmt --manifest-path launcher/Cargo.toml --all -- --check
cargo clippy --manifest-path launcher/Cargo.toml --locked --workspace --all-targets -- -D warnings
cargo test --manifest-path launcher/Cargo.toml --locked --workspace
cargo build --manifest-path launcher/Cargo.toml --locked --release --workspace
```

The Java process regression test requires JAVA_HOME_21_X64, checks the actual
runtime version and compiles a private Java fixture. It launches that fixture
twice from paths containing spaces, captures a nonzero exit, and checks redacted
output and the credential-free argument file. This is Headless-tested process
evidence, not a Minecraft launch. No offline game identity is exposed in the
release CLI or API. Other tests use controlled protocol fixtures and local
loopback service errors, not another launcher's account/app credentials.

On the development host Windows Application Control blocked locally compiled
Rust build scripts with error 4551. Its policy was not disabled or changed.
The downloaded unsigned CI launcher executable was also blocked on this host.
The Windows GitHub workflow supplies build/test/package evidence separately,
including installation with developer tools/JDK removed from PATH.
This limitation is distinct from Java 21, which runs locally and passed the
existing bridge harness and Fabric build. Use SOL-L0 for exact run references.

## Build the real game mod and package

```powershell
./native/tools/build-g0.ps1 -JavaHome $env:JAVA_HOME_21_X64
./gradlew.bat build buildAll --no-daemon --console=plain
./launcher/packaging/prepare-release.ps1 `
  -OozeJar './build/dist/ooze-client-0.1.0-alpha.0+mc1.21.11.jar' `
  -CliExe './launcher/target/release/ooze-launcher-cli.exe' `
  -OutputDirectory './launcher/dist/ooze-l0-windows-x64'
```

The script requires a new output directory; it never overwrites an existing
package. It freezes official game metadata, the exact Fabric 0.20.1 production
profile/libraries, Fabric API 0.141.6+1.21.11 and an Adoptium Java 21 Windows x64
JRE archive. Java is selected at package preparation and then pinned by URL,
size and SHA-256. Ooze's mod and metadata are bundled with SHA-256. Existing
native artifact production retains its own documented reproducibility limits.

For a repeat package, supply `-PinnedPackageDirectory <existing-package>` to
reuse its frozen metadata and release inputs rather than selecting a newer JRE
or receiving a newly timestamped Fabric profile. That reproduces installation
inputs, not a claim of bit-for-bit PE or ZIP identity across machines. Verify
SHA256SUMS from a trusted distribution; hashes are integrity checks, not a
substitute for trusting the package source. No self-updater is included.

Manifest JSON uses recursively sorted ordinal keys so object insertion order
does not change the release digest. Two local repeats with identical frozen
inputs produced identical content across all 343 package files, including
SHA256SUMS. CI repeats this check. This copies the same prebuilt PE/JAR inputs;
it does not claim recompiling Rust/native binaries is bit-for-bit reproducible.

The review package includes the CLI, release.json, metadata, Ooze JAR, licence
texts and SHA256SUMS. Claude supplies the desktop UI as the next consumer of API
1; a CLI package is not represented as the complete player-facing launcher.
Minecraft game files are downloaded, never bundled. Managed Java is a vendor
JRE archive with its legal notices retained after extraction. End users need no
Rust, MSVC, Gradle or developer JDK. Distribution signing is not implemented.

## Exercise the package outside a developer shell

```powershell
./ooze-launcher-cli.exe status
./ooze-launcher-cli.exe install
./ooze-launcher-cli.exe repair
./ooze-launcher-cli.exe settings --java 'C:\path\to\java21\bin\java.exe'
./ooze-launcher-cli.exe settings --managed-java
./ooze-launcher-cli.exe sign-in
./ooze-launcher-cli.exe play
./ooze-launcher-cli.exe sign-out
```

Use `--data-dir <absolute-directory>` before the command for an isolated test
installation and `--release <absolute-release.json>` when not beside the package.
Default data root is `%LOCALAPPDATA%\OozeLauncher`. Configure the owned public app
ID as described in AUTH_SETUP. Status reports remembered display identity, not
live authorization. Play revalidates auth and installation; Spawned/GameStarted
means a Java PID exists, not that Minecraft reached the title screen. Close the
game normally; the CLI waits for it and reports nonzero exits. Claude should
keep the desktop backend alive while hiding its window if desired. A persisted
PID/start-time guard prevents repair/relaunch if the desktop process exits while
the game remains alive; no process command lines are inspected for that check.

## Windows game adapter gate (separate from launcher authentication)

Git Bash is not installed on this host. The Sol-owned equivalent drives Claude's
existing gametest tasks without editing the adapter or its build:

```powershell
./launcher/packaging/test-c0-windows.ps1 `
  -JavaHome $env:JAVA_HOME_21_X64 `
  -NativeResources '<production G0 artifact>/resources' `
  -OutputDirectory '<new absolute evidence directory>'
```

It checks real dev/prod LOADED, the bridge fixture, and missing/corrupt/ABI-mismatch
native resources with vanilla Options round trips. Only the intentionally missing
DLL case skips the module's build-time file-existence check so the actual loader
can see the missing resource. These are Fabric gametests using test credentials;
they are not proof of independent-launcher authentication. They open real game
windows on the Windows machine. No human verification is inferred from automation.
