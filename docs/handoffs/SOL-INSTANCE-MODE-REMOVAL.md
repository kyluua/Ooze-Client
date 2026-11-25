# Sol handoff for instance mode removal

## Status log

- CONTRACT published from base `5050e86`. Implementation and package verification are pending.

Owner Sol (launcher core, Library API, CLI, packaging and Windows evidence).
Counterpart Claude (desktop presentation, Fabric integration and product verification).

## Contract for Claude

Library API 1's in-tree Rust source changes are coordinated with the desktop
consumer. `Command::Create { name, target, preset }` takes no mode. Remove calls
to `ConvertToManaged` and `ConvertToCustom`. `Instance` exposes no mode and
`IntegrityReport` has no `managed` field. `Verify` retains
`baseline_verified`, `verified_files`, `issues` and checked content. The
baseline flag concerns the installed game generation's recorded hashes.
`baseline_verified = false` can also mean there is no installed generation.
It carries no restriction on extra content.

Please remove Managed Fair Play and Custom instance badges, mode actions,
policy text, related fixtures and release claims from the desktop and product
documentation. Keep Verify, per-record repair, and game-install Repair. The
Java runtime selector still supports the launcher-provisioned Java and a
compatible explicit path. Java provisioning is independent of instance mode.
Align tests and localization with the single instance model. If needed to keep
the workspace building until the UI update, Sol will adapt only the core-facing
desktop mapping to the new source types and remove obsolete actions from that
mapping.

## Migration

Schema 1 accepts both legacy mode values (`managed` and `custom`) and maps
both to the same instance behavior on every read, including Play. The private
schema field writes `custom` on new records and the next ordinary atomic
instance write, so older readers can still parse a newly saved record. Opening
the library needs no separate write or marker. Existing on-disk manifests are
left as-is until normal instance mutations. Unknown mode values still fail
validation. Old releases still apply their own policy on downgrade.

The migration does no file moves, downloads or deletions. IDs, selected ID,
revision, profiles, themes, content receipts and game directories carry
through. Install/Repair checks, recorded content hashes, target/loader and
dependency compatibility, safe extraction and the game process guard retain
their existing responsibilities. Ordinary compatible mods and packs are no
longer rejected for being outside the release baseline.

## Verification plan

Exercise both legacy values through repeated opens, new create, content
changes and safe repair. Check a compatible live Modrinth dependency install,
preservation of seeded player files and hashes, and the game process guard.
Build the Windows RC1 ZIP and its matching Prism pack from the combined source.
Windows results and Claude's package verification must identify the exact new
ZIP SHA-256. The previous package's verification is scoped to that package.
