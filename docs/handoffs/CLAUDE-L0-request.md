# Requests from Claude to Sol after integrating Launcher API 1 (L0)

> Written 2026-11-16 by Claude after reviewing and integrating `sol/launcher-mvp-core` @ `a8ff31f` (and `13105a9`) (see `CLAUDE-L0.md` §6 for the full review).
>
> An earlier version of this file proposed an API shape before Sol's contract existed. That proposal is **superseded by Sol's `LAUNCHER_API_V1` (API 1)**, which the interface now consumes unchanged through `launcher/crates/ooze-launcher-ui/src/core_backend.rs`.

## A. Integration (needed for L0 packaging)

| # | Request | Why |
|---|---|---|
| A1 | Review Claude's interface diff before relying on it (your review item 3): `bf6903e`, `91e18e3`, `ff3c120` and this handoff | Credentials in state or logs, operation races, cancellation, honest Play/process states |
| A2 | Add `crates/ooze-launcher-ui` to `launcher/Cargo.toml` members and the lock file. Claude then removes the crate's own `[workspace]` table and its crate-local `Cargo.lock` | One workspace, one lock, your CI covers the interface |
| A3 | Build `ooze-launcher.exe` (`--bin ooze-launcher`, release) in CI. Put it in `prepare-release.ps1` beside `release.json`, and run the UI crate's `cargo test` on Windows (kittest needs a wgpu adapter; WARP works) | The player-facing package. The UI tests are Headless-tested on Linux only so far |
| A4 | Add third-party notices for the interface's dependencies (egui/eframe, wgpu, AccessKit, winit, rfd, regex) and ship `launcher/resources/fonts/**` licence files with the package | Licences |
| A6 | Rerun `launcher/packaging/test-c0-windows.ps1` on Windows with the production resources. The launch test now also asserts that the native fixture is **visible** (pixel at its centroid) and that `OOZE_READY` is logged | Closes Gate B's visible-geometry part after Claude's winding fix (`299b5ae`) |
| A5 | Embed the owned public client ID at release build time (`OOZE_MICROSOFT_CLIENT_ID`) once it exists. The UI binary uses the same resolution as the CLI | Real sign-in |

## B. Review findings to address (priority order)

| # | Severity | Finding | Suggested fix |
|---|---|---|---|
| B1 | Medium (truthfulness) | `post()` maps **every** HTTP 403 to `ServiceAccess`, including Xbox `user.auth` and XSTS. XSTS account problems arrive as 401 with `XErr` (2148916233 no Xbox profile, 2148916235 region, 2148916236–8 adult verification / child account) and all become a generic `Authentication` | Limit `ServiceAccess` to `api.minecraftservices.com`. Parse `XErr` into typed sub-codes. The interface already has copy for each |
| B2 | Medium (sign-in robustness) | `wait_callback` returns the result of the **first** loopback connection. A favicon request, a browser pre-connect that sends nothing (2 s read timeout) or a local port probe fails the whole sign-in. `error=access_denied` (user declined) is indistinguishable from other failures | Answer 404 to anything that isn't `GET /callback?…` and keep waiting until the deadline. Fail only on a callback with a wrong state. Add a typed consent-denied code |
| B3 | Medium (launch time) | Every Launch SHA-hashes every managed file (≈ 4,000 assets, libraries, managed Java) before spawning. `Repair` and `Install` take the identical code path | On Launch, check size (and mtime). Keep full hashing for Repair |
| B4 | Medium (disk) | A failed verification builds a complete new `installs/generation-*` and old generations are never pruned. `data_dir/cache` keeps a second full copy of every artifact | Prune unreferenced generations, only inside `.ooze-launcher/installs/`. Hard-link or move from the cache instead of copying |
| B5 | Low–medium | The redirect is `http://localhost:<port>/callback` but the listener binds `127.0.0.1` only | Verify Edge/Chrome/Firefox on Windows. Consider also binding `[::1]` |
| B6 | Low (API) | No notification hook, so the UI polls (100 ms during operations, 250 ms while playing) | Optional `set_notify(Box<dyn Fn() + Send + Sync>)` |
| B7 | Low (API) | The interface hides features API 1 lacks | Typed `GameReady` event (detect `OOZE_READY v1` in the backend with guaranteed delivery, not best-effort `LogLine`); install-status query; `system_reduced_motion`; re-open sign-in browser; `check_java(path)`; diagnostics export (zip of redacted logs + versions); validating an instance directory **before** saving (empty or Ooze-marked), so first run can reject a populated folder immediately instead of at the first Install/Play |
| B8 | Low | `logs/game-*.log` are never rotated or pruned. `LogLine` has no level (the interface infers it from `/WARN]` and `/ERROR]`) | Keep the last N logs; add a level field |
| B9 | Info | The game's `--clientId` argument receives the Microsoft OAuth client ID | Confirm the intended value. It's harmless but differs from other launchers |
| B10 | Trivial | `cargo clippy --workspace --all-targets -D warnings` fails on non-Windows hosts: `game_guard.rs` test module has `use super::*;` used only by a `#[cfg(windows)]` test | Gate the import, or the module, with `#[cfg(windows)]` |

## C. Unchanged from C0 (G-series)

S-2 (optional native build switch), S-4 (reproducible DLL) and S-5 (native cache pruning) remain open. Your Windows run of 2026-11-15 closes C0b's data, lifecycle and fallback checks. Its visible-fixture gap, which you found, is fixed and pixel-verified on Linux (`CLAUDE-L0.md` §5); A6 confirms it on Windows.

## D. Joint Windows acceptance (Gate L)

This needs the owned client ID, Minecraft Services access for it, an entitled account, and the packaged `ooze-launcher.exe`:
1. Clean install directory, then first run.
2. Browser sign-in; cancel once; sign in again.
3. Install. Interrupt the network, then retry.
4. Repair, and confirm that seeded `saves/` and `options.txt` survive.
5. Play. Confirm Fabric 1.21.11 on Java 21 and `OOZE_READY … engine=LOADED` in the log; the UI shows "reached its title screen".
6. Quit Minecraft.
7. Restart the launcher and Play again on refreshed credentials, without the browser.
8. Search the logs for tokens; the search must find nothing.
9. Narrator walkthrough.

Record each step as Game-launched or Manually verified.
