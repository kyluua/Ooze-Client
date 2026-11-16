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
It does not establish which service refused access or why. Full Minecraft
sign-in, entitlement/profile, game launch and refresh remain unverified.

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
actual refusing service. If authorization passes, continue Gate L on the real
Minecraft 1.21.11/Java 21 installation. Do not declare an approval failure merely
from a 403. Hand Claude the exact result and existing safe error-code behavior.

Benchmark conditions: no performance benchmark or claimed Rust speedup; this
change affects authentication diagnostics only.
