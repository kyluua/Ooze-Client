//! Headless tests of the library pages (LIBRARY_UI.md §9) over the manual fixture library, through
//! the AccessKit tree as a screen reader would see it. They prove interface behaviour only; the
//! real library (Sol's Library API 1) and Modrinth are not involved.

use eframe::egui::{Key, accesskit::Role, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use ooze_ui::profile::{PresetId, Profile};
use ooze_ui::theme_doc::{BaseColors, ThemeDoc};

use crate::api::{
    Command, ErrorKind, Event, GameSessionId, GameState, InstallStatus, LauncherError,
    MotionPreference, OperationId, Outcome,
};
use crate::app::{AppOptions, LauncherApp, Page};
use crate::appearance::rgb;
use crate::fixtures::{FixtureBackend, FixtureHandle, account, snapshot};
use crate::library::*;
use crate::library_fixture::{self as fx, LibFixtureHandle, LibraryFixture};

type H = Harness<'static, LauncherApp>;

fn harness(lib: Option<LibrarySnapshot>) -> (H, FixtureHandle, Option<LibFixtureHandle>) {
    let mut snap = snapshot();
    snap.account = Some(account());
    snap.install = InstallStatus::Ready;
    snap.settings.motion = MotionPreference::Off;
    if let Some(selected) = lib.as_ref().and_then(LibrarySnapshot::selected) {
        snap.settings.install_root = Some(selected.game_dir.clone());
    }
    let (backend, handle) = FixtureBackend::manual(snap);
    let mut app = LauncherApp::new(Box::new(backend), AppOptions::default());
    let mut lib_handle = None;
    if let Some(s) = lib {
        let (library, h) = LibraryFixture::manual(s);
        app = app.with_library(Box::new(library));
        lib_handle = Some(h);
    }
    let mut h = Harness::builder()
        .with_size(vec2(1120.0, 900.0))
        .with_pixels_per_point(1.0)
        .build_ui_state(|ui, app: &mut LauncherApp| app.frame(ui), app);
    h.set_render_cursor(false);
    h.run_steps(6);
    (h, handle, lib_handle)
}

fn steps(h: &mut H) {
    h.run_steps(5);
}

fn click(h: &mut H, role: Role, label: &str) {
    h.get_by_role_and_label(role, label).scroll_to_me();
    // Let the scroll animation finish so the pointer lands on the widget.
    h.run_steps(20);
    h.get_by_role_and_label(role, label).click();
    steps(h);
}

fn type_into(h: &mut H, role: Role, label: &str, text: &str) {
    h.get_by_role_and_label(role, label).scroll_to_me();
    h.run_steps(20);
    h.get_by_role_and_label(role, label).click();
    h.run_steps(2);
    h.get_by_role_and_label(role, label).type_text(text);
    h.run_steps(2);
}

fn shows(h: &H, text: &str) -> bool {
    h.query_all_by_label_contains(text).count() >= 1
}

fn has_button(h: &H, label: &str) -> bool {
    h.query_all_by_role_and_label(Role::Button, label).count() > 0
}

fn enabled(h: &H, label: &str) -> bool {
    !h.get_by_role_and_label(Role::Button, label)
        .accesskit_node()
        .is_disabled()
}

fn go(h: &mut H, tab: &str) {
    h.get_by_role_and_label(Role::Tab, tab).click();
    steps(h);
}

fn finish(h: &mut H, lh: &LibFixtureHandle, payload: LibPayload) {
    lh.push(LibEvent::Finished {
        op: lh.last_op(),
        result: Ok(payload),
    });
    steps(h);
}

fn shot(h: &mut H, name: &str) {
    let image = h.render().expect("software rendering works");
    if let Ok(dir) = std::env::var("OOZE_UI_SCREENSHOT_DIR") {
        std::fs::create_dir_all(&dir).expect("screenshot dir");
        image
            .save(format!("{dir}/lib-{name}.png"))
            .expect("save png");
    }
}

fn tabs(h: &H) -> Vec<String> {
    h.query_all_by_role(Role::Tab)
        .filter_map(|n| n.accesskit_node().label())
        .collect()
}

#[test]
fn without_a_library_the_launcher_has_no_library_pages() {
    let (h, _, _) = harness(None);
    assert_eq!(tabs(&h), ["Play", "Settings", "Diagnostics"]);
    assert!(!shows(&h, "Instance:"));
}

#[test]
fn appearance_only_offers_system_motion_when_the_backend_can_read_it() {
    let (mut h, handle, _) = harness(Some(fx::snapshot()));
    let mut changed = snapshot();
    changed.system.system_reduced_motion = None;
    handle.push(Event::Snapshot(Box::new(changed)));
    steps(&mut h);
    go(&mut h, "Appearance");
    assert_eq!(
        h.query_all_by_role_and_label(Role::RadioButton, "Follow Windows")
            .count(),
        0
    );
    assert!(
        h.query_all_by_role_and_label(Role::RadioButton, "Full")
            .count()
            > 0
    );
}

#[test]
fn an_empty_or_archived_library_cannot_play_the_previous_game_directory() {
    let mut library = fx::snapshot();
    library.instances.clear();
    library.selected = None;
    let (mut h, handle, _) = harness(Some(library));
    assert!(!enabled(&h, "Play"));
    assert!(shows(
        &h,
        "Create or select an instance before installing or playing"
    ));
    click(&mut h, Role::Button, "Instances");
    assert_eq!(h.state().page(), Page::Instances);
    assert!(
        !handle
            .submitted()
            .iter()
            .any(|c| matches!(c, Command::Play | Command::Prepare { .. }))
    );
}

#[test]
fn closing_waits_for_a_library_cancellation_or_committed_operation_to_finish() {
    for committing in [false, true] {
        let (mut h, handle, lh) = harness(Some(fx::snapshot()));
        let lh = lh.unwrap();
        let op = LibOpId(99);
        h.state_mut()
            .library_model_mut_for_tests()
            .unwrap()
            .on_submitted(
                LibCommand::Apply {
                    plan_id: "fixture".into(),
                },
                Ok(op),
            );
        if committing {
            lh.push(LibEvent::Progress {
                op,
                progress: LibProgress {
                    step: LibStep::Committing,
                    done: 0,
                    total: None,
                    item: None,
                },
            });
            steps(&mut h);
        } else {
            handle.push(Event::Game {
                session: GameSessionId(1),
                state: GameState::Starting,
            });
            steps(&mut h);
        }
        assert!(!h.state_mut().on_close_requested());
        steps(&mut h);
        click(
            &mut h,
            Role::Button,
            if committing {
                "Close when finished"
            } else {
                "Cancel and close"
            },
        );
        assert!(
            !h.state_mut().on_close_requested(),
            "wait for terminal result, including a cancellation/commit race"
        );
        lh.push(LibEvent::Finished {
            op,
            result: if committing {
                Ok(LibPayload::Done)
            } else {
                Err(LibError::new(LibErrorKind::Cancelled))
            },
        });
        steps(&mut h);
        if committing {
            assert!(h.state_mut().on_close_requested());
        } else {
            assert!(
                !h.state_mut().on_close_requested(),
                "finishing a library operation must still respect the live-game close guard"
            );
        }
    }
}

#[test]
fn real_library_appearance_persists_and_local_worlds_reach_the_window() {
    use crate::core_library::CoreLibrary;
    fn wait(lib: &mut CoreLibrary, op: LibOpId) -> LibPayload {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        let mut events = Vec::new();
        while std::time::Instant::now() < deadline {
            lib.drain_events(&mut events);
            for event in events.drain(..) {
                if let LibEvent::Finished {
                    op: completed,
                    result,
                } = event
                    && completed == op
                {
                    return result.unwrap();
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        panic!("real library operation timed out");
    }
    let dir = std::env::temp_dir().join(format!(
        "ooze-real-ui-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let mut library = CoreLibrary::open(&dir).unwrap();
    let op = library
        .submit(LibCommand::Create {
            name: "Local UI integration test".into(),
            target: fx::vanilla_target(),
        })
        .unwrap();
    let LibPayload::Created(id) = wait(&mut library, op) else {
        panic!("created")
    };
    let op = library.submit(LibCommand::Select(id.clone())).unwrap();
    wait(&mut library, op);
    let game = library.snapshot().instance(&id).unwrap().game_dir.clone();
    std::fs::create_dir_all(game.join("saves/Local test world")).unwrap();
    std::fs::write(
        game.join("saves/Local test world/level.dat"),
        b"test fixture",
    )
    .unwrap();
    let mut snap = snapshot();
    snap.settings.install_root = Some(game);
    let (backend, _) = FixtureBackend::manual(snap);
    let app =
        LauncherApp::new(Box::new(backend), AppOptions::default()).with_library(Box::new(library));
    let mut h = Harness::builder()
        .with_size(vec2(1120.0, 900.0))
        .with_pixels_per_point(1.0)
        .build_ui_state(|ui, app: &mut LauncherApp| app.frame(ui), app);
    h.run_steps(6);
    go(&mut h, "Appearance");
    click(&mut h, Role::Button, "Mint");
    click(&mut h, Role::Button, "Apply");
    for _ in 0..1000 {
        h.run_steps(1);
        if !h.state().library_model().unwrap().busy() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    assert!(shows(&h, "Appearance saved"));
    let bytes = std::fs::read_to_string(dir.join("library/appearance.json")).unwrap();
    let saved = ooze_launcher_core::library::Appearance::from_json(&bytes).unwrap();
    assert!(saved.document.accent.is_some());
    shot(&mut h, "real-appearance");
    go(&mut h, "Content");
    h.get_by_role_and_label(Role::RadioButton, "Data packs")
        .click();
    for _ in 0..1000 {
        h.run_steps(1);
        if shows(&h, "Local test world") {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    assert!(shows(&h, "Local test world"));
    shot(&mut h, "real-worlds");
    drop(h);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn each_page_appears_only_with_its_capability() {
    let mut snap = fx::snapshot();
    snap.capabilities = LibraryCapabilities {
        instances: true,
        ..LibraryCapabilities::default()
    };
    let (h, _, _) = harness(Some(snap.clone()));
    assert_eq!(tabs(&h), ["Play", "Instances", "Settings", "Diagnostics"]);
    assert!(
        !has_button(&h, "New instance"),
        "no create combinations reported"
    );

    // Profiles need an instance with Ooze: selecting Vanilla hides Profile but not Content.
    snap.capabilities = fx::snapshot().capabilities;
    snap.selected = Some(fx::vanilla_id());
    let (h, _, _) = harness(Some(snap));
    assert_eq!(
        tabs(&h),
        [
            "Play",
            "Instances",
            "Content",
            "Appearance",
            "Settings",
            "Diagnostics"
        ]
    );
}

#[test]
fn content_offers_only_the_kinds_supported_for_the_selected_instance() {
    let mut snap = fx::snapshot();
    snap.selected = Some(fx::vanilla_id());
    let (mut h, _, _) = harness(Some(snap));
    go(&mut h, "Content");
    let kinds: Vec<String> = h
        .query_all_by_role(Role::RadioButton)
        .filter_map(|n| n.accesskit_node().label())
        .filter(|l| l.ends_with("packs") || l == "Mods")
        .collect();
    assert_eq!(
        kinds,
        ["Resource packs", "Data packs"],
        "no mods or shaders for Vanilla"
    );
}

#[test]
fn create_offers_only_reported_combinations_and_submits_the_exact_target() {
    let mut snap = fx::snapshot();
    snap.capabilities.create = vec![fx::fabric_target()];
    let (mut h, _, lh) = harness(Some(snap));
    let lh = lh.unwrap();
    go(&mut h, "Instances");
    click(&mut h, Role::Button, "New instance");
    let options: Vec<String> = h
        .query_all_by_role(Role::RadioButton)
        .filter_map(|n| n.accesskit_node().label())
        .collect();
    assert_eq!(options.len(), 1, "{options:?}");
    assert!(options[0].contains("Fabric 0.20.1") && options[0].contains("1.21.11"));
    assert!(options[0].contains("Ooze Client included"));
    shot(&mut h, "instances-create");
    click(&mut h, Role::Button, "Create instance");
    assert_eq!(
        lh.submitted().last(),
        Some(&LibCommand::Create {
            name: "New instance".into(),
            target: fx::fabric_target()
        })
    );
}

#[test]
fn selecting_an_instance_points_play_at_its_game_directory() {
    let (mut h, handle, lh) = harness(Some(fx::snapshot()));
    let lh = lh.unwrap();
    go(&mut h, "Instances");
    shot(&mut h, "instances");
    assert!(shows(
        &h,
        "Ooze modules are not available for this instance"
    ));
    click(&mut h, Role::Button, "Select for Play");
    assert_eq!(
        lh.submitted().last(),
        Some(&LibCommand::Select(fx::vanilla_id()))
    );
    let dir = fx::snapshot().instances[1].game_dir.clone();
    assert!(
        !handle
            .submitted()
            .iter()
            .any(|c| matches!(c, Command::UpdateSettings(_))),
        "Select must succeed first"
    );
    let mut changed = fx::snapshot();
    changed.selected = Some(fx::vanilla_id());
    lh.push(LibEvent::Changed(changed));
    finish(&mut h, &lh, LibPayload::Done);
    assert!(
        handle.submitted().iter().any(
            |c| matches!(c, Command::UpdateSettings(s) if s.install_root.as_ref() == Some(&dir))
        )
    );
}

#[test]
fn failed_selection_does_not_change_the_play_directory() {
    let (mut h, handle, lh) = harness(Some(fx::snapshot()));
    let lh = lh.unwrap();
    go(&mut h, "Instances");
    click(&mut h, Role::Button, "Select for Play");
    lh.push(LibEvent::Finished {
        op: lh.last_op(),
        result: Err(LibError::new(LibErrorKind::Config)),
    });
    steps(&mut h);
    assert!(handle.submitted().is_empty());
    assert_eq!(
        h.state().library_model().unwrap().snap.selected,
        Some(fx::fabric_id())
    );
}

#[test]
fn failed_directory_save_blocks_play_and_selection_can_be_retried() {
    let (mut h, handle, lh) = harness(Some(fx::snapshot()));
    let lh = lh.unwrap();
    go(&mut h, "Instances");
    click(&mut h, Role::Button, "Select for Play");
    let settings_op = handle.next_op();
    let mut changed = fx::snapshot();
    changed.selected = Some(fx::vanilla_id());
    lh.push(LibEvent::Changed(changed));
    finish(&mut h, &lh, LibPayload::Done);
    handle.push(Event::Finished {
        op: settings_op,
        outcome: Outcome::Failed(crate::api::LauncherError {
            kind: crate::api::ErrorKind::Configuration,
            detail: "Cannot save settings".into(),
            retryable: true,
        }),
    });
    steps(&mut h);
    go(&mut h, "Play");
    assert!(!enabled(&h, "Play"));
    assert!(has_button(&h, "Select for Play"));
    assert!(
        !handle
            .submitted()
            .iter()
            .any(|c| matches!(c, Command::Play))
    );
}

#[test]
fn instance_selection_is_disabled_while_the_game_is_live_and_recovers_on_exit() {
    let (mut h, handle, lh) = harness(Some(fx::snapshot()));
    let lh = lh.unwrap();
    handle.push(Event::Game {
        session: GameSessionId(1),
        state: GameState::Starting,
    });
    steps(&mut h);
    go(&mut h, "Instances");
    assert!(!enabled(&h, "Select for Play"));
    assert!(lh.submitted().is_empty());
    handle.push(Event::Game {
        session: GameSessionId(1),
        state: GameState::Exited { code: 0 },
    });
    steps(&mut h);
    assert!(enabled(&h, "Select for Play"));
}

#[test]
fn archive_and_duplicate_confirm_what_happens_to_files() {
    let (mut h, _, lh) = harness(Some(fx::snapshot()));
    let lh = lh.unwrap();
    go(&mut h, "Instances");
    h.get_all_by_role_and_label(Role::Button, "Archive")
        .next()
        .unwrap()
        .click();
    steps(&mut h);
    assert!(shows(&h, "Nothing is deleted"));
    assert!(
        lh.submitted().is_empty(),
        "nothing happens before confirming"
    );
    h.get_all_by_role_and_label(Role::Button, "Cancel")
        .next()
        .unwrap()
        .click();
    steps(&mut h);
    h.get_all_by_role_and_label(Role::Button, "Duplicate")
        .next()
        .unwrap()
        .click();
    steps(&mut h);
    assert!(shows(
        &h,
        "Worlds, screenshots and game options are not copied"
    ));
}

#[test]
fn a_running_instance_cannot_be_changed_and_says_why() {
    let mut snap = fx::snapshot();
    snap.instances[0].running = true;
    let (mut h, _, _) = harness(Some(snap));
    go(&mut h, "Instances");
    assert!(shows(&h, "Close this instance's game to change it"));
    let rename = h
        .get_all_by_role_and_label(Role::Button, "Rename")
        .next()
        .unwrap();
    assert!(rename.accesskit_node().is_disabled());
}

#[test]
fn file_check_lists_concrete_problems_offers_repair_and_never_deletion() {
    let (mut h, _, lh) = harness(Some(fx::snapshot()));
    let lh = lh.unwrap();
    go(&mut h, "Instances");
    h.get_all_by_role_and_label(Role::Button, "Check files")
        .next()
        .unwrap()
        .click();
    steps(&mut h);
    assert!(shows(&h, "Only files Ooze installed are checked"));
    click(&mut h, Role::Button, "Check now");
    assert_eq!(
        lh.submitted().last(),
        Some(&LibCommand::CheckIntegrity(fx::fabric_id()))
    );
    finish(
        &mut h,
        &lh,
        LibPayload::Integrity {
            instance: fx::fabric_id(),
            report: fx::integrity(&fx::fabric_id()),
        },
    );
    assert!(shows(&h, "mods/fabric-api-0.141.6+1.21.11.jar"));
    assert!(shows(&h, "This file doesn't match the expected version"));
    assert!(shows(&h, "resourcepacks/faithful-32x.zip"));
    assert!(shows(&h, "This file is missing"));
    assert!(has_button(&h, "Open folder"));
    shot(&mut h, "integrity");
    for n in h.query_all_by_role(Role::Button) {
        let l = n
            .accesskit_node()
            .label()
            .unwrap_or_default()
            .to_lowercase();
        assert!(!l.contains("delete") && !l.contains("quarantine"), "{l}");
    }
    // The Fabric row's own Repair comes first; the next one belongs to the first issue.
    h.get_all_by_role_and_label(Role::Button, "Repair")
        .nth(1)
        .unwrap()
        .click();
    steps(&mut h);
    assert_eq!(
        lh.submitted().last(),
        Some(&LibCommand::Resolve {
            instance: fx::fabric_id(),
            file: "mods/fabric-api-0.141.6+1.21.11.jar".into(),
            action: IntegrityAction::Repair
        })
    );
}

/// Owner decision 2026-11-25: one normal instance experience. No mode badges, mode switches or
/// policy statements anywhere on the Instances page or in its file check.
#[test]
fn instances_show_no_mode_badges_switches_or_policy_text() {
    let (mut h, _, lh) = harness(Some(fx::snapshot()));
    let lh = lh.unwrap();
    go(&mut h, "Instances");
    h.get_all_by_role_and_label(Role::Button, "Check files")
        .next()
        .unwrap()
        .click();
    steps(&mut h);
    click(&mut h, Role::Button, "Check now");
    finish(
        &mut h,
        &lh,
        LibPayload::Integrity {
            instance: fx::fabric_id(),
            report: fx::integrity(&fx::fabric_id()),
        },
    );
    for banned in [
        "Managed",
        "managed",
        "Fair Play",
        "fair-play",
        "Custom",
        "anti-cheat",
        "cheat",
        "server",
        "approved",
        "quarantine",
        "Switch to",
    ] {
        assert!(!shows(&h, banned), "{banned}");
    }
    assert_eq!(
        h.query_all_by_role_and_label(Role::Button, "Check files")
            .count(),
        2,
        "every instance has the same file check"
    );
}

/// Game-file Repair goes through Launcher API 1, which acts on the folder Play is pointed at, so
/// the file check offers it only for the selected instance.
#[test]
fn installation_repair_is_offered_only_for_the_instance_play_uses() {
    let (mut h, handle, lh) = harness(Some(fx::snapshot()));
    let lh = lh.unwrap();
    go(&mut h, "Instances");
    // Vanilla isn't selected: the issue says to select it and offers no Repair.
    h.get_all_by_role_and_label(Role::Button, "Check files")
        .nth(1)
        .unwrap()
        .click();
    steps(&mut h);
    click(&mut h, Role::Button, "Check now");
    assert_eq!(
        lh.submitted().last(),
        Some(&LibCommand::CheckIntegrity(fx::vanilla_id()))
    );
    finish(
        &mut h,
        &lh,
        LibPayload::Integrity {
            instance: fx::vanilla_id(),
            report: fx::integrity_installation(),
        },
    );
    assert!(shows(&h, "Game files aren't installed or have changed"));
    assert!(shows(&h, "Select this instance to install or repair"));
    assert!(!has_button(&h, "Repair game files"));
    assert!(
        !handle
            .submitted()
            .iter()
            .any(|c| matches!(c, Command::Prepare { .. }))
    );

    // Fabric is the one Play uses: Repair runs the game installation's repair and shows Play.
    h.get_all_by_role_and_label(Role::Button, "Check files")
        .next()
        .unwrap()
        .click();
    steps(&mut h);
    click(&mut h, Role::Button, "Check now");
    finish(
        &mut h,
        &lh,
        LibPayload::Integrity {
            instance: fx::fabric_id(),
            report: fx::integrity_installation(),
        },
    );
    assert!(shows(&h, "Repair re-checks this instance's game files"));
    assert!(enabled(&h, "Repair game files"));
    h.get_by_role_and_label(Role::Button, "Repair game files")
        .scroll_to_me();
    h.run_steps(20);
    shot(&mut h, "integrity-installation");
    click(&mut h, Role::Button, "Repair game files");
    assert_eq!(
        handle.submitted().last(),
        Some(&Command::Prepare { repair: true })
    );
    assert!(
        !has_button(&h, "Check files"),
        "Play shows the repair's progress"
    );
}

fn open_content(h: &mut H, lh: &LibFixtureHandle) {
    go(h, "Content");
    let _ = lh;
}

fn search(h: &mut H, text: &str) {
    type_into(h, Role::TextInput, "Search Modrinth", text);
    click(h, Role::Button, "Search");
}

#[test]
fn a_stale_search_result_is_ignored_after_switching_content_type() {
    let (mut h, _, lh) = harness(Some(fx::snapshot()));
    let lh = lh.unwrap();
    open_content(&mut h, &lh);
    search(&mut h, "utility");
    let first = lh.last_op();
    let LibCommand::Search(q) = lh.submitted().last().cloned().unwrap() else {
        panic!("search submitted");
    };
    assert_eq!(
        (q.kind, q.text.as_str(), &q.instance),
        (ContentKind::Mod, "utility", &fx::fabric_id())
    );
    h.get_by_role_and_label(Role::RadioButton, "Resource packs")
        .click();
    steps(&mut h);
    assert_eq!(lh.cancelled(), vec![first], "the old search is cancelled");
    lh.push(LibEvent::Finished {
        op: first,
        result: Ok(LibPayload::Search(SearchPage {
            query: q,
            hits: fx::hits(ContentKind::Mod),
            total: 3,
            categories: Vec::new(),
        })),
    });
    steps(&mut h);
    assert!(
        !shows(&h, "Example Utility Mod 1"),
        "late result is not shown"
    );
}

#[test]
fn install_flow_reviews_dependencies_reports_progress_and_never_claims_it_works() {
    let (mut h, _, lh) = harness(Some(fx::snapshot()));
    let lh = lh.unwrap();
    open_content(&mut h, &lh);
    search(&mut h, "");
    let LibCommand::Search(q) = lh.submitted().last().cloned().unwrap() else {
        panic!()
    };
    finish(
        &mut h,
        &lh,
        LibPayload::Search(SearchPage {
            query: q,
            hits: fx::hits(ContentKind::Mod),
            total: 3,
            categories: Vec::new(),
        }),
    );
    assert!(shows(&h, "Compatible with Fabric 0.20.1"));
    shot(&mut h, "content-results");
    click(&mut h, Role::Button, "View Example Utility Mod 1 (fixture)");
    finish(
        &mut h,
        &lh,
        LibPayload::Project(fx::details(
            fx::fabric_id(),
            ContentKind::Mod,
            "fixture-mod-1",
        )),
    );
    assert!(shows(
        &h,
        "Can't be installed here: Needs Minecraft 1.21.10"
    ));
    assert!(
        shows(&h, "<b>Markup</b>"),
        "markup is shown as text, not interpreted"
    );
    assert!(
        !has_button(&h, "Choose 1.0.0"),
        "incompatible versions can't be chosen"
    );
    shot(&mut h, "content-details");
    click(&mut h, Role::Button, "Review install");
    assert!(matches!(
        lh.submitted().last(),
        Some(LibCommand::Plan { version_id, world: None, .. }) if version_id == "fixture-mod-1-v2"
    ));

    // A conflict blocks the install.
    let mut plan = fx::plan(fx::fabric_id(), ContentKind::Mod, "fixture-mod-1-v2", None);
    plan.conflicts = vec![PlanConflict {
        title: "Example Utility Mod 1".into(),
        with: "Other Mod".into(),
    }];
    finish(&mut h, &lh, LibPayload::Plan(plan.clone()));
    assert!(shows(&h, "conflicts with Other Mod"));
    assert!(!enabled(&h, "Install"));

    // Without it: required dependency listed, optional ones not installed.
    plan.conflicts.clear();
    if let Some(m) = h.state_mut().library_model_mut_for_tests() {
        m.plan = Some(plan);
    }
    steps(&mut h);
    assert!(shows(&h, "Example Library (fixture)"));
    assert!(shows(&h, "Optional, not installed"));
    shot(&mut h, "content-review");
    click(&mut h, Role::Button, "Install");
    assert_eq!(
        lh.submitted().last(),
        Some(&LibCommand::Apply {
            plan_id: "42".into()
        })
    );
    let op = lh.last_op();
    lh.push(LibEvent::Progress {
        op,
        progress: LibProgress {
            step: LibStep::Downloading,
            done: 1,
            total: Some(4),
            item: None,
        },
    });
    steps(&mut h);
    assert!(has_button(&h, "Cancel"));
    lh.push(LibEvent::Progress {
        op,
        progress: LibProgress {
            step: LibStep::Committing,
            done: 4,
            total: Some(4),
            item: None,
        },
    });
    steps(&mut h);
    assert!(shows(&h, "can't cancel now"));
    assert!(!has_button(&h, "Cancel"));
    finish(
        &mut h,
        &lh,
        LibPayload::Installed {
            instance: fx::fabric_id(),
            kind: ContentKind::Mod,
            titles: vec!["Example Utility Mod 1 (fixture)".into()],
        },
    );
    assert!(shows(&h, "Installed and verified"));
    assert!(shows(&h, "loads the next time this instance starts"));
    for bad in ["works", "working", "Ready to use"] {
        assert!(!shows(&h, bad), "{bad}");
    }
    shot(&mut h, "content-installed");
}

#[test]
fn search_pages_and_filters_by_the_librarys_categories() {
    let (mut h, _, lh) = harness(Some(fx::snapshot()));
    let lh = lh.unwrap();
    open_content(&mut h, &lh);
    search(&mut h, "");
    let LibCommand::Search(q) = lh.submitted().last().cloned().unwrap() else {
        panic!()
    };
    finish(
        &mut h,
        &lh,
        LibPayload::Search(SearchPage {
            query: q.clone(),
            hits: fx::hits(ContentKind::Mod),
            total: 7,
            categories: vec!["utility".into(), "optimization".into()],
        }),
    );
    assert!(shows(&h, "1–3 of 7"));
    assert!(!has_button(&h, "Previous page"));
    click(&mut h, Role::Button, "Next page");
    assert!(
        matches!(lh.submitted().last(), Some(LibCommand::Search(n)) if n.offset == 3 && n.text == q.text)
    );
    let LibCommand::Search(q2) = lh.submitted().last().cloned().unwrap() else {
        panic!()
    };
    finish(
        &mut h,
        &lh,
        LibPayload::Search(SearchPage {
            query: q2,
            hits: fx::hits(ContentKind::Mod),
            total: 7,
            categories: vec!["utility".into(), "optimization".into()],
        }),
    );
    assert!(shows(&h, "4–6 of 7"));
    click(&mut h, Role::RadioButton, "optimization");
    assert!(matches!(
        lh.submitted().last(),
        Some(LibCommand::Search(n)) if n.categories == ["optimization"] && n.offset == 0
    ));
}

#[test]
fn cancelling_a_download_is_acknowledged_by_the_library() {
    let (mut h, _, lh) = harness(Some(fx::snapshot()));
    let lh = lh.unwrap();
    open_content(&mut h, &lh);
    h.state_mut().library_model_mut_for_tests().unwrap().plan =
        Some(fx::plan(fx::fabric_id(), ContentKind::Mod, "x-v2", None));
    steps(&mut h);
    click(&mut h, Role::Button, "Install");
    let op = lh.last_op();
    click(&mut h, Role::Button, "Cancel");
    assert_eq!(lh.cancelled(), vec![op]);
    lh.push(LibEvent::Finished {
        op,
        result: Err(LibError::new(LibErrorKind::Cancelled)),
    });
    steps(&mut h);
    assert!(shows(&h, "Cancelled"));
}

#[test]
fn data_packs_need_an_existing_world_and_shaders_explain_their_loader() {
    let (mut h, _, lh) = harness(Some(fx::snapshot()));
    let lh = lh.unwrap();
    open_content(&mut h, &lh);
    h.get_by_role_and_label(Role::RadioButton, "Data packs")
        .click();
    steps(&mut h);
    assert_eq!(
        lh.submitted().last(),
        Some(&LibCommand::ListWorlds(fx::fabric_id()))
    );
    finish(
        &mut h,
        &lh,
        LibPayload::Worlds {
            instance: fx::fabric_id(),
            worlds: Vec::new(),
        },
    );
    assert!(shows(&h, "Create a world in this instance first"));
    assert!(
        h.query_by_role_and_label(Role::TextInput, "Search Modrinth")
            .is_none()
    );

    h.state_mut().library_model_mut_for_tests().unwrap().worlds =
        Some((fx::fabric_id(), fx::worlds()));
    steps(&mut h);
    assert!(shows(&h, "Choose the world to install into"));
    h.get_by_role_and_label(Role::RadioButton, "Fixture World")
        .click();
    steps(&mut h);
    assert!(
        h.query_by_role_and_label(Role::TextInput, "Search Modrinth")
            .is_some()
    );
    shot(&mut h, "content-datapack");

    h.get_by_role_and_label(Role::RadioButton, "Shader packs")
        .click();
    steps(&mut h);
    assert!(shows(&h, "Shader packs need a shader loader"));
    assert!(shows(&h, "Installing a pack doesn't turn it on"));
    let mut plan = fx::plan(fx::fabric_id(), ContentKind::Shader, "s-v2", None);
    plan.shader_loader = Some(ShaderLoader::Unavailable);
    h.state_mut().library_model_mut_for_tests().unwrap().plan = Some(plan);
    steps(&mut h);
    assert!(shows(&h, "No compatible shader loader"));
    assert!(!enabled(&h, "Install"));
}

fn open_profile(h: &mut H, lh: &LibFixtureHandle, profile: Profile) {
    go(h, "Profile");
    assert_eq!(
        lh.submitted().last(),
        Some(&LibCommand::LoadProfile(fx::fabric_id()))
    );
    finish(
        h,
        lh,
        LibPayload::Profile {
            id: fx::fabric_id(),
            profile,
            revision: 3,
        },
    );
}

#[test]
fn presets_show_what_they_change_and_switching_keeps_user_overrides() {
    let (mut h, _, lh) = harness(Some(fx::snapshot()));
    let lh = lh.unwrap();
    let mut mine = Profile::from_preset(PresetId::Pvp, "default", "Default");
    mine.set_enabled(ooze_ui::catalog::ModuleId::Coordinates, true);
    open_profile(&mut h, &lh, mine.clone());
    assert!(shows(
        &h,
        "doesn't install performance mods or promise a higher frame rate"
    ));
    assert!(
        shows(&h, "Turns on Keystrokes"),
        "PvP card lists its changes"
    );
    shot(&mut h, "profile");
    click(&mut h, Role::Button, "Use FPS");
    assert!(shows(&h, "Switch to FPS?"));
    assert!(shows(&h, "Your 1 changes are kept"));
    click(&mut h, Role::Button, "Switch preset");
    let Some(LibCommand::SaveProfile {
        profile, revision, ..
    }) = lh.submitted().last().cloned()
    else {
        panic!("saved")
    };
    assert_eq!(revision, 3);
    assert_eq!(profile.base, Some(PresetId::Fps));
    assert!(
        profile.resolve().modules[&ooze_ui::catalog::ModuleId::Coordinates].enabled,
        "the player's own change survives the preset switch"
    );
}

#[test]
fn modules_carry_classes_server_notices_and_validated_settings() {
    let (mut h, _, lh) = harness(Some(fx::snapshot()));
    let lh = lh.unwrap();
    open_profile(
        &mut h,
        &lh,
        Profile::from_preset(PresetId::Minimal, "default", "Default"),
    );
    assert!(shows(&h, "Some servers don't allow this"));
    assert!(
        !shows(&h, "Gameplay (blocked)"),
        "no gameplay modules listed"
    );
    click(&mut h, Role::Switch, "Zoom");
    assert!(shows(&h, "You have unsaved changes"));
    // Open FPS settings and step the averaging window within its range.
    h.get_all_by_role_and_label(Role::Button, "Settings")
        .next()
        .unwrap()
        .scroll_to_me();
    h.run_steps(20);
    h.get_all_by_role_and_label(Role::Button, "Settings")
        .next()
        .unwrap()
        .click();
    steps(&mut h);
    click(&mut h, Role::Button, "Increase Averaging window (seconds)");
    click(&mut h, Role::Button, "Save");
    let Some(LibCommand::SaveProfile { profile, .. }) = lh.submitted().last().cloned() else {
        panic!()
    };
    let e = profile.resolve();
    assert!(e.modules[&ooze_ui::catalog::ModuleId::Zoom].enabled);
    let fps = &e.modules[&ooze_ui::catalog::ModuleId::Fps].settings["window_seconds"];
    assert_eq!(*fps, ooze_ui::settings::Value::Int(4));
}

#[test]
fn appearance_guards_contrast_previews_live_and_persists_through_the_library() {
    let (mut h, handle, lh) = harness(Some(fx::snapshot()));
    let lh = lh.unwrap();
    let base_accent = h.state().theme().c.accent;
    go(&mut h, "Appearance");
    type_into(&mut h, Role::TextInput, "Custom accent", "#F2B33D");
    click(&mut h, Role::Button, "Use this colour");
    assert!(shows(&h, "That accent isn't readable enough"));
    assert!(shows(&h, "3:1 is needed"));
    assert_eq!(
        h.state().theme().c.accent,
        base_accent,
        "refused accent never shown"
    );

    let guard = BaseColors::classic().unwrap();
    let mint = guard.swatches.iter().find(|(n, _)| *n == "mint").unwrap().1;
    click(&mut h, Role::Button, "Mint");
    assert_eq!(h.state().theme().c.accent, rgb(mint), "live preview");
    shot(&mut h, "appearance-preview");
    click(&mut h, Role::Button, "Revert");
    assert_eq!(h.state().theme().c.accent, base_accent);

    click(&mut h, Role::Button, "Mint");
    h.get_by_role_and_label(Role::RadioButton, "High").click();
    steps(&mut h);
    h.get_by_role_and_label(Role::RadioButton, "125%").click();
    steps(&mut h);
    click(&mut h, Role::Button, "Apply");
    let Some(LibCommand::SaveTheme(doc)) = lh.submitted().last().cloned() else {
        panic!("theme saved through the library")
    };
    assert_eq!(doc.accent, Some(mint));
    assert_eq!(doc.ui_scale, 125);
    assert!(
        !handle
            .submitted()
            .iter()
            .any(|c| matches!(c, Command::UpdateSettings(_))),
        "Appearance has one atomic library store; legacy UI preferences cannot override it"
    );
    // Export / import round trip uses the strict shared validator.
    assert_eq!(ThemeDoc::from_json(&doc.to_json(), &guard), Ok(doc));
}

#[test]
fn theme_import_refuses_unknown_fields_with_the_reason() {
    let (mut h, _, _) = harness(Some(fx::snapshot()));
    go(&mut h, "Appearance");
    click(&mut h, Role::Button, "Import theme");
    type_into(
        &mut h,
        Role::MultilineTextInput,
        "Paste a theme (ooze.theme/1)",
        r#"{"schema":"ooze.theme/1","base":"ooze_classic","script":"x"}"#,
    );
    click(&mut h, Role::Button, "Preview");
    assert!(shows(&h, "It contains a field Ooze doesn't accept: script"));
}

#[test]
fn library_errors_are_typed_and_keep_the_safe_detail() {
    let (mut h, _, lh) = harness(Some(fx::snapshot()));
    let lh = lh.unwrap();
    open_content(&mut h, &lh);
    search(&mut h, "x");
    lh.push(LibEvent::Finished {
        op: lh.last_op(),
        result: Err(LibError {
            kind: LibErrorKind::RateLimited,
            detail: None,
            retry_after_s: Some(12),
        }),
    });
    steps(&mut h);
    assert!(shows(&h, "Modrinth asked us to slow down"));
    assert!(shows(&h, "Try again in 12 seconds"));
    lh.reject_next(LibError::new(LibErrorKind::Busy));
    click(&mut h, Role::Button, "Dismiss");
    search(&mut h, "y");
    assert!(shows(&h, "Another library task is running"));
}

#[test]
fn every_library_widget_has_a_name_on_every_page() {
    let (mut h, _, lh) = harness(Some(fx::snapshot()));
    let lh = lh.unwrap();
    let check = |h: &mut H| {
        for role in [
            Role::Button,
            Role::Tab,
            Role::RadioButton,
            Role::TextInput,
            Role::MultilineTextInput,
            Role::Switch,
        ] {
            for node in h.query_all_by_role(role) {
                let n = node.accesskit_node();
                let named = n.label().is_some_and(|l| !l.trim().is_empty())
                    || !n.data().labelled_by().is_empty();
                assert!(named, "{role:?} without a name: {n:?}");
            }
        }
    };
    go(&mut h, "Instances");
    check(&mut h);
    open_profile(
        &mut h,
        &lh,
        Profile::from_preset(PresetId::Pvp, "default", "Default"),
    );
    check(&mut h);
    go(&mut h, "Content");
    check(&mut h);
    go(&mut h, "Appearance");
    check(&mut h);
    shot(&mut h, "appearance");
}

#[test]
fn library_pages_are_reachable_with_the_keyboard() {
    let (mut h, _, _) = harness(Some(fx::snapshot()));
    let mut reached = false;
    for _ in 0..12 {
        h.key_press(Key::Tab);
        h.run_steps(2);
        if h.get_by_role_and_label(Role::Tab, "Instances").is_focused() {
            reached = true;
            break;
        }
    }
    assert!(reached, "Instances tab is reachable with Tab");
    h.key_press(Key::Enter);
    steps(&mut h);
    assert_eq!(h.state().page(), Page::Instances);
}

/// Live (network): the real window over Sol's real library and Modrinth. Appearance evidence of
/// real results; run with `cargo test -p ooze-launcher-ui live_window -- --ignored`.
#[test]
#[ignore = "contacts api.modrinth.com"]
fn live_window_shows_real_modrinth_results_for_a_fabric_instance() {
    use crate::core_library::CoreLibrary;
    let dir = std::env::temp_dir().join(format!("ooze-ui-live-window-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let mut lib = CoreLibrary::open(&dir).unwrap();
    let target = lib.snapshot().capabilities.create[0].clone();
    let mut create = Some(lib.submit(LibCommand::Create {
        name: "Fabric 1.21.11".into(),
        target,
    }));
    // Wait for the instance, then select it, before the window opens.
    let mut events = Vec::new();
    let mut created = None;
    for _ in 0..500 {
        lib.drain_events(&mut events);
        for e in events.drain(..) {
            if let LibEvent::Finished {
                result: Ok(LibPayload::Created(id)),
                ..
            } = e
            {
                created = Some(id);
            }
        }
        if created.is_some() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    let _ = create.take();
    let id = created.expect("instance created");
    lib.submit(LibCommand::Select(id)).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(200));
    lib.drain_events(&mut events);

    let mut snap = snapshot();
    snap.account = Some(account());
    snap.install = InstallStatus::Ready;
    snap.settings.motion = MotionPreference::Off;
    let (backend, _handle) = FixtureBackend::manual(snap);
    let app =
        LauncherApp::new(Box::new(backend), AppOptions::default()).with_library(Box::new(lib));
    // Real Modrinth data includes scripts the shipped fonts don't cover (drawn as boxes in the
    // app, which sets the Tofu policy itself).
    let mut h = Harness::builder()
        .with_size(vec2(1120.0, 900.0))
        .with_pixels_per_point(1.0)
        .allow_missing_glyphs()
        .build_ui_state(|ui, app: &mut LauncherApp| app.frame(ui), app);
    h.set_render_cursor(false);
    h.run_steps(6);
    assert_eq!(
        tabs(&h),
        [
            "Play",
            "Instances",
            "Content",
            "Appearance",
            "Settings",
            "Diagnostics"
        ],
        "Profile waits for runtime modules"
    );
    go(&mut h, "Content");
    search(&mut h, "sodium");
    let mut found = false;
    for _ in 0..300 {
        h.run_steps(1);
        if shows(&h, "results") {
            found = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    assert!(found, "real results arrived");
    assert!(shows(&h, "Sodium"));
    steps(&mut h);
    shot(&mut h, "live-content-results");
    go(&mut h, "Instances");
    shot(&mut h, "live-instances");
    let _ = std::fs::remove_dir_all(&dir);
}

/// RC1 B3: at the smallest window and 200 % interface scale, every New Instance choice stays
/// inside the window (long choices stack and wrap instead of running past the card).
#[test]
fn new_instance_choices_fit_at_the_minimum_window_and_200_percent_scale() {
    let mut lib = fx::snapshot();
    lib.theme.ui_scale = 200;
    let mut snap = snapshot();
    snap.account = Some(account());
    snap.install = InstallStatus::Ready;
    snap.settings.motion = MotionPreference::Off;
    snap.settings.ui_scale_percent = 200;
    if let Some(selected) = lib.selected() {
        snap.settings.install_root = Some(selected.game_dir.clone());
    }
    let (backend, _handle) = FixtureBackend::manual(snap);
    let (library, _lh) = LibraryFixture::manual(lib);
    let app =
        LauncherApp::new(Box::new(backend), AppOptions::default()).with_library(Box::new(library));
    let [w, h_min] = crate::theme::Theme::embedded().control.window_min;
    let mut h = Harness::builder()
        .with_size(vec2(w, h_min))
        .with_pixels_per_point(1.0)
        .build_ui_state(|ui, app: &mut LauncherApp| app.frame(ui), app);
    h.set_render_cursor(false);
    h.run_steps(8);
    go(&mut h, "Instances");
    click(&mut h, Role::Button, "New instance");
    h.run_steps(6);
    // The Name field spans the card's full inner width: the choices must stay within it.
    let card = h
        .get_by_role_and_label(Role::TextInput, "Name")
        .accesskit_node()
        .bounding_box()
        .expect("name field bounds");
    let mut checked = 0;
    for node in h.query_all_by_role(Role::RadioButton) {
        let label = node.accesskit_node().label().unwrap_or_default();
        if !label.contains("Minecraft 1.21.11") {
            continue;
        }
        let b = node.accesskit_node().bounding_box().expect("choice bounds");
        assert!(
            b.x0 >= card.x0 - 0.5 && b.x1 <= card.x1 + 0.5,
            "{label:?} runs outside the card: {b:?} vs {card:?}"
        );
        checked += 1;
    }
    assert_eq!(checked, 2, "both Fabric and Vanilla choices are present");
    h.query_all_by_role(Role::RadioButton)
        .find(|n| {
            n.accesskit_node()
                .label()
                .is_some_and(|l| l.contains("Vanilla"))
        })
        .expect("vanilla choice")
        .scroll_to_me();
    h.run_steps(20);
    shot(&mut h, "new-instance-min-200");
}

/// RC1 B3: at the smallest window and 200 % interface scale, no control on any page extends past
/// the window's right edge (accessibility: large scales must stay usable).
#[test]
fn no_control_runs_off_screen_at_the_minimum_window_and_200_percent_scale() {
    let mut lib = fx::snapshot();
    lib.theme.ui_scale = 200;
    let mut snap = snapshot();
    snap.account = Some(account());
    snap.install = InstallStatus::Ready;
    snap.settings.motion = MotionPreference::Off;
    snap.settings.ui_scale_percent = 200;
    if let Some(selected) = lib.selected() {
        snap.settings.install_root = Some(selected.game_dir.clone());
    }
    let (backend, _handle) = FixtureBackend::manual(snap);
    let (library, _lh) = LibraryFixture::manual(lib);
    let app =
        LauncherApp::new(Box::new(backend), AppOptions::default()).with_library(Box::new(library));
    let [w, h_min] = crate::theme::Theme::embedded().control.window_min;
    let mut h = Harness::builder()
        .with_size(vec2(w, h_min))
        .with_pixels_per_point(1.0)
        .build_ui_state(|ui, app: &mut LauncherApp| app.frame(ui), app);
    h.set_render_cursor(false);
    h.run_steps(8);
    let mut offenders = Vec::new();
    for page in [
        "Play",
        "Instances",
        "Content",
        "Appearance",
        "Settings",
        "Diagnostics",
    ] {
        h.get_by_role_and_label(Role::Tab, page).click();
        h.run_steps(8);
        for role in [
            Role::Button,
            Role::RadioButton,
            Role::TextInput,
            Role::Switch,
            Role::Tab,
        ] {
            for node in h.query_all_by_role(role) {
                let n = node.accesskit_node();
                if let Some(b) = n.bounding_box()
                    && b.x1 > f64::from(w) + 0.5
                {
                    offenders.push(format!("{page}: {role:?} {:?} x1={}", n.label(), b.x1));
                }
            }
        }
    }
    assert!(offenders.is_empty(), "{offenders:#?}");
    // The header chips must not cover the wordmark.
    let mark = h
        // The wordmark's text is "ooze"; its accessible name is the app name.
        .query_all_by_label_contains(crate::i18n::Strings::en_us().get("app.wordmark"))
        .filter(|n| {
            let n = n.accesskit_node();
            n.role() == Role::Label
                && n.label().as_deref() == Some(crate::i18n::Strings::en_us().get("app.name"))
        })
        .find_map(|n| n.accesskit_node().bounding_box())
        .expect("wordmark bounds");
    let chip = h
        .get_by_role_and_label(Role::Button, &account().username)
        .accesskit_node()
        .bounding_box()
        .expect("account chip bounds");
    let overlap_x = chip.x0 < mark.x1 && mark.x0 < chip.x1;
    let overlap_y = chip.y0 < mark.y1 && mark.y0 < chip.y1;
    assert!(
        !(overlap_x && overlap_y),
        "chip {chip:?} covers wordmark {mark:?}"
    );
}

/// RC1 B4: the library selected an instance that API 1 isn't pointed at (for example, the first
/// instance created is selected by the library). The list doesn't claim "Selected for Play" until
/// API 1 has saved that directory; the window reconciles once on its own, and a failed save is
/// not retried in a loop (the player can still use Select).
#[test]
fn selection_reconciles_once_and_the_list_agrees_with_play() {
    let lib = fx::snapshot();
    let fabric_dir = lib.selected().unwrap().game_dir.clone();
    let mut snap = snapshot();
    snap.account = Some(account());
    snap.install = InstallStatus::Ready;
    snap.settings.motion = MotionPreference::Off;
    let legacy = std::path::PathBuf::from(r"C:\Users\Player\AppData\Roaming\.ooze");
    snap.settings.install_root = Some(legacy.clone());
    let (backend, handle) = FixtureBackend::manual(snap.clone());
    let (library, _lh) = LibraryFixture::manual(lib);
    let app =
        LauncherApp::new(Box::new(backend), AppOptions::default()).with_library(Box::new(library));
    let mut h = Harness::builder()
        .with_size(vec2(1120.0, 900.0))
        .with_pixels_per_point(1.0)
        .build_ui_state(|ui, app: &mut LauncherApp| app.frame(ui), app);
    h.set_render_cursor(false);
    h.run_steps(8);
    let saves = |handle: &FixtureHandle| {
        handle
            .submitted()
            .iter()
            .filter(|c| matches!(c, Command::UpdateSettings(s) if s.install_root.as_ref() == Some(&fabric_dir)))
            .count()
    };
    assert_eq!(saves(&handle), 1, "reconciled without a click");
    go(&mut h, "Instances");
    assert!(
        !shows(&h, "Selected for Play"),
        "not claimed before API 1 saved the directory"
    );

    // The save fails: no automatic retry loop, and Select stays available.
    let op = OperationId(handle.next_op().0 - 1);
    handle.push(Event::Finished {
        op,
        outcome: Outcome::Failed(LauncherError {
            kind: ErrorKind::Io,
            detail: "fixture: settings folder is read-only".into(),
            retryable: true,
        }),
    });
    h.run_steps(12);
    assert_eq!(
        saves(&handle),
        1,
        "a failed save is not retried automatically"
    );
    assert!(has_button(&h, "Select for Play"));

    // An explicit Select saves again; once API 1 reports the directory, the list and Play agree.
    h.get_all_by_role_and_label(Role::Button, "Select for Play")
        .next()
        .unwrap()
        .click();
    steps(&mut h);
    let mut saved = snap.settings.clone();
    saved.install_root = Some(fabric_dir.clone());
    handle.push(Event::SettingsChanged(saved));
    h.run_steps(8);
    assert!(shows(&h, "Selected for Play"));
}

/// RC1 B4 (Sol's Windows observation): creating the first instance makes the library select it;
/// the window points Play at it without asking the player to select it again.
#[test]
fn the_first_created_instance_is_ready_for_play_without_another_click() {
    let mut empty = fx::snapshot();
    let created = empty.instances[0].clone();
    empty.instances.clear();
    empty.selected = None;
    let mut snap = snapshot();
    snap.account = Some(account());
    snap.install = InstallStatus::Ready;
    snap.settings.motion = MotionPreference::Off;
    snap.settings.install_root = Some(std::path::PathBuf::from(
        r"C:\Users\Player\AppData\Roaming\.ooze",
    ));
    let (backend, handle) = FixtureBackend::manual(snap);
    let (library, lh) = LibraryFixture::manual(empty.clone());
    let app =
        LauncherApp::new(Box::new(backend), AppOptions::default()).with_library(Box::new(library));
    let mut h = Harness::builder()
        .with_size(vec2(1120.0, 900.0))
        .with_pixels_per_point(1.0)
        .build_ui_state(|ui, app: &mut LauncherApp| app.frame(ui), app);
    h.set_render_cursor(false);
    h.run_steps(8);
    let mut after = empty;
    after.instances = vec![created.clone()];
    after.selected = Some(created.id.clone());
    lh.push(LibEvent::Changed(after));
    h.run_steps(8);
    assert!(
        handle.submitted().iter().any(
            |c| matches!(c, Command::UpdateSettings(s) if s.install_root.as_ref() == Some(&created.game_dir))
        ),
        "Play is pointed at the newly selected instance"
    );
}

/// RC1 B5: Repair needs no sign-in (LAUNCHER_API_V1), so it is available while signed out for
/// the instance Play is pointed at; the install location stays locked to the instance.
#[test]
fn repair_is_available_while_signed_out_for_the_selected_instance() {
    let lib = fx::snapshot();
    let dir = lib.selected().unwrap().game_dir.clone();
    let mut snap = snapshot();
    snap.account = None;
    snap.install = InstallStatus::Ready;
    snap.settings.motion = MotionPreference::Off;
    snap.settings.install_root = Some(dir);
    let (backend, handle) = FixtureBackend::manual(snap);
    let (library, _lh) = LibraryFixture::manual(lib);
    let app =
        LauncherApp::new(Box::new(backend), AppOptions::default()).with_library(Box::new(library));
    let mut h = Harness::builder()
        .with_size(vec2(1120.0, 900.0))
        .with_pixels_per_point(1.0)
        .build_ui_state(|ui, app: &mut LauncherApp| app.frame(ui), app);
    h.set_render_cursor(false);
    h.run_steps(8);
    go(&mut h, "Settings");
    assert!(enabled(&h, "Repair"), "Repair works before sign-in");
    assert!(
        !enabled(&h, "Change location…"),
        "the location belongs to the instance"
    );
    click(&mut h, Role::Button, "Repair");
    assert!(
        handle
            .submitted()
            .contains(&Command::Prepare { repair: true })
    );
}
