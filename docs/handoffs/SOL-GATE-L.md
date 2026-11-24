# Sol Gate L Windows backend handoff

Owner Sol/Codex. Executed on 2026-11-24, America/Los_Angeles.
Starting main commit `b348c7d3dd5a6fe44a25c1b191ab9de73c0c89e2`.
Combined source commit `44c407ff88b5fa4638d3bfef3a28acebbd739714` includes
Claude's Gate L changes through `fe0d6704065e64a67e52ecff83798b3d2b2d2738`.
Find this handoff's commit with `git log -1 -- docs/handoffs/SOL-GATE-L.md`.
JNI ABI 1, Launcher API 1 and Library API 1 apply. Launcher API 1 adds
`ExportDiagnostics(PathBuf)` and retains its existing operation events.

**Gate L remains open.** Authenticated CLI Play and restart/relaunch passed on
Windows x64. Desktop first run, browser handoff, cancellation and a fresh
retry were observed. The new desktop request expired after three minutes
without a completed callback. Desktop sign-in and Play, the resulting status
and restart flow, and manual Narrator are outstanding.

## Windows results on the joint test package

Claude independently verified this same package in [CLAUDE-GATE-L.md](CLAUDE-GATE-L.md).
Runtime source `e82b5989ab41269106442b527202e301926874c6`. Windows 11 Pro x64
build 26300, AMD Ryzen 7 9800X3D, Radeon RX 6900 XT driver 32.0.21036.18.
The packaged installer used Temurin 21.0.13+9-LTS x64. Fabric Loader 0.20.1 and
Fabric API 0.141.6+1.21.11 were pinned. Local ZIP
`launcher/dist/ooze-launcher-e82b598.zip`. GitHub Actions artifact 14303633163
in [run 44026232354](https://github.com/kyluua/Ooze-Client/actions/runs/44026232354).

| File | SHA-256 |
|---|---|
| Joint test ZIP, 598 checked files | `731361d9759f31c43d4c379e70ca73d43717f8dcf5293281142ce5eec3e5a543` |
| Desktop launcher | `d7db14db5c7bc34a39976e46d97492adce9b02159cbb92e0903f6f17c090a5ba` |
| CLI | `c18eedcb5735039d31624160b475c27bbb34b07faf99e20e3260ef4d3eb0ed71` |
| Ooze Fabric JAR | `30a48105d552fa9e1e408615ce999e37fb7f3d14958743cae813da5a830ad6d2` |
| Embedded Windows DLL | `e12b118eef886ed70a4efe4930aab58936b80a8659dd29fd960b94016a0fd29b` |

1. **Clean install and recovery.** The packaged CLI installed and verified a
   new Managed Fabric instance in an isolated directory. A process-local
   connection failure published no active install. With a warmed cache and
   the large pinned Minecraft client artifact absent, a localhost TCP tunnel
   closed three transfers after 262,144 upstream bytes each. TLS stayed intact.
   The operation returned Network in Downloading, with no active receipt or
   published partial cache file. After restoring the test process's connection
   settings, retry fetched and verified the pinned client JAR and committed the
   installation. The PC's network settings were left alone.
2. **Repair.** In the first isolated instance, an intentionally damaged Ooze
   JAR was replaced by the exact package hash in a new generation. A seeded
   save fixture and `options.txt` retained their SHA-256 hashes. Managed
   baseline verification reported no issues.
3. **Game-launched, authenticated Play.** The packaged CLI reused the
   November 23 isolated Ooze session through its protected credential store.
   It was pointed at the repaired Fabric instance. The actual Minecraft
   window showed the Fabric title screen, and the launcher log contained:

   ```text
   Loading Minecraft 1.21.11 with Fabric Loader 0.20.1
   OOZE_READY v1 screen=title engine=LOADED
   ```

   Quit Game returned `GameExited code=0`; the CLI exited 0. A new CLI process
   refreshed credentials without another browser prompt, reached the same
   title marker, and quit through the title screen with code 0 again. Both
   redacted launcher logs contain one Fabric version line and one loaded-engine
   marker. Test-profile sign-out cleared the remembered account; subsequent
   Play returned “No usable saved session” before opening the browser or starting
   Java. No account identifiers or launch command are included in this handoff.
4. **Real desktop UI.** The verified desktop executable opened on Windows
   without any policy change by Codex. Its child-only local data root was
   separate from the CLI and the player's usual data. Clean setup reached Play.
   The first Microsoft browser handoff cancelled successfully. The retry
   opened a fresh request and reported the three-minute timeout. An account
   owner must finish browser sign-in for the joint desktop run.
   The desktop created and saved a separate Fabric instance. Signed out, its
   Play page offered Sign in and Settings disabled Repair. The isolated
   launcher is open on Play for the next owner-assisted attempt.
5. **Game-launched, exact JAR product behavior.** Claude's
   `runProductionClientGameTest` with `-PoozeProductionJar` used the joint
   package JAR above and Temurin JDK 21.0.12.1+1 x64. It reached
   `engine=LOADED` and passed the vanilla Options round trip. That run used
   Fabric test credentials. The separate authenticated evidence is item 3.

The CLI and desktop tests each used isolated storage. Original user changes in
the other worktree were preserved.

## Built and Headless-tested successor

Sol changed launcher core authentication, process redaction, diagnostics and
the worker command; the CLI; release packaging and the Windows CI diagnostics
step; and the API/build documentation. Claude's real desktop export control can
use the new backend command when wired. The package's CLI implements it now.

- The callback ignores unrelated or incomplete local requests, answers
  unrelated HTTP requests with 404 and keeps the original deadline. A
  state-validated browser denial reports a specific decline message under the
  existing Authentication code. Regression coverage exercised an empty
  connection, favicon, valid code, invalid state, denial and cancellation.
  The combined package still needs a live desktop retry.
- The diagnostics ZIP includes bounded package metadata and the newest
  launcher-owned game log when available. It reapplies credential redaction and
  preserves an existing destination. Fixture tests confirmed synthetic Bearer,
  session and Xbox identity fields were removed. The rebuilt packaged CLI
  exported the actual CLI relaunch log; the archive contained exactly
  `diagnostics.json` and `game.log`, retained the loaded marker, and both
  live launcher logs and the export had zero matches in the credential-pattern
  scan. The archive is held locally because game output can contain personal
  paths and player names.
- Windows Rust 1.99.1 formatting and workspace Clippy passed. The final
  workspace test run passed 51 backend and 99 UI tests, with two online UI tests
  ignored. The initial parallel UI run faulted in Windows `d3d10warp.dll`;
  a serial DX12 run passed. The existing Java process/redaction test ran with
  Temurin JDK 21.0.12.1+1. The packaged diagnostics CI step also passed locally.
- Gradle `build buildAll` passed with the verified release DLL and the current
  Fabric sources. The Windows adapter matrix passed five cases on Java 21
  (dev and production loaded; missing, corrupt, and ABI mismatch fallback).
  Its production remapped JAR matches the successor JAR hash below.
  Local raw evidence is under `launcher/test-data/gate-l-native-matrix-2026-11-24/`.
- [Windows CI run 45510107357](https://github.com/kyluua/Ooze-Client/actions/runs/45510107357)
  is the automated run for the combined source. The local ZIP below is the exact
  independently checked artifact used for the diagnostics smoke.

Local successor ZIP `launcher/dist/ooze-launcher-gate-l-44c407f.zip`,
14,254,626 bytes. All 598 payload hashes were verified from inside the ZIP.
Built with the preceding package's pinned upstream metadata and verified DLL.

| File | SHA-256 |
|---|---|
| Successor ZIP | `f8273a3dca5fa1f8f7d3f13887482de3a0d430f200528827e43d70b1dfbd0b78` |
| Desktop launcher | `3d30699aab198f4ca707f7784961333d66d8a744dee876c77e44b2bb66180e79` |
| CLI | `a0f2f6ba3ca8972a1b71499bba009df4627e2bf7d0cde41eddb65434bccaacb8` |
| Ooze Fabric JAR | `3c1209018636ae68a100aad8940110a4bfb0bd54d9311d3c1ab918078e8f9fc5` |

## Review and next joint run

Claude's `ba75d41` readiness matcher accepts the actual Windows format and
rejects ordinary chat and non-render-thread messages in the tests. The parser
still searches for the thread header anywhere in a line, so a nested header in
log message text can be misclassified. Claude should anchor the log prefix and
validate the engine state against the contract. C2's typed readiness event would
also prevent dropping readiness with the best-effort log queue; agree on its
contract before changing the shared event enum. C4's cloud game run still needs
a Windows runner that can launch Minecraft. The physical Windows matrix above
provides runtime evidence in this handoff.

Claude should wire the existing Export Diagnostics control in
`core_backend.rs` to `LauncherCommand::ExportDiagnostics(destination)`.
Use a fresh absolute local ZIP path, map successful completion to
`Payload::Exported(path)`, and advertise the capability only when connected.
Inspect the resulting ZIP locally before sharing. Retain the Prism fallback and
keep the M-series outside this gate.

Two additional UI observations on the joint test package for Claude's review.
The New Instance loader choices extend past the card at the tested window
width. The Instances list initially says “Selected for Play” before the game
directory is saved, and the Play page then asks to select the instance again.
Selecting it explicitly cleared that prompt.

For a joint desktop test, agree on one package hash before starting. The ZIP
`731361d9…a543` has both agents' existing runtime evidence; use the successor
when validating the callback and readiness fixes and identify those results
separately. Use one operator at a time, keep the CLI and GUI data roots separate,
and retain normal player data. After the owner completes sign-in, verify the
selected Fabric instance's Play, the loaded title and honest desktop status,
normal exit, restart/relaunch without a browser, sign-out, and manual Narrator.
Record the real desktop install and repair feedback and verify the desktop
diagnostics path after wiring. Append the evidence in the Claude handoff before
claiming Gate L passed.
