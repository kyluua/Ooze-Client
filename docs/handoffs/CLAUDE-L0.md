# CLAUDE-L0 — Ooze Launcher interface, product reconciliation and integration

| Field | Value |
|---|---|
| Owner | Claude (launcher interface, product, accessibility, Fabric integration) |
| Milestone | L0: usable Windows x64 launcher, sign-in to Play (interface half; Sol owns the backend half) |
| Base commit | `d4b77b491fc7d259d77d8e42894df5c4829f2489` (end of C0) |
| Result commit | The commit that adds this file (`git log -1 -- docs/handoffs/CLAUDE-L0.md`), on `main` and `claude/launcher-mvp-ui` |
| Integrated backend | `sol/launcher-mvp-core` @ `13105a9` (merges `a376b21`, `7138865`), reviewed first (§6) |
| Contracts | **Launcher API 1** (Sol, `docs/launcher/LAUNCHER_API_V1.md`); **JNI ABI 1** unchanged; launch readiness marker `OOZE_READY v1` (new, §3) |
| Branch policy | Owner, 2026-11-16: everything combined on **`main`**; both agents work there from now on (AGENTS.md §Branches) |

## 1. Result in one paragraph

The Ooze Launcher has a working **Rust desktop interface** (egui/eframe 0.37 + AccessKit) with the White Ooze / Ink Gel identity:
- first run, Play, Settings and Diagnostics pages;
- an honest close prompt;
- keyboard operation, visible focus, live status announcements, Reduced/Off motion and localised strings.

It is **integrated with Sol's real backend** (`ooze-launcher-core`, API 1) through a tested adapter. The `ooze-launcher` binary opens Sol's `Launcher` with the same configuration as Sol's CLI.

Anything API 1 cannot do is **absent**, not faked:
- re-opening the sign-in browser;
- a Java check button;
- diagnostics export;
- "Follow Windows" motion;
- up-front install status.

The Fabric mod now logs a launch-readiness marker, and the build writes a launcher profile manifest.

**Not achieved:** a real Microsoft sign-in and a launcher-driven game launch. Three things are missing (§8):
1. the project's owned Microsoft client ID, plus Minecraft Services access for it;
2. a packaged `ooze-launcher.exe` on Windows (Sol's packaging);
3. a Windows run with an entitled account.

## 2. Changed paths (Claude)

| Path | Change |
|---|---|
| `launcher/crates/ooze-launcher-ui/**` | New crate:<br>• `api.rs` (the interface's view + `Capabilities`)<br>• `state.rs` (reducer)<br>• `core_backend.rs` (API 1 adapter, `UiPrefs`, `FailedBackend`)<br>• `app.rs` (screens)<br>• `widgets.rs`, `paint.rs` (components, gel material)<br>• `theme.rs` (tokens + contrast)<br>• `motion.rs`, `i18n.rs`, `redact.rs`<br>• `fixtures.rs` (tests/preview only)<br>• `ui_tests.rs` (egui_kittest)<br>• binaries `ooze-launcher` (real) and `ooze-launcher-preview` (`dev-fixtures`, debug only) |
| `launcher/resources/**` | `lang/en_us.json`; `tokens/launcher.tokens.json`; fonts: Bubblegum Sans Regular, Inter Regular/SemiBold, unmodified, with OFL texts and `fonts.json` (sources + SHA-256) |
| `versions/mc1_21_11/**` | `diag/LaunchReadiness` + `LaunchReadinessTest`; the launch test asserts the marker; `launcherProfile` Gradle task; **fixture visibility fix** (`FixtureRenderState` re-winds triangles; §5) with `FixtureRenderStateTest`, gametest-only `FixtureProbeScreen` and pixel checks |
| `tools/launch-tests.sh` | Keeps `OOZE_READY` in the evidence excerpts |
| `AGENTS.md`, `README.md`, `docs/{ARCHITECTURE,TARGET_PLATFORM,IMPLEMENTATION_PLAN,REPOSITORY_STRUCTURE,PRODUCT_VISION,TESTING,INSTALL,INTERACTION_FLOWS}.md` | Independent launcher decision:<br>• ownership (the backend role is Sol's);<br>• renderer rule scoped to the in-game adapter;<br>• launcher auth rules, with the mod/JNI boundary kept;<br>• L-series and Gate L;<br>• ADR-015/016, ARCHITECTURE §19, TARGET_PLATFORM §4 + T9–T11;<br>• INSTALL routes A and B;<br>• TESTING §11;<br>• branch policy `main` |
| `docs/launcher/LAUNCHER_UI.md` | Interface, product and accessibility spec, including how API 1 is adapted (§2.1) |
| `docs/handoffs/CLAUDE-L0-request.md` | Requests to Sol: integration A1–A5, review findings B1–B10, Gate L script |
| `docs/handoffs/evidence/L0/**` | Screenshots (headless and real window), Linux launch matrix excerpts, smoke scripts |
| `.gitignore` | Launcher target directories |

No Sol-owned file was edited. The UI crate carries its own `[workspace]` table until Sol adds it to `launcher/Cargo.toml` (request A2).

## 3. The Fabric side (distribution inputs)

- **Readiness marker.** `OOZE_READY v1 screen=title engine=<STATE>` is logged once when the vanilla title screen first opens, in every engine state. It carries no account, session or path data.
  - It is pinned by `LaunchReadinessTest` and asserted by every launch-test case.
  - The logger prefix differs between dev and production runs, so consumers match the substring.
  - The interface's adapter reads it from API 1 game log lines (best effort). Sol is asked for a typed event (B7).
- **`build/dist/ooze-launcher-profile.json`** (schema `oozelauncher.profile/1`, written by `buildAll`):
  - Minecraft 1.21.11, Java 21, Fabric Loader 0.20.1 + meta profile URL;
  - Fabric API 0.141.6+1.21.11, Maven URL, size 2,426,039, SHA-256 `bdff7fd7…b9ed0f` (verified against Fabric's published `.sha256`);
  - the mod JAR's size and SHA-256.
  - Sol's `prepare-release.ps1` independently freezes its own `release.json`. The profile is a cross-check input, not a second source of truth.

## 4. Test results (evidence vocabulary)

| Check | Result | Evidence |
|---|---|---|
| UI crate: `cargo fmt --check`, `clippy --all-targets --features dev-fixtures -D warnings` | Pass (Linux x64, and the `x86_64-pc-windows-gnu` target) | — |
| UI crate: `cargo test` | **Headless-tested (Linux x64): 53/53** | See the list after this table |
| UI screenshots (software wgpu, lavapipe) | 14 states rendered at 2× | `evidence/L0/headless/*.png` |
| Developer preview in a real window | Launched (Linux x64, Xvfb). Sign-in → install → Play driven with real X11 input; Tab ×5 reached Play with the focus ring; Enter started once; a double click started one game. The backend was **simulated** and the banner says so | `evidence/L0/window/1–5*.png` |
| Real `ooze-launcher` over Sol's real core | Launched (Linux x64, Xvfb). The core refuses non-Windows with a typed `UnsupportedPlatform`, and the window shows "Ooze Launcher needs Windows" with no controls. No data folder was created | `evidence/L0/window/6-real-binary-real-core-linux.png` |
| Windows build of the interface | **Built (Windows x64 PE32+ GUI, cross-compiled with MinGW `x86_64-pc-windows-gnu`; not run)**. `ooze-launcher.exe` SHA-256 `9dde6b24…ffd7b20c`, a local artifact, not distributed. The MSVC target can't build here (no Windows SDK for `ring`) | — |
| Sol's launcher workspace on Linux | `cargo fmt --check` pass; `cargo test` **15/15** (Windows-only tests skipped by Sol's cfg); Clippy fails off-Windows on one unused test import (B10) | — |
| Fabric: `./gradlew build buildAll` | Built + Headless-tested (Linux x64): **28/28 JUnit** (26 + `LaunchReadinessTest` + `FixtureRenderStateTest`) | `build/dist/` |
| Fabric launch matrix | **Game-launched (Linux x64, Xvfb + Mesa): 6/6 PASS** (dev-default, dev-missing_library, dev-incompatible, dev-load_failed, prod-default, prod-fault-ignored). `OOZE_READY v1 screen=title engine=<state>` was logged in every case | `evidence/L0/fabric-launch/` |
| **Windows x64 game gate (C0b)** | **Game-launched (Windows x64) by Sol on 2026-11-15**: dev and prod `LOADED`, probe and fixture data press/release, real missing / corrupt / ABI-2 DLL cases. The fixture was **not visible**; fixed and pixel-verified on Linux (§5). A Windows rerun is needed for the visible part | `evidence/SOL-L0-C0b/` |
| Fixture visibility (shared submit path) | **Game-launched (Linux x64, Xvfb):** probe triangles of both windings read back their exact colours from the screenshot | `evidence/L0/fabric-launch/` |
| Launcher install/repair on Windows | Sol's CI run 43794161997 (`4841084`) succeeded: package → `install` → `repair` → `status` on `windows-latest`. Claude has not run it | Sol's CI |
| Real Microsoft sign-in, Play through the launcher, refresh on relaunch | **Not run.** No owned client ID or Minecraft Services access exists yet | §8 |
| Narrator / NVDA, Windows high contrast, 125–200% DPI | **Not run** (needs a person on Windows) | — |
| Manually verified | **None** | — |

The 53 UI tests cover:
- the reducer: stale events, single Play, cancel, close guard, minimise/restore, session expiry, bounded redacted logs;
- the API 1 adapter mapping: launch-to-session, crash reported once, `OOZE_READY`, error mapping by operation, install ready/needs-repair, ignored stale terminals, settings saves, prefs round trip, profile parsing, real core refusing non-Windows;
- redaction (JWT, `--accessToken`, `M.C…`, headers, user paths), i18n completeness both ways, token parsing and WCAG contrast, motion, geometry;
- 14 AccessKit-driven UI flows: keyboard first run, sign-in and cancel, missing client ID, real progress with values and live status, double-click Play, crash, close prompt, roving radio focus, redacted log and clipboard, Off motion with no repaint loop, every control named, Tab reaching Play, API 1 capability hiding, startup page.

## 5. Review of Sol's Windows C0b evidence, and the fixture fix

The summaries and excerpts show:
- `LOADED` in dev and production;
- a probe round trip in 1,737 µs with the expected vertex and colour;
- 64 fixture frames with the press/release colour change;
- missing DLL → `MISSING_LIBRARY`, corrupt DLL → `INCOMPATIBLE`, ABI 2 → `INCOMPATIBLE`;
- the vanilla Options round trip in all five cases.

That closes the **data, lifecycle and fallback** part of Gate B.

**Sol's review (`13105a9`) was right about the visible part.** The Windows screenshots show no fixture rectangle; the bright blob is the moon, unchanged between released and pressed.

**Root cause, found and fixed by Claude:**
- Vanilla's `RenderPipelines.GUI` keeps back-face culling. Its snippet never calls `withCull(false)`, and the builder defaults to culling. I checked the 1.21.11 bytecode.
- The native fixture's triangles are wound opposite to vanilla's quads, so they were culled.

**The fix and its evidence:**
- `FixtureRenderState` now re-winds every triangle to vanilla's front-facing order, respecting mirroring poses.
- **Game-launched (Linux x64, Xvfb):** a gametest-only `FixtureProbeScreen` draws one triangle of each winding through the shared submit path, and the launch test reads the screenshot.
  - Before the fix, the clockwise triangle was missing and the vanilla-wound one drew.
  - After it, both read back exactly (`FFF8F9FF` / `FF171923`).
- Every launch case now checks those pixels.
- With the DLL on Windows, the released and pressed screenshots must show the native fixture's own colour at its first triangle's centroid.
- `FixtureRenderStateTest` pins the winding detection.

**Gate B's visible-geometry part needs one Windows rerun** of `test-c0-windows.ps1` (request A6).

Other notes:
- The run predates the `OOZE_READY` marker, so it will show on that rerun too.
- The DLL SHA (`79a168b0…`) differs from the CI artifact that C0 bundled (`5d8ae250…`), as expected from S-4 (not reproducible).

## 6. Review of Sol's launcher backend (`2db9c78`, `4841084`, `a8ff31f`)

**Confirmed (secret exposure, paths, launch, truthful states):**
- **No credential crosses API 1.**
  - Bearer-bearing types are private, `Zeroizing` and not `Debug`/`Serialize`.
  - The command is never printed, and the classpath argument file has no secrets.
  - Game output is redacted (exact token plus patterns) before events and log files.
  - Refresh tokens live in Windows Credential Manager, keyed by client ID + data-dir hash; rotation is persisted; there is no plaintext fallback. The UI received only `{username, uuid}`.
- **OAuth:**
  - PKCE S256, random state, prompt `select_account`;
  - a 127.0.0.1 listener with Host, path, single-code and state checks, a 3-minute deadline and cooperative cancel;
  - no client secret, and a missing client ID becomes `Configuration`.
- **Paths:**
  - absolute paths only; no `..`, network shares or reparse points;
  - Windows reserved names rejected; zip-slip and size bombs confined;
  - an HTTPS allow-list with manually validated redirects;
  - size + SHA-1/SHA-256 verification; atomic settings writes.
- **Player data:**
  - a populated non-Ooze folder is never adopted;
  - installs go to `.ooze-launcher/installs/generation-*` and nothing is deleted;
  - a PID + start-time guard blocks repair or relaunch while the game runs.
- **Launch:**
  - Java 21 x64 is validated by running `java -XshowSettings`;
  - production Fabric profile, managed or custom Java, bounded memory;
  - `GameStarted` = spawned (not ready); a nonzero exit becomes `Failed{Process}`; the game outlives the launcher.

**Actionable findings:** B1–B10 in `CLAUDE-L0-request.md`. The most important:
- **B1:** every 403 becomes `ServiceAccess`, and XSTS `XErr` codes are not typed.
- **B2:** the first stray loopback connection aborts sign-in.
- **B3:** every Play re-hashes about 1 GB.
- **B4:** old install generations and the cache double disk use and are never pruned.

None of these leaks credentials or touches player data.

## 7. Limitations (honest list)

- **Interface:**
  - Backend messages for coarse error codes are English only; the interface shows them as the body. Typed sub-codes are requested (B1, B7).
  - `OOZE_READY` detection depends on best-effort log lines, so "reached its title screen" may not appear even when the game did reach it. The interface never claims more than "running" without the line.
  - The UI polls the backend: 100 ms during operations, 250 ms in game (B6).
  - First run accepts any folder. A populated non-Ooze folder is rejected only at the first Install or Play (B7: pre-validation requested).
  - Windows high-contrast and OS text size are not followed. Narrator is unverified. AccessKit trees are built lazily.
  - The log view puts only visible lines in the accessibility tree; Copy log gives everything.
  - The crate-local `Cargo.lock` and its own `[workspace]` table stay until A2.
- **Packaging:** Sol's package contains the CLI only. `ooze-launcher.exe` is not packaged or signed (A3). No installer or self-updater exists.
- **Authentication:** no owned client ID exists, and Minecraft Services access for it is unverified. Sol could not verify the current approval route; developer reports from February 2026 show unapproved apps get HTTP 403 "Invalid app registration". **Nothing here proves a real sign-in.**
- **Fabric:** this environment builds the mod JAR without the DLL (58 KB, `191bb7cf…`). Packaging needs `-PoozeNativeResources` with Sol's release resources; Sol's `check_native_jar` enforces that.

## 8. External blockers (precise)

1. **An owned Microsoft Entra public client ID** registered by the project owner: personal accounts, `http://localhost/callback` loopback redirect, no secret (`docs/launcher/AUTH_SETUP.md`).
2. **Minecraft Services access for that client ID.** Unapproved apps are rejected (T10). The interface shows "Minecraft's services refused this launcher" with no offline fallback.
3. **A Windows x64 run with a legitimately entitled account** and the packaged `ooze-launcher.exe` (A3), for Gate L (`CLAUDE-L0-request.md` §D).

## 9. Artifacts and checksums

| Artifact | SHA-256 |
|---|---|
| `build/dist/ooze-client-0.1.0-alpha.0+mc1.21.11.jar` (Linux build, no DLL; dev/test only) | `191bb7cfa9bc4e08997e9dc81fe98f9f1d4680fb3439186edacaaac55fef6f94` |
| `build/dist/ooze-launcher-profile.json` | `7ca20dcbc731f4aaaa6b297aa3b13e993cbc008d61a92d5c73995ff6a240e6bf` |
| Fabric API 0.141.6+1.21.11 (upstream) | `bdff7fd7e220085cfad2ff9b1f40dde6534ae0b96cf378f97a374bc54cb9ed0f` |
| Fonts | See `launcher/resources/fonts/fonts.json` |
| `ooze-launcher.exe` (MinGW cross-build, not run, not distributed) | `9dde6b240162137be3c02f2ba8c9776956cbf6b7a11eacdca782c7c3ffd7b20c` |

No player-facing launcher release exists yet.

## 10. Exact next tasks

- **Sol (on `main`):**
  - **A1:** review Claude's interface diff.
  - **A6:** rerun the Windows game gate; it now asserts the native fixture is visible and that `OOZE_READY` is logged.
  - **A2:** add `crates/ooze-launcher-ui` to the launcher workspace.
  - **A3:** build, test and package `ooze-launcher.exe` on Windows CI.
  - **A4:** licence notices.
  - Then address **B1–B4**.
  - Publish `SOL-L0.md`.
- **Claude (on `main`), after A2/A3:**
  - remove the crate-local workspace and lock;
  - adopt any typed events Sol adds (B6/B7), deleting the matching adapter workarounds;
  - with the owner's client ID and Minecraft Services access, run Gate L (`CLAUDE-L0-request.md` §D) together with Sol on Windows x64;
  - record the Narrator pass as Manually verified.
- **Owner:**
  - register the public client application;
  - pursue Minecraft Services access for it;
  - provide or perform the entitled-account Windows run.

No gameplay modules or unrelated milestones were started.
