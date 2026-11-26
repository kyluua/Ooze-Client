# Sol handoff for instance mode removal

## Status log

- CONTRACT published from base `5050e86`. Implementation and package verification are pending.
- Claude accepted the contract in `8a46901`. N1 is covered by a neutral
  issue when installed game files fail verification. A fresh instance returns
  `baseline_verified = false` without a corruption issue. N2 is covered by
  updates to packaging and the active backend instructions. N3 is tracked
  in the Windows evidence below once the successor package is tested.
- BACKEND-LANDED `1beaf85dcab19585d0a314096ce73d0d289517ae`.
- CANDIDATE-BUILT `beff2cd7c1d6f993234850cb41494b4714ed0f9cba098f19c613b843af8d35d0`.
  This local package includes the mechanical desktop adapter, ahead of
  Claude's presentation cleanup. Gate L is open.

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

Fabric loads user mods from the instance game directory. The launcher supplies
the receipt-verified distribution JARs separately and passes their file paths
to Fabric, keeping unrelated files in the installation cache out of that list.

## Desktop compatibility while the presentation changes land

The mechanical edit to `core_library.rs` keeps the existing desktop build
green across the API transition. It submits mode-free creation, advertises no
conversion, and can repair any registered file. Old presentation types use
their former Custom label for all instances until Claude's cleanup. Claude's
adapter update can drop those types and mappings with the rest of the mode UI.

## Verification plan

Exercise both legacy values through repeated opens, new create, content
changes and safe repair. Check a compatible live Modrinth dependency install,
preservation of seeded player files and hashes, and the game process guard.
Build the Windows RC1 ZIP and its matching Prism pack from the combined source.
Windows results and Claude's package verification must identify the exact new
ZIP SHA-256. The previous package's verification is scoped to that package.

## Result and package

Base `5050e86`, published contract `1a37969`, implementation and package
source `1beaf85dcab19585d0a314096ce73d0d289517ae`. Changed paths are the
launcher core, CLI and packaging scripts, Library API and installation/build
documentation, and the mechanical compatibility edit in
`launcher/crates/ooze-launcher-ui/src/core_library.rs`. Library API 1 changes
the in-tree source; Launcher API 1.1 and JNI ABI 1 are unchanged.

**Built** locally on Windows x64, with a clean source marker in the JAR. The
binaries are unsigned. The ZIP contains 599 checked payloads plus
`SHA256SUMS`; all 599 hashes matched and there were no unlisted payloads.

| Local artifact | SHA-256 |
| --- | --- |
| `ooze-rc1-single-instance-1beaf85.zip` | `beff2cd7c1d6f993234850cb41494b4714ed0f9cba098f19c613b843af8d35d0` |
| `ooze-launcher.exe` | `a6dea4dc3fa70d18eb315528c4bc0b6d4235d603dfbb84b5e06a827f722e41b2` |
| `ooze-launcher-cli.exe` | `ed40d6187ba73dcee1df9951031ef2bb146310961fb17f8c4e94de7849cbdf7e` |
| `ooze-client.jar` | `d68f744b44c1d65d580a74242e172d5c379d826c9e35415d9314c3af2aeb630b` |
| embedded Windows x64 `ooze_jni.dll` | `2dfad484aee7381219101c7bd5453b1fffcb00b400243dbeb61ec464cf15b703` |
| `ooze-base-0.1.0-rc1.mrpack` | `38151f4d613d7f77740c5250b1b78f8c5702f58f8085ff12d0120ac49d86489c` |

The Prism validator checked the matching Ooze JAR, Fabric API
0.141.6+1.21.11, Fabric Loader 0.20.1 and the required per-file sizes,
SHA-1 and SHA-512. The pack SHA-512 is
`1b813f993cf9bbe66177f3838b2d390822ab2e4ba162b922d4d4bc2473a8b84ec173d0dc37b1332a2a3ed298ec73167f42b446606495a4638de434c715b3fd3f`.
The local outputs are under `launcher/dist`; the GitHub Actions run for the
same source is [45726754269](https://github.com/kyluua/Ooze-Client/actions/runs/45726754269).
Its library job passed, including live Modrinth verification. The desktop
Windows job was running when this evidence was recorded. Any CI artifacts
have their own hashes.

## Windows evidence

These checks use ZIP
`beff2cd7c1d6f993234850cb41494b4714ed0f9cba098f19c613b843af8d35d0`.
Host Windows 11 Pro build 26300, AMD Ryzen 7 9800X3D, Radeon RX 6900 XT,
Temurin 21.0.12.1+1 x64 for the build and Fabric game tests. The test
directories are separate from the default launcher data.

- **Headless-tested** launcher fmt and workspace Clippy, 52 core, 111 ordinary
  UI and 2 live Modrinth UI tests. Native fmt, Clippy, 29 Rust tests, Java 21
  bridge harness (20,323 assertions), Fabric Gradle build and tests passed.
- **Headless-tested** W4/W6 backend with the ZIP's CLI. Repeated opens accepted
  both legacy modes, preserved IDs and selection, and left the original
  manifests byte-identical until a mutation. Iris 1.10.8 installed with
  Sodium 0.8.14 on the former Managed instance. Verify checked both hashes.
  Changing Iris was detected, project repair restored the pinned
  file, and Verify left unregistered content in place.
- **Game-launched** from the package's exact JAR in the Fabric production
  game test with the two Modrinth downloads from the packaged CLI. Java 21
  logged `Loading Minecraft 1.21.11 with Fabric Loader 0.20.1`,
  `iris 1.10.8+mc1.21.11`, `sodium 0.8.14+mc1.21.11`, and
  `[16:12:02] [Render thread/INFO]: OOZE_READY v1 screen=title engine=LOADED`.
  The test closed normally. This evidence covers the Fabric game test.
  Authenticated Play in the desktop window still needs an owner sign-in.
- **Game-launched** the five Windows adapter cases using the matching DLL
  and the production JAR from the ZIP. The PowerShell matrix passed
  development and production `LOADED`, missing DLL `MISSING_LIBRARY`,
  and corrupted DLL and ABI mismatch `INCOMPATIBLE`. Each case reached
  the title screen and passed the vanilla Options round trip.
- **Headless-tested** W9 with the packaged CLI on the migrated instance after
  the Fabric game test. Repair restored a deliberately corrupted copy of
  the installed Ooze JAR. The hashes of all nine seeded or installed files
  matched, including the two Modrinth JARs, options, world, screenshot and
  packs. The Windows live-process guard also passed its dedicated core test.
  Packaged Java 21 install and game Repair also passed on fresh Fabric and
  Vanilla 1.21.11 instances.
- **Headless-tested** W11 CLI diagnostic fixture. ZIP contained
  `diagnostics.json` and `game.log`; two redactions and zero known credential
  fixture matches. Desktop export and live game logs require their own check.
- **Headless-tested** signed-out Play asks for sign-in in the packaged CLI.

## Remaining acceptance

Claude's presentation changes need to land with the removal of the temporary
desktop mapping. Assemble a fresh ZIP from that combined commit and record
the CI run, artifact IDs and independent package verification for its hash.
On that package, use the migrated Fabric instance with Iris and Sodium for
authenticated desktop Play, normal game exit and Repair. The account owner
completes the browser prompts and the Narrator pass. Rerun the affected desktop
checks and keep Gate L open until both handoffs cover the same package.
