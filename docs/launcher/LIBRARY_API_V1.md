# Library API 1 — instances, presets, themes and content

Owner: Sol. This additive Rust source API is exposed as
`ooze_launcher_core::library`. Launcher API 1 and JNI ABI 1 remain unchanged.
The implementation and verification status is recorded in SOL-L1.md; this
contract alone is not execution evidence.

## Facade and ownership

`Library::open(data_dir)` performs bounded local startup I/O. Call it off the
paint path, beside `Launcher::open`, using the same absolute data directory.
`snapshot`, `submit`, `cancel`, `try_event` mirror the existing asynchronous
pattern. One library operation is active at a time; concurrent submit is Busy.
IDs are monotonic per facade lifetime, independent of Launcher OperationId.
Every accepted operation has one terminal result unless its process terminates.
Callers must keep draining the bounded queue, discard stale IDs, and perform no
network/filesystem work from UI callbacks. Cancellation is cooperative with
bounded network timeouts. A mutation's durable commit is its cancellation
boundary; cancellation after commit does not undo successful work.

Library owns an exclusive `library/library.lock`. The game/installer and content
mutations share the existing per-instance lock and PID/start-time game guard.
Opening the library does not acquire credentials or change the API 1 selected
game directory. To select Play, obtain the instance directory from the library
and submit existing `SaveSettings`, then `Launch`, through Launcher API 1.

## Persistence and capabilities

All new data lives below `data_dir/library/`. Stable IDs are random, lowercase
32-character hexadecimal values, never names or paths. `state.json` is schema 1,
bounded to 4 MiB and 128 instances. Writes are validated, flushed and atomically
replaced. Unknown schema versions/fields and malformed settings fail with a
configuration error; the original bytes are preserved. No automatic migration
of L0 settings or existing `.minecraft` directories occurs. Profiles/themes are
portable versioned data without accounts, credentials, absolute paths or code.
No silent corrupt-file reset occurs. Recovery uses a separate directory or an
explicit owner-restored backup.

Instance configuration records game version, loader/version, Java requirement,
Ooze availability, preset and overrides, theme, mode, content receipts and
revision. Internal paths derive from IDs. Duplicate creates a new identity and
copies configuration/registered content, not worlds, screenshots or options.
Removal archives the entire instance under the library rather than deleting
player data. Mutations are rejected while that game instance is active.

Initial install adapters are pinned Minecraft 1.21.11 / Java 21 for Vanilla and
Fabric 0.20.1. Ooze is available only for the tested Fabric target. Other loader
enum values are reserved and rejected until their adapters and validation exist.
Capabilities, not the existence of an enum value, govern UI exposure.

Preset IDs are fps, pvp and minimal. Presets contain desired module settings and
HUD layouts; they do not install performance mods or promise an FPS gain. A
desired enabled flag does not prove that the game build implements that module.
Module IDs/settings are explicitly bounded and validated; gameplay hooks and
actual capability negotiation are a separate Claude/native milestone.

Theme data includes bounded RGBA tokens, scale, motion and approved font IDs.
Imports contain no arbitrary paths, scripts, URLs or plugin code. Contrast and
numeric validation runs before persistence. UI token consumption remains Claude's.

## Modrinth and content

Search uses public Modrinth API v2, descriptive User-Agent, bounded response
sizes, timeouts, bounded 429/5xx retries respecting rate headers and a bounded
short-lived cache. Requests include operation IDs and cancellation. Project
details, versions and dependency plans are separate typed results. Unknown
provider fields can be ignored at the HTTP parsing layer; exported/persisted
Ooze schema rejects unknown fields. Provider response bodies/errors never become
diagnostic strings. Rich text is data, never executable markup.

Types: mod, resourcepack, datapack, shader. Search is advisory; exact selected
versions are revalidated against game version, loader, project type and side.
Required dependencies are resolved deterministically; optional/embedded items
are reported, not blindly installed. Conflicts, external unresolved required
dependencies, cycles, multiple versions of a project and bounded-graph overflow
fail closed. A plan is a backend-held opaque ID tied to instance revision, not
an arbitrary UI-supplied download URL. Apply rechecks freshness and hashes.

Files come only from validated Modrinth CDN HTTPS URLs, have bounded size and
SHA-512, and are downloaded to staging. ZIP/JAR entries are validated for path
traversal, links, duplicate case-insensitive names and expansion limits without
executing or extracting them into game files. Commit records the exact stable
project/version IDs, relative path, hashes, dependency edges and enabled state.
An interrupted transaction is recovered before subsequent instance use.

Mods go in mods, resource packs in resourcepacks, shaders in shaderpacks.
Shader installation requires a compatible installed shader-loader dependency;
installing a ZIP does not enable rendering. Data packs require an explicitly
selected existing local world under the instance saves directory; no remote
server installation occurs. Data-pack modification must not run while Minecraft
is active. Updates/removal/disable operate only on recorded files and preserve
unrelated data; modified registered files require explicit repair handling.

## Integrity and trust

Custom mode verifies content downloads and reports registered-file integrity;
it carries no fair-play assertion. Managed mode is rooted in a distributed,
trusted release manifest, never a manifest approved by arbitrary UI input.
It verifies all relevant managed artifacts, rejects unexpected executable mods,
unapproved packs and prohibited launch configuration, and never deletes those
files automatically. Conversion to Custom is explicit. There is no name-only
cheat detector, kernel component, system-wide scan, ban system or server
attestation. Hashes identify bytes, not whether a mod is ethical or server-allowed.
Client-only checks cannot prevent modification by a hostile local user.

## References

- [Modrinth API](https://docs.modrinth.com/api/)
- [Search facets](https://docs.modrinth.com/api/operations/searchprojects/)
- [Versions and dependencies](https://docs.modrinth.com/api/operations/getprojectversions/)
- [Fabric metadata](https://meta.fabricmc.net/)
