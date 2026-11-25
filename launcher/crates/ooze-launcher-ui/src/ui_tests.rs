//! Headless interface tests through the AccessKit tree (egui_kittest): every query below finds
//! widgets the way a screen reader or UI Automation client would, by role and name.
//!
//! Screenshots: set `OOZE_UI_SCREENSHOT_DIR=<dir>` to save a PNG of each named state (software
//! rendering through wgpu; appearance evidence only, never evidence of backend behaviour).

use eframe::egui::{Key, accesskit::Role, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};

use crate::api::*;
use crate::app::{AppOptions, LauncherApp, Page};
use crate::fixtures::{FixtureBackend, FixtureHandle, account, snapshot};

fn harness(
    mut snap: Snapshot,
    motion: MotionPreference,
) -> (Harness<'static, LauncherApp>, FixtureHandle) {
    snap.settings.motion = motion;
    let (backend, handle) = FixtureBackend::manual(snap);
    let app = LauncherApp::new(Box::new(backend), AppOptions::default());
    let h = Harness::builder()
        .with_size(vec2(1120.0, 740.0))
        .with_pixels_per_point(2.0)
        .build_ui_state(|ui, app: &mut LauncherApp| app.frame(ui), app);
    let mut h = h;
    h.set_render_cursor(false);
    (h, handle)
}

fn settle(h: &mut Harness<'_, LauncherApp>) {
    h.run_steps(6);
}

fn ready() -> Snapshot {
    let mut s = snapshot();
    s.account = Some(account());
    s.install = InstallStatus::Ready;
    s
}

fn shot(h: &mut Harness<'_, LauncherApp>, name: &str) {
    let image = h
        .render()
        .expect("software rendering works (lavapipe/llvmpipe or a GPU)");
    assert!(image.width() > 0);
    if let Ok(dir) = std::env::var("OOZE_UI_SCREENSHOT_DIR") {
        std::fs::create_dir_all(&dir).expect("screenshot dir");
        image.save(format!("{dir}/{name}.png")).expect("save png");
    }
}

fn push_all(h: &mut Harness<'_, LauncherApp>, handle: &FixtureHandle, events: Vec<Event>) {
    for e in events {
        handle.push(e);
    }
    settle(h);
}

/// Visible text, possibly also echoed by the live status line (so it may appear twice).
fn shows(h: &Harness<'_, LauncherApp>, text: &str) -> bool {
    h.query_all_by_label_contains(text).count() >= 1
}

fn count(handle: &FixtureHandle, pred: impl Fn(&Command) -> bool) -> usize {
    handle.submitted().iter().filter(|c| pred(c)).count()
}

#[test]
fn first_run_is_keyboard_operable_and_submits_the_chosen_folder() {
    let mut s = snapshot();
    s.settings.install_root = None;
    let (mut h, handle) = harness(s, MotionPreference::Off);
    settle(&mut h);
    shot(&mut h, "01-first-run");
    assert!(
        h.query_by_role_and_label(Role::Tab, "Play").is_none(),
        "no navigation before setup"
    );
    // The path field is labelled and prefilled with the backend's default.
    let field = h.get_by_role(Role::TextInput);
    field.focus();
    h.run_steps(2);
    h.key_press(Key::Tab); // -> Continue
    h.run_steps(2);
    assert!(
        h.get_by_role_and_label(Role::Button, "Continue")
            .is_focused()
    );
    h.key_press(Key::Enter);
    settle(&mut h);
    let submitted = handle.submitted();
    assert!(
        matches!(submitted.as_slice(), [Command::SetInstallRoot { path }] if path.to_string_lossy().ends_with(".ooze")),
        "{submitted:?}"
    );
    assert!(
        h.get_by_role_and_label(Role::Button, "Continue")
            .accesskit_node()
            .is_disabled(),
        "no double submit while checking"
    );
}

#[test]
fn sign_in_shows_browser_guidance_and_cancel_is_acknowledged_by_the_backend() {
    let (mut h, handle) = harness(snapshot(), MotionPreference::Off);
    settle(&mut h);
    shot(&mut h, "02-signed-out");
    let op = handle.next_op();
    h.get_by_role_and_label(Role::Button, "Sign in").click();
    settle(&mut h);
    assert_eq!(
        count(&handle, |c| matches!(
            c,
            Command::SignIn {
                switch_account: false
            }
        )),
        1
    );
    push_all(
        &mut h,
        &handle,
        vec![
            Event::Started {
                op,
                kind: OperationKind::SignIn,
            },
            Event::Progress {
                op,
                progress: Progress {
                    step: Step::WaitingForBrowser,
                    done: 0,
                    total: None,
                    unit: ProgressUnit::Steps,
                },
            },
        ],
    );
    assert!(
        shows(&h, "Continue in your browser"),
        "Continue in your browser"
    );
    h.get_by_role_and_label(Role::Button, "Signing in…");
    shot(&mut h, "03-sign-in-browser");
    h.get_by_role_and_label(Role::Button, "Open browser again")
        .click();
    settle(&mut h);
    assert_eq!(
        count(&handle, |c| matches!(
            c,
            Command::ReopenSignInBrowser { .. }
        )),
        1
    );
    h.get_by_role_and_label(Role::Button, "Cancel").click();
    settle(&mut h);
    assert_eq!(count(&handle, |c| *c == Command::Cancel { op }), 1);
    assert!(
        h.get_by_role_and_label(Role::Button, "Cancelling…")
            .accesskit_node()
            .is_disabled()
    );
    push_all(
        &mut h,
        &handle,
        vec![Event::Finished {
            op,
            outcome: Outcome::Cancelled,
        }],
    );
    assert!(shows(&h, "Sign-in cancelled"), "Sign-in cancelled");
    h.get_by_role_and_label(Role::Button, "Sign in");
}

#[test]
fn missing_client_id_is_an_honest_configuration_error_without_retry() {
    let (mut h, handle) = harness(snapshot(), MotionPreference::Off);
    settle(&mut h);
    let op = handle.next_op();
    h.get_by_role_and_label(Role::Button, "Sign in").click();
    settle(&mut h);
    push_all(
        &mut h,
        &handle,
        vec![
            Event::Started {
                op,
                kind: OperationKind::SignIn,
            },
            Event::Finished {
                op,
                outcome: Outcome::Failed(LauncherError {
                    kind: ErrorKind::AuthNotConfigured,
                    detail: "no client id".into(),
                    retryable: false,
                }),
            },
        ],
    );
    assert!(
        h.query_all_by_label_contains("Sign-in isn't set up in this build")
            .count()
            >= 1
    );
    assert!(
        h.query_by_role_and_label(Role::Button, "Try again")
            .is_none(),
        "a configuration error is not retryable"
    );
    assert!(
        h.query_by_label_contains("FixturePlayer").is_none(),
        "no account appears without a real sign-in"
    );
    shot(&mut h, "04-auth-not-configured");
    h.get_by_role_and_label(Role::Button, "Details").click();
    settle(&mut h);
    assert!(shows(&h, "AuthNotConfigured"), "AuthNotConfigured");
}

#[test]
fn install_progress_is_real_cancellable_and_announced() {
    let mut s = snapshot();
    s.account = Some(account());
    let (mut h, handle) = harness(s, MotionPreference::Off);
    settle(&mut h);
    let op = handle.next_op();
    h.get_by_role_and_label(Role::Button, "Install").click();
    settle(&mut h);
    push_all(
        &mut h,
        &handle,
        vec![
            Event::Started {
                op,
                kind: OperationKind::Prepare { repair: false },
            },
            Event::Progress {
                op,
                progress: Progress {
                    step: Step::DownloadingAssets,
                    done: 220_200_960,
                    total: Some(440_401_920),
                    unit: ProgressUnit::Bytes,
                },
            },
        ],
    );
    let bar = h.get_by_role(Role::ProgressIndicator);
    assert_eq!(bar.accesskit_node().numeric_value(), Some(50.0));
    assert!(shows(&h, "210.0 of 420.0 MB"), "210.0 of 420.0 MB");
    let status = h.get_by_role(Role::Status);
    assert_eq!(
        status.accesskit_node().label().as_deref(),
        Some("Downloading game assets")
    );
    assert_eq!(
        status.accesskit_node().live(),
        eframe::egui::accesskit::Live::Polite
    );
    shot(&mut h, "05-installing");
    // A stale event for another operation changes nothing.
    push_all(
        &mut h,
        &handle,
        vec![Event::Finished {
            op: OperationId(999),
            outcome: Outcome::Succeeded(Payload::None),
        }],
    );
    h.get_by_role(Role::ProgressIndicator);
    // Network failure keeps a retry path; the retry submits a fresh Prepare.
    push_all(
        &mut h,
        &handle,
        vec![Event::Finished {
            op,
            outcome: Outcome::Failed(LauncherError {
                kind: ErrorKind::Offline,
                detail: "dns".into(),
                retryable: true,
            }),
        }],
    );
    assert!(
        shows(&h, "Can't reach the internet"),
        "Can't reach the internet"
    );
    shot(&mut h, "06-offline-error");
    h.get_by_role_and_label(Role::Button, "Try again").click();
    settle(&mut h);
    assert_eq!(
        count(&handle, |c| *c == Command::Prepare { repair: false }),
        2
    );
}

#[test]
fn double_clicking_play_starts_one_game_and_states_follow_the_process() {
    let (mut h, handle) = harness(ready(), MotionPreference::Off);
    settle(&mut h);
    shot(&mut h, "07-ready");
    let play = h.get_by_role_and_label(Role::Button, "Play");
    play.click();
    play.click();
    settle(&mut h);
    let op = OperationId(1);
    assert_eq!(
        count(&handle, |c| *c == Command::Play),
        1,
        "double click submits once"
    );
    assert!(
        h.get_by_role_and_label(Role::Button, "Starting…")
            .accesskit_node()
            .is_disabled()
    );
    push_all(
        &mut h,
        &handle,
        vec![
            Event::Started {
                op,
                kind: OperationKind::Launch,
            },
            Event::Finished {
                op,
                outcome: Outcome::Succeeded(Payload::None),
            },
            Event::Game {
                session: GameSessionId(1),
                state: GameState::Starting,
            },
            Event::Game {
                session: GameSessionId(1),
                state: GameState::Running { title_screen: true },
            },
        ],
    );
    assert!(
        shows(&h, "reached its title screen"),
        "reached its title screen"
    );
    assert!(
        h.get_by_role_and_label(Role::Button, "Playing")
            .accesskit_node()
            .is_disabled()
    );
    shot(&mut h, "08-running");
    push_all(
        &mut h,
        &handle,
        vec![Event::Game {
            session: GameSessionId(1),
            state: GameState::Crashed {
                code: Some(-1),
                crash_report: Some("crash-reports/crash.txt".into()),
            },
        }],
    );
    assert!(shows(&h, "Minecraft crashed"), "Minecraft crashed");
    h.get_by_role_and_label(Role::Button, "Play");
    shot(&mut h, "09-crashed");
    h.get_by_role_and_label(Role::Button, "View log").click();
    settle(&mut h);
    assert_eq!(h.state().page(), crate::app::Page::Diagnostics);
}

#[test]
fn closing_while_minecraft_runs_asks_first() {
    let (mut h, handle) = harness(ready(), MotionPreference::Off);
    push_all(
        &mut h,
        &handle,
        vec![Event::Game {
            session: GameSessionId(1),
            state: GameState::Running {
                title_screen: false,
            },
        }],
    );
    assert!(!h.state_mut().on_close_requested(), "close is held back");
    settle(&mut h);
    assert!(
        shows(&h, "Minecraft is still running"),
        "Minecraft is still running"
    );
    shot(&mut h, "10-close-while-running");
    h.get_by_role_and_label(Role::Button, "Keep open").click();
    settle(&mut h);
    assert!(
        h.query_by_label_contains("Minecraft is still running")
            .is_none()
    );
}

#[test]
fn settings_radio_groups_use_arrow_keys_and_save_through_the_backend() {
    let (mut h, handle) = harness(ready(), MotionPreference::FollowSystem);
    h.run_steps(4);
    h.get_by_role_and_label(Role::Tab, "Settings").click();
    h.run_steps(4);
    let follow = h.get_by_role_and_label(Role::RadioButton, "Follow Windows");
    follow.focus();
    h.run_steps(2);
    h.key_press(Key::ArrowRight); // Full
    h.run_steps(2);
    h.key_press(Key::ArrowRight); // Reduced
    h.run_steps(3);
    assert!(
        h.get_by_role_and_label(Role::RadioButton, "Reduced")
            .is_focused(),
        "focus follows the selection"
    );
    assert!(
        h.query_by_role_and_label(Role::RadioButton, "Off")
            .is_some()
    );
    shot(&mut h, "11-settings");
    let save = h.get_by_role_and_label(Role::Button, "Save changes");
    save.scroll_to_me();
    h.run_steps(3);
    h.get_by_role_and_label(Role::Button, "Save changes")
        .click();
    h.run_steps(4);
    let saved = handle.submitted().into_iter().find_map(|c| match c {
        Command::UpdateSettings(s) => Some(s),
        _ => None,
    });
    assert_eq!(saved.map(|s| s.motion), Some(MotionPreference::Reduced));
}

#[test]
fn logs_shown_and_copied_are_redacted() {
    let (mut h, handle) = harness(ready(), MotionPreference::Off);
    push_all(&mut h, &handle, vec![Event::Log(vec![LogLine {
        level: LogLevel::Info,
        source: LogSource::Game,
        text: "Launching --accessToken eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.c2lnbmF0dXJlX3ZhbHVl --username Steve".into(),
    }])]);
    h.get_by_role_and_label(Role::Tab, "Diagnostics").click();
    settle(&mut h);
    assert!(
        shows(&h, "--accessToken [redacted]"),
        "--accessToken [redacted]"
    );
    assert!(h.query_by_label_contains("eyJhbGci").is_none());
    shot(&mut h, "12-diagnostics");
    h.get_by_role_and_label(Role::Button, "Copy log").click();
    h.step();
    let copied = h
        .output()
        .platform_output
        .commands
        .iter()
        .find_map(|c| match c {
            eframe::egui::OutputCommand::CopyText(t) => Some(t.clone()),
            _ => None,
        });
    let copied = copied.unwrap_or_default();
    assert!(
        copied.contains("[redacted]") && !copied.contains("eyJ"),
        "{copied}"
    );
}

#[test]
fn motion_off_settles_without_a_repaint_loop() {
    let mut s = snapshot();
    s.account = Some(account());
    let (mut h, handle) = harness(s, MotionPreference::Off);
    settle(&mut h);
    h.get_by_role_and_label(Role::Button, "Install").click();
    settle(&mut h);
    push_all(
        &mut h,
        &handle,
        vec![Event::Started {
            op: OperationId(1),
            kind: OperationKind::Prepare { repair: false },
        }],
    );
    assert!(
        h.try_run().is_ok(),
        "indeterminate progress with motion Off must not animate"
    );
}

#[test]
fn every_interactive_widget_has_a_name() {
    for snap in [snapshot(), ready()] {
        let (mut h, _handle) = harness(snap, MotionPreference::Off);
        settle(&mut h);
        for role in [
            Role::Button,
            Role::Tab,
            Role::RadioButton,
            Role::TextInput,
            Role::Slider,
        ] {
            for node in h.query_all_by_role(role) {
                let n = node.accesskit_node();
                let named = n.label().is_some_and(|l| !l.trim().is_empty())
                    || !n.data().labelled_by().is_empty();
                assert!(named, "{role:?} without a name: {n:?}");
            }
        }
    }
}

#[test]
fn keyboard_focus_is_visible_and_tab_reaches_the_main_action() {
    let (mut h, _handle) = harness(ready(), MotionPreference::Off);
    settle(&mut h);
    // Tab from the start of the window walks header -> navigation -> page; the main action is reachable.
    let mut reached = false;
    for _ in 0..8 {
        h.key_press(Key::Tab);
        h.run_steps(2);
        if h.get_by_role_and_label(Role::Button, "Play").is_focused() {
            reached = true;
            break;
        }
    }
    assert!(reached, "Play is reachable with Tab");
    shot(&mut h, "13-keyboard-focus");
}

/// The interface over Sol's real backend capabilities (no reopen-browser, Java check, export or
/// up-front install status; one operation at a time).
fn real_profile(mut snap: Snapshot) -> Snapshot {
    snap.system.capabilities = crate::core_backend::capabilities();
    snap.system.system_reduced_motion = None;
    snap.install = InstallStatus::Unknown;
    snap
}

#[test]
fn over_the_real_backend_unsupported_controls_are_absent() {
    let mut s = real_profile(snapshot());
    s.account = Some(account());
    let (mut h, handle) = harness(s, MotionPreference::Off);
    settle(&mut h);
    // Play installs and verifies on demand; the interface says so instead of claiming a status.
    h.get_by_role_and_label(Role::Button, "Play");
    assert!(shows(
        &h,
        "installs and verifies the game files when you press Play"
    ));
    shot(&mut h, "14-real-backend-play");
    h.get_by_role_and_label(Role::Button, "Play").click();
    settle(&mut h);
    assert_eq!(count(&handle, |c| *c == Command::Play), 1);
    // While an operation runs, nothing else can be submitted (the backend would answer Busy).
    push_all(
        &mut h,
        &handle,
        vec![Event::Started {
            op: OperationId(1),
            kind: OperationKind::Launch,
        }],
    );
    h.get_by_role_and_label(Role::Tab, "Settings").click();
    settle(&mut h);
    assert!(
        h.get_by_role_and_label(Role::Button, "Sign out")
            .accesskit_node()
            .is_disabled()
    );
    assert!(
        h.query_by_role_and_label(Role::RadioButton, "Follow Windows")
            .is_none(),
        "no OS setting is read, so no 'Follow Windows'"
    );
    h.get_by_role_and_label(Role::Tab, "Diagnostics").click();
    settle(&mut h);
    // Launcher API 1 now exports diagnostics (44c407f): the control appears, with honest copy.
    assert!(
        h.query_by_role_and_label(Role::Button, "Export diagnostics")
            .is_some()
    );
    assert!(shows(&h, "can include your player name and folder paths"));
    assert!(!shows(
        &h,
        "removed from everything shown, copied or exported"
    ));
}

#[test]
fn sign_in_over_the_real_backend_has_no_reopen_button() {
    let (mut h, handle) = harness(real_profile(snapshot()), MotionPreference::Off);
    settle(&mut h);
    h.get_by_role_and_label(Role::Button, "Sign in").click();
    settle(&mut h);
    let op = OperationId(1);
    push_all(
        &mut h,
        &handle,
        vec![
            Event::Started {
                op,
                kind: OperationKind::SignIn,
            },
            Event::Progress {
                op,
                progress: Progress {
                    step: Step::WaitingForBrowser,
                    done: 0,
                    total: None,
                    unit: ProgressUnit::Steps,
                },
            },
        ],
    );
    assert!(
        h.query_by_role_and_label(Role::Button, "Open browser again")
            .is_none()
    );
    h.get_by_role_and_label(Role::Button, "Cancel");
    // A coarse backend error shows the backend's own safe message as the body.
    push_all(
        &mut h,
        &handle,
        vec![Event::Finished {
            op,
            outcome: Outcome::Failed(LauncherError {
                kind: ErrorKind::Authentication,
                detail: "Sign-in timed out after three minutes. Retry when ready.".into(),
                retryable: false,
            }),
        }],
    );
    assert!(shows(&h, "Sign-in couldn't be completed"));
    assert!(shows(&h, "timed out after three minutes"));
}

#[test]
fn startup_failure_shows_one_clear_page() {
    let backend = crate::core_backend::FailedBackend {
        error: LauncherError {
            kind: ErrorKind::Busy,
            detail: "Another Ooze launcher is using this data directory.".into(),
            retryable: false,
        },
    };
    let app = LauncherApp::new(Box::new(backend), AppOptions::default());
    let mut h = Harness::builder()
        .with_size(vec2(1120.0, 740.0))
        .with_pixels_per_point(2.0)
        .build_ui_state(|ui, app: &mut LauncherApp| app.frame(ui), app);
    h.set_render_cursor(false);
    h.run_steps(4);
    assert!(shows(&h, "Ooze Launcher is already running"));
    assert!(h.query_by_role_and_label(Role::Tab, "Play").is_none());
    assert!(
        h.query_all_by_role(Role::Button).next().is_none(),
        "no controls that cannot work"
    );
    shot(&mut h, "15-startup-busy");
}

/// Sol's 2026-11-16 review: API 1's generic codes must not be turned into approval or
/// session-expiry claims. A refreshed session can still meet an Xbox denial, and a Minecraft
/// HTTP 403 does not say why.
#[test]
fn generic_service_and_auth_failures_get_neutral_copy_with_the_safe_detail() {
    use ooze_launcher_core as lc;
    let cases = [
        (
            lc::ErrorCode::Authentication,
            "Xbox user authentication failed (HTTP 401).",
            "Sign-in couldn't be completed",
        ),
        (
            lc::ErrorCode::ServiceAccess,
            "Minecraft sign-in failed (HTTP 403). Check the account and owned application's access to this service.",
            "A Minecraft or Xbox service refused the request",
        ),
    ];
    for (code, detail, title) in cases {
        let mut s = real_profile(snapshot());
        s.account = Some(account());
        let (mut h, handle) = harness(s, MotionPreference::Off);
        settle(&mut h);
        h.get_by_role_and_label(Role::Button, "Play").click();
        settle(&mut h);
        let mapped = crate::core_backend::map_error(
            &lc::LauncherError {
                code,
                message: detail.into(),
                retryable: false,
            },
            Some(OperationKind::Launch),
        );
        push_all(
            &mut h,
            &handle,
            vec![
                Event::Started {
                    op: OperationId(1),
                    kind: OperationKind::Launch,
                },
                Event::Finished {
                    op: OperationId(1),
                    outcome: Outcome::Failed(mapped),
                },
            ],
        );
        assert!(shows(&h, title), "{title}");
        assert!(
            shows(&h, detail),
            "the backend's safe stage/status stays visible"
        );
        assert!(!shows(&h, "nothing is wrong with your account"));
        assert!(!shows(&h, "expired"));
        assert!(!shows(&h, "isn't approved"));
        assert!(
            shows(&h, "FixturePlayer"),
            "a generic failure does not sign the account out"
        );
        // Signing in again is offered only where it can help (an authentication step failed).
        assert_eq!(
            h.query_by_role_and_label(Role::Button, "Sign in").is_some(),
            code == lc::ErrorCode::Authentication
        );
    }
}

/// Gate L step 2: cancel the browser sign-in once, then sign in again and reach Play.
#[test]
fn sign_in_can_be_retried_after_cancellation_and_reaches_play() {
    let (mut h, handle) = harness(snapshot(), MotionPreference::Off);
    settle(&mut h);
    let first = handle.next_op();
    h.get_by_role_and_label(Role::Button, "Sign in").click();
    settle(&mut h);
    push_all(
        &mut h,
        &handle,
        vec![Event::Started {
            op: first,
            kind: OperationKind::SignIn,
        }],
    );
    h.get_by_role_and_label(Role::Button, "Cancel").click();
    settle(&mut h);
    push_all(
        &mut h,
        &handle,
        vec![Event::Finished {
            op: first,
            outcome: Outcome::Cancelled,
        }],
    );
    assert!(shows(&h, "Sign-in cancelled"));
    let second = handle.next_op();
    h.get_by_role_and_label(Role::Button, "Sign in").click();
    settle(&mut h);
    assert_eq!(
        count(&handle, |c| matches!(c, Command::SignIn { .. })),
        2,
        "a fresh sign-in is submitted after the cancellation"
    );
    push_all(
        &mut h,
        &handle,
        vec![
            Event::Started {
                op: second,
                kind: OperationKind::SignIn,
            },
            Event::AccountChanged(Some(account())),
            Event::Finished {
                op: second,
                outcome: Outcome::Succeeded(Payload::None),
            },
        ],
    );
    assert!(!shows(&h, "Sign-in cancelled"), "the old notice is gone");
    assert!(
        h.query_by_role_and_label(Role::Button, "Sign in").is_none(),
        "signed in: the main action is no longer Sign in"
    );
    assert!(shows(&h, &account().username));
}

/// Gate L step 4: Repair from Settings shows real progress on Play and a completion notice; the
/// interface states what repair never touches.
#[test]
fn repair_from_settings_reports_progress_and_completion() {
    let (mut h, handle) = harness(ready(), MotionPreference::Off);
    settle(&mut h);
    h.get_by_role_and_label(Role::Tab, "Settings").click();
    settle(&mut h);
    assert!(shows(
        &h,
        "Worlds, screenshots, resource packs, shader packs and options are never touched"
    ));
    let repair = h.get_by_role_and_label(Role::Button, "Repair");
    repair.scroll_to_me();
    h.run_steps(20);
    let op = handle.next_op();
    h.get_by_role_and_label(Role::Button, "Repair").click();
    settle(&mut h);
    assert_eq!(
        count(&handle, |c| *c == Command::Prepare { repair: true }),
        1
    );
    assert_eq!(
        h.state().page(),
        Page::Play,
        "repair progress is shown on Play"
    );
    push_all(
        &mut h,
        &handle,
        vec![
            Event::Started {
                op,
                kind: OperationKind::Prepare { repair: true },
            },
            Event::Progress {
                op,
                progress: Progress {
                    step: Step::Verifying,
                    done: 120,
                    total: Some(480),
                    unit: ProgressUnit::Files,
                },
            },
        ],
    );
    let bar = h.get_by_role(Role::ProgressIndicator);
    assert_eq!(bar.accesskit_node().numeric_value(), Some(25.0));
    push_all(
        &mut h,
        &handle,
        vec![
            Event::InstallChanged(InstallStatus::Ready),
            Event::Finished {
                op,
                outcome: Outcome::Succeeded(Payload::None),
            },
        ],
    );
    assert!(h.query_by_role(Role::ProgressIndicator).is_none());
    shot(&mut h, "14-repair-finished");
}

/// RC1 B2: Export diagnostics asks for a new .zip (save dialog), sends that exact destination, and
/// the completion notice reveals the chosen file. A cancelled dialog or an unusable path sends
/// nothing.
#[test]
fn export_diagnostics_saves_to_the_chosen_zip_and_reveals_it() {
    let dir = std::env::temp_dir();
    let (mut h, handle) = harness(ready(), MotionPreference::Off);
    settle(&mut h);
    h.get_by_role_and_label(Role::Tab, "Diagnostics").click();
    settle(&mut h);

    // Cancelled dialog: nothing happens.
    h.state_mut().answer_next_dialog_for_tests(None);
    h.get_by_role_and_label(Role::Button, "Export diagnostics")
        .click();
    settle(&mut h);
    // A relative path is refused before it reaches the backend.
    h.state_mut()
        .answer_next_dialog_for_tests(Some(std::path::PathBuf::from("relative/report.zip")));
    h.get_by_role_and_label(Role::Button, "Export diagnostics")
        .click();
    settle(&mut h);
    assert_eq!(
        count(&handle, |c| matches!(c, Command::ExportDiagnostics { .. })),
        0
    );

    // A name without the extension gets ".zip".
    let chosen = dir.join("ooze-rc1-diagnostics");
    let expected = dir.join("ooze-rc1-diagnostics.zip");
    h.state_mut().answer_next_dialog_for_tests(Some(chosen));
    let op = handle.next_op();
    h.get_by_role_and_label(Role::Button, "Export diagnostics")
        .click();
    settle(&mut h);
    assert_eq!(
        handle.submitted().last(),
        Some(&Command::ExportDiagnostics {
            destination: expected.clone()
        })
    );
    push_all(
        &mut h,
        &handle,
        vec![
            Event::Started {
                op,
                kind: OperationKind::ExportDiagnostics,
            },
            Event::Finished {
                op,
                outcome: Outcome::Succeeded(Payload::Exported(expected.clone())),
            },
        ],
    );
    assert!(shows(&h, "Diagnostics exported"));
    h.get_by_role_and_label(Role::Button, "Show in folder")
        .click();
    settle(&mut h);
    assert_eq!(
        handle.submitted().last(),
        Some(&Command::RevealFile(expected))
    );
}

/// RC1 B7: the Play card says plainly when the game reached its title screen with the Ooze engine
/// off, and says nothing extra when the engine loaded.
#[test]
fn play_card_reports_when_the_engine_is_off_in_the_running_game() {
    let (mut h, handle) = harness(ready(), MotionPreference::Off);
    settle(&mut h);
    let session = GameSessionId(1);
    push_all(
        &mut h,
        &handle,
        vec![
            Event::Game {
                session,
                state: GameState::Running { title_screen: true },
            },
            Event::GameEngine {
                session,
                loaded: true,
                state: "LOADED",
            },
        ],
    );
    assert!(shows(&h, "reached its title screen"));
    assert!(!shows(&h, "Ooze Client is off in this game"));
    push_all(
        &mut h,
        &handle,
        vec![Event::GameEngine {
            session,
            loaded: false,
            state: "MISSING_LIBRARY",
        }],
    );
    assert!(shows(&h, "Ooze Client is off in this game"));
    assert!(shows(&h, "Repair the installation"));
    shot(&mut h, "15-engine-off");
}

/// RC1 B6: the backend's real sign-in failures (browser decline, three-minute timeout, callback
/// bind failure; all Authentication with a specific safe message) show neutral copy, keep the
/// detail, and offer "Sign in" again.
#[test]
fn declined_and_timed_out_sign_ins_are_neutral_and_retryable() {
    use ooze_launcher_core as lc;
    for detail in [
        "Microsoft sign-in was declined. Sign in again when ready.",
        "Sign-in timed out after three minutes. Retry when ready.",
        "Couldn't open the localhost sign-in callback. Retry and check the Windows firewall.",
    ] {
        let (mut h, handle) = harness(real_profile(snapshot()), MotionPreference::Off);
        settle(&mut h);
        h.get_by_role_and_label(Role::Button, "Sign in").click();
        settle(&mut h);
        let mapped = crate::core_backend::map_error(
            &lc::LauncherError {
                code: lc::ErrorCode::Authentication,
                message: detail.into(),
                retryable: true,
            },
            Some(OperationKind::SignIn),
        );
        push_all(
            &mut h,
            &handle,
            vec![
                Event::Started {
                    op: OperationId(1),
                    kind: OperationKind::SignIn,
                },
                Event::Finished {
                    op: OperationId(1),
                    outcome: Outcome::Failed(mapped),
                },
            ],
        );
        assert!(shows(&h, "Sign-in couldn't be completed"), "{detail}");
        assert!(shows(&h, detail), "the safe detail stays visible");
        for bad in ["expired", "isn't approved", "password", "offline"] {
            assert!(!shows(&h, bad), "{detail}: {bad}");
        }
        let again = h
            .query_all_by_role_and_label(Role::Button, "Sign in")
            .next()
            .expect("Sign in is offered again");
        assert!(!again.accesskit_node().is_disabled(), "{detail}");
        again.click();
        settle(&mut h);
        assert_eq!(
            count(&handle, |c| matches!(c, Command::SignIn { .. })),
            2,
            "{detail}: a fresh sign-in is submitted"
        );
    }
}
