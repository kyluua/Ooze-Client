# L0 installation and process decisions

The release manifest schema is `Release` in core/src/metadata.rs, schema 1. It
pins Minecraft 1.21.11, Fabric Loader 0.20.1, a bundled production Ooze JAR, Fabric
API, local hash-checked game/Fabric metadata, exact Fabric library artifacts and
a managed Java 21 Windows x64 ZIP plus its executable's relative path. Each
Artifact has exactly one URL or package-relative file, a positive bounded size,
and SHA-1 and/or SHA-256. Local artifacts and Java require SHA-256. Mojang's
upstream SHA-1 remains required for its libraries/assets; SHA-256 receipts are
computed after verified copies. The packaging script freezes these inputs.

Only HTTPS upstreams listed in net.rs are accepted. Downloads have bounded
connect/overall timeouts, at most six redirect hops and three attempts, with
every redirect checked. OAuth never follows redirects. One download is active
at a time in L0; there is no claim of optimized installation throughput. Files
download into a temporary cache file, verify size/digest, sync and atomically
replace the cache entry. Failure/cancellation never replaces a valid existing
entry. Cache identifiers derive only from validated digests.

The Minecraft resolver evaluates ordered allow/disallow OS/feature rules for
Windows x64, omits disabled feature arguments, and rejects unknown placeholders
or unsupported matching OS-version conditions. The pinned 1.21.11 metadata uses
modern native JARs on its classpath, with LWJGL extraction directed at the
generation's natives directory. Legacy native-classifier profiles and arbitrary
version-inheritance chains are not supported: the one Fabric production profile
must inherit directly from pinned 1.21.11. Fabric libraries override same-named
vanilla libraries. Changes to the target/profile require a new validated release.

New game directories must be empty. An `.ooze-launcher-instance-v1` marker and
exclusive lock identify managed instances; another launcher data root cannot
concurrently operate on that same instance. No existing Minecraft directory,
launcher account store or worlds directory is adopted/deleted. Launcher data and
instance paths reject symlinks and Windows reparse points. Archive entries reject
absolute/parent/drive/ADS/reserved-device paths, links, duplicate case-insensitive
entries, oversized entries and oversized total extraction. These checks do not
claim isolation against a malicious actor concurrently modifying files as the
same Windows user.

Files are assembled in a new temporary generation under
`<instance>/.ooze-launcher/installs/`. The Ooze JAR must contain a hash-matching
production Windows native DLL, ABI 1, capability 3; fault-injection artifacts are
rejected. Java is validated as actual 21 x64 before activation. A receipt in
`active-install.json` lists file size/SHA-256, classpath, non-secret argument
templates, managed Java and the release digest. Activation atomically replaces
this pointer after complete preparation. Play/Repair verifies the receipt's
files; bad files rebuild a new generation. Cancellation before activation leaves
the previous generation and worlds untouched. Successful old generations and
cache files are retained in L0; automatic pruning and a disk quota are future
work, so repeated upgrades/repairs can retain substantial disk space.

The production Fabric entrypoint launches through structured Command arguments,
never a shell. Fabric's `fabric.addMods` lists the verified distribution JARs
from the game installation receipt. Fabric discovers additional user mods in
the game directory's `mods/` folder. Long classpaths use a
non-secret Java argument file. Launcher-controlled JVM memory is bounded; freeform
JVM flags are not accepted. Inherited Java agent/options environment variables
are removed. Minecraft must receive its game token through the game protocol;
it is never written to the argument file, public API or launcher logs.

Java stdout/stderr are drained concurrently with bounded line size and a 16 MiB
redacted log cap per run; UI log events may be dropped under backpressure.
Exactly identified session tokens, token fields, authorization lines and JWT-like
strings are redacted. Oversized output lines are omitted in full. GameStarted
is emitted after spawn/guard recording; GameExited contains the real exit code.
The process monitor retains instance ownership until exit. PID and OS start time
prevent a second launcher from modifying a still-running orphaned game instance.

Sources: [Mojang version manifest](https://piston-meta.mojang.com/mc/game/version_manifest_v2.json),
[Fabric metadata](https://meta.fabricmc.net/),
[Fabric loader system properties](https://github.com/FabricMC/fabric-loader/blob/0.20.1/src/main/java/net/fabricmc/loader/impl/util/SystemProperties.java),
[Adoptium API](https://api.adoptium.net/).
Installation performance and end-to-end game frame costs have not been benchmarked.
