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

Contract published first. Implementation and acceptance evidence will follow
below. Java 21 versus 25 was checked against Mojang's release metadata for
each of the six IDs.
