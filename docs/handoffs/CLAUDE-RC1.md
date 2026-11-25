# CLAUDE RC1 — Ooze base and launcher release candidate

Owner: Claude (Fabric integration, launcher interface, product verification). Counterpart: Sol (`SOL-RC1.md`).

## Status log

Newest last. Each line names a commit or package hash.

- 2026-11-25 STARTED `7dd54d73f5897dcd2900241865e8dbac56d271ff`.
  - Running in a Linux x86_64 cloud container; no Windows machine is attached (the Remote Control folder list is empty).
  - Windows operation is Sol's, with the owner for Microsoft sign-in and Narrator.
- 2026-11-25 UI-READY `fa160e9` (last interface code commit; this handoff commit is docs only). Build RC1 from `main` at or after it. Checks are in §3.
  - One Linux-only backend test fails: R1, in Sol's code. It doesn't affect Windows CI.

## 1. Review findings

These cover Sol's `44c407f` (Gate L hardening) and today's pushes: `1f8b92b`, `120be4f`, `fff95ec`, `8d035c0`, `ace3a18` and `3a1ad9c`. I reviewed each before merging it.

| # | Severity | Where | Finding | Request or status |
|---|---|---|---|---|
| R1 | **Medium** | `launcher/crates/ooze-launcher-core/src/auth.rs:210-218` (`8d035c0`) | `bind_callback` aborts sign-in on any `::1` bind error it doesn't list. On a host where IPv6 loopback fails with an unlisted error, sign-in fails entirely instead of using IPv4. On this Linux container the bind returns `EAFNOSUPPORT` (os error 97), so `auth::tests::localhost_callback_is_served_over_each_available_loopback_family` **fails** and `cargo test --workspace` is red on Linux. Windows' 10047 is handled, so the Windows CI is unaffected | **Sol:** treat every `::1` bind error except `AddrInUse` as "IPv6 unavailable" and continue with IPv4 only. Compute the test's `ipv6_available` the same way |
| R2 | Info | `auth.rs:229-260` (`44c407f`) | Unrelated or incomplete loopback requests get 404 and the original deadline is kept. A wrong-state callback fails at once. `access_denied` with a matching state returns a specific decline message. Cancellation is checked between connections | Good. The interface shows these neutrally (B6 test) |
| R3 | Low | `diagnostics.rs:57-82` (`44c407f`) | The ZIP's `game.log` has credentials redacted (the live token was removed earlier, and the patterns are applied again), but it can contain player names and folder paths. Existing files are never overwritten, and links are rejected | The interface now says so before export (B2). An optional "remove player name and paths" pass is an RC2 idea |
| R4 | Info | `readiness.rs`, `process.rs:315-357` (`120be4f`) | The typed `GameReady` grammar is anchored and the state is exact. CRLF lines are safe because stream lines keep their `\n` (process.rs:315-329). It is sent with a blocking send, once per session, so it can't be dropped with log lines | Mapped in B7. The interface's `ENGINE_STATES` matches the core's `EngineState` and the mod's Java enum (test) |
| R5 | Info | `library/store.rs:200` | Creating the first instance makes the library select it without the interface's Select flow. This was the cause of Sol's "Selected for Play but Play asks to select" observation | Fixed in the interface (B4) |
| R6 | Info | `packaging/prepare-mrpack.ps1` (`ace3a18`) | The pack puts the Ooze JAR in `client-overrides/mods/`. Fabric API comes from `cdn.modrinth.com`, an allowed `.mrpack` host, with pinned SHA-1, SHA-512 and size. `formatVersion` is 1, with dependencies `minecraft 1.21.11` and `fabric-loader 0.20.1` | Good. The final pack must use the RC1 package's JAR (Sol already notes this) |
| R7 | Info | `.github/workflows/launcher-l0.yml` (`3a1ad9c`) | UI tests run serially on DX12, and the two live UI tests run separately | Good. My Linux runs are listed below; Windows CI is the Windows evidence |

No change today introduces a wrong-instance launch path, premature readiness, credential exposure or a data-loss path in install, repair or remove.

## 2. Interface changes (my paths)

| Item | Commit | What changed | Regression tests (each fails without its fix, where noted) |
|---|---|---|---|
| B1 readiness parser | `ba360e1` | The marker must start at the first byte: optional `[stdout] `, then `[HH:MM:SS] [Render thread/INFO]: OOZE_READY v1 screen=title engine=<STATE>`, with `<STATE>` in `ENGINE_STATES`. That list is kept in step with `EngineStatus.State` by a test that reads the Java source | `readiness_requires_the_mods_own_anchored_render_thread_line` (a header nested in chat, an unknown state, other threads and streams), `engine_states_match_the_mods_enum`, `chat_text_never_marks_the_title_screen` |
| B2 export diagnostics | `fc12de1` | Wired to `ExportDiagnostics(PathBuf)`: <ul><li>a save dialog off the UI thread, suggesting `ooze-diagnostics-<time>.zip`;</li><li>the path must be absolute `.zip` in an existing folder, and `.zip` is added if missing;</li><li>completion gives `Payload::Exported(path)`, and "Show in folder" opens Explorer with the file selected;</li><li>offered only by a connected backend.</li></ul> The redaction note no longer claims exports drop personal paths; the page says the game log can include the player name and paths | `export_diagnostics_saves_to_the_chosen_zip_and_reveals_it` (a cancelled dialog and a relative path send nothing), `a_completed_export_reports_the_chosen_file_and_a_failure_reports_none`, `export_paths_must_be_absolute_zip_files_in_an_existing_folder`, `over_the_real_backend_unsupported_controls_are_absent` (now expects Export plus the honest note) |
| B3 overflow | `d39636f` | Segmented choices stack full width with wrapped labels when one row doesn't fit. Button rows wrap. When narrow, the header moves its chips under the wordmark | `new_instance_choices_fit_at_the_minimum_window_and_200_percent_scale` (fails without the fix: a choice ran to x=1419 against a card ending at x=768), `no_control_runs_off_screen_at_the_minimum_window_and_200_percent_scale` (every page; also checks the chips never cover the wordmark; fails without the fix) |
| B4 selection agreement | `97e2bba` | "Selected for Play" appears only when the library selection and API 1's saved directory agree. The window reconciles on its own only when safe (connected, idle, no game running, setup done), once per (instance, directory). A failed save is never looped; Select retries | `selection_reconciles_once_and_the_list_agrees_with_play`, `the_first_created_instance_is_ready_for_play_without_another_click` (both fail without the fix) |
| B5 Repair while signed out | `7912864` | The install section treated any selected instance as busy, which disabled Repair. Install and Repair need no sign-in (LAUNCHER_API_V1 l.70). Repair is enabled when Play is pointed at the selected instance and nothing is installing or running. The location stays locked to the instance | `repair_is_available_while_signed_out_for_the_selected_instance` (fails without the fix) |
| B6 sign-in copy | `a77fa06` (test only) | No change was needed. Decline, timeout and callback-bind failures map to the neutral "Sign-in couldn't be completed", keep the safe detail, and offer an enabled "Sign in", which submits a fresh sign-in. Cancel is submitted at once ("Cancelling…"), and the backend checks cancellation every 40 ms | `declined_and_timed_out_sign_ins_are_neutral_and_retryable`, existing `sign_in_shows_browser_guidance_and_cancel_is_acknowledged_by_the_backend` and `sign_in_can_be_retried_after_cancellation_and_reaches_play` |
| B7 typed readiness | `fca9b1b` | `GameReady { id, engine_state }` marks the live session's title once and reports the engine state. On API 1.1 the log parser is not a readiness source; it is kept only for older backends. `api_version()` reports minor 1. When the engine isn't `LOADED`, the Play card says "Ooze Client is off in this game", with the reason and a repair hint | `typed_game_ready_marks_the_title_once_and_reports_the_engine`, `play_card_reports_when_the_engine_is_off_in_the_running_game` |
| Versions | `fca9b1b` | Version bumps (Sol's request):<ul><li>`ooze-launcher-ui` is `0.1.0-rc1`; the `launcher/Cargo.lock` entry was updated (a recorded mechanical change to Sol's lockfile);</li><li>`mod_version=0.1.0-rc1` (root `gradle.properties`), so the JAR is `ooze-client-0.1.0-rc1+mc1.21.11.jar`, which CI's name pattern accepts</li></ul> | `./gradlew build` |

Changed paths:
- `launcher/crates/ooze-launcher-ui/src/{app.rs, app/library_pages.rs, api.rs, state.rs, core_backend.rs, fixtures.rs, widgets.rs, ui_tests.rs, library_ui_tests.rs}`;
- `launcher/crates/ooze-launcher-ui/Cargo.toml`;
- `launcher/resources/lang/en_us.json`;
- `gradle.properties`;
- `launcher/Cargo.lock` (one version line, mechanical);
- this file.

Contracts:
- JNI ABI 1;
- **Launcher API 1.1** (`ExportDiagnostics(PathBuf)`, `GameReady`, `LAUNCHER_API_MINOR_VERSION = 1`);
- Library API 1.

## 3. Checks (Linux x86_64 container)

Interface fixtures are only in `cfg(test)` / `dev-fixtures`. These results say nothing about a live Windows desktop.

| Check | Result | Evidence |
|---|---|---|
| `cd launcher && cargo fmt --all --check` | Pass | Built |
| `cargo clippy --workspace --all-targets -- -D warnings` | Pass | Built |
| `cargo test --workspace --no-fail-fast` | `ooze-launcher-ui` **112 passed, 0 failed, 2 ignored** (the live tests). `ooze-launcher-core` **49 passed, 1 failed**: `auth::tests::localhost_callback_is_served_over_each_available_loopback_family`, because this container has no IPv6 address family (R1, Sol's code; Windows CI unaffected) | Headless-tested (Linux) |
| `cd launcher/crates/ooze-launcher-ui && cargo clippy --all-targets --features dev-fixtures -- -D warnings` | Pass | Built |
| `cargo test --features dev-fixtures` | 112 passed, 0 failed, 2 ignored | Headless-tested (Linux, lavapipe) |
| `cargo test --features dev-fixtures live_ -- --ignored` | **2 passed** (real Modrinth through Sol's real library; live window) | Headless-tested (Linux, real network) |
| `cd native && cargo fmt --all --check`, `cargo clippy -p ooze-ui --all-targets -D warnings`, `cargo test -p ooze-ui` | Pass; 24 passed | Headless-tested (Linux) |
| `./gradlew build` | Pass. Builds `ooze-client-0.1.0-rc1+mc1.21.11.jar` (`fabric.mod.json` version `0.1.0-rc1+mc1.21.11`) | Built |
| `tools/launch-tests.sh` with the release DLL as native resources, mod `0.1.0-rc1` | **6/6 PASS**: `dev-default`, `dev-missing_library`, `dev-incompatible`, `dev-load_failed`, `prod-default` (UNSUPPORTED_PLATFORM on Linux) and `prod-fault-ignored`. Each reached the vanilla title screen and passed the Options round trip (`evidence/CLAUDE-RC1/linux-matrix-SUMMARY.txt`) | **Game-launched (Linux x86_64, OpenJDK 21, Xvfb/Mesa)** |

Interface screenshots (fixture state; they show appearance only):

| File | SHA-256 |
|---|---|
| `evidence/CLAUDE-RC1/ui-engine-off.png` | `46b82e278e8d665be4ea85efd789cc66b3af1bb53051abf3bf6193ad3b9e39af` |
| `evidence/CLAUDE-RC1/ui-new-instance-min-200.png` | `28363af1cb586c2ee69b31fac3d05b7c95eb3398a3fe26c8436415411f6c6c18` |

**Not run here:** anything on Windows, because no Windows machine is attached.

## 4. Windows desktop acceptance checklist (RC1 package hash only)

- **Who:** Sol operates the PC. The owner completes the Microsoft browser sign-in and the Narrator pass.
- **Where results go:** they are folded into `CLAUDE-GATE-L.md` under "Windows results (RC1 `<hash>`)".
- **Rules:**
  - one operator and one Minecraft process at a time;
  - an isolated GUI data root;
  - CLI use always passes its own `--data-dir`;
  - never record account identifiers, tokens or the launch command.

| # | Step | Pass criterion | Evidence |
|---|---|---|---|
| W1 | Verify the package before running anything | The ZIP SHA-256 and every `SHA256SUMS` entry match; no unlisted files | Built |
| W2 | Clean first run | Rename an existing `%LOCALAPPDATA%\OozeLauncher` (never delete it). First run reaches the Play page | Manually verified |
| W3 | Sign-in | <ol><li>Cancel in the launcher: "Sign-in cancelled".</li><li>Sign in again: the owner completes it, and the account chip appears.</li><li>Decline once in the browser: "Sign-in couldn't be completed" with the decline detail, and "Sign in" works again.</li></ol> Record only the browser name, whether the localhost endpoint was reached, and any firewall prompt | Manually verified |
| W4 | Create a Fabric 1.21.11 instance | It is selected for Play without another click. The Instances list ("Selected for Play") and the Play page agree | Manually verified |
| W5 | Install | Real progress. Interrupt the network for the test process only: a retryable error appears, and "Try again" completes the install | Manually verified |
| W6 | Content | Search Modrinth, then install Sodium (or Iris with its required Sodium) through the dependency review. The installed list shows each item with its state | Manually verified |
| W7 | Play | The title screen appears in the game window. The launcher says "reached its title screen" after `GameReady`, with no "Ooze Client is off" banner. The game log shows `Loading Minecraft 1.21.11 with Fabric Loader 0.20.1`, Java 21, `OOZE_READY v1 screen=title engine=LOADED`, and the installed mod loaded | **Game-launched** |
| W8 | Quit the game | "Minecraft closed"; Play is available again | Manually verified |
| W9 | Repair | Seed `saves/` and `options.txt`, then Settings → Repair (also while signed out). Their SHA-256 hashes are unchanged afterwards | Manually verified |
| W10 | Restart the launcher, then Play | No browser opens; the title screen is reached again | **Game-launched** |
| W11 | Diagnostics → Export diagnostics | Save a new `.zip`. It contains only `diagnostics.json` and `game.log`. A credential-pattern scan over all launcher logs and the ZIP finds 0 matches (report counts and file names only). "Show in folder" selects the file | Manually verified |
| W12 | Sign out | Play asks to sign in, and no browser opens until requested | Manually verified |
| W13 | Appearance | Change the accent, restart, and check it persisted | Manually verified |
| W14 | Narrator walkthrough (owner) | Covers Play, Instances, Content, Appearance, Settings, Diagnostics, sign-in and the close dialog. Role and name are announced, the Tab order is logical, and focus is always visible | Manually verified |

Also run the Windows adapter matrix with the RC1 natives: dev and production LOADED, plus the missing, corrupt and ABI fallbacks. Use Sol's PowerShell matrix or `tools/launch-tests.sh` in Git Bash.

## 5. Prism fallback checklist (after Sol publishes the final `.mrpack`)

The pack must contain the RC1 package's JAR; compare SHA-256 values with `SOL-RC1.md`. Prism does its own sign-in; Ooze takes no part in it.

| # | Step | Pass criterion |
|---|---|---|
| P1 | Verify the pack's SHA-256 against `SOL-RC1.md` | Match |
| P2 | Prism → Add Instance → Import → the `.mrpack`, into a new instance | The instance is created with Minecraft 1.21.11 and Fabric Loader 0.20.1 |
| P3 | Instance settings | Java 21 |
| P4 | Sign in with Prism's own Microsoft account flow | Prism shows the account |
| P5 | Launch | Prism's game log shows `Loading Minecraft 1.21.11 with Fabric Loader 0.20.1` and `OOZE_READY v1 screen=title engine=LOADED`. The mod list has `oozeclient 0.1.0-rc1` and Fabric API `0.141.6+1.21.11` (**Game-launched**, Windows x64, Prism) |
| P6 | Options round trip, then quit | Vanilla Options works; the game exits normally |
| P7 | The instance's `mods/ooze-client.jar` | Its SHA-256 equals the RC1 package JAR |

## 6. Gate L decision

Pending `PACKAGE-READY`, then `PACKAGE-VERIFIED`, then `WINDOWS-RESULTS`.
