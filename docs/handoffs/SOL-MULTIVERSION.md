# Sol handoff for the Windows version catalog

Owner Sol (launcher core, Library API, Java/Rust bridge, packaging and CI).
Counterpart Claude (Fabric adapters, client artifacts and desktop UI).
Base `c4a5236`. Windows x64. The existing 1.21.11 RC1 distribution remains
available under the hashes in [SOL-WINDOWS-SETUP](SOL-WINDOWS-SETUP.md).

## Target contract for Claude

The exact Minecraft Java releases are 1.21.11, 26.1, 26.1.1, 26.1.2,
26.2, and 26.3. Each can have Vanilla and Fabric records. Java 21 is
required for 1.21.11 and Java 25 for each 26.x release. Mojang's official
version manifest and per-version metadata establish the game/runtime pairs.
Fabric metadata establishes the exact stable loader and its dependencies
for each record.

- Library API 1 keeps `Target { minecraft, loader, loader_version, java_major,
  ooze }` and `Capabilities.targets`. No new loader variants. The additive
  entry point is `Library::open_with_release(data_dir, release_manifest)`.
  Pass the same path as `LauncherConfig.release_manifest` in the installed
  desktop adapter. Keep `Library::open(data_dir)` for existing 1.21.11
  callers and fixtures. The per-game manifest stays in instance schema 1.
- Creation must use an exact capability target. The catalog returns Vanilla,
  Fabric without Ooze, and Fabric with Ooze when a matching bundle is pinned.
  The existing target record is retained when selecting, listing, opening,
  or repairing an instance. No version change is performed in place.
- Render every supported version and loader combination using the capability
  data. `ooze` distinguishes Fabric with and without the client bundle. The
  same instance target drives Modrinth and Play. Vanilla and plain Fabric
  have no Ooze engine readiness event. Existing Launcher API 1.1 and JNI ABI 1
  remain the same. Use live title-screen evidence for production validation.
- Catalog schema 2 occupies the package's `release.json` and keeps schema 1
  readable. The root is `{ schema: 2, id, targets: [...] }`. Each list
  entry is `{ minecraft, java_major, minecraft_metadata, java, fabric }`.
  `fabric` is null for Vanilla or holds `{ loader, profile, libraries, bundle }`
  for Fabric. A non-null bundle contains `{ api_version, api, ooze }` with
  the exact Fabric API Maven version and the corresponding pinned artifacts.
  The record key is the exact Minecraft version and loader family, derived
  from `fabric`. There is one selected loader pin per Fabric record.
  `Artifact` and Java archive shapes are reused from schema 1.
- For each validated Ooze artifact, supply the exact Minecraft version,
  compatible Fabric Loader and Fabric API pins, Java major, production JAR
  path and SHA-256, native ABI, and the artifact's production launch evidence.
  A shared JAR may appear on multiple rows only after real testing on each
  listed Minecraft version. Until you publish a validated artifact, package
  those Fabric rows without an Ooze bundle. Keep the existing 1.21.11 Gradle
  profile intact while adding your 26.x production outputs.

Please acknowledge the API and artifact contract in your committed handoff.
Adapt the desktop consumer to the release-aware library after the method
lands. Sol will build a candidate containing only exact, pinned rows and
record verification by version and loader before any release claim.

## Status

Contract published at `4b7379c`, before source changes. Backend, packaging,
and CI landed at `53adaa4`. The launcher API is still 1.1 and the JNI ABI
is still 1. Library API 1 adds the release-aware constructor without changing
the stored target schema.

Fabric Meta returned loader 0.20.1 on every target. Candidate packaging
accepts an optional `build/dist/validated-ooze-clients.json` from Claude
alongside the production JARs. Its schema is
`{ schema: 1, targets: [{ minecraft, java_major, loader, api_version, file,
sha256, evidence }] }`. `file` is a JAR filename beside the manifest.
`evidence` references the per-version production launch result. Leave rows
without that evidence out of this file. The 1.21.11 bundle comes from the
base package. Catalog packaging verifies each incoming hash before use.

## Backend and packaging results

The candidate catalogs the 12 Vanilla/Fabric pairs. The Fabric Loader pin is
0.20.1 throughout. Java 21 versus 25 was checked against Mojang's release
metadata for every ID. The original release manifest remains readable and the
original installer and portable distribution remain available.

| Minecraft | Java | Vanilla and plain Fabric descriptors | Ooze bundle |
| --- | --- | --- | --- |
| 1.21.11 | 21 | Pinned | Original 1.21.11 production path |
| 26.1 | 25 | Pinned | Await Claude's validated JAR |
| 26.1.1 | 25 | Pinned | Await Claude's validated JAR |
| 26.1.2 | 25 | Pinned | Await Claude's validated JAR |
| 26.2 | 25 | Pinned | Await Claude's validated JAR |
| 26.3 | 25 | Pinned | Await Claude's validated JAR |

- Local launcher formatting and workspace Clippy passed. The
  packaging script fetched Fabric Meta profiles and validated the Mojang
  metadata hashes for the added versions. All six asset indexes fit the
  current limits. The pinned Temurin 25.0.5.1+2 Windows x64 JRE ZIP matched its
  size and SHA-256 and contained the selected Java executable.
- **Built** a local catalog metadata preflight and generated its 1.21.11 Prism
  pack from the existing RC1 client JAR. The owner machine blocked a local
  native DLL load and a Rust test dependency with Application Control error
  4551. This preflight does not carry launcher or JNI execution evidence.
- [Windows CI run 45785733894](https://github.com/kyluua/Ooze-Client/actions/runs/45785733894)
  packages the candidate ZIP, Setup and eligible Prism packs. It checks JNI on
  Java 21 and 25, opens a previously installed 1.21.11 library from the catalog,
  and runs install/repair jobs for the game pairs. Candidate data and artifacts
  are separate from the 1.21.11 RC1 distribution.

Authenticated desktop Play and real title-screen evidence are still required
on the exact combined package, along with Claude's desktop integration and
matching 26.x client JARs. Use the owner for sign-in and report each target
individually. The version catalog does not promote this candidate to a release.

Claude's next desktop edit is to wire the config's manifest path at
`ooze-launcher.rs:49` and `core_library.rs:71`. The current translation file
still hard-codes the old game and Java in onboarding, progress and runtime
diagnostics. Those messages and the Play title logic need to use the selected
target and distinguish targets without the Ooze readiness event.
