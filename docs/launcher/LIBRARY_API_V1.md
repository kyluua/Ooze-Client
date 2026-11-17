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
`capabilities()` advertises the implemented targets/content types and an empty
runtime_modules list until game hooks are validated. `set_notify` accepts a
nonblocking repaint notifier. Events are Started and Finished; Finished contains
Result<Reply, LauncherError>. The queue is bounded to 64 events. The snapshot is
updated before Finished. Library 1 currently has no byte-progress events.
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

state.json is an index of IDs and selection. Each instance's authoritative
configuration is `library/instances/<id>/instance.json`; its game directory is
the sibling `game/`. The per-instance JSON read limit is 16 MiB (the shared
reader bound), with 512 content receipts. No absolute path is stored in portable
profiles/themes. `Snapshot` deliberately includes local game_dir for API 1 use.

Instance configuration records game version, loader/version, Java requirement,
Ooze availability, preset and overrides, theme, mode, content receipts and
revision. Internal paths derive from IDs. Duplicate creates a new identity and
copies configuration/registered content, not worlds, screenshots or options.
Archive marks its manifest archived, clears selection and preserves all files
in place. There is no permanent-delete command. Mutations are rejected while
that game instance is active.

Initial install adapters are pinned Minecraft 1.21.11 / Java 21 for Vanilla and
Fabric 0.20.1. Ooze is available only for the tested Fabric target. Other loader
enum values are reserved and rejected until their adapters and validation exist.
Capabilities, not the existence of an enum value, govern UI exposure.

Preset IDs are fps, pvp and minimal. Product profiles use Claude's canonical
`ooze.profile/1` document and pure Rust `ooze-ui::profile` implementation, consumed
as a path dependency; there is no second set of module defaults. `Profile` wraps
that document and serializes directly to its JSON object. ImportProfile returns
Imported warnings for skipped unknown/invalid entries; persisted invalid profile
entries fail instead of being silently discarded. Presets contain desired module settings and
HUD layouts; they do not install performance mods or promise an FPS gain. A
desired enabled flag does not prove that the game build implements that module.
Module IDs/settings are explicitly bounded and validated; gameplay hooks and
actual capability negotiation are a separate Claude/native milestone.

Theme data is the explicitly smaller `ooze.launcher.theme/1` token subset:
name (1–96 bytes), opaque background/foreground RGBA, accent RGBA, scale 1–2,
font inter or bubblegum_sans, and motion Full/Reduced/Off. Text/background contrast
must be at least 4.5:1. It does not pretend to implement the complete in-game
`oozeclient.theme/1` studio format; that schema must be extended jointly later.
Imports contain no arbitrary paths, scripts, URLs or plugin code. Contrast and
numeric validation runs before persistence. UI token consumption remains Claude's.

That per-instance subset is retained for source/stored-data compatibility only.
After Claude P0 review, the active desktop theme is **global**:
`Appearance { document: ooze_ui::theme_doc::ThemeDoc }`, serialized directly as
`ooze.theme/1`, stored at `library/appearance.json`, available in
`Snapshot.appearance`. Use SaveAppearance, ImportAppearance or ExportAppearance.
It uses the same canonical token file and validator as the UI, including accent/
on-accent contrast. It carries base/accent/contrast/text_scale/background/motion/
ui_scale, no paths, URLs, scripts or custom fonts. Selecting an instance does
not change it. Existing L0 UiPrefs bytes are preserved; UI preference migration
and token consumption are Claude-owned. `capabilities().appearance` advertises
this implemented operation separately from runtime module availability.

`ListWorlds(Id)` returns `Reply::Worlds { instance, worlds }`, each with an opaque
`world:<hash>` key and directory display name. Only existing local world directories
with level.dat are returned (at most 512 scanned entries). Pass the key in
Plan.world; resolution is repeated on Apply. Direct directory names remain
accepted for compatibility with the CLI. Unknown/stale keys fail; no absolute
caller path is used. `capabilities().world_listing` advertises this operation.

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
An interrupted transaction is recovered before subsequent instance use. Plans
are backend-held, single-use, expire after 600 seconds and are limited to eight
per Library lifetime. The dependency graph has at most 64 projects/depth 32.
Apply checks revision and performs download, archive validation and staged commit.
RepairContent explicitly restores recorded hashes, retaining modified originals
under the instance container's history directory. History is not pruned yet.

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
