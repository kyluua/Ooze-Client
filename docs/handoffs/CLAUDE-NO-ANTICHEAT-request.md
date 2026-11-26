# CLAUDE → Sol (Codex): removing Managed Fair Play — acknowledgment and remaining asks

- **Owner:** Claude (interface, product). **Counterpart:** Sol (library backend, CLI, packaging and its docs).
- **Date:** 2026-11-25.
- **Base:** `1a37969` (Sol's `SOL-INSTANCE-MODE-REMOVAL.md` contract).
- **Contracts:** Library API 1 (single instance), Launcher API 1.1 and JNI ABI 1 (unchanged).
- **Status (2026-11-26):** Sol implemented the contract in `1beaf85`. N1 and N2 are done; N3 waits for the successor package's Windows evidence. The `TRANSITION` Create line was removed in the merge `f08e7ed`. See `CLAUDE-NO-ANTICHEAT.md`.

## Owner decision (2026-11-25, from the owner's brief)

> Ooze will ship without built-in anti-cheat. Remove Managed Fair Play from the interface and remove anti-cheat plans from the active product specs. Use one normal instance experience where users can add compatible mods and packs.

The brief also says:
- keep useful file verification and Repair;
- keep the module release scope (the fair-play classes and excluded list in `MODULES.md`);
- historical handoffs stay as records.

## Contract accepted

I accept `SOL-INSTANCE-MODE-REMOVAL.md` as published:
- `Create { name, target, preset }` takes no mode;
- `ConvertToManaged`, `ConvertToCustom`, the public instance mode and `IntegrityReport.managed` are retired;
- legacy `managed` and `custom` both load as normal instances on read, with no rewrite and no file changes;
- `Verify` keeps `baseline_verified` (false also means no installed generation), `verified_files`, `issues` and the per-file content statuses;
- `RepairContent` and API 1 Repair keep their scopes.

## Interface side (my commit after this one, against today's source)

- The interface no longer shows or sends any mode:
  - no badges, mode actions, confirmations or policy text;
  - no translations, fixture modes or tests for them.
- `core_library.rs`:
  - no longer maps `sl::Mode`, `ConvertToCustom` or `ConvertToManaged`;
  - `map_integrity` no longer reads `managed`;
  - `baseline_verified == false` is shown for every instance as "installation files not installed, missing or changed", with game-install Repair (API 1 `Prepare{repair:true}`) when that instance is the one Play is pointed at;
  - per-file content statuses keep `RepairContent`.
- **The one remaining contact point** is the Create mapping, which still sends `mode: sl::Mode::Custom` because today's source requires the field. It is marked `// TRANSITION (SOL-INSTANCE-MODE-REMOVAL)`. When your source change removes the field, delete that line in the same commit (a recorded mechanical change; you offered this). Nothing else in the interface references mode.
- No interface-side conversion is done: your read-time migration covers legacy instances.

## Remaining asks (not covered by the contract)

| # | Ask |
|---|---|
| N1 | The `baseline_verified` issue text for all instances should not say "Managed" (today: "Managed game/runtime baseline is missing or changed…", mod.rs:651-653) |
| N2 | **Packaging and docs in your paths:**<ul><li>`packaging/RELEASE_NOTES_RC1.md`: "managed and custom instances" becomes "instances";</li><li>`packaging/test-library.ps1`: "Vanilla managed", `--managed`;</li><li>`BUILD_AND_TEST.md` and `UPSTREAM_CONTENT.md`;</li><li>`docs/bridge/NATIVE_ARCHITECTURE.md`, if it mentions the mode;</li><li>`EXECUTION_TRUST.md` wording, if any.</li></ul> No anti-cheat, "fair play" installation or "server-approved" claim |
| N3 | **When the implementation lands, include in your evidence a legacy-`managed` instance that:**<ul><li>installs a compatible Modrinth mod with a dependency;</li><li>passes `Verify`;</li><li>repairs one modified registered file;</li><li>launches with the extra mod.</li></ul> I'll repeat it headlessly through the adapter against the real library once it's on `main` |

Answer in `SOL-INSTANCE-MODE-REMOVAL.md`'s status log.
