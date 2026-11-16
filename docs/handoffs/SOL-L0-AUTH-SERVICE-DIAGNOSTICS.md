# SOL L0 — live Microsoft callback and service diagnostics

Owner: Sol. Date: 2026-11-16 (America/Los_Angeles).
Base: main `2c96d2b`, with the pending SOL-L0-AUTH-CONFIG-RESULT evidence preserved.
Result: resolve with `git log -1 -- docs/handoffs/SOL-L0-AUTH-SERVICE-DIAGNOSTICS.md`.
Launcher API 1 and JNI ABI 1 remain unchanged.

## Observed live attempt

The owner reported that supported accounts was single tenant. The first audience
edit failed with Microsoft's requirement to accept access token version 2. The
owner then reported saving the paired audience/token-version changes. Sol did
not edit the registration or enter any authentication credentials.

A fresh invocation of the verified backend package from 48b53f2 used the same
isolated `launcher/test-data/owner-auth` directory. The owner confirmed that the
Microsoft browser window opened and completed their own sign-in. The CLI moved
past its callback, Microsoft code exchange and protected refresh-token save,
then returned the old generic ServiceAccess error (exit 1). In this source that
error means a downstream HTTP 403, including possibly Xbox user auth or XSTS.
It did not establish which service refused access or why. The diagnostic build
and saved-session retry below now identify that service.

No provider response, authorization URL/code, token, cookie or credential-store
content was collected in this handoff. Do not export the isolated vault entry.

## Narrow diagnostic correction (Claude B1, partial)

The private AuthService enum binds each fixed endpoint to a stage label. Failed
requests expose only that label and numeric HTTP status. Only Minecraft
sign-in/entitlement/profile HTTP 403 maps to ServiceAccess. Xbox user/XSTS failures
map to Authentication. Rate limits/server errors remain Network, and missing
entitlement/profile remains Entitlement. The provider body is discarded.

No public enum variants, fields, commands, credential flows or permissions were
added. Claude's API 1 adapter continues to compile unchanged. Parsing XSTS XErr
into finer account subcodes is still pending; this change does not claim B1's
entire requested improvement. B2–B9 and A1–A4 remain separate integration work.

Changed paths: core `src/auth.rs`, AUTH_SETUP.md, LAUNCHER_API_V1.md and this
handoff. Earlier visible-fixture/configuration evidence is published alongside
this change, without rewriting its historical results.

## Checks and next task

Rust 1.99.1 is installed. Formatting executed locally. Local clippy was attempted
but Windows Application Control again refused dependency build scripts (4551),
so the chained tests/release build did not execute here. No protection was
disabled or bypassed. The Windows CI run will provide actual compile/lint/test
and artifact evidence; record its result before claiming the new CLI tested.

Regression coverage checks stage-specific 403 classification, 401/404 handling,
and that a controlled HTTP response body containing a fake token never reaches
the public diagnostic. Tests use loopback fixtures, not live credentials.

Next: build/test the diagnostic CLI in Windows CI, verify its artifact hashes,
then retry the saved-session path using the same data directory to identify the
actual refusing service. These steps are completed in the result below. Gate L
on the real Minecraft 1.21.11/Java 21 installation remains blocked at Minecraft
sign-in. Do not declare an approval failure merely from a 403. Hand Claude the
exact result and existing safe error-code behavior.

Benchmark conditions: no performance benchmark or claimed Rust speedup; this
change affects authentication diagnostics only.

## Focused review request to Claude (API 1 unchanged)

At base 2c96d2b, `core_backend.rs::map_error` maps every ServiceAccess to
AuthAppNotApproved, and every Authentication during Launch to SessionExpired.
The UI's corresponding copy asserts that the account is fine or that its saved
session expired/revoked. Those conclusions do not follow from the existing API
codes: a successful Microsoft refresh may still encounter an Xbox failure, and
a Minecraft 403 does not establish its cause. The backend now preserves the
safe failing stage/status in `LauncherError.message`.

Claude: use neutral service/authentication copy for these generic API 1 codes
and keep the safe detail visible. Reserve approval-specific or session-expired
claims for a future explicit backend subtype. Adjust the mapper/localization
regressions to cover a refreshed session followed by an Xbox denial and a
Minecraft denial without an identified cause. Do not infer subtypes by parsing
English messages. No peer-owned files were edited by Sol.

## Verified build and live result

The first CI run on ca0698d stopped on a needless-borrow clippy error; no release
or tests were claimed from that run. The one-line fix is main
`f254af50d2e549c7ea0349e4540f2b18bc25ad45`.

**Built / Headless-tested:** [Windows CI 43899705612](https://github.com/kyluua/Ooze-Client/actions/runs/43899705612),
job 133412680396, passed all steps: fmt/clippy, 20 backend tests, release CLI,
5 native Rust tests, real Java 21.0.13+9-LTS JNI harness with 20,323 assertions,
Fabric build/tests, pinned-package repeat and clean managed-Java install/repair.
These tests do not exercise live Minecraft authentication or an in-game window.

Artifact 14240293572, `ooze-launcher-l0-backend-windows-x64`, 3,794,457 bytes.
Archive SHA-256: `d3de091a7d48588e8f1f066e952ad2a5bb6a2c7f0dd8d09ce7a52796af4c8c64`.
CLI SHA-256: `f1585798fc572d702e0d26813ae99cb4d9730939a05c117ad43370fb78e8887e`.
The archive and all 342 files in SHA256SUMS verified locally before execution.
Ignored local locations: `launcher/dist/ooze-launcher-l0-backend-f254af5.zip`
and extracted `launcher/dist/backend-f254af5/`.

On Windows x64, `--version` returned 0.1.0-l0. Running that CLI from its package
directory with the same owner-auth data directory and `play` produced:

```text
Started { id: OperationId(1) }
Progress { id: OperationId(1), stage: Authorizing, completed: 0, total: None }
ServiceAccess: Minecraft sign-in failed (HTTP 403). Check the account and owned application's access to this service.
```

Exit 1. The non-interactive path does not open a browser: it successfully used
the saved Microsoft refresh credential, saved any rotation, and completed Xbox
user authentication and XSTS before Minecraft login_with_xbox refused the
request. Entitlement/profile requests and install/game spawn were not reached.
Status afterward reported account None, no active operation and no game PID.
No game launch, full account sign-in or title-screen readiness is claimed.
The private refresh credential remains in the scoped Windows vault for retry;
it was neither read into tool output nor copied to a repository file.

One initial invocation via a computed executable path returned a CLI argument
error before backend work. The successful diagnostic attempt above used the
literal relative executable path from its extracted package directory.

## Verified request route and exact next task

Sol read the live Microsoft **AppID Review** form through
<https://aka.ms/mce-reviewappid> on 2026-11-16. It is a Microsoft Forms page that
offers new application approval requests. It asks for contact information,
application name/client ID, tenant ID, a project URL, justification and the
owner's EULA/usage-guidelines acknowledgment. No fields were filled and no form
was submitted. AUTH_SETUP.md supplies the owner with a draft justification.

Owner: request review for Ooze Launcher / the configured client ID using the
current form and record the result. A missing app approval is plausible for a
new client, but the observed 403 alone does not establish the precise cause.
If the reviewed app remains denied, ask official support about this exact
Minecraft endpoint and status. Do not repeatedly change unrelated Entra flags.

Sol: after app access is confirmed, repeat the saved-session test and then the
entitlement/profile, Play/readiness and restart checks in Gate L. Claude:
consume the unchanged API 1 behavior, correct the generic-error UI copy above,
and finish the independent Windows desktop package/accessibility integration
(A1–A4). The configured backend package is not the player-facing UI package.
