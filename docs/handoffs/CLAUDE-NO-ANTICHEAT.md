# CLAUDE NO-ANTICHEAT — one instance experience without Managed Fair Play

- **Owner:** Claude (launcher interface, Fabric integration, product documentation). **Counterpart:** Sol (`SOL-INSTANCE-MODE-REMOVAL.md`).
- **Decision:** owner, 2026-11-25. Ooze ships without built-in anti-cheat:
  - Managed Fair Play is removed from the interface;
  - anti-cheat plans are removed from the active product specs;
  - there is one normal instance experience where players add compatible mods and packs.
- **Base:** `5050e86` (RC1 handoff state).
- **Tested tree:** `f08e7ed`, the merge of my `06afee8` (interface) and `61ad174` (specs) with Sol's `1beaf85` (library implementation). This handoff commit adds documentation only.
- **Contracts:**
  - **Library API 1:** single instance (Sol `1beaf85`): `Create` without mode; conversion commands, the instance mode and `IntegrityReport.managed` retired; legacy `managed` and `custom` load as normal instances.
  - **Launcher API 1.1:** unchanged.
  - **JNI ABI 1:** unchanged.
- **Environment:** Linux x86_64 cloud container. No Windows machine is attached.

## Status log

Newest last.

- 2026-11-25 `5cac579`, then `8a46901`: request to Sol, then acceptance of Sol's contract (`CLAUDE-NO-ANTICHEAT-request.md`).
- 2026-11-26 `06afee8` interface and `61ad174` specs.
- 2026-11-26 Sol's `1beaf85` reviewed (§4), then merged in `f08e7ed`. Checks in §3 ran on the merged tree.
- 2026-11-26 Sol's `157fdae` reviewed (docs only) and merged. It records Windows evidence on Sol's local ZIP `beff2cd7…`, built from `1beaf85` with Sol's temporary adapter, so its interface still predates this change. That evidence covers N3 and A4 on the backend and game side (§5).
- 2026-11-26 **UI-READY for the successor package.** Build it from `main` at or after this handoff commit. Neither `b5fc945d…` (RC1 candidate) nor `beff2cd7…` contains this interface. The successor needs its own CLAUDE-RC1 §3a verification.

## 1. Interface changes (`launcher/crates/ooze-launcher-ui`, `launcher/resources`)

**Removed:**

| Item | Where |
|---|---|
| `IntegrityMode`, `InstanceSummary.mode`, `IntegrityReport.mode` | `library.rs` |
| `LibraryCapabilities.managed_mode`, `InstanceAction::ChangeMode`, `LibCommand::SetMode` | `library.rs`, `library_model.rs` |
| `IntegrityProblem::{UnexpectedMod, UnapprovedPack, ProhibitedLaunchSetting}` and `IntegrityAction::Quarantine` | `library.rs` |
| Mode badge on each instance row | `app/library_pages.rs` |
| "Switch to Custom" and "Switch to Managed Fair Play", with their confirmations | `app/library_pages.rs` |
| Mode body text and the policy caption ("each server decides what it allows") | `app/library_pages.rs` |
| Mode mapping and `ConvertToCustom`/`ConvertToManaged` mapping; reading `r.managed` | `core_library.rs` |
| 15 translation keys: `integrity.mode.*`, `integrity.custom.body`, `integrity.managed.body`, `integrity.to_custom*`, `integrity.to_managed*`, `integrity.problem.{unexpected_mod, unapproved_pack, launch_setting, baseline}`, `integrity.action.quarantine` | `en_us.json` |
| Fixture modes and the managed fixture report | `library_fixture.rs` |
| The managed integrity UI test | `library_ui_tests.rs` |

**Kept and reworded.** The per-instance section is now **Check files**: the same for every instance, with no statement about servers (`LIBRARY_UI.md §7`).
- Content files Ooze installed are shown as "doesn't match the expected version" or "is missing".
  - **Repair** maps to the library's `RepairRecord`.
  - **Open folder** appears where the library offers it.
- `baseline_verified == false` is shown for every instance as one typed issue, **"Game files aren't installed or have changed"** (`IntegrityProblem::InstallationFiles`).
  - **Repair game files** sends Launcher API 1 `Prepare{repair:true}` and opens Play. That operation acts on the folder Play is pointed at, so the button exists only for the selected instance and is disabled while anything is busy.
  - Any other instance says "Select this instance to install or repair its game files."
- The library's CLI sentence for the same problem ("Installed game files changed or are missing. Use game Repair.") is not listed a second time.
- Counts are plural-neutral: "Problems: N. Checked files: M."
- Nothing is deleted or moved aside.

**Unchanged:**
- the module fair-play classes and their tags (`fair_play.*`);
- the server-sensitive notice;
- the wording-ban test (its comment now says "instance or module");
- managed **Java** (the launcher-managed runtime), which is unrelated to the mode.

## 2. Product changes

**Active specs** (historical handoffs are unchanged):

| Spec | Change |
|---|---|
| `PRODUCT_VISION.md` | The mode bullet becomes "File verification and Repair". Adds the owner decision and a non-goal: built-in anti-cheat, fair-play installation modes, "cheat-proof" or "server-approved" claims |
| `ARCHITECTURE.md` | ADR-018 is amended. §19.1 gains "one kind of instance" and legacy migration |
| `IMPLEMENTATION_PLAN.md` | The expansion note now records the decision. L2 is "file verification" with no modes |
| `MODULES.md` | §3 keeps the four classes, `GAMEPLAY` blocked and the excluded list. It loses the Managed/Custom columns and definitions |
| `LIBRARY_UI.md` | §1.6, §3 (no mode column; Check files action), §7 (rewritten as File check) and §9 tests |
| `AGENTS.md` | Decision record (2026-11-25). The module rule now says "no built-in anti-cheat and no instance modes; never claim a module or an instance is cheat-proof or server-approved" |

**Product model (`native/crates/ooze-ui`).** `FairPlay::Gameplay` no longer mentions Managed Fair Play. The classes are unchanged.

**Fabric mod review (`versions/mc1_21_11`).** There are no anti-cheat hooks or claims:
- the mod has its entrypoint, the engine gate and diagnostics, developer-only probe screens, and one `GuiGraphicsAccessor` mixin (`require = 0`);
- there is no mod-list inspection, allowlist, hashing of other mods, server policy or fair-play text;
- `fabric.mod.json` makes no such claim.

Nothing was removed there.

## 3. Checks

All checks ran on the tree `f08e7ed`, on Linux x86_64 unless stated.

| Check | Result |
|---|---|
| `cargo fmt --all --check` (launcher workspace) | Pass |
| `cargo clippy --workspace --all-targets -D warnings` and UI `--features dev-fixtures` | Pass |
| UI `clippy --target x86_64-pc-windows-gnu --features dev-fixtures` | Pass. **Built** (type-checked) for Windows; not run there |
| `cargo test` UI crate (`dev-fixtures`) | **Headless-tested:** 116 passed, 3 live tests ignored by default |
| `cargo test --workspace` (core and CLI) | **Headless-tested:** 47 passed. R1's IPv6 test was skipped: it fails in this container (Sol's code, CLAUDE-RC1 R1) |
| Live tests (`-- --ignored`, network: `api.modrinth.com`) | **Headless-tested**, 3 passed; details below |
| `native` `ooze-ui` fmt, clippy and test | 24 passed |
| `./gradlew build` (Fabric mod and Java tests) | Pass. Run on `61ad174`; the merge changed no Java or Gradle input |

**Live tests:**
- **`live_modrinth_search_details_plan_install_and_verify`:** a new Fabric instance through the adapter and Sol's real library.
  - Searches mods, resource packs and shaders.
  - Plans Iris with its required Sodium and installs both.
  - Lists and verifies them; the only issue is the uninstalled game.
  - Changes the Sodium JAR, sees `HashMismatch` with Repair, sends `Resolve{Repair}`, and checks the original bytes are back.
- **`live_legacy_managed_instance_installs_verifies_and_repairs_modrinth_content`:**
  - Plans and installs Sodium into a legacy `managed` instance with no conversion.
  - Verifies it and repairs a changed JAR.
  - Before Sol's `1beaf85` this was refused ("Convert to Custom…"); after it, it passes.
- **`live_window_shows_real_modrinth_results_for_a_fabric_instance`:** passes.

**What the owner asked to verify:**

| Ask | Evidence |
|---|---|
| Migrated instances display correctly | `a_legacy_managed_instance_displays_and_verifies_as_a_normal_instance`, through the real library. The instance manifest is rewritten on disk to schema-1 `managed`, then reopened. It shows: same name, target and game folder; actions Rename, Duplicate and Archive; mods offered; the file unchanged by opening. Verify gives exactly one typed installation issue and no policy text. UI: `instances_show_no_mode_badges_switches_or_policy_text` |
| The selected instance is the one Play uses | The same legacy test selects the instance, and the library's selected game folder is that instance's. Existing UI tests still pass: `selecting_an_instance_points_play_at_its_game_directory`, `failed_selection_does_not_change_the_play_directory`, `selection_reconciles_once_and_the_list_agrees_with_play`, `the_first_created_instance_is_ready_for_play_without_another_click` and `repair_is_available_while_signed_out_for_the_selected_instance`. New: `installation_repair_is_offered_only_for_the_instance_play_uses` |
| Modrinth installs work | The live tests above, for a new and a legacy instance |
| Repair works | **Content:** live Repair restores the bytes, and the UI test `file_check_lists_concrete_problems_offers_repair_and_never_deletion` sends `Resolve{Repair}`. **Game files:** the UI sends `Prepare{repair:true}` for the selected instance only. The Windows execution of game Repair isn't run here (Sol's package) |
| Baseline issue listed once | `a_changed_game_installation_is_one_typed_issue` (real library, with a deliberately unverifiable installation record). A mutation check confirmed the test fails if the de-duplication breaks |

**Not done here:**
- Nothing is **Game-launched** or **Manually verified** for this change.
- Screenshots of the Instances page and the file check were rendered and reviewed for appearance only; they are not committed. No mode badge, switch or policy text appears.

## 4. Review of Sol's `1beaf85` (before merging)

| # | Severity | Where | Finding | Status |
|---|---|---|---|---|
| A1 | Low | `library/mod.rs` Verify | The new neutral sentence for installed game files that fail verification duplicates the interface's typed `baseline_verified` issue | Fixed on my side: the adapter drops that exact sentence and shows the typed issue (`BASELINE_ISSUE`, tested). **Sol:** if you change that sentence, change `core_library.rs` `BASELINE_ISSUE` in the same commit |
| A2 | Info | `model.rs` `LegacyMode` | `#[serde(rename = "custom", alias = "managed")]` reads both schema-1 values and writes `custom`. `Instance` keeps `deny_unknown_fields`. Opening doesn't save | Good. My test confirms the manifest bytes are unchanged after opening |
| A3 | Info | `backend.rs` | Only the two Managed gates (`validate_game_settings`, `verify_prepared`) were removed. Sign-in, credential handling, readiness and instance-directory selection are untouched. Nothing new is logged | Good. No wrong-instance, readiness or credential impact |
| A4 | Low | `process.rs` `-Dfabric.addMods` | It now lists only the hash-verified distribution `mods/*.jar` files (via `join_paths`), not the whole cache folder. Player mods load from the instance's own `mods/`. That is the right narrowing. An install with no distribution JARs would pass an empty value, but a valid Fabric/Ooze installation always has them | **Covered** by Sol's `157fdae` on `beff2cd7…`: Fabric game test with Iris 1.10.8 and Sodium 0.8.14 from the packaged CLI, both in the loaded-mod list, then `OOZE_READY v1 screen=title engine=LOADED` (Game-launched, Windows 11, by Sol) |
| A5 | Info | `modrinth.rs:386-399` (existing) | For Ooze instances, Fabric API is "provided by the pinned Ooze release" and never installed twice through the library | Unchanged; compatibility, not policy |
| A6 | Info | `core_library.rs` (Sol's mechanical edit) | It kept the build green (mode-free Create, `SetMode` → Invalid) | Superseded by my adapter in the merge. Sol's Create line kept |

**R1 (CLAUDE-RC1) is still open:** the IPv6 loopback test fails on Linux.

## 5. Requests to Sol

- **N1** (neutral wording) and **N2** (packaging and backend docs) are done in `1beaf85`. Remaining mentions are the retirement notes in `LIBRARY_API_V1.md` and the legacy-migration test in `test-library.ps1`, which is correct.
- **N3 (covered by Sol's `157fdae`, backend and game side; ZIP `beff2cd7…`):** on the former Managed instance, Iris with Sodium installed and verified, a changed Iris was repaired, and the game launched with both to `OOZE_READY`. Authenticated desktop Play on the successor package is still open (owner sign-in).
- **Package:** build the successor RC from `main` at or after this handoff, and post `PACKAGE-READY` with its hash. I will verify it as in CLAUDE-RC1 §3a.

## 6. Limitations

- No Windows run of the changed screens. Before release, Narrator must cover:
  - the **Check files** section;
  - the selected-instance **Repair game files** button.

  These are W14 (owner) in CLAUDE-RC1.
- The de-duplication of the library's baseline sentence matches the exact text (A1).
- `IntegrityAction::OpenFolder` is offered only by the fixture library; Sol's library doesn't report it, so it doesn't appear in release.

## 7. Next tasks

- **Sol:**
  - successor package from the combined commit and `PACKAGE-READY` with CI run and artifact IDs;
  - on it, authenticated desktop Play, normal game exit and Repair with the migrated Fabric instance (Iris and Sodium), with the owner signing in;
  - R1.
- **Claude:**
  - verify the successor package (CLAUDE-RC1 §3a procedure) and record its hash;
  - re-run the Instances-related Windows checks with the owner (W14 Narrator over Check files);
  - update CLAUDE-RC1's status log.
- **Owner:** W3 (browser sign-in and decline) and W14 (Narrator), unchanged from CLAUDE-RC1.
