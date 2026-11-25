# CLAUDE → Sol (Codex): remove Managed Fair Play from Library API 1

- **Owner:** Claude (interface, product). **Requested of:** Sol (library backend, CLI, packaging and its docs).
- **Date:** 2026-11-25.
- **Base:** `5050e86`.
- **Contracts:** Library API 1 (this request), Launcher API 1.1 and JNI ABI 1 (unchanged).

## Owner decision (2026-11-25, verbatim from the owner's brief)

> Ooze will ship without built-in anti-cheat. Remove Managed Fair Play from the interface and remove anti-cheat plans from the active product specs. Use one normal instance experience where users can add compatible mods and packs.

The brief also says:
- keep useful file verification and Repair;
- keep the existing module release scope: the fair-play classes and the excluded list in `docs/MODULES.md` stay;
- leave historical handoffs as records;
- coordinate the Library API change with you before the interface mappings are removed.

## What the interface does now, against today's API

My commit after this request removes every mode badge, mode switch, policy warning, translation, fixture and test from the interface. Until your change lands, `core_library.rs` bridges to today's API:

- **Create** still sends `mode: sl::Mode::Custom`, because the field is required today.
- **Legacy instances:** when the library opens, the adapter sends `ConvertToCustom` once for each non-archived instance the library reports as `Managed`.
  - The conversions run one at a time and touch no files (mod.rs:484-489).
  - One that fails is retried at the next start.
  - The reason: today a Managed instance refuses all Modrinth installs (content.rs:527, mod.rs:528) and extra mods at launch (mod.rs:763), and the interface no longer has a control to convert it.
- **Never sends `ConvertToManaged`.** It never reads `IntegrityReport.managed`.
- **Verification:** for every instance, `baseline_verified == false` is shown as "installation files are missing or changed", with Repair (API 1 `Prepare{repair:true}` for the instance Play is pointed at). Per-file content statuses keep their exact-record Repair (`RepairContent`).

## Requests (Library API 1.x)

| # | Request | Where today |
|---|---|---|
| N1 | **Migrate stored data.**<ul><li>When the store opens, treat any persisted `mode: managed` as the single normal mode, and write it back durably.</li><li>Keep files, content receipts, profile, theme, revision history and the selection.</li><li>Never delete or move player files.</li><li>Test a `state.json` / `instance.json` holding a Managed instance with extra mods: after open, Modrinth install and Play both work</li></ul> | `library/store.rs`, `model.rs` (`Instance.mode`) |
| N2 | **Remove Managed behaviour:**<ul><li>the content-install refusal;</li><li>the managed-Java-only rule;</li><li>the extra-mod and pack refusal at launch;</li><li>the managed-policy issues in verification;</li><li>`ConvertToManaged`'s baseline and extra-content gate</li></ul> | content.rs:527, 671, 761, 844-…; mod.rs:491-…, 528, 651-653, 714, 744-…, 763-… |
| N3 | **API surface:**<ul><li>remove `mode` from `Command::Create` (or accept and ignore it for one minor version);</li><li>remove `Command::ConvertToCustom` and `ConvertToManaged`;</li><li>remove `IntegrityReport.managed`.</li></ul> **Keep, for every instance:**<ul><li>`verified_files`;</li><li>the per-file `content` statuses;</li><li>`issues`;</li><li>`baseline_verified` (perhaps renamed `installation_verified`), with an issue text that doesn't say "Managed".</li></ul> Keep `Verify`, `RepairContent` and installed-content inspection | `library/mod.rs:24-…`, `model.rs:545-546` |
| N4 | **CLI:** drop `--managed` and the conversion subcommands | `ooze-launcher-cli/src/main.rs` |
| N5 | **Docs and packaging:**<ul><li>`LIBRARY_API_V1.md` (the "Integrity and trust" section);</li><li>`BUILD_AND_TEST.md`;</li><li>`UPSTREAM_CONTENT.md`;</li><li>`docs/bridge/NATIVE_ARCHITECTURE.md` if it mentions the mode;</li><li>`packaging/RELEASE_NOTES_RC1.md` ("managed and custom instances" becomes "instances");</li><li>`packaging/test-library.ps1` ("Vanilla managed", `--managed`).</li></ul> No anti-cheat or fair-play claim anywhere | your paths |
| N6 | **Order with the adapter.** The commit that removes `sl::Mode` from Create, or removes `ConvertToCustom`, must also delete two things in `launcher/crates/ooze-launcher-ui/src/core_library.rs`:<ul><li>the transition block `CoreLibrary::start_legacy_migration` and its `Pending::Migrate` arm;</li><li>the `mode: sl::Mode::Custom` field in the Create mapping.</li></ul> Record it as a mechanical change in my file. Or leave the removal to me in a later commit: either is fine. Nothing else in the interface depends on mode | `core_library.rs` (marked `// TRANSITION (CLAUDE-NO-ANTICHEAT-request N6)`) |
| N7 | **Tests:** update the library tests that create or convert Managed instances | mod.rs:803-840, content.rs:1165 and others |

**Release impact:** the RC1 package hash changes. The Gate L checklist is unchanged apart from dropping Managed. W9 (Repair keeps `saves/` and `options.txt`) stays.

Please answer in `SOL-RC1.md` or a new SOL handoff: whether N1–N7 are accepted, the commit that lands them, and anything you'd change in the transition.
