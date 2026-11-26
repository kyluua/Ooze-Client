# CLAUDE RC1 — Ooze base and launcher release candidate

Owner: Claude (Fabric integration, launcher interface, product verification). Counterpart: Sol (`SOL-RC1.md`).

## Status log

Newest last. Each line names a commit or package hash.

- 2026-11-25 STARTED `7dd54d73f5897dcd2900241865e8dbac56d271ff`.
  - Running in a Linux x86_64 cloud container; no Windows machine is attached (the Remote Control folder list is empty).
  - Windows operation is Sol's, with the owner for Microsoft sign-in and Narrator.
- 2026-11-25 UI-READY `fa160e9` (last interface code commit; this handoff commit is docs only). Build RC1 from `main` at or after it. Checks are in §3.
  - One Linux-only backend test fails: R1, in Sol's code. It doesn't affect Windows CI.
- 2026-11-25 07:05 UTC CANDIDATE-VERIFIED `b5fc945d4f114915fbb0b69d31ca09b099389f40e442bbd7e58398d5ae674b38`.
  - This is the `ooze-launcher-windows-x64` artifact 14997773226 from green Windows CI run 45559630278, on the UI-READY merge `94ae4ae`.
  - If Sol's `PACKAGE-READY` names this hash, it counts as `PACKAGE-VERIFIED` with no further checks. A different package gets its own verification.
  - Details are in §3a.
  - R1 is still open. If Sol fixes it for RC1, the package changes and needs a new hash.
- 2026-11-25 09:12 UTC WAITING.
  - Sol has pushed nothing since `c7a728d` (07:02 UTC). There is no `PACKAGE-READY` and no `WINDOWS-RESULTS`.
  - `main` is green on Windows CI (runs 45559630278 and 45565077565).
  - **Blocked on:**
    - **Sol:** `PACKAGE-READY`. Candidate `b5fc945d…` is already verified (§3a). Also the R1 decision, then W1–W14 as operator.
    - **The owner:** Microsoft browser sign-in and decline (W3), and Narrator (W14).
  - Automatic polling has stopped; Claude resumes when this session is messaged.
- 2026-11-26 SUPERSEDED-CANDIDATE.
  - Owner decision 2026-11-25: Ooze ships without built-in anti-cheat, and Managed Fair Play is removed. Interface: `06afee8`; library: Sol `1beaf85`; merge: `f08e7ed`.
  - Candidate `b5fc945d…` predates this change, so it is no longer the release candidate. Sol's successor package needs its own `PACKAGE-READY` and §3a verification.
  - Details: `CLAUDE-NO-ANTICHEAT.md`.

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

## 3a. Package verification (candidate from CI run 45559630278)

Downloaded into new empty directories; nothing from the package was executed before its hashes checked out.

| Item | SHA-256 / result |
|---|---|
| ZIP `ooze-launcher-windows-x64` (artifact 14997773226) | `b5fc945d4f114915fbb0b69d31ca09b099389f40e442bbd7e58398d5ae674b38`, matching GitHub's artifact digest |
| `SHA256SUMS` | **599/599** entries verified; no unlisted files. `RELEASE-NOTES.md` is new since `e82b598` |
| `ooze-launcher.exe` (GUI) | `5a1ee2a4cefe09378fae9f1ce6082cea3806c1cc03ca0b6053f815b70acd8066` |
| `ooze-launcher-cli.exe` | `63bbd4c5ec2ae31783cc81f7b66c9697b220dfe8413a3c13069ad5b11c668d7f` |
| `ooze-client.jar` | `5f12e35f474d852c7850b93109d72306c7c318a1fdd087df96a4abfc97d8c160`; `fabric.mod.json` version `0.1.0-rc1+mc1.21.11` |
| Embedded `natives/windows-x86_64/ooze_jni.dll` | `bb019c8704547c73852edc94f54c0b4347d62e6781a59d9d92834cf3da0046f8`, matching `native.properties`: release, ABI 1, `source.commit=94ae4ae…`, `source.dirty=false` |
| JAR ↔ source | A `remapJar` of `94ae4ae` with this DLL gives the same 32 classes, DLL, `native.properties` and `fabric.mod.json`. `oozeclient.mixins.json` and the lang file differ only by Windows line endings |
| Exact JAR in a production Fabric run (`-PoozeProductionJar`) | **Game-launched (Linux x86_64, OpenJDK 21, Xvfb/Mesa).** <ul><li>`Loading Minecraft 1.21.11 with Fabric Loader 0.20.1`</li><li>`Ooze Client 0.1.0-rc1+mc1.21.11 is OFF: linux-x86_64 is not a supported Ooze native platform yet.`</li><li>`OOZE_READY v1 screen=title engine=UNSUPPORTED_PLATFORM`</li><li>`C0 launch test PASSED … vanilla Options round-trip OK`</li></ul> `engine=LOADED` needs Windows (W7) |
| Prism artifact `ooze-base-rc1-prism` (artifact 14998720719) | ZIP digest `532d8c15cef05adf904225e6526623b3a3029011fdf8dc03115c7a2ef66a790a` (matches) |
| `.mrpack` contents | `ooze-base-0.1.0-rc1.mrpack` SHA-256 is `c6845c58df40004492acb9f787d8522e412149bfb7c425484fcf0ba6c0859780`, matching `mrpack-evidence.json`. The index has `formatVersion` 1, `minecraft 1.21.11` and `fabric-loader 0.20.1`, and its one download is Fabric API from `cdn.modrinth.com` (client required, server unsupported) |
| Its `client-overrides/mods/ooze-client.jar` | `5f12e35f…`, the same JAR as the package |
| Its Fabric API, fetched from the CDN | Size 2,426,039 bytes, SHA-1 and SHA-512 match the index. SHA-256 `bdff7fd7e220085cfad2ff9b1f40dde6534ae0b96cf378f97a374bc54cb9ed0f` equals the launcher's pin in `release.json` |

Package **Built**, and its JAR **Game-launched on Linux** as above. Windows acceptance (W1–W14) and Prism import and Play (P1–P7) are still to be run.

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

**OPEN** (09:12 UTC). No W1–W14 evidence exists on an RC1 hash yet.

| Missing | Owner |
|---|---|
| `PACKAGE-READY` naming the RC1 hash (candidate `b5fc945d…` is verified) | Sol |
| W1, W2, W4–W13 on that hash (Windows desktop run) | Sol, as operator |
| W3: browser sign-in, then decline and retry | Owner, with Sol |
| W14: Narrator | Owner |
| P1–P7: Prism import and Play | Owner, or Sol with the owner's Prism account |

R1 (IPv6 callback fallback) is recommended before W3, and would change the hash.
