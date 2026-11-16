# SOL L0 — independent launcher backend

Owner: Codex / Sol. Date: 2026-11-15 (America/Los_Angeles).
Base: `d4b77b491fc7d259d77d8e42894df5c4829f2489`.
Implementation result: `13105a9e4ef3f745a5c41164a4b8c399a19ab10c`;
this handoff and the screenshot-review correction follow in a documentation commit.
Branch: `sol/launcher-mvp-core`. Launcher Rust API **1**; installation schema **1**;
JNI ABI **1**, native library **0.1.0-g0**, production capabilities **3**, unchanged.

Status: backend implementation **Built / Headless-tested** and packaged. The
whole-launcher acceptance remains incomplete for the explicit blockers below.

## Delivered

- Rust launcher workspace/lock with `ooze-launcher-core` and a developer CLI.
  Versioned non-secret API, bounded event queue, background operations, settings,
  operation IDs, cooperative cancellation, single-operation and process guards.
- Owned Microsoft public-client browser/PKCE flow, private Xbox/XSTS/Minecraft
  exchange, entitlement/profile validation, refresh, Windows-protected credential
  storage scoped to app/data root, sign-out and account selection. No client
  secret, password collection, borrowed app ID or offline game identity.
- Frozen game/Fabric metadata, checksum/size verified libraries/assets/mods,
  safe ZIP extraction, atomic cache/generation activation, verified repair,
  managed Java 21 x64 or validated selection, production Fabric process launch,
  child PID/exit monitoring and bounded/redacted logs. Worlds remain outside
  installation generations. No gameplay modules and no new in-game engine scope.
- Portable backend package production, dependency licences, upstream manifest,
  SHA256SUMS, Windows CI and reproducible installation-input instructions.
- Independent review of Claude C0 and real Windows DLL/fallback gametests. The
  screenshot review found a visible-fixture validation gap (below).

Changed paths: `launcher/**`, `.github/workflows/launcher-l0.yml`,
`docs/launcher/**`, `docs/handoffs/SOL-L0*.md` and
`docs/handoffs/evidence/SOL-L0-C0b/**`. No Claude-owned Fabric/product/UI file was
edited. The workspace remains separate from native/. Rust stays pinned at 1.99.1.

## Executed evidence

| Evidence | What actually ran |
|---|---|
| **Built / Headless-tested, Windows CI** | Implementation `13105a9`: fmt, clippy with warnings denied, **19 backend tests**, release CLI, 5 native Rust tests, real Java 21 bridge harness, Gradle mod assembly, identical pinned package repeat, clean managed-JRE installation and repair with Cargo/Gradle/JDK paths removed. [Run 43795981631](https://github.com/kyluua/Ooze-Client/actions/runs/43795981631) passed |
| **Built / Headless-tested, local Windows** | Existing Fabric build/buildAll under actual Java 21.0.12.1+1-LTS; 26 adapter unit tests and 20,323 native bridge assertions passed using the production G0 artifact |
| **Game-launched, local Windows** | Five real Fabric gametests: dev/prod LOADED; real missing DLL MISSING_LIBRARY; real corrupt DLL and ABI 2 manifest INCOMPATIBLE. All reached the title screen and passed vanilla Options round trips. Native probe and CPU frame-colour checks passed |
| **Manually verified** | None. No human sign-off is inferred from automation or agent screenshot inspection |

The 19 tests include interrupted/corrupt/cancelled transfer preservation,
malformed paths/metadata, OAuth state/denial/cancellation/refresh errors,
service-error redaction, unrelated-instance protection, live PID/start-time
guards, repeated Play rejection, atomic settings, and actual Java 21 processes.

The process fixture is a private Java 21 test program, launched twice from paths
with spaces with a real nonzero exit and token-like output. It is not Minecraft
and is not a release offline-login path. CI install/repair is separate from a
real entitled sign-in/Play test. No newer JVM is substituted for Java 21.

Local conditions: Windows x64; AMD Ryzen 7 9800X3D, Radeon RX 6900 XT,
driver 32.0.21036.18; G0 DLL SHA-256
`79a168b0d54cdbb3111c6c2f2a9565d3208ac74640ba70de9349b3b733994fa8`.
No game FPS, JNI overhead, frame-time or Rust-versus-Java performance benchmark
was conducted. These are correctness gates, not performance improvement claims.

## Artifact

Final backend package: artifact **14194567979**, run **43795981631**,
source **13105a9**, **3,792,399 bytes**, archive SHA-256
`ca1152f7333e4a3a1b8c2dc53440a3e2be72b3858869b0a9acde0301efb99e7a`.
[GitHub artifact](https://github.com/kyluua/Ooze-Client/actions/runs/43795981631/artifacts/14194567979).
Downloaded locally to ignored `launcher/dist/ooze-launcher-l0-backend-13105a9.zip`;
all **342** files listed in its SHA256SUMS verified. It contains the real mod,
CLI and frozen download inputs, not Minecraft's game files or a desktop UI.
CLI SHA-256: `5a2d87243a216acf6539db78796c4f11336a2f3dc8e7fa98c3f5cac5958903b0`.
Release manifest SHA-256: `19850544c5e41e7c51fbf186a4fa2ebc72bda503525dbdc55ea95f0c85e3b029`.
Mod JAR SHA-256: `e518aae0b53fcf6b162d36e65c1bf70243fce234a2bffe9e48468d71620120a0`.
Two local frozen-input repeats matched all 343 content files including SHA256SUMS;
the final CI repeated the comparison successfully. This reuses identical prebuilt
PE/JAR inputs, not a claim of reproducible recompilation across machines.
Managed runtime input: Temurin JRE 21.0.13+9 Windows x64, SHA-256
`2332c0b4cff6bed722da3567c91d5e47568bcba863fa4945d0a8d6fc76158628`.

See `docs/launcher/BUILD_AND_TEST.md`, `AUTH_SETUP.md`, `INSTALLATION_V1.md` and
`LAUNCHER_API_V1.md` for commands, manifest/ownership decisions and limits.

## Limitations and blockers

1. **No live authentication evidence:** no owned OAuth client ID was configured.
   Current Minecraft Services access approval remains unverified; the official
   support entry returned no readable policy. Microsoft consent, actual token
   exchange/refresh and direct authenticated game launch/relaunch remain pending
   an owned authorized public client ID and an entitled interactive account test.
2. **Windows Application Control:** this host rejects newly compiled Rust build
   scripts (OS error 4551) and the downloaded unsigned CI launcher executable.
   No policy/AV protection was disabled or bypassed. The executable ran in Windows
   CI; local packaged-launcher execution remains blocked. Signing/trusted
   distribution must be addressed before claiming this host can run the launcher.
3. **Claude UI/integration pending:** no Claude L0 desktop UI commit/handoff was
   available on the integration branch when last inspected. This is a backend
   review package, not a finished player-facing launcher.
4. **Visible native fixture remains unverified:** the existing gametest asserts
   native lastArgb/framesRendered, but released/pressed screenshots show no
   diagnostic rectangle. Claude must investigate rendering order/submission and
   add a pixel assertion before closing the visible-geometry gate. DLL loading,
   native data and vanilla fallback are proven separately. Review details are in
   `SOL-L0-CLAUDE-C0-REVIEW.md`; no peer files were silently changed.
5. L0 retains old generations/cache, has no automatic updater/pruning/disk quota,
   supports one game/Fabric target and Windows x64 only, and does not claim
   bit-for-bit PE reproducibility or isolation from malicious same-user software.

## Exact next task and main integration

Claude: take the backend implementation result above and API 1, create the Rust
desktop crate under `launcher/crates/ooze-launcher-ui`, and request its workspace
membership. Consume backend commands/events, never tokens. Add actual account,
install/repair/progress, settings and Play/exit/error UI with Ooze identity and
accessibility. Reconcile product/AGENTS authentication scope; fix the fixture
evidence gap in your owned adapter/tests. Publish `CLAUDE-L0.md` with exact
result commit and actual evidence. Do not start gameplay modules.

Sol then reviews the UI/integration diff for secrets, event races, cancellation,
launch state, thread/render/memory rules and packages the combined app. Configure
the owned public OAuth ID/access and perform the entitled sign-in → direct Play
→ title screen → exit → launcher restart → refreshed Play acceptance run.

The owner's later instruction explicitly requests combining both reviewed pieces
into **main** when ready, then doing subsequent work on main. Preserve all
concurrent changes and coordinate file ownership there. No merge into main has
been performed while the UI/auth acceptance is incomplete; main is not replaced
with this backend half.
