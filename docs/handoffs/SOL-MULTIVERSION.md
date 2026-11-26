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
and CI landed at `53adaa4`. Follow-up `0ae6d7e` preserved the legacy
1.21.11 error detail expected by the existing desktop adapter. The original
run failed that assertion. The launcher API is still 1.1 and the JNI ABI
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

| Minecraft | Java | Exact candidate CI Install / Repair | Ooze bundle |
| --- | --- | --- | --- |
| 1.21.11 | 21 | Vanilla and Fabric with Ooze passed | Original 1.21.11 production path |
| 26.1 | 25 | Vanilla and plain Fabric passed | Await Claude's validated JAR |
| 26.1.1 | 25 | Vanilla and plain Fabric passed | Await Claude's validated JAR |
| 26.1.2 | 25 | Vanilla and plain Fabric passed | Await Claude's validated JAR |
| 26.2 | 25 | Vanilla and plain Fabric passed | Await Claude's validated JAR |
| 26.3 | 25 | Vanilla and plain Fabric passed | Await Claude's validated JAR |

- Local launcher formatting and workspace Clippy passed. The
  packaging script fetched Fabric Meta profiles and validated the Mojang
  metadata hashes for the added versions. All six asset indexes fit the
  current limits. The pinned Temurin 25.0.5.1+2 Windows x64 JRE ZIP matched its
  size and SHA-256 and contained the selected Java executable.
- **Built** a local catalog metadata preflight and generated its 1.21.11 Prism
  pack from the existing RC1 client JAR. The owner machine blocked a local
  native DLL load and a Rust test dependency with Application Control error
  4551. This preflight does not carry launcher or JNI execution evidence.
- **Headless-tested** local 26.3 preflight with the compiled CLI from
  `53adaa4` ([artifact 15099949459](https://github.com/kyluua/Ooze-Client/actions/runs/45785733894/artifacts/15099949459)).
  Used the locally pinned catalog and a new data directory. Opened schema 1
  instances created by the 1.21.11 RC1 CLI and preserved both IDs and the
  selected ID. Installed Sodium `mc26.3-0.9.4-beta.2-fabric` on the new
  Fabric 26.3 instance, installed the game with Fabric Loader 0.20.1 and
  actual Temurin 25 x64, and repaired a corrupted game JAR. Repair preserved
  seven checked player files including Sodium and the separate legacy
  options fixture. Vanilla 26.3 independently installed and repaired with the
  same runtime and preserved its options. An explicit Java 21 override returned
  the Java 25 requirement and kept the original installation receipt.
- [Windows CI run 45786508447](https://github.com/kyluua/Ooze-Client/actions/runs/45786508447)
  **Built** the candidate ZIP, Setup and eligible Prism pack on `0ae6d7e`.
  **Headless-tested** 53 core, 115 ordinary UI, 3 live Modrinth, and 29 native
  Rust tests. The packaged JNI bridge passed 20,323 assertions each under
  actual Java 21.0.13.1+2 and Java 25.0.5.1+2, both Windows amd64, with
  `-Xcheck:jni`. The catalog migration job opened a previously installed
  1.21.11 library and verified the original selection and a separate 26.3
  instance directory. The installer was built. All six matrix jobs passed
  (12 install/repair cases) on the exact candidate ZIP below. Each case used
  the catalog-selected Java x64 release, verified the managed game generation,
  repaired a corrupted game file, and kept the seeded player file hashes.
- **Headless-tested** all 12 local install/repair pairs with the earlier
  CLI preflight and pinned metadata. These checks exercised real downloads,
  Java selection, corruption detection and retained player files.

### Candidate downloads

| Artifact | SHA-256 |
| --- | --- |
| [Portable Windows x64 ZIP](https://github.com/kyluua/Ooze-Client/actions/runs/45786508447/artifacts/15101992148) | `40708483d5fed7175127666c2311f7cc86872f220adfc52817203372e0e11604` |
| [Windows x64 Setup EXE, inside artifact](https://github.com/kyluua/Ooze-Client/actions/runs/45786508447/artifacts/15101493470) | `eca69cf2247f47e11bea7c94f03df17d2f163f1168f4b0f3359e2da289403b13` |
| [1.21.11 Prism pack, inside artifact](https://github.com/kyluua/Ooze-Client/actions/runs/45786508447/artifacts/15101757759) | `11daee9c1bae1573a6923265f55368a23750a3b6de930bae7b773c394ba3c98b` |

Verified all 609 payload files against the downloaded portable ZIP's
`SHA256SUMS`. The manifest SHA-256 is
`110dba72b47b1394c64ec774beaac417a8eb0a7a4d3f505a60c8ab844249c1e3`.
The desktop EXE SHA-256 is
`731a17c6daeb7e105b7b197a2bf413b19340ea51a1aebcb77f624e01a6d7cde1`,
the CLI is `a9e7116e8ab843b28ea80e564aecd23b1795f1c3b4aada0429a8ece8abd5a2ce`,
and the Ooze JAR is
`9b3262521262c0fcc2c3c053e2e1dc012159ed4186705660f1e351a49a16f7c9`.
The embedded Windows JNI DLL SHA-256 is
`154c591dbf48f9a7a0d21d908f995b6b2b99b642c5ae5cbab7e6c6e3ce37bce4`.
The matching Prism pack includes that same JAR and verified Fabric API.
Its SHA-512 is
`0ac6e173f448d0f7699f1ce9b3fefd798df350cedc8192d22e7c2f6bd9a35d5065c5c7192b53c7abe5d7ea3a8bdbde123cabeaf8f3f83c3ed1e6bd03a8d5a1c6`.

Authenticated desktop Play and real title-screen evidence are still required
on the exact combined package, along with Claude's desktop integration and
matching 26.x client JARs. The candidate installer is unsigned. The owner
PC's Windows Application Control blocked the exact packaged CLI before it
could create a signed-out test instance. No settings or Windows execution
policy were changed. Use an approved execution environment and the owner
for sign-in. Report each target's actual title-screen evidence individually
before marking support.

Claude's next desktop edit is to wire the config's manifest path at
`ooze-launcher.rs:49` and `core_library.rs:71`. The current translation file
still hard-codes the old game and Java in onboarding, progress and runtime
diagnostics. Those messages and the Play title logic need to use the selected
target and distinguish targets without the Ooze readiness event.
