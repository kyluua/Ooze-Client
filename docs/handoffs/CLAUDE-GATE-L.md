# CLAUDE Gate L — player-facing acceptance with the verified Windows x64 package

- **Owner:** Claude (Fabric integration, launcher interface, product verification).
- **Date:** 2026-11-24 (session in a Linux x86_64 cloud container).
- **Base:** `b348c7d3dd5a6fe44a25c1b191ab9de73c0c89e2` (main after Sol's 2026-11-23 sign-in record).
- **Fixes:** `ba75d41a255e3466664cf8d928e952d65be95918`.
- **Result:** this file's commit (`git log -1 -- docs/handoffs/CLAUDE-GATE-L.md`).
- **Contracts:** JNI ABI 1, Launcher API 1, Library API 1 (all unchanged).

## Summary

**Gate L is not passed.** Every check that needs Windows x64 is still open:
- the production mod loading its engine (`engine=LOADED`);
- the desktop launcher with real sign-in, install, repair, Play and exit;
- Narrator.

No Windows machine could be attached to this session (the Remote Control folder list was empty), and no CI job launches the game on Windows. This handoff records:
- the checks done here and their exact evidence;
- one readiness bug found in review and fixed;
- the review findings for Codex;
- the joint Windows protocol, so the remaining steps can be run once, on one package, by one operator at a time.

The CLI's verified sign-in on 2026-11-23 (`SOL-L0-AUTH-RECHECK-2026-11-23.md`) is the only live authentication evidence. Nothing here claims sign-in, Play or readiness on Windows.

## 1. Package under test

Codex's verified candidate (`SOL-L1-END-TO-END.md`):
- source commit `e82b5989ab41269106442b527202e301926874c6`;
- GitHub artifact **14303633163** (`ooze-launcher-windows-x64`) from run 44026232354.

I downloaded it in this session and checked, before anything else:

| Item | SHA-256 | Check |
|---|---|---|
| ZIP | `731361d9759f31c43d4c379e70ca73d43717f8dcf5293281142ce5eec3e5a543` | Matches Codex's record and GitHub's artifact digest |
| `SHA256SUMS` | — | **598/598** entries verified; no unlisted files |
| `ooze-launcher.exe` (GUI) | `d7db14db5c7bc34a39976e46d97492adce9b02159cbb92e0903f6f17c090a5ba` | Matches |
| `ooze-launcher-cli.exe` | `c18eedcb5735039d31624160b475c27bbb34b07faf99e20e3260ef4d3eb0ed71` | Matches; same binary as the 2026-11-23 sign-in |
| `ooze-client.jar` | `30a48105d552fa9e1e408615ce999e37fb7f3d14958743cae813da5a830ad6d2` | Matches |
| `natives/windows-x86_64/ooze_jni.dll` (inside the JAR) | `e12b118eef886ed70a4efe4930aab58936b80a8659dd29fd960b94016a0fd29b` | Matches `native.properties`: release variant, `source.commit=e82b598…`, `source.dirty=false`, ABI 1 |

**JAR ↔ source check.** A build of this checkout with the same DLL produces a JAR whose:
- 32 classes, DLL, `native.properties` and `fabric.mod.json` are byte-identical to the package JAR;
- text resources differ only by Windows line endings;
- licence notices differ only because the package also carries Codex's notice files.

So the Linux launch matrix below exercised the same code that ships.

`fabric.mod.json` requires `fabricloader >=0.20.1`, `minecraft ~1.21.11`, `java >=21` and `fabric-api`.

## 2. Results

Evidence words follow AGENTS.md. "Not run" means no evidence exists; it is listed in §5.

### 2.1 Fabric mod (task item 1)

| Check | Result | Evidence |
|---|---|---|
| Exact package JAR in a production Fabric run (Linux x86_64, OpenJDK 21.0.12, Xvfb/Mesa) | **Game-launched (Linux):**<br>• Minecraft 1.21.11 with Fabric Loader 0.20.1 loaded `oozeclient 0.1.0-alpha.0+mc1.21.11`<br>• the engine correctly reported `UNSUPPORTED_PLATFORM` (the DLL is Windows-only)<br>• vanilla title screen<br>• `OOZE_READY v1 screen=title engine=UNSUPPORTED_PLATFORM`<br>• the vanilla Options screen opened and closed<br>• the honest "Ooze Client is off" notice<br>• no Ooze key mappings or screens registered | `evidence/CLAUDE-GATE-L/linux-package-jar.excerpt.txt`; `linux-package-jar-title.png` (`589de1a1…f3d3f3`). Run with the new `-PoozeProductionJar` (§3) |
| Native-load failure leaves vanilla usable: `tools/launch-tests.sh` with the package's release DLL as native resources | **Game-launched (Linux), 6/6 PASS:**<br>• `dev-default`<br>• `dev-missing_library`<br>• `dev-incompatible`<br>• `dev-load_failed`<br>• `prod-default`<br>• `prod-fault-ignored` (release builds ignore developer fault injection)<br><br>Each reached the title screen with the expected state; the Options round trip passed. | `linux-matrix-SUMMARY.txt`, `linux-*.excerpt.txt`, `linux-dev-load_failed-title.png` (`93c0bf72…ffb55a`) |
| Engine **LOADED** on Windows x64 / Java 21 with this JAR | **Not run** | §5 W1 |
| Real-DLL failures on Windows (missing / corrupt / ABI mismatch) | **Not run** (the script runs them automatically on Windows Git Bash) | §5 W1 |

### 2.2 Desktop launcher (task item 2)

The desktop window was not opened on Windows. The flows were exercised headlessly through the AccessKit tree (egui_kittest, lavapipe), over the fixture API 1 backend and Sol's real library:

| Flow | Result | Tests |
|---|---|---|
| Browser sign-in: Cancel, then sign in again | **Headless-tested (Linux)**: the cancel is acknowledged; a second SignIn is submitted and reaches the signed-in state | `sign_in_shows_browser_guidance_and_cancel_is_acknowledged_by_the_backend`, **new** `sign_in_can_be_retried_after_cancellation_and_reaches_play` |
| Install progress, network failure, retry | **Headless-tested (Linux)** | `install_progress_is_real_cancellable_and_announced` |
| Repair feedback (Settings → Repair: progress on Play, completion; states what repair never touches) | **Headless-tested (Linux)** | **new** `repair_from_settings_reports_progress_and_completion` |
| Play for the selected instance (directory saved first; wrong-instance protection) | **Headless-tested (Linux)** | `selecting_an_instance_points_play_at_its_game_directory`, `failed_selection_does_not_change_the_play_directory`, `failed_directory_save_blocks_play_and_selection_can_be_retried`, `an_empty_or_archived_library_cannot_play_the_previous_game_directory`, `instance_selection_is_disabled_while_the_game_is_live_and_recovers_on_exit` |
| Game start, readiness, exit | **Headless-tested (Linux)** | `double_clicking_play_starts_one_game_and_states_follow_the_process`, `launch_ends_at_spawn_and_the_game_is_followed_as_a_session`, **new** `readiness_requires_the_mods_own_render_thread_line`, **new** `chat_text_never_marks_the_title_screen` |
| Closing while Minecraft runs | **Headless-tested (Linux)** | `closing_while_minecraft_runs_asks_first`, `closing_waits_for_a_library_cancellation_or_committed_operation_to_finish` |
| Keyboard navigation, visible focus, names | **Headless-tested (Linux)** | `keyboard_focus_is_visible_and_tab_reaches_the_main_action`, `every_interactive_widget_has_a_name`, `every_library_widget_has_a_name_on_every_page`, `library_pages_are_reachable_with_the_keyboard`, `settings_radio_groups_use_arrow_keys_and_save_through_the_backend` |
| Logs redacted | **Headless-tested (Linux)** | `logs_shown_and_copied_are_redacted`; core `credentials_redacted_even_in_crash_output` |
| Windows Narrator | **Not run** | §5 W4 |

Totals on this checkout (Linux):
- `launcher` workspace `cargo test --workspace`: **145 passed, 0 failed, 2 ignored**;
- the 2 ignored live tests run separately with `--ignored` and **pass** (real Modrinth search, Iris + Sodium install with SHA-512, live window);
- `cargo fmt --all --check` and `cargo clippy --workspace --all-targets -D warnings`: clean;
- `./gradlew build`: success.

## 3. Changes in my paths (`ba75d41`)

- **Readiness fix** (`launcher/crates/ooze-launcher-ui/src/core_backend.rs`).
  - Before: the API 1 adapter marked "reached its title screen" on any game line *containing* `OOZE_READY v1`. A chat message (`[CHAT] <x> OOZE_READY v1 …`) or another mod could therefore produce a false readiness claim. A Vanilla instance, where the mod never logs the marker, was exposed most.
  - Now: `is_ready_marker` requires the marker to be the whole message of a `[Render thread/INFO]` line, optionally with Fabric's `(logger)` tag.
  - Tests cover chat, other threads and levels, embedded text and malformed states.
  - **The tested package `e82b598` still has the old matcher.** For a Fabric instance the real marker arrives before any chat is possible, so Gate L on that package is not affected. The next package carries the fix.
- **Tests:** `sign_in_can_be_retried_after_cancellation_and_reaches_play` and `repair_from_settings_reports_progress_and_completion` (`ui_tests.rs`).
- **`-PoozeProductionJar=<path>`** (`versions/mc1_21_11/build.gradle.kts`): runs the production game test against an exact packaged JAR and logs its SHA-256.
- **Mod description** (`fabric.mod.json`) no longer says the mod is "loaded by the official Minecraft Launcher"; it names the launcher that starts it. This changes only the next JAR.
- **Evidence:** `docs/handoffs/evidence/CLAUDE-GATE-L/`.

## 4. Review of Codex's changes (`6d72250..e82b598` launcher core/UI/CLI, plus `b348c7d` docs)

**Wrong-instance launches: no defect found.**
- Selection is two-phase. Library `Select` must succeed, then API 1 `UpdateSettings` with the instance's `game_dir`.
- `model.settings` changes only on the backend's completion event. `selection_ready()` enables Play/Install only when the saved directory equals the selected instance's.
- Selection is blocked while either side is busy or the game is live. Archived or empty selections can't fall back to the previous directory.
- The backend launches `saved.settings.instance_dir` captured at operation start, and derives Vanilla vs Fabric+Ooze from that instance's `instance.json`. The prepared install must contain `mods/ooze-client.jar` exactly when the target includes Ooze.

**Premature readiness: one defect (mine, fixed above).**
- A spawned process maps to "Minecraft is running." Only the marker maps to "reached its title screen." Exit code 0 maps to Exited, otherwise Crashed.
- Readiness is still inferred by the interface from a log line. See C2.

**Exposed credentials: no defect found in the diff.**
- The library never receives credentials.
- Game stdout/stderr pass through the core `Redactor` (exact session token plus header, token-name and JWT patterns) before logs or events. The UI redacts again for display and copy.
- CLI additions (`appearance`, `worlds`) print library data only.
- Credentials are keyed per data directory (SHA-256 of the canonical path plus client ID), so separate test directories never share a session.

**Requests to Codex** (backend; not edited by me):

| # | Severity | Finding | Request |
|---|---|---|---|
| C1 | Medium (Gate L step 2) | **L0 finding B2 is still open.** `wait_callback` returns the result of the *first* loopback connection. A browser's speculative pre-connect (no request; 2 s read timeout), a favicon fetch or a port probe fails an otherwise valid sign-in. `error=access_denied` (the player declines in the browser) can't be told apart from other failures | Answer 404 to anything other than `GET /callback?…` and keep waiting until the deadline. Fail only on a callback with a wrong state. Add a typed consent-declined error; the UI already has neutral sign-in copy and will map it |
| C2 | Low–medium | Readiness depends on the UI parsing best-effort `LogLine` events (L0 B7) | Emit a typed `GameReady { session }` from the core using the same strict rule as `is_ready_marker`, with guaranteed delivery. Keep `LogLine` for display |
| C3 | Process | The GUI has never been opened on a Windows machine. Your handoff records Windows Application Control blocking the unsigned package locally | Arrange the legitimate execution-trust path (signing or administrator-approved trust) so the owner can run `ooze-launcher.exe`. Don't bypass policy |
| C4 | Low (CI evidence) | No CI job game-launches on Windows | Add a windows-latest step after "Assemble real Ooze mod": `./gradlew :versions:mc1_21_11:runProductionClientGameTest -PoozeProductionJar=<package>/ooze-client.jar -PoozeExpectEngineState=LOADED`, plus `tools/launch-tests.sh` in Git Bash with `OOZE_NATIVE_RESOURCES` set to the package natives. That gives **Game-launched (Windows x64, CI)** evidence for W1 without an account |
| C5 | Info | The push of `ba75d41` builds a new package candidate in CI | Verify it as usual before it replaces `e82b598`. Gate L evidence stays tied to whichever package hash was actually run |

## 5. Joint Windows Play test: protocol and remaining manual checks

**Coordination rules (both agents and the owner):**
1. **One package.** Everyone uses the ZIP with SHA-256 `731361d9…a543` (artifact 14303633163). Verify `SHA256SUMS` before running anything, and don't rebuild. If a newer candidate replaces it, restart the protocol with the new hash and say so in the results.
2. **Separate directories, never the same one at once.**
   - **Codex/CLI** always passes `--data-dir <isolated dir>`, as in the 2026-11-23 run. The CLI's default is `%LOCALAPPDATA%\OozeLauncher`, which is the GUI's directory, so omitting `--data-dir` would share the GUI's instances, locks and session.
   - **GUI** uses `%LOCALAPPDATA%\OozeLauncher`. For the clean first run, rename an existing folder to `OozeLauncher.backup-<date>`; never delete it, because it may hold worlds.
3. **One operator at a time.**
   - Codex runs the CLI Play first: sign-in reuse, then install, then Play to `OOZE_READY`, then exit.
   - Codex records the result in a SOL handoff before anyone starts the GUI.
   - The GUI run (the owner, following W2–W5) happens only after Codex reports "done".
   - No two Minecraft processes run at the same time during the test.
4. **Results.** Append them to this file under "Windows results" (or reference the SOL handoff), with:
   - the package hash;
   - Windows build;
   - Java version from the game log;
   - exact log lines;
   - the evidence word for each step.

   **Never paste tokens, account IDs or the raw launch command.**

**Remaining checks (Windows x64 only):**

| # | Step | Pass criterion | Evidence word |
|---|---|---|---|
| W1 | Mod loads the bundled engine | The game log shows `OOZE_READY v1 screen=title engine=LOADED`, with Java 21 on the `Loading Minecraft 1.21.11 with Fabric Loader 0.20.1` line. Options opens and closes. On Git Bash, `tools/launch-tests.sh` passes all cases including `win-real-missing_library`, `win-real-corrupt` and `win-real-abi-mismatch` | Game-launched |
| W2 | GUI clean first run, then browser sign-in: cancel once in the launcher, then sign in again | "Sign-in cancelled", then signed in with the account chip. Also try declining in the browser (expect a neutral error and a retry path, C1) | Manually verified |
| W3 | Install; disconnect the network mid-download; reconnect; **Try again** | Real progress; the network error is retryable; the install completes | Manually verified |
| W4 | Seed `saves/` and `options.txt` in the selected instance, then **Settings → Repair** | Progress and completion. The seeded files are unchanged (compare hashes before and after) | Manually verified |
| W5 | **Play** the selected Fabric 1.21.11 instance | The window shows "Minecraft is running and reached its title screen." only after the game log has the marker with `engine=LOADED` | Game-launched |
| W6 | Quit Minecraft | The launcher shows "Minecraft closed" and Play is available again | Manually verified |
| W7 | Restart the launcher and **Play** | No browser opens (refreshed session); reaches the title screen again | Game-launched |
| W8 | Token search | **0 matches**, using the PowerShell command below. Report counts and file names only | Manually verified |
| W9 | Narrator: Play, Instances, Content, Appearance, Settings, Diagnostics, sign-in and the close dialog | Every control is announced with role and name; Tab order matches the visual order; focus is always visible | Manually verified |

W8 command, run after W5–W7:

```powershell
Get-ChildItem "$env:LOCALAPPDATA\OozeLauncher" -Recurse -File -Include *.log,*.txt,*.json |
  Select-String -List -Pattern 'eyJ[A-Za-z0-9_-]{8,}\.','(?i)access[_-]?token["'':=\s]+[^\s",}]{8}','(?i)refresh[_-]?token','(?i)authorization:\s*bearer' |
  Select-Object -ExpandProperty Path
```

## 6. Unchanged boundaries

- **Prism fallback:** untouched. The MP0 modpack route (`SOL-MP0-CLAUDE-REQUEST.md`) still uses the same Fabric production producer. Only the next JAR's description text changes.
- **No new modules in Gate L.** The Profile page stays hidden until `runtime_modules` is non-empty. No M-series code was added.
- **No offline authentication, borrowed client ID or other launcher's credentials.**

## 7. Exact next tasks

- **Codex:**
  - C1 (callback robustness, consent-declined error) and C4 (Windows CI game launch for W1).
  - Then the CLI Play to `OOZE_READY` in an isolated `--data-dir` on package `731361d9…`.
  - Report under the protocol above.
- **Owner:** after Codex's report, run W2–W9 with the GUI on the same package (C3 must allow running it), and record the results here.
- **Claude:**
  - map C1's typed consent error and C2's `GameReady` when they land;
  - review the successor package;
  - fold the Windows results into this handoff;
  - mark Gate L passed only when W1–W9 have their evidence.
