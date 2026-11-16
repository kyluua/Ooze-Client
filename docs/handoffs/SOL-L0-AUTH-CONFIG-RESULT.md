# SOL L0 — public application build and Windows fixture verification

Owner: Sol. Date: 2026-11-15 (America/Los_Angeles).
Base/configuration commit: `48b53f2d3d0201b1108349f15ef9bbadac83e73c` on main.
Result: this evidence commit; resolve with `git log -1 -- docs/handoffs/SOL-L0-AUTH-CONFIG-RESULT.md`.
Launcher API 1, JNI ABI 1, native library 0.1.0-g0 and capabilities 3 unchanged.

## Public application configuration

The owner-supplied public ID `359e0deb-13e8-4616-978f-0fc1f8e273d2` is in
`launcher/packaging/public-auth.json`. Launcher CI validates and exports it before
compiling; the CLI's existing compile-time fallback therefore embeds it. It is
not a token or client secret. Nothing in this change proves app approval,
Microsoft consent, Minecraft entitlement or a usable live session.

The setup guide includes the actual public ID, development/build commands and
the expected native `http://localhost/callback` redirect. The core auth protocol
and public API were not changed. A runtime environment override remains possible.

**Built / Headless-tested:** [Windows CI run 43819945690](https://github.com/kyluua/Ooze-Client/actions/runs/43819945690)
passed on configuration commit 48b53f2: public GUID setup, fmt/clippy, 19 launcher
tests, release CLI, 5 native Rust tests, actual Java 21 bridge harness (20,323
assertions), Fabric build/tests, pinned-package repeat and clean install/repair.

Configured backend artifact **14204336381**, **3,794,595 bytes**, archive SHA-256
`58ed17c0f9901c7e6b3fd87940a1daada3e1b69926a1d21cef0944ef09970f80`.
It is available from that run and locally at ignored
`launcher/dist/ooze-launcher-l0-backend-48b53f2.zip`. All 342 package files listed
in SHA256SUMS verified; inspection confirmed the supplied public GUID is embedded
in the CLI executable. CLI SHA-256:
`5a0047bf5d41d23352ac8647cc36c90933a98aaaf5d65040db22879ccb5acd8d`.
This is still a CLI/backend package, not the missing player-facing UI executable.

## Claude request A6 — now executed on Windows

Reviewed Claude's fixture fix `299b5ae`: Java re-winds each diagnostic triangle
for the vanilla GUI pipeline, accounting for a mirrored pose, while retaining
the eager-copy lease lifetime and vanilla submission. No second renderer or
unsafe native change was introduced. The gametest now checks screenshot pixels,
not merely the native packet's ARGB value. No peer-owned file was edited.

**Game-launched, Windows x64:** all five cases in
`launcher/packaging/test-c0-windows.ps1` passed on the integrated main:

| Case | Actual result |
|---|---|
| dev-default | LOADED; probe, native fixture's released/pressed pixels, readiness marker and vanilla Options round trip passed |
| prod-default | LOADED; readiness marker, both-winding GUI pixel probe and vanilla round trip passed |
| win-real-missing | MISSING_LIBRARY; readiness/pixel probe and vanilla round trip passed |
| win-real-corrupt | INCOMPATIBLE; readiness/pixel probe and vanilla round trip passed |
| win-real-abi | INCOMPATIBLE; readiness/pixel probe and vanilla round trip passed |

The native fixture centroid at (136,96) read `FFF8F9FF` released and `FF171923`
pressed. The screenshots now visibly contain the rectangle. The readiness log
includes `OOZE_READY v1 screen=title engine=LOADED`. Both GUI triangle windings
read back their expected colors in every case. This closes the previously open
Windows visible-geometry part of C0b/Gate B and Claude's A6 request.

Evidence: `docs/handoffs/evidence/SOL-L0-C0b-visible/` contains the five excerpts,
summary and the two new native-fixture screenshots. Readiness lines were
explicitly collected from the raw logs because the current PowerShell runner's
excerpt filter predates OOZE_READY. The gametests themselves assert readiness.

Conditions: actual Java 21.0.12.1+1-LTS, Windows x64, AMD Ryzen 7 9800X3D,
Radeon RX 6900 XT, driver 32.0.21036.18; production DLL SHA-256
`79a168b0d54cdbb3111c6c2f2a9565d3208ac74640ba70de9349b3b733994fa8`.
These use Fabric test credentials. They are not independent-launcher Microsoft
authentication evidence. **Manually verified:** none; agent image inspection and
automated pixels do not assert a person's sign-off. No performance benchmark or
Rust-versus-Java improvement is claimed.

## Remaining integration and live checks

The public ID removes the missing-configuration prerequisite for configured CLI
builds. Saved portal settings, Microsoft/Xbox/Minecraft access and real
refresh/Play remain unverified until the entitled-account Gate L run. The local
Windows Application Control block on earlier unsigned Rust executables was not
changed or bypassed. **This configured CLI did run locally** (`--version` passed).
This does not establish that local Rust build scripts or a future UI binary run.

A real CLI `sign-in` was started using an isolated, ignored
`launcher/test-data/owner-auth` data directory. It reached `AwaitingBrowser`, then
returned `Authentication: Sign-in timed out after three minutes` without a valid
callback. No successful consent, account, service-access or refresh result was
observed. The owner reported that no browser window opened. Inspection found
only background Edge processes; the Windows helper could not surface an Edge
window. Opening Brave through Computer Use surfaced a Microsoft account page
for this public ID. Its safe error text was `unauthorized_client`: the client
does not exist or is not enabled for consumers. This is a Microsoft identity
registration/audience failure before the Xbox/Minecraft exchange, not proof of
a Minecraft Services approval denial. The exact portal setting is not yet
verified; check the owned app ID and personal-account sign-in audience first.
No credentials were entered by the agent, no security settings were changed,
and no authorization URL, code, token or cookie was copied into repository
evidence. Browser visibility needs a separate recheck on the next attempt.

While this work ran, Claude committed the missing UI entrypoints and wired the
same public-auth.json in `2c96d2b` (merged before publishing this result). A5's
source configuration is therefore present for both consumers. Full UI review,
workspace integration, Windows UI tests/package and licences (A1–A4) remain
before player-facing acceptance. See CLAUDE-L0-request.md. Historical SOL-L0.md
predates the provided ID, merged UI and
this completed visible-fixture rerun; these later handoffs supersede those
specific pending statements, without changing the original test evidence.

Changed paths in this result: this handoff, the status note in
`docs/launcher/BUILD_AND_TEST.md`, and `docs/handoffs/evidence/SOL-L0-C0b-visible/**`.
Native and launcher contracts did not change.
