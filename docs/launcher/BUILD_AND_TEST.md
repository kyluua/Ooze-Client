# Building and testing the launcher and L1 library

Owner: Sol. Windows x64 only. Rust is pinned by the repository's
`rust-toolchain.toml` (1.99.1); Cargo.lock pins launcher dependencies independently
of the existing native workspace. Java runtime and adapter target remain 21.
The standalone launcher does not call JNI; it installs the real remapped Ooze
mod containing bridge ABI 1 and the production Windows x64 native library.
The shared launcher workspace includes the desktop GUI and consumes the pure
Rust product model from native/crates/ooze-ui. Changes to that model, its base
tokens and the pinned Rust toolchain also trigger launcher CI.

L1 adds Library API 1 alongside unchanged Launcher API 1. It implements instance
and content infrastructure, not live in-game modules or additional loader ports.

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
An earlier downloaded unsigned CI launcher executable was also blocked on this
host. The later public-ID-configured CLI from commit 48b53f2 ran locally without
any policy change; its browser sign-in attempt timed out before a callback.
See SOL-L0-AUTH-CONFIG-RESULT.md for the per-build result. This is not a claim that
local Rust compilation, every unsigned executable or the desktop UI now runs.
The Windows GitHub workflow supplies build/test/package evidence separately,
including installation with developer tools/JDK removed from PATH.
This limitation is distinct from Java 21, which runs locally and passed the
existing bridge harness and Fabric build. Use SOL-L0 for exact run references.

## Build the real game mod and package

The release build above produces both `ooze-launcher.exe` (desktop interface)
and `ooze-launcher-cli.exe`. The package requires both and includes workspace
dependency notices and unmodified font/OFL files. It is an unsigned review build.

```powershell
./native/tools/build-g0.ps1 -JavaHome $env:JAVA_HOME_21_X64
./gradlew.bat build buildAll --no-daemon --console=plain
./launcher/packaging/prepare-release.ps1 `
  -OozeJar './build/dist/ooze-client-0.1.0-alpha.0+mc1.21.11.jar' `
  -CliExe './launcher/target/release/ooze-launcher-cli.exe' `
  -GuiExe './launcher/target/release/ooze-launcher.exe' `
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

## Exercise Library API 1 without signing in

Use a new, dedicated absolute data directory. Library commands do not access
Microsoft credentials. In a verified package directory:

```powershell
$data = 'C:\Ooze test libraries\L1'
$created = ./ooze-launcher-cli.exe --data-dir $data library create 'PvP' --preset pvp | ConvertFrom-Json
$instance = $created.Instance
./ooze-launcher-cli.exe --data-dir $data library list
./ooze-launcher-cli.exe --data-dir $data library search mod iris
./ooze-launcher-cli.exe --data-dir $data library search resourcepack
./ooze-launcher-cli.exe --data-dir $data library search datapack
./ooze-launcher-cli.exe --data-dir $data library search shader
./ooze-launcher-cli.exe --data-dir $data library versions iris
./ooze-launcher-cli.exe --data-dir $data library install $instance YL57xq9U ka9PDlsN
./ooze-launcher-cli.exe --data-dir $data library verify $instance
./ooze-launcher-cli.exe --data-dir $data library disable $instance YL57xq9U
./ooze-launcher-cli.exe --data-dir $data library enable $instance YL57xq9U
./ooze-launcher-cli.exe --data-dir $data library repair $instance YL57xq9U
./ooze-launcher-cli.exe --data-dir $data library export-profile $instance
./ooze-launcher-cli.exe --data-dir $data library appearance
./ooze-launcher-cli.exe --data-dir $data library worlds $instance
./ooze-launcher-cli.exe --data-dir $data instance $instance install
```

The exact Iris version above is the integration fixture, not a permanent latest
recommendation. Resolve a compatible version when selecting other content.
It requires Sodium, which is resolved/verified as a dependency. These Custom
content downloads are not bundled in the review package. See UPSTREAM_CONTENT.md
for provenance and license metadata. Installing is distinct from enabling a pack
in game; the launcher does not assert a shader is rendering from a saved ZIP.

The dedicated `test-library.ps1` exercises the real executable and API, including
modified-file repair and managed extra-mod detection. `test-targets.ps1` tests
actual managed Java 21 installation/repair for the pinned Vanilla and Fabric
1.21.11 targets, preserving a seeded options.txt. Neither starts Minecraft or
performs Microsoft authentication.

For local baseline timing/process working-set samples:

```powershell
./launcher/packaging/measure-library.ps1 -CliExe '<package>\ooze-launcher-cli.exe' `
  -DataDirectory $data -InstanceId $instance -OutputJson '<new-output-path>.json'
```

Each sample launches a fresh CLI. OS file caches are not flushed. The sampled
process working set is not native allocator accounting or a measurement of GUI,
Java, JNI, GPU or game frame time. The committed example records its actual CLI
hash and conditions; it makes no comparative FPS claim.

The desktop currently consumes its implemented library capabilities; Claude's
SOL-L1 review requests identify remaining UI mappings and selection sequencing.
Module switches remain hidden until actual game hooks are available. Minecraft
sign-in/Play is still a separate acceptance gate after the owner's AppID review.
