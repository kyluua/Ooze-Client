# Ooze Launcher — Interface, Product and Accessibility Specification (L0)

> Owner: **Claude** (interface, product, accessibility). The backend contract (`LAUNCHER_API_V1.md`), Microsoft sign-in setup (`AUTH_SETUP.md`) and build/packaging (`BUILD_AND_TEST.md`) in this folder are **Codex's**.
> Owner decision of 2026-11-16: Ooze ships a **fully independent desktop launcher with its own Microsoft sign-in** that installs and directly launches Minecraft Java Edition 1.21.11 with Fabric and Ooze. Sending the player to the official launcher to press Play does not satisfy that decision.

## 1. Scope of L0

The launcher has only what is needed to get from a clean PC to a running, signed-in Minecraft 1.21.11 with Fabric and Ooze. Everything visible is backed by the real backend (**Real or absent**).

| In L0 | Not in L0 (absent, not disabled) |
|---|---|
| First-run install location | Module catalogue, ClickGUI controls, HUD widgets, cosmetics |
| Microsoft sign-in in the system browser; cancel; typed errors; sign-out; switch account | Friends, store, news, telemetry, promotional content |
| Install, verify, repair with real progress; cancel and retry | More than one profile or Minecraft version |
| Play backed by the real Java process; starting, running, title-screen-reached, exited and crashed states | Mod management beyond Fabric API + Ooze |
| Redacted log view, copy log, export diagnostics | An Ooze account or server of any kind |
| Settings: install location, Java (managed or own Java 21), bounded memory, behaviour while playing, motion, interface size | Offline or "demo" play |

**Fixtures** (scripted backends for tests and the developer preview) exist only under `cfg(test)` or the `dev-fixtures` Cargo feature. A release build refuses that feature at compile time, and every fixture screen shows the "Developer preview" banner.

## 2. Architecture

```
launcher/
├── crates/ooze-launcher-ui/    Claude   egui/eframe window: screens, components, focus, i18n, UI state
├── crates/ooze-launcher-core/  Codex    auth (browser + PKCE), Xbox/Minecraft services, entitlement,
│                                        credential storage (Windows DPAPI), installer, Java, process lifecycle
├── crates/ooze-launcher-cli/   Codex    headless driver of the same core
└── resources/                  Claude   fonts (+ OFL texts), lang/en_us.json, tokens/launcher.tokens.json
```

- The interface talks to the backend only through the versioned Rust API (`LAUNCHER_API_V1`, independent of the in-game JNI ABI). Until Codex publishes it, `ooze-launcher-ui/src/api.rs` holds the interface's **proposed** shape (see `docs/handoffs/CLAUDE-L0-request.md`); it then becomes a thin adapter.
- **Non-blocking:** `Backend::submit` accepts or rejects immediately; work runs on backend threads; events arrive through `drain_events`, and the backend wakes the window. The window never waits on network, disk, auth or the game.
- **No credentials in the interface.** The API has no token, code or session field. Accounts are `{ username, uuid }`. Errors are typed with a backend-redacted detail string.
- **Rendering:** the launcher window is ordinary desktop GUI drawn by eframe (wgpu: Direct3D 12 / Vulkan on Windows). The "no raw graphics / Java submits through vanilla pipelines" rule applies to the **in-game** adapter only (ARCHITECTURE §19, ADR-016).

## 3. Screens

| Screen | Shown when | Contents | Primary action |
|---|---|---|---|
| Connecting | Before the backend's first snapshot | Spinner and "Starting Ooze Launcher…" | none |
| Incompatible build | Backend API major ≠ interface major | Explanation and "reinstall" guidance | none |
| First run | `settings.install_root` is unset | Wordmark, one sentence, install location field, Browse…, a note that existing installs are kept | Continue (`SetInstallRoot`) |
| Play | Default | Profile title and versions; account line; install line; one large primary button; operation progress; game state; notices | Derived (§4) |
| Settings | Navigation | Account, Installation (open, change, repair), Java, Memory, While playing, Accessibility, About; explicit **Save changes / Discard** | Save |
| Diagnostics | Navigation, or "View log" | Redaction note, Copy log, Export diagnostics, Open logs folder, Open crash reports, virtualised log view | — |
| Close prompt | Window close while signing in, installing, starting or playing | Consequence in one sentence | **Keep open** (default) / confirm |

Navigation has exactly three items (Play, Settings, Diagnostics); every item is implemented.

## 4. The primary button

Derived only from backend state (`state.rs`, `Model::primary`):

| Backend state | Label | Enabled | Command |
|---|---|---|---|
| No snapshot | Starting Ooze Launcher… | no | — |
| Launch pending or game `Starting` | Starting… | no | — |
| Game `Running` | Playing | no | — |
| Install/repair running | Installing… | no (Cancel beside the progress) | — |
| Sign-in running | Signing in… | no (Open browser again, Cancel) | — |
| No account | Sign in | yes | `SignIn` |
| Not installed / not checked | Install | yes | `Prepare { repair: false }` |
| Needs repair | Repair | yes | `Prepare { repair: true }` |
| Ready | Play | yes | `Play` |

- **Double activation:** the model switches to *Starting…* in the same frame as the submit, so a double click or a held Enter sends one `Play` (unit and UI tests; also checked in a real window).
- **Stale events:** progress or completion for an operation the interface is not tracking, or already saw finish, is ignored. Game events for an older session are ignored.
- **Cancellation:** Cancel marks the operation *Cancelling…*. Only the backend's `Finished { Cancelled }` ends it.
- **Spawned ≠ ready:** the launch operation finishes when Java starts (`Starting`). "Reached its title screen" is shown only after the backend sees the mod's `OOZE_READY v1` log line (§6).

## 5. Errors and notices

Every `ErrorKind` has a localised title and body (`error.<kind>.*`). Actions depend on the kind:
- **Try again:** only when the backend marks the error retryable.
- **Repair:** for checksum failures.
- **Sign in:** for an expired session.
- **Details:** shows the redacted technical detail.
- **Dismiss:** always offered.

Errors persist until dismissed or superseded by a new attempt.

| Configuration and service states (honest, actionable) | Text |
|---|---|
| `AuthNotConfigured`: build without a registered public client ID | "Sign-in isn't set up in this build". It points players to the official release and developers to `AUTH_SETUP.md`. **Not retryable** |
| `AuthAppNotApproved`: Minecraft Services rejects the app registration (new third-party apps need approval) | Says the account is fine and the launcher isn't approved yet. **Not retryable**. No offline fallback |

There is no offline or "demo" substitute for a failed sign-in.

Notices appear **below** the Play card, so the main button never moves under the pointer. Game crashes offer View log, Open crash reports and Dismiss.

## 6. Launch readiness contract (with the Fabric mod)

The Ooze mod logs exactly one line when the vanilla title screen first opens, in every engine state:

```
OOZE_READY v1 screen=title engine=<LOADED|MISSING_LIBRARY|UNSUPPORTED_PLATFORM|INCOMPATIBLE|LOAD_FAILED>
```

The backend scans each line of the game's stdout for the substring `OOZE_READY v1` (the logger prefix differs between development and production runs, for example `[Render thread/INFO]: OOZE_READY v1 …`) and reports `GameState::Running { title_screen: true }`. The line carries no account, session or path data. It is covered by `LaunchReadinessTest`, and asserted by every launch-test case.

## 7. Keyboard

| Key | Effect |
|---|---|
| Tab / Shift+Tab | Next / previous control (header → navigation → page) |
| Enter, Space | Activate the focused button or navigation item |
| ← → ↑ ↓ | Within a choice group (radio group), move the selection. Only the selected option is a Tab stop (roving focus) |
| ← → on the memory slider | Step by 512 MiB |
| Esc | Close the close-prompt (= Keep open) |

- **Focus ring:** shown in keyboard modality (the last input was Tab or an arrow key). It is a `focus` ring with a white halo outside the control, ≥ 3 : 1 against both surfaces (theme test).
- **Window zoom:** egui's Ctrl +/- zoom is disabled so the Interface size setting stays authoritative.

## 8. Accessibility

| Requirement | Implementation | Evidence |
|---|---|---|
| Every control has a role and a name | AccessKit `WidgetInfo` on every component; text fields `labelled_by` their visible label; radio groups `labelled_by` their heading | `every_interactive_widget_has_a_name` (Headless-tested) |
| Progress is announced and has a value | `ProgressIndicator` with 0–100 numeric value; indeterminate work has no value | `install_progress_is_real_cancellable_and_announced` |
| Status changes are announced | One `Status` node with `Live::Polite` in the footer (visible to sighted users too). Step changes, notices, sign-in and game milestones update it; byte counters do not | Headless-tested; Narrator not yet run |
| Contrast | Body text ≥ 4.5 : 1, focus and boundaries ≥ 3 : 1, computed from the theme tokens | `rendered_pairs_meet_contrast_requirements` |
| Not colour alone | Icons (check, info, warning, error) beside every status line; quiet actions are underlined | Screenshots |
| Minimum sizes | Text ≥ 12 pt (caption); controls 38 pt tall (≥ 32 hit target) | Token test |
| Interface size | 100–200% in five steps (egui zoom) | Settings |
| Motion | Follow Windows / Full / Reduced / Off. Reduced: short fades, no ambient drips or sweeps. Off: instant, **no repaint loop** | `motion_off_settles_without_a_repaint_loop` |
| No time limits | No timeouts in the interface; sign-in timeouts come from the backend as a typed error | — |

**Toolkit assessment (egui/eframe 0.37 + AccessKit 0.25, `accesskit_windows` 0.35):**
- **Strengths:**
  - Implements Windows UI Automation (names, roles, toggled state, numeric values, focus, invoke/click, live regions via `UIA_LiveRegionChangedEventId`).
  - Testable headlessly through the same tree (egui_kittest).
  - Maintained.
- **Known limits, recorded for L0:**
  1. The AccessKit tree is built lazily, when an assistive client first asks for it.
  2. Windows high-contrast themes and the OS text-size setting are not followed automatically. Interface size and our contrast-checked tokens partly compensate. A High Contrast theme is future work.
  3. "Show animations in Windows" is not read by egui. The backend reports it (`system_reduced_motion`) so **Follow Windows** works. Until Codex implements it, Follow Windows means Full.
  4. The log view is virtualised. Only visible lines are in the accessibility tree; **Copy log** gives the whole redacted log.
  5. Native folder and file pickers come from `rfd` (the Windows common item dialog), which is accessible by the OS.
- **Not yet verified:** Narrator and NVDA have **not** been run against the launcher. That needs a Windows session and a person (Manually verified).

## 9. Visual design

- Tokens come from `docs/tokens/ooze_classic.theme.json` (colours, radii, outlines, elevation, material) and `launcher/resources/tokens/launcher.tokens.json` (layout, type roles, control metrics, motion durations). Components contain no literal colours, sizes or durations.
- **White Ooze / Ink Gel material:**
  - superellipse corners (exponent 2.6, chord error ≤ 0.15 px);
  - vertical body gradient;
  - gloss band kept above the label and glint;
  - rim light on large surfaces;
  - the ink outline;
  - two-level shadows;
  - decorative drips under the Play card, cycling only with Full motion.
- **Type:** Bubblegum Sans for the wordmark and titles; Inter Regular and SemiBold for everything else. These are upstream static TTFs, unmodified, with their OFL texts (`launcher/resources/fonts/fonts.json` records sources and SHA-256).

## 10. Redaction (defence in depth)

The backend redacts logs and exports. The interface redacts again before display or copy (`redact.rs`):
- the values of `--accessToken`, `--session`, `--xuid`, `--clientId`;
- `access_token`, `refresh_token`, `id_token`, `code`, `client_secret`, `password` and similar keys;
- Bearer and XBL3.0 headers;
- JWTs and Microsoft `M.*_…` codes;
- legacy `token:<…>:<uuid>` strings;
- user names in profile paths.

The game's `--username` (display name) is kept.

## 11. Tests and how to run them

```bash
cd launcher/crates/ooze-launcher-ui
cargo fmt --check
cargo clippy --all-targets --features dev-fixtures -- -D warnings
cargo test                                         # unit + egui_kittest (needs a wgpu adapter; lavapipe works)
OOZE_UI_SCREENSHOT_DIR=/tmp/shots cargo test       # also writes PNGs of 12 states
cargo check --target x86_64-pc-windows-msvc --all-targets --features dev-fixtures
cargo run --features dev-fixtures --bin ooze-launcher-preview -- ready   # developer preview (simulated)
```

Preview scenarios: `first-run`, `signed-out`, `ready`, `auth-not-configured`, `app-not-approved`, `offline`, `crash`.

## 12. Checks that need the real backend, Windows or a person (pending)

| Check | Needs |
|---|---|
| Real Microsoft sign-in, cancel in the browser, consent denied, child account, not entitled | Codex's backend, the owned client ID **and** Minecraft Services approval, a real entitled account |
| Install / verify / repair against Mojang and Fabric servers, network loss mid-download, disk full | Codex's backend on Windows x64 |
| Play → Fabric 1.21.11 under Java 21 → `OOZE_READY` with the real Ooze DLL (`engine=LOADED`) → exit → relaunch with refreshed credentials | Packaged app on Windows x64 + entitled account |
| Narrator / NVDA walkthrough, Windows high-contrast check, 125–200% DPI | A person on Windows |
| Repair keeps `saves/`, `screenshots/`, `resourcepacks/`, `shaderpacks/`, `options.txt` | Codex's installer + a seeded instance |
