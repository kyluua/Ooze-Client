//! Library pages: Instances, Profile, Content (Modrinth) and Appearance (`LIBRARY_UI.md`).
//!
//! Every page is shown only when the library reports its capability. Pages render the library
//! model and submit typed commands; they never decide compatibility, resolve dependencies or touch
//! files. One library operation runs at a time, so actions are disabled (still focusable, with
//! the reason shown) while another runs.

use eframe::egui::{self, Key, Ui};
use ooze_ui::catalog::{FairPlay, MODULES, ModuleDef, ModuleId, VISUAL_SETTINGS};
use ooze_ui::json::Json;
use ooze_ui::profile::{self, Change, ImportError, ImportWarning, PresetId, Profile};
use ooze_ui::settings::{SettingDef, SettingKind, Value, is_key_name};
use ooze_ui::theme_doc::{Background, Contrast, Motion, Rgb, TEXT_SCALES, ThemeDoc, UI_SCALES};

use super::{LauncherApp, Page};
use crate::api::Command;
use crate::appearance::{self, import_error_text};
use crate::library::*;
use crate::library_model::{LibNotice, LibOpKind};
use crate::widgets::{ButtonKind, Kit, Tone};

#[derive(Clone, Debug, PartialEq)]
pub(super) enum Confirm {
    Archive(InstanceId),
    Duplicate(InstanceId),
    ToCustom(InstanceId),
    ToManaged(InstanceId),
    Remove(InstalledItem),
    ResetProfile,
}

/// Page-local interface state (drafts, open dialogs, typed text). Never persisted.
#[derive(Debug, Default)]
pub(super) struct LibUi {
    create_open: bool,
    create_name: String,
    create_target: usize,
    rename: Option<(InstanceId, String)>,
    confirm: Option<Confirm>,
    integrity_for: Option<InstanceId>,
    /// Editable copy of the selected instance's profile and the revision it was read at.
    draft: Option<(InstanceId, Profile, u64)>,
    expanded: Option<ModuleId>,
    preset_confirm: Option<PresetId>,
    reset_overrides_too: bool,
    color_text: Vec<(String, String)>,
    color_error: Option<String>,
    capturing: Option<(ModuleId, &'static str)>,
    profile_import: Option<String>,
    profile_import_result: Option<Result<(Profile, Vec<ImportWarning>), ImportError>>,
    kind: Option<ContentKind>,
    query: String,
    sort: Option<SortBy>,
    world: Option<String>,
    version: Option<String>,
    pub(super) appearance: Option<ThemeDoc>,
    accent_hex: String,
    accent_error: Option<String>,
    theme_import: Option<String>,
    theme_import_error: Option<String>,
}

fn target_text(kit: &Kit<'_>, t: &GameTarget) -> String {
    let loader = kit.t(t.loader.key());
    match &t.loader_version {
        Some(v) => kit.tr.fmt(
            "instance.target",
            &[loader, v, &t.game_version, &t.java_major.to_string()],
        ),
        None => kit.tr.fmt(
            "instance.target_no_loader_version",
            &[loader, &t.game_version, &t.java_major.to_string()],
        ),
    }
}

fn mode_key(m: IntegrityMode) -> &'static str {
    match m {
        IntegrityMode::Custom => "integrity.mode.custom",
        IntegrityMode::ManagedFairPlay => "integrity.mode.managed",
    }
}

fn mb(bytes: u64) -> String {
    format!("{:.1}", bytes as f64 / 1_000_000.0)
}

fn setting_label_key(def: &SettingDef, option: &str) -> String {
    format!("{}.{option}", def.label_key)
}

/// Readable "what this changes" line for a profile [`Change`].
pub(super) fn change_text(kit: &Kit<'_>, c: &Change) -> String {
    let parts: Vec<&str> = c.path.split('.').collect();
    let name = match parts.as_slice() {
        ["modules", m, "enabled"] => ModuleId::parse(m).map(|id| {
            let n = kit.t(id.def().name_key).to_owned();
            if c.to == "true" {
                kit.tr.fmt("profile.change.turns_on", &[&n])
            } else {
                kit.tr.fmt("profile.change.turns_off", &[&n])
            }
        }),
        ["modules", m, "settings", key] => ModuleId::parse(m).and_then(|id| {
            let def = id.def().setting(key)?;
            Some(kit.tr.fmt(
                "profile.change.setting",
                &[
                    kit.t(id.def().name_key),
                    kit.t(def.label_key),
                    &c.from,
                    &c.to,
                ],
            ))
        }),
        ["hud", m] => ModuleId::parse(m).map(|id| {
            kit.tr
                .fmt("profile.change.hud", &[kit.t(id.def().name_key)])
        }),
        ["visual", key] => ooze_ui::catalog::visual_setting(key).map(|def| {
            kit.tr.fmt(
                "profile.change.visual",
                &[kit.t(def.label_key), &c.from, &c.to],
            )
        }),
        _ => None,
    };
    name.unwrap_or_else(|| c.path.clone())
}

/// Vanilla key name for an egui key, or `None` for keys Ooze does not bind.
fn vanilla_key(key: Key) -> Option<String> {
    let name = key.name();
    let lower = name.to_ascii_lowercase();
    let n = match key {
        Key::Space => "space".to_owned(),
        Key::Tab => "tab".to_owned(),
        Key::Backspace => "backspace".to_owned(),
        Key::Enter => "enter".to_owned(),
        Key::ArrowUp => "up".to_owned(),
        Key::ArrowDown => "down".to_owned(),
        Key::ArrowLeft => "left".to_owned(),
        Key::ArrowRight => "right".to_owned(),
        _ if lower.len() == 1 && lower.bytes().all(|b| b.is_ascii_alphanumeric()) => lower,
        _ if lower.starts_with('f') && lower[1..].parse::<u8>().is_ok() => lower,
        _ => return None,
    };
    let full = format!("key.keyboard.{n}");
    is_key_name(&full).then_some(full)
}

impl LauncherApp {
    // ----------------------------------------------------------------------------------------
    // Plumbing

    pub(super) fn lib_submit(&mut self, command: LibCommand) {
        let (Some(backend), Some(model)) = (self.library.as_mut(), self.lib.as_mut()) else {
            return;
        };
        if model.busy() {
            return;
        }
        let result = backend.submit(command.clone());
        model.on_submitted(command, result);
    }

    /// Cancel and keep waiting for the acknowledgement (shown as "Cancelled").
    fn lib_cancel(&mut self) {
        if let (Some(backend), Some(model)) = (self.library.as_mut(), self.lib.as_ref())
            && let Some((op, _)) = model.active()
        {
            backend.cancel(op);
        }
    }

    /// Stop waiting (navigation): the late result is ignored as stale.
    pub(super) fn lib_abandon(&mut self) {
        if let (Some(backend), Some(model)) = (self.library.as_mut(), self.lib.as_mut())
            && let Some(op) = model.abandon()
        {
            backend.cancel(op);
        }
    }

    fn lib_busy(&self) -> bool {
        self.lib.as_ref().is_some_and(|m| m.busy())
    }

    fn selected_instance(&self) -> Option<InstanceSummary> {
        self.lib.as_ref().and_then(|m| m.snap.selected().cloned())
    }

    /// Which library pages are available now (navigation and routing both use this).
    pub(super) fn library_pages(&self) -> Vec<Page> {
        let Some(m) = &self.lib else {
            return Vec::new();
        };
        let caps = m.caps();
        let sel = m.snap.selected();
        let mut out = Vec::new();
        if caps.instances {
            out.push(Page::Instances);
        }
        if caps.profiles && sel.is_some_and(|i| i.target.ooze) {
            out.push(Page::Profile);
        }
        if sel.is_some_and(|i| i.content.iter().any(|k| caps.content.contains(k))) {
            out.push(Page::Content);
        }
        if caps.themes {
            out.push(Page::Appearance);
        }
        out
    }

    fn busy_hint(&self, ui: &mut Ui, kit: &Kit<'_>) {
        if self.lib_busy() {
            kit.secondary(ui, kit.t("library.busy_hint"));
        }
    }

    /// Notices and progress shared by every library page.
    fn lib_status(&mut self, ui: &mut Ui, kit: &Kit<'_>) {
        let th = kit.theme;
        let Some(model) = &self.lib else { return };
        if let Some(e) = &model.open_error {
            let body = e.detail.clone().unwrap_or_default();
            kit.banner(
                ui,
                Tone::Danger,
                kit.t("library.error.config.title"),
                &body,
                &[],
            );
            ui.add_space(th.space.md);
        }
        if let Some((_, kind)) = model.active() {
            let progress = model.progress.clone();
            let cancellable = model.cancellable();
            let label = match progress.as_ref().map(|p| p.step) {
                Some(LibStep::Contacting) | None => kit.t(match kind {
                    LibOpKind::Search | LibOpKind::Project | LibOpKind::Plan => {
                        "library.step.contacting"
                    }
                    _ => "library.step.working",
                }),
                Some(LibStep::Downloading) => kit.t("library.step.downloading"),
                Some(LibStep::Verifying) => kit.t("library.step.verifying"),
                Some(LibStep::Committing) => kit.t("library.step.committing"),
            }
            .to_owned();
            kit.progress(
                ui,
                progress.as_ref().and_then(LibProgress::fraction),
                &label,
            );
            if kind == LibOpKind::Apply || kind == LibOpKind::Instance {
                ui.add_space(th.space.xs);
                if cancellable {
                    if kit
                        .button(ui, kit.t("common.cancel"), ButtonKind::Secondary, true)
                        .clicked()
                    {
                        self.lib_cancel();
                    }
                } else {
                    kit.secondary(ui, kit.t("library.cannot_cancel_now"));
                }
            }
            ui.add_space(th.space.md);
        }
        let Some(notice) = self.lib.as_ref().and_then(|m| m.notice.clone()) else {
            return;
        };
        let dismiss = kit.t("common.dismiss");
        let pressed = match &notice {
            LibNotice::Error { error, .. } => {
                let (title, body) = lib_error_keys(error.kind);
                let mut text = kit.t(body).to_owned();
                if let Some(s) = error.retry_after_s {
                    text = kit
                        .tr
                        .fmt("library.error.rate_limited.body_seconds", &[&s.to_string()]);
                }
                if let Some(d) = &error.detail {
                    text = format!("{text}\n{d}");
                }
                kit.banner(
                    ui,
                    Tone::Danger,
                    kit.t(title),
                    &text,
                    &[(dismiss, ButtonKind::Quiet)],
                )
            }
            LibNotice::Cancelled => kit.banner(
                ui,
                Tone::Info,
                kit.t("library.cancelled"),
                "",
                &[(dismiss, ButtonKind::Quiet)],
            ),
            LibNotice::Installed { kind, titles } => {
                let hint = match kind {
                    ContentKind::Mod => "content.installed.hint.mod",
                    ContentKind::ResourcePack => "content.installed.hint.resourcepack",
                    ContentKind::DataPack => "content.installed.hint.datapack",
                    ContentKind::Shader => "content.installed.hint.shader",
                };
                let body = format!("{}\n{}", titles.join(", "), kit.t(hint));
                kit.banner(
                    ui,
                    Tone::Success,
                    kit.t("content.installed.title"),
                    &body,
                    &[(dismiss, ButtonKind::Quiet)],
                )
            }
            LibNotice::ProfileSaved => kit.banner(
                ui,
                Tone::Success,
                kit.t("profile.saved"),
                kit.t("profile.saved.body"),
                &[(dismiss, ButtonKind::Quiet)],
            ),
            LibNotice::ThemeSaved => kit.banner(
                ui,
                Tone::Success,
                kit.t("appearance.saved"),
                "",
                &[(dismiss, ButtonKind::Quiet)],
            ),
            LibNotice::ContentChanged => kit.banner(
                ui,
                Tone::Success,
                kit.t("content.changed"),
                "",
                &[(dismiss, ButtonKind::Quiet)],
            ),
            LibNotice::IntegrityResolved => kit.banner(
                ui,
                Tone::Success,
                kit.t("integrity.resolved"),
                kit.t("integrity.resolved.body"),
                &[(dismiss, ButtonKind::Quiet)],
            ),
        };
        if pressed.is_some()
            && let Some(m) = self.lib.as_mut()
        {
            m.dismiss_notice();
        }
        ui.add_space(th.space.md);
    }

    // ----------------------------------------------------------------------------------------
    // Instances

    pub(super) fn instances_page(&mut self, ui: &mut Ui, kit: &Kit<'_>) {
        let th = kit.theme;
        kit.title(ui, kit.t("nav.instances"));
        kit.secondary(ui, kit.t("instances.intro"));
        ui.add_space(th.space.md);
        self.lib_status(ui, kit);
        let Some(model) = &self.lib else { return };
        let snap = model.snap.clone();
        let busy = model.busy();
        if !snap.capabilities.create.is_empty() && !self.lib_ui.create_open {
            if kit
                .button(ui, kit.t("instances.new"), ButtonKind::Secondary, !busy)
                .clicked()
            {
                self.lib_ui.create_open = true;
                self.lib_ui.create_name = kit.t("instances.new.default_name").to_owned();
                self.lib_ui.create_target = 0;
            }
            ui.add_space(th.space.md);
        }
        if self.lib_ui.create_open {
            self.create_card(ui, kit, &snap.capabilities.create, busy);
            ui.add_space(th.space.md);
        }
        for inst in &snap.instances {
            let selected = snap.selected.as_ref() == Some(&inst.id);
            kit.card(ui, crate::paint::Surface::Card, |ui| {
                self.instance_row(ui, kit, inst, selected, busy, &snap.capabilities);
            });
            ui.add_space(th.space.sm);
        }
        self.busy_hint(ui, kit);
    }

    fn create_card(&mut self, ui: &mut Ui, kit: &Kit<'_>, targets: &[GameTarget], busy: bool) {
        let th = kit.theme;
        kit.card(ui, crate::paint::Surface::Card, |ui| {
            kit.section(ui, kit.t("instances.new"));
            let hint = kit.t("instances.name.hint");
            kit.text_field(
                ui,
                kit.t("instances.name"),
                &mut self.lib_ui.create_name,
                hint,
            );
            ui.add_space(th.space.sm);
            let labels: Vec<String> = targets
                .iter()
                .map(|t| {
                    let ooze = kit.t(if t.ooze {
                        "instance.ooze_available"
                    } else {
                        "instance.ooze_unavailable"
                    });
                    format!("{} · {ooze}", target_text(kit, t))
                })
                .collect();
            let options: Vec<(usize, &str)> = labels
                .iter()
                .enumerate()
                .map(|(i, l)| (i, l.as_str()))
                .collect();
            kit.segmented(
                ui,
                "create-target",
                kit.t("instances.version_loader"),
                &options,
                &mut self.lib_ui.create_target,
            );
            kit.caption(ui, kit.t("instances.only_supported"));
            ui.add_space(th.space.sm);
            let name = self.lib_ui.create_name.trim().to_owned();
            let valid = (1..=48).contains(&name.chars().count());
            ui.horizontal(|ui| {
                if kit
                    .button(
                        ui,
                        kit.t("instances.create"),
                        ButtonKind::Primary,
                        valid && !busy,
                    )
                    .clicked()
                    && let Some(target) = targets.get(self.lib_ui.create_target).cloned()
                {
                    self.lib_ui.create_open = false;
                    self.lib_submit(LibCommand::Create { name, target });
                }
                if kit
                    .button(ui, kit.t("common.cancel"), ButtonKind::Quiet, true)
                    .clicked()
                {
                    self.lib_ui.create_open = false;
                }
            });
            if !valid {
                kit.caption(ui, kit.t("instances.name.invalid"));
            }
        });
    }

    fn instance_row(
        &mut self,
        ui: &mut Ui,
        kit: &Kit<'_>,
        inst: &InstanceSummary,
        selected: bool,
        busy: bool,
        caps: &LibraryCapabilities,
    ) {
        let th = kit.theme;
        ui.horizontal_wrapped(|ui| {
            kit.text(ui, th.text.subtitle, th.c.text_primary, &inst.name);
            if selected {
                kit.tag(ui, kit.t("instances.selected"), Some(Tone::Info));
            }
            kit.tag(ui, kit.t(mode_key(inst.mode)), None);
            match inst.install {
                InstallState::Ready => {}
                InstallState::NeedsInstall => {
                    kit.tag(ui, kit.t("instances.needs_install"), Some(Tone::Warning));
                }
                InstallState::NeedsRepair => {
                    kit.tag(ui, kit.t("instances.needs_repair"), Some(Tone::Danger));
                }
            }
            if inst.running {
                kit.tag(ui, kit.t("instances.running"), Some(Tone::Success));
            }
        });
        kit.body(ui, &target_text(kit, &inst.target));
        kit.secondary(
            ui,
            kit.t(if inst.target.ooze {
                "instance.ooze_available"
            } else {
                "instance.ooze_unavailable_long"
            }),
        );
        if inst.target.ooze {
            let profile = match inst.preset {
                Some(p) if inst.overrides > 0 => kit.tr.fmt(
                    "instances.profile_with_changes",
                    &[kit.t(p.name_key()), &inst.overrides.to_string()],
                ),
                Some(p) => kit.tr.fmt("instances.profile", &[kit.t(p.name_key())]),
                None => kit.t("instances.profile_custom").to_owned(),
            };
            kit.secondary(ui, &profile);
        }
        if inst.running {
            kit.caption(ui, kit.t("instances.running_locked"));
        }
        ui.add_space(th.space.sm);
        if let Some((id, name)) = self.lib_ui.rename.clone()
            && id == inst.id
        {
            let mut name = name;
            let hint = kit.t("instances.name.hint");
            kit.text_field(ui, kit.t("instances.name"), &mut name, hint);
            let ok = (1..=48).contains(&name.trim().chars().count());
            ui.horizontal(|ui| {
                if kit
                    .button(ui, kit.t("common.save"), ButtonKind::Primary, ok && !busy)
                    .clicked()
                {
                    self.lib_ui.rename = None;
                    self.lib_submit(LibCommand::Rename {
                        id: inst.id.clone(),
                        name: name.trim().to_owned(),
                    });
                    return;
                }
                if kit
                    .button(ui, kit.t("common.cancel"), ButtonKind::Quiet, true)
                    .clicked()
                {
                    self.lib_ui.rename = None;
                    return;
                }
                self.lib_ui.rename = Some((id.clone(), name.clone()));
            });
            return;
        }
        let free = !busy && !inst.running;
        ui.horizontal_wrapped(|ui| {
            if !selected
                && kit
                    .button(ui, kit.t("instances.select"), ButtonKind::Primary, !busy)
                    .clicked()
            {
                self.select_instance(inst);
            }
            if inst.actions.contains(&InstanceAction::Install)
                && kit
                    .button(ui, kit.t("instances.install"), ButtonKind::Secondary, free)
                    .clicked()
            {
                self.lib_submit(LibCommand::Install(inst.id.clone()));
            }
            if inst.actions.contains(&InstanceAction::Repair)
                && kit
                    .button(ui, kit.t("instances.repair"), ButtonKind::Secondary, free)
                    .clicked()
            {
                self.lib_submit(LibCommand::Install(inst.id.clone()));
            }
            if inst.actions.contains(&InstanceAction::Rename)
                && kit
                    .button(ui, kit.t("instances.rename"), ButtonKind::Secondary, free)
                    .clicked()
            {
                self.lib_ui.rename = Some((inst.id.clone(), inst.name.clone()));
            }
            if inst.actions.contains(&InstanceAction::Duplicate)
                && kit
                    .button(
                        ui,
                        kit.t("instances.duplicate"),
                        ButtonKind::Secondary,
                        free,
                    )
                    .clicked()
            {
                self.lib_ui.confirm = Some(Confirm::Duplicate(inst.id.clone()));
            }
            if kit
                .button(ui, kit.t("integrity.title"), ButtonKind::Secondary, true)
                .clicked()
            {
                let open = self.lib_ui.integrity_for.as_ref() == Some(&inst.id);
                self.lib_ui.integrity_for = (!open).then(|| inst.id.clone());
            }
            if inst.actions.contains(&InstanceAction::Archive)
                && kit
                    .button(ui, kit.t("instances.archive"), ButtonKind::Quiet, free)
                    .clicked()
            {
                self.lib_ui.confirm = Some(Confirm::Archive(inst.id.clone()));
            }
        });
        self.instance_confirm(ui, kit, inst);
        if self.lib_ui.integrity_for.as_ref() == Some(&inst.id) {
            ui.add_space(th.space.md);
            self.integrity_section(ui, kit, inst, caps, free);
        }
    }

    pub(super) fn select_instance(&mut self, inst: &InstanceSummary) {
        self.lib_abandon();
        if let Some(m) = self.lib.as_mut() {
            m.clear_instance_views();
        }
        self.lib_ui.draft = None;
        self.lib_ui.kind = None;
        self.lib_ui.world = None;
        self.lib_submit(LibCommand::Select(inst.id.clone()));
        // Play uses API 1: point the launcher at this instance's game directory.
        if let Some(mut settings) = self.model.settings.clone()
            && settings.install_root.as_ref() != Some(&inst.game_dir)
            && !self.model.game_live()
        {
            settings.install_root = Some(inst.game_dir.clone());
            self.submit(Command::UpdateSettings(settings));
        }
    }

    fn instance_confirm(&mut self, ui: &mut Ui, kit: &Kit<'_>, inst: &InstanceSummary) {
        let th = kit.theme;
        let Some(confirm) = self.lib_ui.confirm.clone() else {
            return;
        };
        let (title, body, action, command) = match &confirm {
            Confirm::Archive(id) if *id == inst.id => (
                kit.tr.fmt("instances.archive.confirm_title", &[&inst.name]),
                kit.t("instances.archive.confirm_body").to_owned(),
                kit.t("instances.archive"),
                LibCommand::Archive(id.clone()),
            ),
            Confirm::Duplicate(id) if *id == inst.id => (
                kit.tr
                    .fmt("instances.duplicate.confirm_title", &[&inst.name]),
                kit.t("instances.duplicate.confirm_body").to_owned(),
                kit.t("instances.duplicate"),
                LibCommand::Duplicate {
                    id: id.clone(),
                    name: kit.tr.fmt("instances.duplicate.name", &[&inst.name]),
                },
            ),
            Confirm::ToCustom(id) if *id == inst.id => (
                kit.t("integrity.to_custom.title").to_owned(),
                kit.t("integrity.to_custom.body").to_owned(),
                kit.t("integrity.to_custom"),
                LibCommand::SetMode {
                    id: id.clone(),
                    mode: IntegrityMode::Custom,
                },
            ),
            Confirm::ToManaged(id) if *id == inst.id => (
                kit.t("integrity.to_managed.title").to_owned(),
                kit.t("integrity.to_managed.body").to_owned(),
                kit.t("integrity.to_managed"),
                LibCommand::SetMode {
                    id: id.clone(),
                    mode: IntegrityMode::ManagedFairPlay,
                },
            ),
            _ => return,
        };
        ui.add_space(th.space.sm);
        match kit.banner(
            ui,
            Tone::Warning,
            &title,
            &body,
            &[
                (action, ButtonKind::Secondary),
                (kit.t("common.cancel"), ButtonKind::Quiet),
            ],
        ) {
            Some(0) => {
                self.lib_ui.confirm = None;
                self.lib_submit(command);
            }
            Some(_) => self.lib_ui.confirm = None,
            None => {}
        }
    }

    fn integrity_section(
        &mut self,
        ui: &mut Ui,
        kit: &Kit<'_>,
        inst: &InstanceSummary,
        caps: &LibraryCapabilities,
        free: bool,
    ) {
        let th = kit.theme;
        kit.section(ui, kit.t("integrity.title"));
        kit.body(
            ui,
            kit.t(match inst.mode {
                IntegrityMode::Custom => "integrity.custom.body",
                IntegrityMode::ManagedFairPlay => "integrity.managed.body",
            }),
        );
        kit.caption(ui, kit.t("integrity.limits"));
        ui.add_space(th.space.sm);
        ui.horizontal_wrapped(|ui| {
            if kit
                .button(
                    ui,
                    kit.t("integrity.check"),
                    ButtonKind::Secondary,
                    !self.lib_busy(),
                )
                .clicked()
            {
                self.lib_submit(LibCommand::CheckIntegrity(inst.id.clone()));
            }
            if inst.actions.contains(&InstanceAction::ChangeMode) {
                match inst.mode {
                    IntegrityMode::ManagedFairPlay => {
                        if kit
                            .button(ui, kit.t("integrity.to_custom"), ButtonKind::Quiet, free)
                            .clicked()
                        {
                            self.lib_ui.confirm = Some(Confirm::ToCustom(inst.id.clone()));
                        }
                    }
                    IntegrityMode::Custom if caps.managed_mode && inst.target.ooze => {
                        if kit
                            .button(ui, kit.t("integrity.to_managed"), ButtonKind::Quiet, free)
                            .clicked()
                        {
                            self.lib_ui.confirm = Some(Confirm::ToManaged(inst.id.clone()));
                        }
                    }
                    IntegrityMode::Custom => {}
                }
            }
        });
        let report = self
            .lib
            .as_ref()
            .and_then(|m| m.integrity.clone())
            .filter(|(id, _)| *id == inst.id)
            .map(|(_, r)| r);
        let Some(report) = report else { return };
        ui.add_space(th.space.sm);
        if report.issues.is_empty() {
            kit.banner(
                ui,
                Tone::Success,
                &kit.tr
                    .fmt("integrity.ok", &[&report.checked_files.to_string()]),
                "",
                &[],
            );
            return;
        }
        kit.text(
            ui,
            th.text.label,
            th.c.text_primary,
            kit.tr.fmt(
                "integrity.issues",
                &[
                    &report.issues.len().to_string(),
                    &report.checked_files.to_string(),
                ],
            ),
        );
        kit.caption(ui, kit.t("integrity.never_deleted"));
        for issue in &report.issues {
            let problem = match &issue.problem {
                IntegrityProblem::HashMismatch => kit.t("integrity.problem.hash").to_owned(),
                IntegrityProblem::Missing => kit.t("integrity.problem.missing").to_owned(),
                IntegrityProblem::UnexpectedMod => {
                    kit.t("integrity.problem.unexpected_mod").to_owned()
                }
                IntegrityProblem::UnapprovedPack => {
                    kit.t("integrity.problem.unapproved_pack").to_owned()
                }
                IntegrityProblem::ProhibitedLaunchSetting(s) => {
                    kit.tr.fmt("integrity.problem.launch_setting", &[s])
                }
            };
            let labels: Vec<(&str, ButtonKind)> = issue
                .actions
                .iter()
                .map(|a| {
                    (
                        kit.t(match a {
                            IntegrityAction::Repair => "integrity.action.repair",
                            IntegrityAction::Quarantine => "integrity.action.quarantine",
                            IntegrityAction::OpenFolder => "integrity.action.open_folder",
                        }),
                        ButtonKind::Secondary,
                    )
                })
                .collect();
            ui.add_space(th.space.xs);
            if let Some(i) = kit.banner(ui, Tone::Warning, &issue.file, &problem, &labels)
                && let Some(action) = issue.actions.get(i).copied()
            {
                self.lib_submit(LibCommand::Resolve {
                    instance: inst.id.clone(),
                    file: issue.file.clone(),
                    action,
                });
            }
        }
    }

    // ----------------------------------------------------------------------------------------
    // Profile

    pub(super) fn profile_page(&mut self, ui: &mut Ui, kit: &Kit<'_>) {
        let th = kit.theme;
        kit.title(ui, kit.t("nav.profile"));
        let Some(inst) = self.selected_instance().filter(|i| i.target.ooze) else {
            kit.body(ui, kit.t("instance.ooze_unavailable_long"));
            return;
        };
        kit.secondary(ui, &kit.tr.fmt("profile.intro", &[&inst.name]));
        ui.add_space(th.space.md);
        self.lib_status(ui, kit);
        let loaded = self
            .lib
            .as_ref()
            .and_then(|m| m.profile.clone())
            .filter(|(id, _, _)| *id == inst.id);
        let Some((_, saved, revision)) = loaded else {
            if !self.lib_busy() {
                self.lib_submit(LibCommand::LoadProfile(inst.id.clone()));
            }
            kit.spinner(ui, kit.t("profile.loading"));
            return;
        };
        // Adopt the saved profile unless the player has unsaved edits. Saving sets the draft to what
        // was sent, so a successful save (new revision, same content) is adopted here.
        let adopt = match &self.lib_ui.draft {
            Some((id, d, r)) => *id != inst.id || (*r != revision && *d == saved),
            None => true,
        };
        if adopt {
            self.lib_ui.draft = Some((inst.id.clone(), saved.clone(), revision));
        }
        let Some((_, mut draft, draft_rev)) = self.lib_ui.draft.clone() else {
            return;
        };
        let busy = self.lib_busy();
        let current = draft.resolve();

        // Presets
        kit.section(ui, kit.t("profile.presets"));
        kit.caption(ui, kit.t("profile.presets.intro"));
        ui.add_space(th.space.sm);
        for preset in PresetId::ALL {
            let is_current = draft.base == Some(preset);
            kit.card(ui, crate::paint::Surface::Card, |ui| {
                ui.horizontal_wrapped(|ui| {
                    kit.text(
                        ui,
                        th.text.subtitle,
                        th.c.text_primary,
                        kit.t(preset.name_key()),
                    );
                    if is_current {
                        kit.tag(ui, kit.t("profile.preset.current"), Some(Tone::Info));
                    }
                });
                kit.body(ui, kit.t(&format!("{}.description", preset.name_key())));
                kit.caption(ui, kit.t("profile.what_changes"));
                let changes = profile::diff(
                    &profile::defaults(),
                    &Profile::from_preset(preset, "p", "p").resolve(),
                );
                for c in &changes {
                    kit.caption(ui, &format!("• {}", change_text(kit, c)));
                }
                if !is_current
                    && kit
                        .button(
                            ui,
                            &kit.tr
                                .fmt("profile.use_preset", &[kit.t(preset.name_key())]),
                            ButtonKind::Secondary,
                            !busy,
                        )
                        .clicked()
                {
                    self.lib_ui.preset_confirm = Some(preset);
                    self.lib_ui.reset_overrides_too = false;
                }
            });
            ui.add_space(th.space.sm);
        }
        if let Some(preset) = self.lib_ui.preset_confirm {
            let mut next = draft.clone();
            next.base = Some(preset);
            if self.lib_ui.reset_overrides_too {
                next.reset_all();
            }
            let changes = next.diff_from(&current);
            let kept = crate::library::override_count(&draft);
            let mut body = changes
                .iter()
                .map(|c| format!("• {}", change_text(kit, c)))
                .collect::<Vec<_>>()
                .join("\n");
            if changes.is_empty() {
                body = kit.t("profile.no_changes").to_owned();
            }
            kit.banner(
                ui,
                Tone::Info,
                &kit.tr
                    .fmt("profile.switch.title", &[kit.t(preset.name_key())]),
                &body,
                &[],
            );
            if kept > 0 {
                kit.caption(ui, &kit.tr.fmt("profile.switch.kept", &[&kept.to_string()]));
                kit.switch(
                    ui,
                    kit.t("profile.switch.reset_too"),
                    &mut self.lib_ui.reset_overrides_too,
                    true,
                );
            }
            ui.horizontal(|ui| {
                if kit
                    .button(
                        ui,
                        kit.t("profile.switch.apply"),
                        ButtonKind::Primary,
                        !busy,
                    )
                    .clicked()
                {
                    self.lib_ui.preset_confirm = None;
                    self.save_profile(&inst.id, next.clone(), draft_rev);
                }
                if kit
                    .button(ui, kit.t("common.cancel"), ButtonKind::Quiet, true)
                    .clicked()
                {
                    self.lib_ui.preset_confirm = None;
                }
            });
            ui.add_space(th.space.md);
        }

        // Modules
        ui.add_space(th.space.md);
        kit.section(ui, kit.t("profile.modules"));
        kit.caption(ui, kit.t("profile.modules.intro"));
        ui.add_space(th.space.sm);
        for def in MODULES.iter().filter(|d| d.class != FairPlay::Gameplay) {
            kit.card(ui, crate::paint::Surface::Card, |ui| {
                self.module_row(ui, kit, def, &mut draft, &current);
            });
            ui.add_space(th.space.sm);
        }

        // Visual
        ui.add_space(th.space.md);
        kit.section(ui, kit.t("profile.visual"));
        for def in VISUAL_SETTINGS.iter() {
            let value = current
                .visual
                .get(def.key)
                .cloned()
                .unwrap_or_else(|| def.default());
            if let Some(json) = self.setting_editor(ui, kit, def, &value, "visual", None) {
                let _ = draft.set_visual(def.key, &json);
            }
        }

        // HUD
        ui.add_space(th.space.md);
        kit.section(ui, kit.t("profile.hud"));
        kit.caption(ui, kit.t("profile.hud.in_game"));
        if kit
            .button(
                ui,
                kit.t("profile.hud.reset"),
                ButtonKind::Secondary,
                !draft.overrides.hud.is_empty(),
            )
            .clicked()
        {
            draft.reset("hud");
        }

        if self.lib_ui.draft.as_ref().map(|d| &d.1) != Some(&draft) {
            self.lib_ui.draft = Some((inst.id.clone(), draft.clone(), draft_rev));
        }

        // Save bar and profile actions
        ui.add_space(th.space.lg);
        let dirty = draft != saved;
        if dirty {
            kit.banner(ui, Tone::Info, kit.t("profile.unsaved"), "", &[]);
        }
        ui.horizontal_wrapped(|ui| {
            if kit
                .button(
                    ui,
                    kit.t("common.save"),
                    ButtonKind::Primary,
                    dirty && !busy,
                )
                .clicked()
            {
                self.save_profile(&inst.id, draft.clone(), draft_rev);
            }
            if kit
                .button(ui, kit.t("profile.discard"), ButtonKind::Secondary, dirty)
                .clicked()
            {
                self.lib_ui.draft = Some((inst.id.clone(), saved.clone(), revision));
            }
            if kit
                .button(ui, kit.t("profile.reset_all"), ButtonKind::Secondary, !busy)
                .clicked()
            {
                self.lib_ui.confirm = Some(Confirm::ResetProfile);
            }
            if kit
                .button(ui, kit.t("profile.export"), ButtonKind::Secondary, true)
                .clicked()
            {
                ui.ctx().copy_text(saved.to_json());
                if let Some(m) = self.lib.as_mut() {
                    m.dismiss_notice();
                }
                self.live_text = kit.t("profile.exported").to_owned();
            }
            if kit
                .button(ui, kit.t("profile.import"), ButtonKind::Secondary, true)
                .clicked()
            {
                self.lib_ui.profile_import = Some(String::new());
                self.lib_ui.profile_import_result = None;
            }
        });
        if self.lib_ui.confirm == Some(Confirm::ResetProfile) {
            match kit.banner(
                ui,
                Tone::Warning,
                kit.t("profile.reset_all.confirm_title"),
                kit.t("profile.reset_all.confirm_body"),
                &[
                    (kit.t("profile.reset_all"), ButtonKind::Secondary),
                    (kit.t("common.cancel"), ButtonKind::Quiet),
                ],
            ) {
                Some(0) => {
                    self.lib_ui.confirm = None;
                    let mut next = draft.clone();
                    next.reset_all();
                    self.save_profile(&inst.id, next, draft_rev);
                }
                Some(_) => self.lib_ui.confirm = None,
                None => {}
            }
        }
        self.profile_import(ui, kit, &inst, &draft, draft_rev, &current);
    }

    fn save_profile(&mut self, id: &InstanceId, profile: Profile, revision: u64) {
        self.lib_ui.draft = Some((id.clone(), profile.clone(), revision));
        self.lib_submit(LibCommand::SaveProfile {
            id: id.clone(),
            profile,
            revision,
        });
    }

    fn profile_import(
        &mut self,
        ui: &mut Ui,
        kit: &Kit<'_>,
        inst: &InstanceSummary,
        draft: &Profile,
        revision: u64,
        current: &profile::Effective,
    ) {
        let th = kit.theme;
        let Some(mut text) = self.lib_ui.profile_import.take() else {
            return;
        };
        ui.add_space(th.space.md);
        kit.text_area(ui, kit.t("profile.import.paste"), &mut text, 6);
        let mut close = false;
        ui.horizontal(|ui| {
            if kit
                .button(
                    ui,
                    kit.t("profile.import.check"),
                    ButtonKind::Secondary,
                    !text.trim().is_empty(),
                )
                .clicked()
            {
                self.lib_ui.profile_import_result = Some(Profile::from_json(&text));
            }
            if kit
                .button(ui, kit.t("common.cancel"), ButtonKind::Quiet, true)
                .clicked()
            {
                close = true;
            }
        });
        match self.lib_ui.profile_import_result.clone() {
            Some(Err(e)) => {
                let key = match e {
                    ImportError::NotJson(_) => "profile.import.not_json",
                    ImportError::WrongSchema => "profile.import.wrong_schema",
                    ImportError::MissingField(_) => "profile.import.missing_field",
                    ImportError::BadId => "profile.import.bad_id",
                };
                kit.banner(
                    ui,
                    Tone::Danger,
                    kit.t("profile.import.refused"),
                    kit.t(key),
                    &[],
                );
            }
            Some(Ok((imported, warnings))) => {
                let mut next = draft.clone();
                next.base = imported.base;
                next.overrides = imported.overrides.clone();
                let changes = next.diff_from(current);
                let mut body: Vec<String> = changes
                    .iter()
                    .map(|c| format!("• {}", change_text(kit, c)))
                    .collect();
                if body.is_empty() {
                    body.push(kit.t("profile.no_changes").to_owned());
                }
                for w in &warnings {
                    body.push(kit.tr.fmt("profile.import.skipped", &[&w.path]));
                }
                if kit
                    .banner(
                        ui,
                        Tone::Info,
                        &kit.tr.fmt("profile.import.review", &[&imported.name]),
                        &body.join("\n"),
                        &[(kit.t("profile.import.apply"), ButtonKind::Primary)],
                    )
                    .is_some()
                {
                    close = true;
                    self.save_profile(&inst.id, next, revision);
                }
            }
            None => {}
        }
        if close {
            self.lib_ui.profile_import_result = None;
        } else {
            self.lib_ui.profile_import = Some(text);
        }
    }

    fn module_row(
        &mut self,
        ui: &mut Ui,
        kit: &Kit<'_>,
        def: &ModuleDef,
        draft: &mut Profile,
        current: &profile::Effective,
    ) {
        let th = kit.theme;
        let state = &current.modules[&def.id];
        let mut on = state.enabled;
        ui.horizontal_wrapped(|ui| {
            if kit.switch(ui, kit.t(def.name_key), &mut on, true).changed() {
                draft.set_enabled(def.id, on);
            }
            kit.tag(ui, kit.t(def.class.key()), None);
            let changed = draft.overrides.modules.get(&def.id).is_some_and(|o| {
                o.enabled
                    .is_some_and(|e| Some(e) != def_default_enabled(draft, def.id))
                    || !o.settings.is_empty()
            });
            if changed {
                kit.tag(ui, kit.t("profile.changed"), Some(Tone::Info));
            }
        });
        kit.secondary(ui, kit.t(def.description_key));
        if def.server_sensitive {
            kit.caption(ui, kit.t("profile.server_sensitive"));
        }
        if def.settings.is_empty() {
            return;
        }
        let open = self.lib_ui.expanded == Some(def.id);
        if kit
            .button(
                ui,
                kit.t(if open {
                    "profile.hide_settings"
                } else {
                    "profile.show_settings"
                }),
                ButtonKind::Quiet,
                true,
            )
            .clicked()
        {
            self.lib_ui.expanded = (!open).then_some(def.id);
        }
        if !open {
            return;
        }
        ui.add_space(th.space.xs);
        for s in def.settings {
            let value = state
                .settings
                .get(s.key)
                .cloned()
                .unwrap_or_else(|| s.default());
            let overridden = draft
                .overrides
                .modules
                .get(&def.id)
                .is_some_and(|o| o.settings.contains_key(s.key));
            if let Some(json) =
                self.setting_editor(ui, kit, s, &value, def.id.as_str(), Some(def.id))
            {
                let _ = draft.set_setting(def.id, s.key, &json);
            }
            if overridden
                && kit
                    .button(
                        ui,
                        &kit.tr.fmt("profile.reset_setting", &[kit.t(s.label_key)]),
                        ButtonKind::Quiet,
                        true,
                    )
                    .clicked()
            {
                draft.reset(&format!("modules.{}.settings.{}", def.id.as_str(), s.key));
            }
            ui.add_space(th.space.xs);
        }
    }

    /// One typed editor; returns a new value (already shaped for `SettingDef::validate`).
    fn setting_editor(
        &mut self,
        ui: &mut Ui,
        kit: &Kit<'_>,
        def: &'static SettingDef,
        value: &Value,
        scope: &str,
        module: Option<ModuleId>,
    ) -> Option<Json> {
        let th = kit.theme;
        let label = kit.t(def.label_key);
        match (&def.kind, value) {
            (SettingKind::Bool { .. }, Value::Bool(b)) => {
                let mut v = *b;
                kit.switch(ui, label, &mut v, true)
                    .changed()
                    .then_some(Json::Bool(v))
            }
            (SettingKind::Int { min, max, step, .. }, Value::Int(n)) => {
                let (dec, inc) = self.stepper(ui, kit, label, &n.to_string(), *n > *min, *n < *max);
                dec.then(|| Json::Number((n - step) as f64))
                    .or_else(|| inc.then(|| Json::Number((n + step) as f64)))
            }
            (SettingKind::Float { min, max, step, .. }, Value::Float(f)) => {
                let (dec, inc) = self.stepper(
                    ui,
                    kit,
                    label,
                    &format!("{f}"),
                    *f > *min + 1e-9,
                    *f < *max - 1e-9,
                );
                dec.then(|| Json::Number(f - step))
                    .or_else(|| inc.then(|| Json::Number(f + step)))
            }
            (SettingKind::Choice { options, .. }, Value::Choice(c)) => {
                let keys: Vec<String> = options.iter().map(|o| setting_label_key(def, o)).collect();
                let opts: Vec<(usize, &str)> = keys
                    .iter()
                    .enumerate()
                    .map(|(i, k)| (i, kit.t(k)))
                    .collect();
                let mut idx = options.iter().position(|o| o == c).unwrap_or(0);
                let salt = format!("{scope}.{}", def.key);
                kit.segmented(ui, &salt, label, &opts, &mut idx)
                    .then(|| Json::String(options[idx].to_owned()))
            }
            (SettingKind::Color { .. }, Value::Color(c)) => {
                let key = format!("{scope}.{}", def.key);
                let idx = match self.lib_ui.color_text.iter().position(|(k, _)| *k == key) {
                    Some(i) => i,
                    None => {
                        self.lib_ui
                            .color_text
                            .push((key.clone(), format!("#{c:08X}")));
                        self.lib_ui.color_text.len() - 1
                    }
                };
                let mut text = self.lib_ui.color_text[idx].1.clone();
                let resp = kit.text_field(ui, label, &mut text, kit.t("profile.color.hint"));
                self.lib_ui.color_text[idx].1 = text.clone();
                if resp.lost_focus()
                    || (resp.has_focus() && ui.input(|i| i.key_pressed(Key::Enter)))
                {
                    let json = Json::String(text.trim().to_uppercase());
                    match def.validate(&json) {
                        Ok(_) => {
                            self.lib_ui.color_error = None;
                            return Some(json);
                        }
                        Err(_) => self.lib_ui.color_error = Some(key.clone()),
                    }
                }
                if self.lib_ui.color_error.as_deref() == Some(key.as_str()) {
                    kit.caption(ui, kit.t("profile.color.invalid"));
                }
                ui.add_space(th.space.xxs);
                None
            }
            (SettingKind::Key { .. }, Value::Key(k)) => {
                let capturing = module.is_some_and(|m| self.lib_ui.capturing == Some((m, def.key)));
                let shown = k
                    .trim_start_matches("key.keyboard.")
                    .trim_start_matches("key.mouse.");
                kit.text(ui, th.text.label, th.c.text_primary, label);
                if capturing {
                    kit.caption(ui, kit.t("profile.key.press"));
                    let pressed = ui.input(|i| {
                        i.events.iter().find_map(|e| match e {
                            egui::Event::Key {
                                key, pressed: true, ..
                            } => Some(*key),
                            _ => None,
                        })
                    });
                    match pressed {
                        Some(Key::Escape) => self.lib_ui.capturing = None,
                        Some(key) => {
                            if let Some(name) = vanilla_key(key) {
                                self.lib_ui.capturing = None;
                                return Some(Json::String(name));
                            }
                        }
                        None => {}
                    }
                } else if let Some(m) = module
                    && kit
                        .button(
                            ui,
                            &kit.tr.fmt("profile.key.change", &[&shown.to_uppercase()]),
                            ButtonKind::Secondary,
                            true,
                        )
                        .clicked()
                {
                    self.lib_ui.capturing = Some((m, def.key));
                }
                None
            }
            _ => None,
        }
    }

    fn stepper(
        &self,
        ui: &mut Ui,
        kit: &Kit<'_>,
        label: &str,
        value: &str,
        can_dec: bool,
        can_inc: bool,
    ) -> (bool, bool) {
        let th = kit.theme;
        kit.text(ui, th.text.label, th.c.text_primary, label);
        let mut out = (false, false);
        ui.horizontal(|ui| {
            out.0 = kit
                .button(
                    ui,
                    &kit.tr.fmt("profile.decrease", &[label]),
                    ButtonKind::Secondary,
                    can_dec,
                )
                .clicked();
            kit.text(ui, th.text.body, th.c.text_primary, value);
            out.1 = kit
                .button(
                    ui,
                    &kit.tr.fmt("profile.increase", &[label]),
                    ButtonKind::Secondary,
                    can_inc,
                )
                .clicked();
        });
        out
    }

    // ----------------------------------------------------------------------------------------
    // Content (Modrinth)

    pub(super) fn content_page(&mut self, ui: &mut Ui, kit: &Kit<'_>) {
        let th = kit.theme;
        kit.title(ui, kit.t("nav.content"));
        let Some(inst) = self.selected_instance() else {
            return;
        };
        // Only kinds the library supports for this particular instance.
        let kinds: Vec<ContentKind> = self
            .lib
            .as_ref()
            .map(|m| m.caps().content.clone())
            .unwrap_or_default()
            .into_iter()
            .filter(|k| inst.content.contains(k))
            .collect();
        if kinds.is_empty() {
            return;
        }
        kit.secondary(
            ui,
            &kit.tr.fmt(
                "content.intro",
                &[&inst.name, &target_text(kit, &inst.target)],
            ),
        );
        ui.add_space(th.space.md);
        self.lib_status(ui, kit);
        let mut kind = self
            .lib_ui
            .kind
            .filter(|k| kinds.contains(k))
            .unwrap_or(kinds[0]);
        let labels: Vec<(ContentKind, &str)> = kinds.iter().map(|k| (*k, kit.t(k.key()))).collect();
        if kit.segmented(
            ui,
            "content-kind",
            kit.t("content.kind"),
            &labels,
            &mut kind,
        ) {
            self.lib_abandon();
            if let Some(m) = self.lib.as_mut() {
                m.search = None;
                m.project = None;
                m.plan = None;
                m.installed = None;
            }
            self.lib_ui.version = None;
        }
        self.lib_ui.kind = Some(kind);
        ui.add_space(th.space.sm);
        let busy = self.lib_busy();

        if kind == ContentKind::Shader {
            kit.banner(
                ui,
                Tone::Info,
                kit.t("content.shader.title"),
                kit.t("content.shader.body"),
                &[],
            );
            ui.add_space(th.space.sm);
        }
        if kind == ContentKind::DataPack {
            let worlds = self
                .lib
                .as_ref()
                .and_then(|m| m.worlds.clone())
                .filter(|(id, _)| *id == inst.id)
                .map(|(_, w)| w);
            match worlds {
                None => {
                    if !busy {
                        self.lib_submit(LibCommand::ListWorlds(inst.id.clone()));
                    }
                    kit.spinner(ui, kit.t("content.datapack.loading_worlds"));
                    return;
                }
                Some(w) if w.is_empty() => {
                    kit.banner(
                        ui,
                        Tone::Info,
                        kit.t("content.datapack.no_worlds.title"),
                        kit.t("content.datapack.no_worlds.body"),
                        &[],
                    );
                    return;
                }
                Some(w) => {
                    let mut idx = self
                        .lib_ui
                        .world
                        .as_ref()
                        .and_then(|k| w.iter().position(|x| &x.key == k))
                        .unwrap_or(usize::MAX);
                    let opts: Vec<(usize, &str)> = w
                        .iter()
                        .enumerate()
                        .map(|(i, x)| (i, x.name.as_str()))
                        .collect();
                    if kit.segmented(
                        ui,
                        "content-world",
                        kit.t("content.datapack.world"),
                        &opts,
                        &mut idx,
                    ) {
                        self.lib_ui.world = w.get(idx).map(|x| x.key.clone());
                    }
                    if inst.running {
                        kit.caption(ui, kit.t("content.datapack.game_running"));
                    }
                    if self.lib_ui.world.is_none() {
                        kit.caption(ui, kit.t("content.datapack.choose_world"));
                        return;
                    }
                }
            }
            ui.add_space(th.space.sm);
        }

        // Search
        let search_label = kit.t("content.search");
        let hint = kit.t("content.search.hint");
        let resp = kit.text_field(ui, search_label, &mut self.lib_ui.query, hint);
        let enter = resp.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
        let mut sort = self.lib_ui.sort.unwrap_or(SortBy::Relevance);
        kit.segmented(
            ui,
            "content-sort",
            kit.t("content.sort"),
            &[
                (SortBy::Relevance, kit.t("content.sort.relevance")),
                (SortBy::Downloads, kit.t("content.sort.downloads")),
                (SortBy::Updated, kit.t("content.sort.updated")),
            ],
            &mut sort,
        );
        self.lib_ui.sort = Some(sort);
        ui.horizontal(|ui| {
            if (kit
                .button(ui, kit.t("content.search.go"), ButtonKind::Secondary, !busy)
                .clicked()
                || enter)
                && !busy
            {
                self.lib_submit(LibCommand::Search(SearchQuery {
                    instance: inst.id.clone(),
                    kind,
                    text: self.lib_ui.query.trim().to_owned(),
                    sort,
                    categories: Vec::new(),
                    offset: 0,
                }));
            }
            if kit
                .button(
                    ui,
                    kit.t("content.installed.show"),
                    ButtonKind::Quiet,
                    !busy,
                )
                .clicked()
            {
                self.lib_submit(LibCommand::ListInstalled {
                    instance: inst.id.clone(),
                    kind,
                });
            }
        });
        ui.add_space(th.space.md);

        self.plan_review(ui, kit, &inst, busy);
        self.project_details(ui, kit, &inst, kind, busy);
        self.search_results(ui, kit, &inst, kind, busy);
        self.installed_list(ui, kit, &inst, kind, busy);
    }

    fn search_results(
        &mut self,
        ui: &mut Ui,
        kit: &Kit<'_>,
        inst: &InstanceSummary,
        kind: ContentKind,
        busy: bool,
    ) {
        let th = kit.theme;
        let Some(page) = self
            .lib
            .as_ref()
            .and_then(|m| m.search.clone())
            .filter(|p| p.query.instance == inst.id && p.query.kind == kind)
        else {
            return;
        };
        if self
            .lib
            .as_ref()
            .is_some_and(|m| m.project.is_some() || m.plan.is_some())
        {
            return;
        }
        kit.text(
            ui,
            th.text.label,
            th.c.text_primary,
            kit.tr.fmt("content.results", &[&page.total.to_string()]),
        );
        kit.caption(
            ui,
            &kit.tr
                .fmt("content.compat_note", &[&target_text(kit, &inst.target)]),
        );
        if page.hits.is_empty() {
            kit.body(ui, kit.t("content.no_results"));
        }
        for hit in &page.hits {
            ui.add_space(th.space.sm);
            kit.card(ui, crate::paint::Surface::Card, |ui| {
                kit.text(ui, th.text.subtitle, th.c.text_primary, &hit.title);
                kit.caption(
                    ui,
                    &kit.tr.fmt(
                        "content.by",
                        &[&hit.author, &hit.downloads.to_string(), &hit.updated],
                    ),
                );
                kit.body(ui, &hit.summary);
                if kit
                    .button(
                        ui,
                        &kit.tr.fmt("content.view", &[&hit.title]),
                        ButtonKind::Secondary,
                        !busy,
                    )
                    .clicked()
                {
                    self.lib_ui.version = None;
                    self.lib_submit(LibCommand::Project {
                        instance: inst.id.clone(),
                        kind,
                        project_id: hit.project_id.clone(),
                    });
                }
            });
        }
    }

    fn project_details(
        &mut self,
        ui: &mut Ui,
        kit: &Kit<'_>,
        inst: &InstanceSummary,
        kind: ContentKind,
        busy: bool,
    ) {
        let th = kit.theme;
        let Some(d) = self
            .lib
            .as_ref()
            .and_then(|m| m.project.clone())
            .filter(|d| d.instance == inst.id && d.kind == kind)
        else {
            return;
        };
        if self.lib.as_ref().is_some_and(|m| m.plan.is_some()) {
            return;
        }
        if kit
            .button(
                ui,
                kit.t("content.back_to_results"),
                ButtonKind::Quiet,
                true,
            )
            .clicked()
            && let Some(m) = self.lib.as_mut()
        {
            m.project = None;
            return;
        }
        kit.card(ui, crate::paint::Surface::Card, |ui| {
            kit.text(ui, th.text.subtitle, th.c.text_primary, &d.title);
            kit.caption(ui, &kit.tr.fmt("content.author", &[&d.author]));
            if let Some(l) = &d.license {
                kit.caption(ui, &kit.tr.fmt("content.license", &[l]));
            }
            ui.add_space(th.space.xs);
            kit.body(ui, &d.description);
            for (label, url) in &d.links {
                let host = url
                    .strip_prefix("https://")
                    .and_then(|r| r.split('/').next())
                    .unwrap_or_default();
                if !host.is_empty()
                    && kit
                        .button(
                            ui,
                            &kit.tr.fmt("content.open_link", &[label, host]),
                            ButtonKind::Quiet,
                            true,
                        )
                        .clicked()
                {
                    ui.ctx().open_url(egui::OpenUrl::new_tab(url));
                }
            }
        });
        ui.add_space(th.space.md);
        kit.section(ui, kit.t("content.versions"));
        let compatible: Vec<&VersionSummary> =
            d.versions.iter().filter(|v| v.compatible.is_ok()).collect();
        if self.lib_ui.version.is_none() {
            self.lib_ui.version = compatible
                .iter()
                .find(|v| v.channel == Channel::Release)
                .or(compatible.first())
                .map(|v| v.version_id.clone());
        }
        for v in &d.versions {
            let channel = kit.t(match v.channel {
                Channel::Release => "content.channel.release",
                Channel::Beta => "content.channel.beta",
                Channel::Alpha => "content.channel.alpha",
            });
            let line = kit.tr.fmt(
                "content.version_line",
                &[
                    &v.number,
                    channel,
                    &v.published,
                    &v.game_versions.join(", "),
                    &v.loaders.join(", "),
                    &mb(v.size_bytes),
                ],
            );
            match &v.compatible {
                Ok(()) => {
                    let chosen = self.lib_ui.version.as_deref() == Some(v.version_id.as_str());
                    kit.body(ui, &line);
                    if chosen {
                        kit.tag(ui, kit.t("content.version_chosen"), Some(Tone::Info));
                    } else if kit
                        .button(
                            ui,
                            &kit.tr.fmt("content.choose_version", &[&v.number]),
                            ButtonKind::Secondary,
                            true,
                        )
                        .clicked()
                    {
                        self.lib_ui.version = Some(v.version_id.clone());
                    }
                }
                Err(reason) => {
                    kit.secondary(ui, &line);
                    kit.caption(ui, &kit.tr.fmt("content.version_incompatible", &[reason]));
                }
            }
        }
        ui.add_space(th.space.sm);
        let world = self.lib_ui.world.clone();
        let ready = self.lib_ui.version.is_some()
            && (kind != ContentKind::DataPack || (world.is_some() && !inst.running));
        if kit
            .button(
                ui,
                kit.t("content.review"),
                ButtonKind::Primary,
                ready && !busy,
            )
            .clicked()
            && let Some(version_id) = self.lib_ui.version.clone()
        {
            self.lib_submit(LibCommand::Plan {
                instance: inst.id.clone(),
                kind,
                version_id,
                world: if kind == ContentKind::DataPack {
                    world
                } else {
                    None
                },
            });
        }
        if compatible.is_empty() {
            kit.caption(ui, kit.t("content.no_compatible_version"));
        }
    }

    fn plan_review(&mut self, ui: &mut Ui, kit: &Kit<'_>, inst: &InstanceSummary, busy: bool) {
        let th = kit.theme;
        let Some(plan) = self
            .lib
            .as_ref()
            .and_then(|m| m.plan.clone())
            .filter(|p| p.instance == inst.id)
        else {
            return;
        };
        kit.card(ui, crate::paint::Surface::Card, |ui| {
            kit.section(ui, kit.t("content.plan.title"));
            if let Some(w) = &plan.world {
                kit.body(ui, &kit.tr.fmt("content.plan.world", &[w]));
            }
            kit.text(
                ui,
                th.text.label,
                th.c.text_primary,
                kit.t("content.plan.install"),
            );
            for item in &plan.install {
                let role = kit.t(match item.role {
                    PlanRole::Selected => "content.plan.selected",
                    PlanRole::Required => "content.plan.required",
                });
                kit.body(
                    ui,
                    &kit.tr.fmt(
                        "content.plan.item",
                        &[
                            &item.title,
                            &item.version_number,
                            role,
                            &mb(item.size_bytes),
                        ],
                    ),
                );
            }
            if !plan.optional.is_empty() {
                kit.text(
                    ui,
                    th.text.label,
                    th.c.text_primary,
                    kit.t("content.plan.optional"),
                );
                for o in &plan.optional {
                    kit.secondary(ui, &format!("• {o}"));
                }
            }
            if !plan.embedded.is_empty() {
                kit.text(
                    ui,
                    th.text.label,
                    th.c.text_primary,
                    kit.t("content.plan.embedded"),
                );
                for e in &plan.embedded {
                    kit.secondary(ui, &format!("• {e}"));
                }
            }
            match &plan.shader_loader {
                Some(ShaderLoader::Present(n)) => {
                    kit.body(ui, &kit.tr.fmt("content.plan.loader_present", &[n]));
                }
                Some(ShaderLoader::InPlan(n)) => {
                    kit.body(ui, &kit.tr.fmt("content.plan.loader_in_plan", &[n]));
                }
                Some(ShaderLoader::Unavailable) => {
                    kit.banner(
                        ui,
                        Tone::Danger,
                        kit.t("content.plan.loader_unavailable"),
                        kit.t("content.plan.loader_unavailable.body"),
                        &[],
                    );
                }
                None => {}
            }
            for c in &plan.conflicts {
                kit.banner(
                    ui,
                    Tone::Danger,
                    &kit.tr.fmt("content.plan.conflict", &[&c.title, &c.with]),
                    kit.t("content.plan.conflict.body"),
                    &[],
                );
            }
            kit.caption(ui, kit.t("content.plan.verified_note"));
            ui.add_space(th.space.sm);
            ui.horizontal(|ui| {
                if kit
                    .button(
                        ui,
                        kit.t("content.install"),
                        ButtonKind::Primary,
                        !plan.blocked()
                            && !busy
                            && !(plan.kind == ContentKind::DataPack && inst.running),
                    )
                    .clicked()
                {
                    self.lib_submit(LibCommand::Apply {
                        plan_id: plan.plan_id,
                    });
                }
                if kit
                    .button(ui, kit.t("content.plan.back"), ButtonKind::Quiet, !busy)
                    .clicked()
                    && let Some(m) = self.lib.as_mut()
                {
                    m.plan = None;
                }
            });
        });
        ui.add_space(th.space.md);
    }

    fn installed_list(
        &mut self,
        ui: &mut Ui,
        kit: &Kit<'_>,
        inst: &InstanceSummary,
        kind: ContentKind,
        busy: bool,
    ) {
        let th = kit.theme;
        let Some((_, _, items)) = self
            .lib
            .as_ref()
            .and_then(|m| m.installed.clone())
            .filter(|(id, k, _)| *id == inst.id && *k == kind)
        else {
            return;
        };
        ui.add_space(th.space.md);
        kit.section(ui, kit.t("content.installed.list"));
        if items.is_empty() {
            kit.body(ui, kit.t("content.installed.none"));
        }
        let free = !busy && !inst.running;
        for item in &items {
            ui.add_space(th.space.sm);
            kit.card(ui, crate::paint::Surface::Card, |ui| {
                ui.horizontal_wrapped(|ui| {
                    kit.text(ui, th.text.label, th.c.text_primary, &item.title);
                    let (key, tone) = match item.state {
                        ContentState::Enabled => ("content.state.enabled", None),
                        ContentState::Disabled => ("content.state.disabled", None),
                        ContentState::Modified => ("content.state.modified", Some(Tone::Warning)),
                        ContentState::Missing => ("content.state.missing", Some(Tone::Danger)),
                    };
                    kit.tag(ui, kit.t(key), tone);
                });
                kit.caption(
                    ui,
                    &kit.tr.fmt(
                        "content.installed.file",
                        &[&item.version_number, &item.file_name],
                    ),
                );
                if let Some(w) = &item.world {
                    kit.caption(ui, &kit.tr.fmt("content.plan.world", &[w]));
                }
                ui.horizontal_wrapped(|ui| {
                    if let Some(v) = &item.update
                        && kit
                            .button(
                                ui,
                                &kit.tr.fmt("content.update", &[v]),
                                ButtonKind::Secondary,
                                free,
                            )
                            .clicked()
                    {
                        self.lib_submit(LibCommand::Update {
                            instance: inst.id.clone(),
                            record_id: item.record_id.clone(),
                        });
                    }
                    match item.state {
                        ContentState::Enabled => {
                            if kit
                                .button(ui, kit.t("content.disable"), ButtonKind::Secondary, free)
                                .clicked()
                            {
                                self.lib_submit(LibCommand::SetEnabled {
                                    instance: inst.id.clone(),
                                    record_id: item.record_id.clone(),
                                    enabled: false,
                                });
                            }
                        }
                        ContentState::Disabled => {
                            if kit
                                .button(ui, kit.t("content.enable"), ButtonKind::Secondary, free)
                                .clicked()
                            {
                                self.lib_submit(LibCommand::SetEnabled {
                                    instance: inst.id.clone(),
                                    record_id: item.record_id.clone(),
                                    enabled: true,
                                });
                            }
                        }
                        ContentState::Modified | ContentState::Missing => {}
                    }
                    if kit
                        .button(ui, kit.t("content.remove"), ButtonKind::Quiet, free)
                        .clicked()
                    {
                        self.lib_ui.confirm = Some(Confirm::Remove(item.clone()));
                    }
                });
                if self.lib_ui.confirm == Some(Confirm::Remove(item.clone())) {
                    match kit.banner(
                        ui,
                        Tone::Warning,
                        &kit.tr.fmt("content.remove.confirm_title", &[&item.title]),
                        &kit.tr
                            .fmt("content.remove.confirm_body", &[&item.file_name]),
                        &[
                            (kit.t("content.remove"), ButtonKind::Secondary),
                            (kit.t("common.cancel"), ButtonKind::Quiet),
                        ],
                    ) {
                        Some(0) => {
                            self.lib_ui.confirm = None;
                            self.lib_submit(LibCommand::Remove {
                                instance: inst.id.clone(),
                                record_id: item.record_id.clone(),
                            });
                        }
                        Some(_) => self.lib_ui.confirm = None,
                        None => {}
                    }
                }
            });
        }
    }

    // ----------------------------------------------------------------------------------------
    // Appearance

    /// The theme document currently being shown: the draft (live preview) or the saved one.
    pub(super) fn shown_theme_doc(&self) -> Option<ThemeDoc> {
        let m = self.lib.as_ref().filter(|m| m.caps().themes)?;
        Some(
            self.lib_ui
                .appearance
                .clone()
                .unwrap_or_else(|| self.saved_theme_doc(m)),
        )
    }

    fn saved_theme_doc(&self, m: &crate::library_model::LibraryModel) -> ThemeDoc {
        let mut doc = m.snap.theme.clone();
        if let Some(s) = &self.model.settings {
            doc.motion = appearance::motion_doc(s.motion);
            doc.ui_scale = s.ui_scale_percent;
        }
        doc
    }

    pub(super) fn appearance_page(&mut self, ui: &mut Ui, kit: &Kit<'_>) {
        let th = kit.theme;
        kit.title(ui, kit.t("nav.appearance"));
        kit.secondary(ui, kit.t("appearance.intro"));
        ui.add_space(th.space.md);
        self.lib_status(ui, kit);
        let (Some(model), Some(guard)) = (self.lib.as_ref(), self.guard.clone()) else {
            return;
        };
        let saved = self.saved_theme_doc(model);
        let mut doc = self
            .lib_ui
            .appearance
            .clone()
            .unwrap_or_else(|| saved.clone());
        let busy = self.lib_busy();

        kit.section(ui, kit.t("appearance.accent"));
        ui.horizontal_wrapped(|ui| {
            let default_label = kit.t("appearance.accent.ink");
            let chosen = doc.accent.is_none();
            if kit
                .button(
                    ui,
                    default_label,
                    if chosen {
                        ButtonKind::Primary
                    } else {
                        ButtonKind::Secondary
                    },
                    true,
                )
                .clicked()
            {
                doc.accent = None;
            }
            for (name, rgb) in guard.swatches.iter().filter(|(n, _)| *n != "ink") {
                let label = kit.t(&format!("appearance.accent.{name}")).to_owned();
                let on = doc.accent == Some(*rgb);
                if kit
                    .button(
                        ui,
                        &label,
                        if on {
                            ButtonKind::Primary
                        } else {
                            ButtonKind::Secondary
                        },
                        true,
                    )
                    .clicked()
                {
                    doc.accent = Some(*rgb);
                    self.lib_ui.accent_error = None;
                }
            }
        });
        let hint = kit.t("appearance.accent.custom.hint");
        let resp = kit.text_field(
            ui,
            kit.t("appearance.accent.custom"),
            &mut self.lib_ui.accent_hex,
            hint,
        );
        let submit = resp.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
        if kit
            .button(
                ui,
                kit.t("appearance.accent.use_custom"),
                ButtonKind::Secondary,
                !self.lib_ui.accent_hex.trim().is_empty(),
            )
            .clicked()
            || submit
        {
            match Rgb::parse_hex(&self.lib_ui.accent_hex) {
                None => {
                    self.lib_ui.accent_error = Some(kit.t("appearance.accent.bad_hex").to_owned())
                }
                Some(c) => match guard.check_accent(c) {
                    Ok(_) => {
                        doc.accent = Some(c);
                        self.lib_ui.accent_error = None;
                    }
                    Err(issue) => {
                        self.lib_ui.accent_error =
                            Some(appearance::accent_issue_text(kit.tr, issue));
                    }
                },
            }
        }
        if let Some(e) = &self.lib_ui.accent_error {
            kit.banner(
                ui,
                Tone::Warning,
                kit.t("appearance.accent.refused"),
                e,
                &[],
            );
        }
        ui.add_space(th.space.md);

        kit.segmented(
            ui,
            "appearance-contrast",
            kit.t("appearance.contrast"),
            &[
                (Contrast::Standard, kit.t("appearance.contrast.standard")),
                (Contrast::High, kit.t("appearance.contrast.high")),
            ],
            &mut doc.contrast,
        );
        let text_labels: Vec<String> = TEXT_SCALES.iter().map(|s| format!("{s}%")).collect();
        let text_opts: Vec<(u16, &str)> = TEXT_SCALES
            .iter()
            .zip(&text_labels)
            .map(|(s, l)| (*s, l.as_str()))
            .collect();
        kit.segmented(
            ui,
            "appearance-text",
            kit.t("appearance.text_size"),
            &text_opts,
            &mut doc.text_scale,
        );
        let ui_labels: Vec<String> = UI_SCALES.iter().map(|s| format!("{s}%")).collect();
        let ui_opts: Vec<(u16, &str)> = UI_SCALES
            .iter()
            .zip(&ui_labels)
            .map(|(s, l)| (*s, l.as_str()))
            .collect();
        kit.segmented(
            ui,
            "appearance-scale",
            kit.t("appearance.ui_scale"),
            &ui_opts,
            &mut doc.ui_scale,
        );
        kit.segmented(
            ui,
            "appearance-background",
            kit.t("appearance.background"),
            &[
                (Background::Drips, kit.t("appearance.background.drips")),
                (
                    Background::Gradient,
                    kit.t("appearance.background.gradient"),
                ),
                (Background::Plain, kit.t("appearance.background.plain")),
            ],
            &mut doc.background,
        );
        kit.segmented(
            ui,
            "appearance-motion",
            kit.t("appearance.motion"),
            &[
                (Motion::System, kit.t("settings.a11y.motion.system")),
                (Motion::Full, kit.t("settings.a11y.motion.full")),
                (Motion::Reduced, kit.t("settings.a11y.motion.reduced")),
                (Motion::Off, kit.t("settings.a11y.motion.off")),
            ],
            &mut doc.motion,
        );
        ui.add_space(th.space.md);

        // Live preview: the whole window already shows the draft; this card gathers the parts.
        kit.section(ui, kit.t("appearance.preview"));
        kit.card(ui, crate::paint::Surface::Card, |ui| {
            kit.text(
                ui,
                th.text.subtitle,
                th.c.text_primary,
                kit.t("appearance.preview.title"),
            );
            kit.body(ui, kit.t("appearance.preview.body"));
            kit.secondary(ui, kit.t("appearance.preview.secondary"));
            ui.horizontal(|ui| {
                let _ = kit.button(
                    ui,
                    kit.t("appearance.preview.primary"),
                    ButtonKind::Primary,
                    true,
                );
                let _ = kit.button(
                    ui,
                    kit.t("appearance.preview.button"),
                    ButtonKind::Secondary,
                    true,
                );
                kit.tag(ui, kit.t("appearance.preview.tag"), Some(Tone::Info));
            });
        });
        ui.add_space(th.space.md);

        let dirty = doc != saved;
        ui.horizontal_wrapped(|ui| {
            if kit
                .button(
                    ui,
                    kit.t("appearance.apply"),
                    ButtonKind::Primary,
                    dirty && !busy,
                )
                .clicked()
                && doc.validate(&guard).is_ok()
            {
                self.lib_submit(LibCommand::SaveTheme(doc.clone()));
                if let Some(mut s) = self.model.settings.clone() {
                    let motion = appearance::motion_pref(doc.motion);
                    if s.motion != motion || s.ui_scale_percent != doc.ui_scale {
                        s.motion = motion;
                        s.ui_scale_percent = doc.ui_scale;
                        self.submit(Command::UpdateSettings(s));
                    }
                }
            }
            if kit
                .button(ui, kit.t("appearance.revert"), ButtonKind::Secondary, dirty)
                .clicked()
            {
                doc = saved.clone();
            }
            if kit
                .button(
                    ui,
                    kit.t("appearance.reset"),
                    ButtonKind::Secondary,
                    doc != ThemeDoc::default(),
                )
                .clicked()
            {
                doc = ThemeDoc::default();
                self.lib_ui.accent_hex.clear();
            }
            if kit
                .button(ui, kit.t("appearance.export"), ButtonKind::Secondary, true)
                .clicked()
            {
                ui.ctx().copy_text(doc.to_json());
                self.live_text = kit.t("appearance.exported").to_owned();
            }
            if kit
                .button(ui, kit.t("appearance.import"), ButtonKind::Secondary, true)
                .clicked()
            {
                self.lib_ui.theme_import = Some(String::new());
                self.lib_ui.theme_import_error = None;
            }
        });
        if let Some(mut text) = self.lib_ui.theme_import.take() {
            ui.add_space(th.space.sm);
            kit.text_area(ui, kit.t("appearance.import.paste"), &mut text, 6);
            let mut keep = true;
            ui.horizontal(|ui| {
                if kit
                    .button(
                        ui,
                        kit.t("appearance.import.preview"),
                        ButtonKind::Secondary,
                        !text.trim().is_empty(),
                    )
                    .clicked()
                {
                    match ThemeDoc::from_json(&text, &guard) {
                        Ok(imported) => {
                            doc = imported;
                            keep = false;
                            self.lib_ui.theme_import_error = None;
                        }
                        Err(e) => {
                            self.lib_ui.theme_import_error = Some(import_error_text(kit.tr, &e))
                        }
                    }
                }
                if kit
                    .button(ui, kit.t("common.cancel"), ButtonKind::Quiet, true)
                    .clicked()
                {
                    keep = false;
                }
            });
            if let Some(e) = &self.lib_ui.theme_import_error {
                kit.banner(ui, Tone::Danger, kit.t("appearance.import.refused"), e, &[]);
            }
            if keep {
                self.lib_ui.theme_import = Some(text);
            }
        }
        self.lib_ui.appearance = (doc != saved).then_some(doc);
    }
}

fn def_default_enabled(draft: &Profile, id: ModuleId) -> Option<bool> {
    let mut base = draft.clone();
    base.reset(&format!("modules.{}.enabled", id.as_str()));
    Some(base.resolve().modules[&id].enabled)
}

pub(super) fn lib_error_keys(kind: LibErrorKind) -> (&'static str, &'static str) {
    match kind {
        LibErrorKind::Busy => ("library.error.busy.title", "library.error.busy.body"),
        LibErrorKind::GameRunning => (
            "library.error.game_running.title",
            "library.error.game_running.body",
        ),
        LibErrorKind::Incompatible => (
            "library.error.incompatible.title",
            "library.error.incompatible.body",
        ),
        LibErrorKind::Conflict => (
            "library.error.conflict.title",
            "library.error.conflict.body",
        ),
        LibErrorKind::HashMismatch => ("library.error.hash.title", "library.error.hash.body"),
        LibErrorKind::RateLimited => (
            "library.error.rate_limited.title",
            "library.error.rate_limited.body",
        ),
        LibErrorKind::Offline => ("library.error.offline.title", "library.error.offline.body"),
        LibErrorKind::NotFound => (
            "library.error.not_found.title",
            "library.error.not_found.body",
        ),
        LibErrorKind::Config => ("library.error.config.title", "library.error.config.body"),
        LibErrorKind::Invalid => ("library.error.invalid.title", "library.error.invalid.body"),
        LibErrorKind::Stale => ("library.error.stale.title", "library.error.stale.body"),
        LibErrorKind::Cancelled => ("library.cancelled", "library.cancelled"),
        LibErrorKind::Internal => (
            "library.error.internal.title",
            "library.error.internal.body",
        ),
    }
}
