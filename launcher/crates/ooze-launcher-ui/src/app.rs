//! The launcher window: shell, pages and dialogs. It holds no business logic: commands go to the
//! backend, events update [`Model`], and every visible state is derived from that model.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, TryRecvError};

use eframe::egui::{
    self, Align, CentralPanel, Color32, Context, FontData, FontDefinitions, FontFamily, Frame, Key,
    Layout, Margin, Panel, RichText, ScrollArea, Sense, Ui, UiBuilder, ViewportCommand, WidgetInfo,
    accesskit, pos2, vec2,
};

use crate::api::*;
use crate::i18n::Strings;
use crate::library::{LibEvent, LibraryBackend};
use crate::library_model::LibraryModel;
use crate::motion::{MotionMode, ease_viscous};
use crate::paint::{Gel, Surface, paint_drip, paint_focus_ring, paint_gel};
use crate::redact::redact;
use crate::state::{Announcement, CloseGuard, JavaCheckView, Model, Notice, Primary, WindowEffect};
use crate::theme::Theme;
use crate::widgets::{ButtonKind, FAMILY_DISPLAY, FAMILY_STRONG, Icon, Kit, Tone, font};

const FONT_DISPLAY: &[u8] =
    include_bytes!("../../../resources/fonts/bubblegum_sans/BubblegumSans-Regular.ttf");
const FONT_BODY: &[u8] = include_bytes!("../../../resources/fonts/inter/Inter-Regular.ttf");
const FONT_STRONG: &[u8] = include_bytes!("../../../resources/fonts/inter/Inter-SemiBold.ttf");

const UI_SCALES: [u16; 5] = [100, 125, 150, 175, 200];

mod library_pages;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Page {
    Play,
    /// Library pages: shown only when the library reports the capability (`LIBRARY_UI.md §2`).
    Instances,
    Profile,
    Content,
    Appearance,
    Settings,
    Diagnostics,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PickTarget {
    InstallRoot,
    SettingsInstallRoot,
    Java,
    /// Save dialog for an exported diagnostics ZIP.
    DiagnosticsZip,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum JavaMode {
    Managed,
    Custom,
}

/// Options for constructing the window.
#[derive(Clone, Copy, Debug, Default)]
pub struct AppOptions {
    /// Show the developer-preview banner (fixture backends only).
    pub developer_preview: bool,
}

pub struct LauncherApp {
    backend: Box<dyn Backend>,
    model: Model,
    theme: Arc<Theme>,
    tr: Arc<Strings>,
    options: AppOptions,
    page: Page,
    events: Vec<Event>,
    initialised: bool,
    keyboard: bool,
    draft: Option<Settings>,
    install_path: String,
    java_path: String,
    pick: Option<(PickTarget, Receiver<Option<PathBuf>>)>,
    close_prompt: Option<CloseGuard>,
    allow_close: bool,
    close_after_library: bool,
    details_open: bool,
    applied_scale: f32,
    live_seq: u64,
    live_text: String,
    /// Tests answer the next file dialog without opening one.
    #[cfg(test)]
    pick_for_tests: Option<Option<PathBuf>>,
    library: Option<Box<dyn LibraryBackend>>,
    lib: Option<LibraryModel>,
    lib_events: Vec<LibEvent>,
    lib_ui: library_pages::LibUi,
    lib_notice_seq: u64,
    /// Ooze Classic tokens before the player's appearance is applied.
    base_theme: Arc<Theme>,
    /// The contrast guard's base colours (shared validator in `ooze-ui`).
    guard: Option<ooze_ui::theme_doc::BaseColors>,
    applied_doc: Option<ooze_ui::theme_doc::ThemeDoc>,
    /// Select must finish successfully before API 1 may change the Play directory.
    pending_selection: Option<(crate::library::LibOpId, crate::library::InstanceId)>,
    bind_initial_selection: bool,
}

impl LauncherApp {
    pub fn new(backend: Box<dyn Backend>, options: AppOptions) -> Self {
        let theme = Arc::new(Theme::embedded());
        let log_cap = theme.limits.log_lines;
        Self {
            backend,
            model: Model::new(log_cap),
            theme: Arc::clone(&theme),
            tr: Arc::new(Strings::en_us()),
            options,
            page: Page::Play,
            events: Vec::new(),
            initialised: false,
            keyboard: false,
            draft: None,
            install_path: String::new(),
            java_path: String::new(),
            pick: None,
            close_prompt: None,
            allow_close: false,
            close_after_library: false,
            details_open: false,
            applied_scale: 0.0,
            live_seq: 0,
            live_text: String::new(),
            #[cfg(test)]
            pick_for_tests: None,
            library: None,
            lib: None,
            lib_events: Vec::new(),
            lib_ui: library_pages::LibUi::default(),
            lib_notice_seq: 0,
            base_theme: theme,
            guard: ooze_ui::theme_doc::BaseColors::classic(),
            applied_doc: None,
            pending_selection: None,
            bind_initial_selection: false,
        }
    }

    /// Attach Sol's library (Library API 1). Without it the launcher is the L0 launcher: no
    /// library page, tab or button is shown.
    pub fn with_library(mut self, library: Box<dyn LibraryBackend>) -> Self {
        let model = LibraryModel::new(library.snapshot(), library.open_error());
        self.lib = Some(model);
        self.library = Some(library);
        self.bind_initial_selection = true;
        self
    }

    /// The design tokens in use (after the player's appearance).
    pub fn theme(&self) -> &Theme {
        &self.theme
    }

    #[cfg(test)]
    pub(crate) fn library_model_mut_for_tests(&mut self) -> Option<&mut LibraryModel> {
        self.lib.as_mut()
    }

    pub fn library_model(&self) -> Option<&LibraryModel> {
        self.lib.as_ref()
    }

    pub fn set_page(&mut self, page: Page) {
        self.page = page;
    }

    /// Keep the tokens in step with the shown appearance (draft or saved) and keep the page valid
    /// when a capability disappears.
    fn sync_library(&mut self, ctx: &Context) {
        let doc = self.shown_theme_doc();
        if doc != self.applied_doc {
            self.theme = match (&doc, &self.guard) {
                (Some(d), Some(g)) => Arc::new(crate::appearance::apply(&self.base_theme, d, g)),
                _ => Arc::clone(&self.base_theme),
            };
            self.applied_doc = doc;
            self.apply_theme_styles(ctx);
        }
        let library_page = matches!(
            self.page,
            Page::Instances | Page::Profile | Page::Content | Page::Appearance
        );
        if library_page && !self.library_pages().contains(&self.page) {
            self.page = Page::Play;
        }
        if let Some(m) = &self.lib
            && m.notice_seq != self.lib_notice_seq
        {
            self.lib_notice_seq = m.notice_seq;
            if let Some(n) = &m.notice {
                self.live_text = library_notice_text(&self.tr, n);
            }
        }
    }

    /// The next file or folder dialog returns `picked` immediately (tests only).
    #[cfg(test)]
    pub(crate) fn answer_next_dialog_for_tests(&mut self, picked: Option<PathBuf>) {
        self.pick_for_tests = Some(picked);
    }

    pub fn model(&self) -> &Model {
        &self.model
    }

    pub fn page(&self) -> Page {
        self.page
    }

    /// Native window options from the design tokens.
    pub fn native_options(title: &str) -> eframe::NativeOptions {
        let theme = Theme::embedded();
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_title(title)
                .with_inner_size(theme.control.window_default)
                .with_min_inner_size(theme.control.window_min),
            ..Default::default()
        }
    }

    fn initialise(&mut self, ctx: &Context) {
        let mut fonts = FontDefinitions::empty();
        fonts
            .font_data
            .insert("inter".into(), Arc::new(FontData::from_static(FONT_BODY)));
        fonts.font_data.insert(
            "inter-semibold".into(),
            Arc::new(FontData::from_static(FONT_STRONG)),
        );
        fonts.font_data.insert(
            "bubblegum".into(),
            Arc::new(FontData::from_static(FONT_DISPLAY)),
        );
        fonts
            .families
            .insert(FontFamily::Proportional, vec!["inter".into()]);
        fonts
            .families
            .insert(FontFamily::Monospace, vec!["inter".into()]);
        fonts.families.insert(
            FontFamily::Name(FAMILY_STRONG.into()),
            vec!["inter-semibold".into(), "inter".into()],
        );
        fonts.families.insert(
            FontFamily::Name(FAMILY_DISPLAY.into()),
            vec!["bubblegum".into(), "inter".into()],
        );
        ctx.set_fonts(fonts);
        // Modrinth titles and descriptions can use any script; the shipped fonts (Inter, Bubblegum
        // Sans) cover Latin, Greek and Cyrillic. Missing glyphs draw as boxes rather than failing,
        // and AccessKit still reads the real text. (Wider script coverage needs a bundled font:
        // owner decision recorded in CLAUDE-P0.)
        ctx.set_missing_glyph_policy(egui::MissingGlyphPolicy::Tofu);
        ctx.options_mut(|o| o.zoom_with_keyboard = false);
        self.apply_theme_styles(ctx);
        let repaint = ctx.clone();
        self.backend
            .set_waker(Box::new(move || repaint.request_repaint()));
        if let Some(library) = self.library.as_mut() {
            let repaint = ctx.clone();
            library.set_waker(Box::new(move || repaint.request_repaint()));
        }
        self.initialised = true;
    }

    fn apply_theme_styles(&self, ctx: &Context) {
        let th = Arc::clone(&self.theme);
        ctx.all_styles_mut(|style| {
            let v = &mut style.visuals;
            *v = egui::Visuals::light();
            v.panel_fill = th.c.canvas;
            v.window_fill = th.c.surface_raised;
            v.extreme_bg_color = th.c.surface_raised;
            v.faint_bg_color = th.c.surface_sunken;
            v.override_text_color = Some(th.c.text_primary);
            v.hyperlink_color = th.c.text_link;
            v.selection.bg_fill = th.c.selection;
            v.selection.stroke = egui::Stroke::new(th.shape.outline_control, th.c.focus);
            v.text_cursor.stroke = egui::Stroke::new(th.shape.outline_control, th.c.text_primary);
            for w in [
                &mut v.widgets.inactive,
                &mut v.widgets.hovered,
                &mut v.widgets.active,
                &mut v.widgets.open,
            ] {
                w.bg_fill = th.c.surface_raised;
                w.weak_bg_fill = th.c.surface_raised;
                w.bg_stroke = egui::Stroke::new(th.shape.outline_control, th.c.outline_strong);
                w.fg_stroke = egui::Stroke::new(th.shape.outline_control, th.c.text_primary);
                w.corner_radius = egui::CornerRadius::same(th.shape.radius_sm as u8);
            }
            v.widgets.hovered.bg_stroke =
                egui::Stroke::new(th.shape.outline_panel, th.c.outline_strong);
            v.widgets.active.bg_stroke = egui::Stroke::new(th.shape.outline_panel, th.c.focus);
            v.widgets.inactive.bg_fill = th.c.surface_sunken;
            v.widgets.noninteractive.fg_stroke =
                egui::Stroke::new(th.shape.outline_hairline, th.c.text_primary);
            v.widgets.noninteractive.bg_stroke =
                egui::Stroke::new(th.shape.outline_hairline, th.c.outline_soft);
            style.spacing.item_spacing = vec2(th.space.sm, th.space.sm);
            style.spacing.interact_size.y = th.control.min_hit;
            style.spacing.slider_width = th.control.content_max_width * 0.5;
        });
    }

    fn submit(&mut self, command: Command) {
        if matches!(command, Command::Play | Command::Prepare { .. }) && !self.selection_ready() {
            return;
        }
        if self.pending_selection.is_some()
            && !matches!(command, Command::Cancel { .. } | Command::OpenFolder(_))
        {
            return;
        }
        if !self.model.may_submit(&command) {
            return;
        }
        let result = self.backend.submit(command.clone());
        self.model.on_submitted(&command, result);
    }

    fn motion(&self) -> MotionMode {
        let pref = self
            .shown_theme_doc()
            .map(|d| crate::appearance::motion_pref(d.motion))
            .or_else(|| self.model.settings.as_ref().map(|s| s.motion))
            .unwrap_or(MotionPreference::FollowSystem);
        MotionMode::resolve(
            pref,
            self.model
                .system
                .as_ref()
                .and_then(|s| s.system_reduced_motion),
        )
    }

    /// Apply queued backend events. Called before every frame and from `App::logic`, which eframe
    /// also runs while the window is hidden or minimised, so the backend's bounded queue drains.
    pub fn pump(&mut self, ctx: &Context) {
        self.backend.drain_events(&mut self.events);
        for event in self.events.drain(..) {
            self.model.apply(event);
        }
        let mut selected = None;
        let mut refresh_content = None;
        if let (Some(library), Some(model)) = (self.library.as_mut(), self.lib.as_mut()) {
            library.drain_events(&mut self.lib_events);
            for event in self.lib_events.drain(..) {
                if let LibEvent::Finished {
                    op,
                    result: Ok(payload),
                } = &event
                    && model.active().is_some_and(|(active, _)| active == *op)
                {
                    match payload {
                        crate::library::LibPayload::Installed { instance, kind, .. } => {
                            refresh_content = Some((instance.clone(), *kind))
                        }
                        _ if matches!(
                            model.active_command(),
                            Some(
                                crate::library::LibCommand::SetEnabled { .. }
                                    | crate::library::LibCommand::Remove { .. }
                            )
                        ) =>
                        {
                            refresh_content = model
                                .installed
                                .as_ref()
                                .map(|(id, kind, _)| (id.clone(), *kind));
                        }
                        _ => {}
                    }
                }
                if let LibEvent::Finished { op, result } = &event
                    && self
                        .pending_selection
                        .as_ref()
                        .is_some_and(|(pending, _)| pending == op)
                {
                    if result.is_ok() {
                        selected = self.pending_selection.as_ref().map(|(_, id)| id.clone());
                    }
                    self.pending_selection = None;
                }
                model.apply(event);
            }
            if library.wants_polling() && model.busy() {
                ctx.request_repaint_after(std::time::Duration::from_millis(100));
            }
        }
        if self.bind_initial_selection && self.model.connected() && !self.model.busy() {
            self.bind_initial_selection = false;
            selected = selected.or_else(|| self.lib.as_ref().and_then(|m| m.snap.selected.clone()));
        }
        if let Some(id) = selected
            && let Some(instance) = self.lib.as_ref().and_then(|m| m.snap.instance(&id))
            && let Some(mut settings) = self.model.settings.clone()
            && settings.install_root.as_ref() != Some(&instance.game_dir)
        {
            settings.install_root = Some(instance.game_dir.clone());
            self.submit(Command::UpdateSettings(settings));
        }
        if let Some((instance, kind)) = refresh_content {
            self.lib_submit(crate::library::LibCommand::ListInstalled { instance, kind });
        }
        if self.close_after_library && !self.lib_busy() {
            self.close_after_library = false;
            match self.model.close_guard() {
                CloseGuard::Allow => {
                    self.close_prompt = None;
                    self.allow_close = true;
                    ctx.send_viewport_cmd(ViewportCommand::Close);
                }
                guard => self.close_prompt = Some(guard),
            }
        }
        if self.backend.wants_polling() {
            // The real backend has no waker: poll while an operation or the game is active.
            let ms = if self.model.game_live() { 250 } else { 100 };
            ctx.request_repaint_after(std::time::Duration::from_millis(ms));
        }
    }

    /// Run one frame. `eframe::App::ui` and the test harness both call this.
    pub fn frame(&mut self, ui: &mut Ui) {
        let ctx = ui.ctx().clone();
        if !self.initialised {
            self.initialise(&ctx);
        }
        self.pump(&ctx);
        self.poll_pick();
        if let Some(effect) = self.model.take_window_effect() {
            match effect {
                WindowEffect::Minimize => ctx.send_viewport_cmd(ViewportCommand::Minimized(true)),
                WindowEffect::Restore => {
                    ctx.send_viewport_cmd(ViewportCommand::Minimized(false));
                    ctx.send_viewport_cmd(ViewportCommand::Focus);
                }
            }
        }
        ctx.input(|i| {
            for e in &i.events {
                match e {
                    egui::Event::Key { key: Key::Tab | Key::ArrowLeft | Key::ArrowRight | Key::ArrowUp | Key::ArrowDown, pressed: true, .. } => {
                        self.keyboard = true;
                    }
                    egui::Event::PointerButton { pressed: true, .. } => self.keyboard = false,
                    _ => {}
                }
            }
        });
        self.sync_library(&ctx);
        let animation_time = self
            .motion()
            .tween(&self.theme.motion, self.theme.motion.short);
        if (ctx.style_of(egui::Theme::Light).animation_time - animation_time).abs() > f32::EPSILON {
            ctx.all_styles_mut(|style| style.animation_time = animation_time);
        }
        // A draft appearance previews its interface scale live; otherwise the saved setting.
        let scale = self
            .shown_theme_doc()
            .map(|d| d.ui_scale)
            .or_else(|| self.model.settings.as_ref().map(|s| s.ui_scale_percent))
            .unwrap_or(100) as f32
            / 100.0;
        if (scale - self.applied_scale).abs() > f32::EPSILON {
            ctx.set_zoom_factor(scale);
            self.applied_scale = scale;
        }
        self.update_live_text();

        let theme = Arc::clone(&self.theme);
        let tr = Arc::clone(&self.tr);
        let kit = Kit {
            theme: &theme,
            tr: &tr,
            motion: self.motion(),
            keyboard: self.keyboard,
        };
        self.layout(ui, &kit);
        self.close_dialog(&ctx, &kit);
    }

    fn update_live_text(&mut self) {
        let Some((seq, a)) = self.model.announcement().cloned() else {
            return;
        };
        if seq == self.live_seq {
            return;
        }
        self.live_seq = seq;
        self.live_text = match a {
            Announcement::Step(step) => self.tr.get(step_key(step)).to_owned(),
            Announcement::Notice(n) => notice_title(&self.tr, &n),
            Announcement::SignedIn(name) => self.tr.fmt("account.signed_in_as", &[&name]),
            Announcement::GameStarting => self.tr.get("game.starting").to_owned(),
            Announcement::GameTitleScreen => self.tr.get("game.title_screen").to_owned(),
        };
    }

    fn layout(&mut self, ui: &mut Ui, kit: &Kit<'_>) {
        let th = kit.theme;
        if self.options.developer_preview {
            Panel::top("preview-banner")
                .frame(
                    Frame::new()
                        .fill(th.c.warning_soft)
                        .inner_margin(Margin::symmetric(th.space.lg as i8, th.space.xs as i8)),
                )
                .show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        kit.icon(ui, Icon::Warning, th.c.warning);
                        kit.text(
                            ui,
                            th.text.label,
                            th.c.text_primary,
                            kit.t("preview.banner"),
                        );
                    });
                });
        }
        Panel::bottom("status")
            .frame(
                Frame::new()
                    .fill(th.c.canvas)
                    .inner_margin(Margin::symmetric(th.space.lg as i8, th.space.xs as i8)),
            )
            .show(ui, |ui| self.status_line(ui, kit));

        if let Some(error) = self.backend.startup_error() {
            CentralPanel::default()
                .frame(Frame::new().fill(th.c.canvas))
                .show(ui, |ui| {
                    self.centered(ui, kit, |_, ui, kit| {
                        let (title, body) = match error.kind {
                            ErrorKind::UnsupportedPlatform => (
                                "app.startup.unsupported.title",
                                "app.startup.unsupported.body",
                            ),
                            ErrorKind::Busy => ("app.startup.busy.title", "app.startup.busy.body"),
                            _ => ("app.startup.failed.title", "app.startup.failed.body"),
                        };
                        kit.title(ui, kit.t(title));
                        kit.body(ui, kit.t(body));
                        ui.add_space(th.space.sm);
                        kit.caption(
                            ui,
                            &kit.tr.fmt(
                                "error.details_line",
                                &[&format!("{:?}", error.kind), &redact(&error.detail)],
                            ),
                        );
                    });
                });
            return;
        }
        let api = self.backend.api_version();
        if api.0 != API_MAJOR {
            CentralPanel::default()
                .frame(Frame::new().fill(th.c.canvas))
                .show(ui, |ui| {
                    self.centered(ui, kit, |_, ui, kit| {
                        kit.title(ui, kit.t("app.incompatible_backend.title"));
                        kit.body(
                            ui,
                            &kit.tr.fmt(
                                "app.incompatible_backend.body",
                                &[&api.0.to_string(), &API_MAJOR.to_string()],
                            ),
                        );
                    });
                });
            return;
        }
        if !self.model.connected() {
            CentralPanel::default()
                .frame(Frame::new().fill(th.c.canvas))
                .show(ui, |ui| {
                    self.centered(ui, kit, |_, ui, kit| {
                        ui.horizontal_wrapped(|ui| {
                            kit.spinner(ui, kit.t("app.connecting"));
                            kit.body(ui, kit.t("app.connecting"));
                        });
                    });
                });
            return;
        }
        if self.model.needs_first_run() {
            CentralPanel::default()
                .frame(Frame::new().fill(th.c.canvas))
                .show(ui, |ui| self.first_run(ui, kit));
            return;
        }

        Panel::top("header")
            .frame(
                Frame::new()
                    .fill(th.c.canvas)
                    .inner_margin(Margin::symmetric(th.space.xl as i8, th.space.md as i8)),
            )
            .show(ui, |ui| self.header(ui, kit));
        Panel::left("nav")
            .exact_size(th.control.nav_width)
            .resizable(false)
            .show_separator_line(false)
            .frame(
                Frame::new()
                    .fill(th.c.canvas)
                    .inner_margin(Margin::symmetric(th.space.lg as i8, th.space.md as i8)),
            )
            .show(ui, |ui| self.nav(ui, kit));
        CentralPanel::default()
            .frame(
                Frame::new()
                    .fill(th.c.canvas)
                    .inner_margin(Margin::symmetric(th.space.xl as i8, th.space.md as i8)),
            )
            .show(ui, |ui| {
                ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        let width = ui.available_width().min(th.control.content_max_width);
                        ui.set_max_width(width);
                        match self.page {
                            Page::Play => self.play_page(ui, kit),
                            Page::Instances => self.instances_page(ui, kit),
                            Page::Profile => self.profile_page(ui, kit),
                            Page::Content => self.content_page(ui, kit),
                            Page::Appearance => self.appearance_page(ui, kit),
                            Page::Settings => self.settings_page(ui, kit),
                            Page::Diagnostics => self.diagnostics_page(ui, kit),
                        }
                    });
            });
    }

    fn centered(
        &mut self,
        ui: &mut Ui,
        kit: &Kit<'_>,
        add: impl FnOnce(&mut Self, &mut Ui, &Kit<'_>),
    ) {
        let th = kit.theme;
        let width = th.control.content_max_width.min(ui.available_width());
        let left = (ui.available_width() - width) / 2.0;
        ui.add_space(th.space.xxxl);
        ui.horizontal_wrapped(|ui| {
            ui.add_space(left.max(0.0));
            ui.vertical(|ui| {
                ui.set_width(width);
                kit.card(ui, Surface::Panel, |ui| add(self, ui, kit));
            });
        });
    }

    fn status_line(&self, ui: &mut Ui, kit: &Kit<'_>) {
        let th = kit.theme;
        let text = if self.live_text.is_empty() {
            kit.t("app.status_idle").to_owned()
        } else {
            self.live_text.clone()
        };
        let r = kit.text(ui, th.text.caption, th.c.text_secondary, text.clone());
        ui.ctx().accesskit_node_builder(r.id, |n| {
            n.set_role(accesskit::Role::Status);
            n.set_label(text);
            n.set_live(accesskit::Live::Polite);
        });
    }

    fn wordmark(&self, ui: &mut Ui, kit: &Kit<'_>) {
        let th = kit.theme;
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = th.space.xs;
            let r = kit.text(ui, th.text.title, th.c.text_primary, kit.t("app.wordmark"));
            ui.ctx()
                .accesskit_node_builder(r.id, |n| n.set_label(kit.t("app.name")));
            kit.text(
                ui,
                th.text.caption,
                th.c.text_secondary,
                kit.t("app.descriptor").to_uppercase(),
            );
        });
    }

    fn header(&mut self, ui: &mut Ui, kit: &Kit<'_>) {
        let th = kit.theme;
        // Narrow windows (small window or a large interface scale): the chips go on their own
        // wrapped row under the wordmark instead of overlapping it.
        if ui.available_width() < th.control.window_min[0] * 0.75 {
            self.wordmark(ui, kit);
            ui.horizontal_wrapped(|ui| self.header_chips(ui, kit));
            return;
        }
        ui.horizontal(|ui| {
            self.wordmark(ui, kit);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                self.header_chips(ui, kit);
            });
        });
    }

    /// The selected-instance and account chips. In the wide header they are laid out right to
    /// left (instance chip rightmost); in the narrow header left to right (account first).
    fn header_chips(&mut self, ui: &mut Ui, kit: &Kit<'_>) {
        let th = kit.theme;
        let rtl = ui.layout().main_dir() == egui::Direction::RightToLeft;
        let instance = self
            .lib
            .as_ref()
            .and_then(|m| m.snap.selected().cloned())
            .filter(|_| self.library_pages().contains(&Page::Instances));
        let instance_chip = |this: &mut Self, ui: &mut Ui| {
            if let Some(inst) = &instance {
                let label = kit.tr.fmt("instances.chip", &[&inst.name]);
                if kit.button(ui, &label, ButtonKind::Quiet, true).clicked() {
                    this.page = Page::Instances;
                }
                ui.add_space(th.space.sm);
            }
        };
        if rtl {
            instance_chip(self, ui);
        }
        if let Some(account) = self.model.account.clone() {
            let label = kit.tr.fmt("account.chip", &[&account.username]);
            if kit
                .button(ui, &label, ButtonKind::Secondary, true)
                .clicked()
            {
                self.page = Page::Settings;
            }
        } else if self.model.sign_in_op().is_none() {
            kit.secondary(ui, kit.t("account.not_signed_in"));
        }
        ui.add_space(th.space.sm);
        if !rtl {
            instance_chip(self, ui);
        }
    }

    fn nav(&mut self, ui: &mut Ui, kit: &Kit<'_>) {
        let th = kit.theme;
        let mut items = vec![(Page::Play, "nav.play")];
        for page in self.library_pages() {
            items.push((
                page,
                match page {
                    Page::Instances => "nav.instances",
                    Page::Profile => "nav.profile",
                    Page::Content => "nav.content",
                    _ => "nav.appearance",
                },
            ));
        }
        items.push((Page::Settings, "nav.settings"));
        items.push((Page::Diagnostics, "nav.diagnostics"));
        ui.add_space(th.space.md);
        for (page, key) in items {
            let selected = self.page == page;
            let label = kit.t(key);
            let galley = ui.painter().layout_no_wrap(
                label.to_owned(),
                font(th.text.button),
                if selected {
                    th.c.text_on_accent
                } else {
                    th.c.text_primary
                },
            );
            let (rect, resp) = ui.allocate_exact_size(
                vec2(ui.available_width(), th.control.height),
                Sense::click(),
            );
            resp.widget_info(|| WidgetInfo::selected(accesskit::Role::Tab, true, selected, label));
            if resp.clicked() && !selected {
                self.page = page;
                self.draft = None;
            }
            let painter = ui.painter();
            if selected {
                paint_gel(
                    painter,
                    th,
                    rect,
                    Gel::new(Surface::Inverse, th.shape.radius_md),
                );
            } else if resp.hovered() {
                painter.rect_filled(rect, th.shape.radius_md, th.c.surface_sunken);
            }
            painter.galley(
                pos2(
                    rect.left() + th.control.pad_x,
                    rect.center().y - galley.size().y / 2.0,
                ),
                galley,
                th.c.text_primary,
            );
            if resp.has_focus() && kit.keyboard {
                paint_focus_ring(painter, th, rect, th.shape.radius_md);
            }
            ui.add_space(th.space.xs);
        }
    }

    // ----------------------------------------------------------------------------------------
    // First run

    fn first_run(&mut self, ui: &mut Ui, kit: &Kit<'_>) {
        if self.install_path.is_empty()
            && let Some(system) = &self.model.system
        {
            self.install_path = system.default_install_root.display().to_string();
        }
        self.centered(ui, kit, |app, ui, kit| {
            let th = kit.theme;
            app.wordmark(ui, kit);
            ui.add_space(th.space.lg);
            kit.title(ui, kit.t("first_run.title"));
            kit.body(ui, kit.t("first_run.body"));
            ui.add_space(th.space.lg);
            let busy = app.model.first_run_op().is_some();
            ui.add_enabled_ui(!busy, |ui| {
                kit.text_field(
                    ui,
                    kit.t("first_run.location"),
                    &mut app.install_path,
                    kit.t("first_run.location_hint"),
                );
            });
            kit.caption(ui, kit.t("first_run.location_note"));
            ui.add_space(th.space.md);
            if let Some(Notice::Error {
                during: OperationKind::SetInstallRoot,
                error,
            }) = app.model.notice.clone()
            {
                app.error_banner(ui, kit, OperationKind::SetInstallRoot, &error);
                ui.add_space(th.space.md);
            }
            ui.horizontal_wrapped(|ui| {
                let path = app.install_path.trim().to_owned();
                let can = !busy && !path.is_empty();
                if kit
                    .button(ui, kit.t("first_run.continue"), ButtonKind::Primary, can)
                    .clicked()
                {
                    app.submit(Command::SetInstallRoot {
                        path: PathBuf::from(path),
                    });
                }
                if kit
                    .button(
                        ui,
                        kit.t("common.browse"),
                        ButtonKind::Secondary,
                        !busy && app.pick.is_none(),
                    )
                    .clicked()
                {
                    app.start_pick(PickTarget::InstallRoot);
                }
                if busy {
                    kit.spinner(ui, kit.t("first_run.checking"));
                    kit.secondary(ui, kit.t("first_run.checking"));
                }
            });
        });
    }

    // ----------------------------------------------------------------------------------------
    // Play

    fn play_page(&mut self, ui: &mut Ui, kit: &Kit<'_>) {
        let th = kit.theme;
        kit.title(ui, kit.t("nav.play"));
        ui.add_space(th.space.md);
        let card_top = ui.cursor().top();
        kit.card(ui, Surface::Card, |ui| self.play_card(ui, kit));
        let card_bottom = ui.min_rect().bottom();
        if self
            .applied_doc
            .as_ref()
            .is_none_or(crate::appearance::draws_drips)
        {
            self.drips(ui, kit, card_top, card_bottom);
        }
        // Notices go below the card so the main button never moves under the pointer.
        ui.add_space(th.space.md);
        self.notice_area(ui, kit);
    }

    /// Two slow decorative drips under the play card (DESIGN_SYSTEM §4.5, MOTION_SPEC §4.11).
    /// They cycle only with Full motion; Reduced and Off show them static.
    fn drips(&self, ui: &mut Ui, kit: &Kit<'_>, top: f32, bottom: f32) {
        let th = kit.theme;
        let s = &th.shape;
        let rect = egui::Rect::from_min_max(
            pos2(ui.min_rect().left(), top),
            pos2(ui.min_rect().right(), bottom),
        );
        let t = if kit.motion.ambient() {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(33));
            let period = f64::from(th.motion.drip_period);
            let phase = ((ui.input(|i| i.time) % period) / period) as f32;
            ease_viscous(if phase < 0.5 {
                phase * 2.0
            } else {
                2.0 - phase * 2.0
            })
        } else {
            0.5
        };
        let [len_min, len_max] = s.drip_length;
        for (i, frac) in th.control.drip_positions.into_iter().enumerate() {
            let k = if i == 0 { t } else { 1.0 - t };
            let bulb_clear = 2.0 * s.drip_bulb[1] + s.outline_panel;
            let len = (len_min + (len_max - len_min) * k).max(bulb_clear);
            let neck = s.drip_neck[0] + (s.drip_neck[1] - s.drip_neck[0]) * 0.25;
            let bulb = s.drip_bulb[1];
            paint_drip(
                ui.painter(),
                th,
                rect,
                rect.left() + rect.width() * frac,
                neck,
                len,
                bulb,
            );
        }
        ui.add_space(len_max);
    }

    fn play_card(&mut self, ui: &mut Ui, kit: &Kit<'_>) {
        let th = kit.theme;
        if let Some(instance) = self.selected_instance() {
            kit.text(ui, th.text.display, th.c.text_primary, &instance.name);
            kit.caption(ui, &library_pages::target_text(kit, &instance.target));
        } else if let Some(profile) = self.model.profile.clone() {
            kit.text(
                ui,
                th.text.display,
                th.c.text_primary,
                kit.tr.fmt("play.profile_title", &[&profile.minecraft]),
            );
            let java = profile.java_major.to_string();
            let parts: Vec<String> = [
                ("play.version.fabric_loader", profile.fabric_loader.as_str()),
                ("play.version.fabric_api", profile.fabric_api.as_str()),
                ("play.version.ooze", profile.ooze.as_str()),
                ("play.version.java", java.as_str()),
            ]
            .into_iter()
            .filter(|(_, v)| !v.is_empty())
            .map(|(k, v)| kit.tr.fmt(k, &[v]))
            .collect();
            kit.caption(ui, &parts.join(" · "));
        }
        ui.add_space(th.space.lg);

        // Account and install status lines (icon + text; never colour alone).
        ui.horizontal(|ui| match &self.model.account {
            Some(a) => {
                kit.icon(ui, Icon::Check, th.c.success);
                kit.body(ui, &kit.tr.fmt("account.playing_as", &[&a.username]));
            }
            None => {
                kit.icon(ui, Icon::Info, th.c.info);
                kit.body(ui, kit.t("account.sign_in_to_play"));
            }
        });
        ui.horizontal_wrapped(|ui| {
            let (icon, color, key) = match self.model.install {
                InstallStatus::Ready => (Icon::Check, th.c.success, "install.status.ready"),
                InstallStatus::NotInstalled => {
                    (Icon::Info, th.c.info, "install.status.not_installed")
                }
                InstallStatus::NeedsRepair => {
                    (Icon::Warning, th.c.warning, "install.status.needs_repair")
                }
                InstallStatus::Unknown if !self.model.capabilities().install_status => {
                    (Icon::Info, th.c.info, "install.status.checked_on_play")
                }
                InstallStatus::Unknown => (Icon::Info, th.c.info, "install.status.unknown"),
            };
            kit.icon(ui, icon, color);
            kit.body(ui, kit.t(key));
        });
        ui.add_space(th.space.xl);

        let primary = self.model.primary();
        let label = match primary {
            Primary::Connecting => "app.connecting",
            Primary::SignIn => "play.action.sign_in",
            Primary::Install => "play.action.install",
            Primary::Repair => "play.action.repair",
            Primary::Play => "play.action.play",
            Primary::SigningIn(_) => "play.action.signing_in",
            Primary::Preparing(_) => "play.action.preparing",
            Primary::Launching => "play.action.launching",
            Primary::Playing => "play.action.playing",
        };
        if kit
            .button_large(
                ui,
                kit.t(label),
                primary.actionable()
                    && (self.selection_ready()
                        || primary == Primary::SignIn && self.pending_selection.is_none()),
            )
            .clicked()
            && let Some(cmd) = self.model.primary_command()
        {
            self.submit(cmd);
        }
        if !self.selection_ready() && !self.lib_busy() && !self.model.busy() {
            if let Some(instance) = self.selected_instance() {
                kit.secondary(ui, kit.t("play.instance_mismatch"));
                if kit
                    .button(
                        ui,
                        kit.t("instances.select"),
                        ButtonKind::Secondary,
                        !instance.running,
                    )
                    .clicked()
                {
                    self.select_instance(&instance);
                }
            } else {
                kit.secondary(ui, kit.t("play.choose_instance"));
                if kit
                    .button(ui, kit.t("nav.instances"), ButtonKind::Secondary, true)
                    .clicked()
                {
                    self.page = Page::Instances;
                }
            }
        }
        ui.add_space(th.space.lg);

        if let Some(op) = self.model.sign_in_op().cloned() {
            self.sign_in_block(ui, kit, op.id, op.progress, op.cancelling);
        } else if let Some(op) = self.model.prepare_op().cloned() {
            self.progress_block(ui, kit, op.id, op.progress, op.cancelling, true);
        } else if let Some(op) = self.model.launch_op().cloned() {
            self.progress_block(ui, kit, op.id, op.progress, false, false);
        }
        self.game_block(ui, kit);
    }

    fn sign_in_block(
        &mut self,
        ui: &mut Ui,
        kit: &Kit<'_>,
        op: OperationId,
        progress: Option<Progress>,
        cancelling: bool,
    ) {
        let th = kit.theme;
        kit.text(
            ui,
            th.text.subtitle,
            th.c.text_primary,
            kit.t("sign_in.continue_in_browser"),
        );
        kit.secondary(ui, kit.t("sign_in.browser_help"));
        ui.add_space(th.space.sm);
        let step = progress.map_or(Step::WaitingForBrowser, |p| p.step);
        kit.label_step(ui, step, progress);
        ui.horizontal_wrapped(|ui| {
            let waiting = step == Step::WaitingForBrowser;
            if self.model.capabilities().reopen_browser
                && kit
                    .button(
                        ui,
                        kit.t("sign_in.reopen_browser"),
                        ButtonKind::Secondary,
                        waiting && !cancelling,
                    )
                    .clicked()
            {
                self.submit(Command::ReopenSignInBrowser { op });
            }
            let cancel_label = if cancelling {
                kit.t("common.cancelling")
            } else {
                kit.t("common.cancel")
            };
            if kit
                .button(ui, cancel_label, ButtonKind::Secondary, !cancelling)
                .clicked()
            {
                self.submit(Command::Cancel { op });
            }
        });
    }

    fn progress_block(
        &mut self,
        ui: &mut Ui,
        kit: &Kit<'_>,
        op: OperationId,
        progress: Option<Progress>,
        cancelling: bool,
        cancellable: bool,
    ) {
        let th = kit.theme;
        let step = progress.map_or(Step::Working, |p| p.step);
        kit.label_step(ui, step, progress);
        if cancellable {
            ui.add_space(th.space.sm);
            let label = if cancelling {
                kit.t("common.cancelling")
            } else {
                kit.t("common.cancel")
            };
            if kit
                .button(ui, label, ButtonKind::Secondary, !cancelling)
                .clicked()
            {
                self.submit(Command::Cancel { op });
            }
        }
    }

    fn game_block(&mut self, ui: &mut Ui, kit: &Kit<'_>) {
        let th = kit.theme;
        let Some((_, state)) = self.model.game.clone() else {
            return;
        };
        match state {
            GameState::Starting => {
                ui.horizontal_wrapped(|ui| {
                    kit.spinner(ui, kit.t("game.starting"));
                    kit.body(ui, kit.t("game.starting"));
                });
            }
            GameState::Running { title_screen } => {
                ui.horizontal_wrapped(|ui| {
                    kit.icon(ui, Icon::Check, th.c.success);
                    kit.body(
                        ui,
                        kit.t(if title_screen {
                            "game.title_screen"
                        } else {
                            "game.running"
                        }),
                    );
                });
                // The mod reached its title screen but reported its engine off: say so plainly.
                if let Some((engine_session, false, state)) = self.model.game_engine
                    && Some(engine_session) == self.model.game.as_ref().map(|(s, _)| *s)
                {
                    let reason = kit.t(engine_off_key(state));
                    kit.banner(
                        ui,
                        Tone::Warning,
                        kit.t("game.engine_off.title"),
                        &kit.tr.fmt("game.engine_off.body", &[reason]),
                        &[],
                    );
                }
                kit.caption(ui, kit.t("game.running_note"));
            }
            GameState::Exited { .. } | GameState::Crashed { .. } => {}
        }
    }

    fn notice_area(&mut self, ui: &mut Ui, kit: &Kit<'_>) {
        let th = kit.theme;
        let Some(notice) = self.model.notice.clone() else {
            return;
        };
        match &notice {
            Notice::Error { during, error } => self.error_banner(ui, kit, *during, error),
            Notice::GameCrashed { code, crash_report } => {
                let body = match code {
                    Some(c) => kit
                        .tr
                        .fmt("notice.game_crashed.body_code", &[&c.to_string()]),
                    None => kit.t("notice.game_crashed.body").to_owned(),
                };
                let mut actions = vec![(kit.t("notice.action.view_logs"), ButtonKind::Secondary)];
                let can_open = crash_report.is_some() && self.model.capabilities().open_folders;
                if can_open {
                    actions.push((
                        kit.t("notice.action.open_crash_reports"),
                        ButtonKind::Secondary,
                    ));
                }
                actions.push((kit.t("common.dismiss"), ButtonKind::Quiet));
                match kit.banner(
                    ui,
                    Tone::Danger,
                    kit.t("notice.game_crashed.title"),
                    &body,
                    &actions,
                ) {
                    Some(0) => self.page = Page::Diagnostics,
                    Some(1) if can_open => self.submit(Command::OpenFolder(Folder::CrashReports)),
                    Some(_) => self.model.dismiss_notice(),
                    None => {}
                }
            }
            other => {
                let tone = if other.is_error() {
                    Tone::Warning
                } else if matches!(other, Notice::InstallReady | Notice::Exported(_)) {
                    Tone::Success
                } else {
                    Tone::Info
                };
                let title = notice_title(kit.tr, other);
                let body = match other {
                    Notice::Exported(path) => redact(&path.display().to_string()).into_owned(),
                    Notice::GameExited { code } if *code != 0 => kit
                        .tr
                        .fmt("notice.game_exited.body_code", &[&code.to_string()]),
                    _ => String::new(),
                };
                let mut actions = Vec::new();
                if matches!(other, Notice::Exported(_)) {
                    actions.push((kit.t("diag.open_folder"), ButtonKind::Secondary));
                }
                actions.push((kit.t("common.dismiss"), ButtonKind::Quiet));
                match kit.banner(ui, tone, &title, &body, &actions) {
                    Some(0) if matches!(other, Notice::Exported(_)) => {
                        if let Notice::Exported(path) = other {
                            self.submit(Command::RevealFile(path.clone()));
                        }
                    }
                    Some(_) => self.model.dismiss_notice(),
                    None => {}
                }
            }
        }
        ui.add_space(th.space.lg);
    }

    fn error_banner(
        &mut self,
        ui: &mut Ui,
        kit: &Kit<'_>,
        during: OperationKind,
        error: &LauncherError,
    ) {
        let th = kit.theme;
        let (title_key, body_key) = error_keys(error.kind);
        let retry = error.retryable.then(|| retry_command(during)).flatten();
        let repair = matches!(error.kind, ErrorKind::ChecksumMismatch);
        let sign_in = matches!(
            error.kind,
            ErrorKind::SessionExpired | ErrorKind::NotSignedIn | ErrorKind::Authentication
        );
        let mut actions: Vec<(&str, ButtonKind)> = Vec::new();
        if retry.is_some() {
            actions.push((kit.t("common.retry"), ButtonKind::Secondary));
        }
        if repair {
            actions.push((kit.t("play.action.repair"), ButtonKind::Secondary));
        }
        if sign_in {
            actions.push((kit.t("play.action.sign_in"), ButtonKind::Secondary));
        }
        let details_index = actions.len();
        actions.push((
            kit.t(if self.details_open {
                "common.hide_details"
            } else {
                "common.details"
            }),
            ButtonKind::Quiet,
        ));
        actions.push((kit.t("common.dismiss"), ButtonKind::Quiet));
        // Coarse backend categories carry the backend's safe, specific message; typed kinds use our copy.
        let body = if error.kind.body_from_backend() && !error.detail.is_empty() {
            redact(&error.detail).into_owned()
        } else {
            kit.t(body_key).to_owned()
        };
        let pressed = kit.banner(ui, Tone::Danger, kit.t(title_key), &body, &actions);
        if self.details_open {
            ui.add_space(th.space.xs);
            let detail = redact(&error.detail).into_owned();
            kit.caption(
                ui,
                &kit.tr.fmt(
                    "error.details_line",
                    &[&format!("{:?}", error.kind), &detail],
                ),
            );
        }
        let Some(i) = pressed else { return };
        let mut idx = 0;
        if let Some(cmd) = retry {
            if i == idx {
                self.submit(cmd);
                return;
            }
            idx += 1;
        }
        if repair {
            if i == idx {
                self.submit(Command::Prepare { repair: true });
                return;
            }
            idx += 1;
        }
        if sign_in {
            if i == idx {
                self.submit(Command::SignIn {
                    switch_account: false,
                });
                return;
            }
            idx += 1;
        }
        debug_assert_eq!(idx, details_index);
        if i == details_index {
            self.details_open = !self.details_open;
        } else {
            self.details_open = false;
            self.model.dismiss_notice();
        }
    }

    // ----------------------------------------------------------------------------------------
    // Settings

    fn settings_page(&mut self, ui: &mut Ui, kit: &Kit<'_>) {
        let th = kit.theme;
        let Some(saved) = self.model.settings.clone() else {
            return;
        };
        let mut draft = self.draft.take().unwrap_or_else(|| saved.clone());
        kit.title(ui, kit.t("nav.settings"));
        ui.add_space(th.space.md);
        self.notice_area(ui, kit);

        kit.card(ui, Surface::Card, |ui| self.account_section(ui, kit));
        ui.add_space(th.space.lg);
        kit.card(ui, Surface::Card, |ui| {
            self.install_section(ui, kit, &saved)
        });
        ui.add_space(th.space.lg);
        kit.card(ui, Surface::Card, |ui| {
            self.java_section(ui, kit, &mut draft)
        });
        ui.add_space(th.space.lg);
        kit.card(ui, Surface::Card, |ui| {
            kit.section(ui, kit.t("settings.memory.section"));
            ui.add_space(th.space.sm);
            if let Some(system) = &self.model.system {
                let b = system.memory;
                let label = kit.t("settings.memory.label");
                let label_resp = kit.text(ui, th.text.label, th.c.text_primary, label);
                let gib = |mib: f64| format!("{:.1}", mib / 1024.0);
                let mut value = draft.memory_mib.clamp(b.min_mib, b.max_mib);
                let resp = ui
                    .add(
                        egui::Slider::new(&mut value, b.min_mib..=b.max_mib)
                            .step_by(f64::from(th.limits.memory_step_mib))
                            .custom_formatter(|v, _| {
                                kit.tr.fmt("settings.memory.value", &[&gib(v)])
                            })
                            .custom_parser(|s| {
                                s.trim()
                                    .trim_end_matches(|c: char| !c.is_ascii_digit())
                                    .parse::<f64>()
                                    .ok()
                                    .map(|g| g * 1024.0)
                            }),
                    )
                    .labelled_by(label_resp.id);
                if resp.has_focus() && kit.keyboard {
                    paint_focus_ring(ui.painter(), th, resp.rect, th.shape.radius_sm);
                }
                draft.memory_mib = value;
                kit.caption(
                    ui,
                    &kit.tr.fmt(
                        "settings.memory.range",
                        &[
                            &gib(f64::from(b.min_mib)),
                            &gib(f64::from(b.max_mib)),
                            &gib(f64::from(b.recommended_mib)),
                        ],
                    ),
                );
            }
        });
        ui.add_space(th.space.lg);
        kit.card(ui, Surface::Card, |ui| {
            kit.section(ui, kit.t("settings.while_playing.section"));
            ui.add_space(th.space.sm);
            kit.segmented(
                ui,
                "while_playing",
                kit.t("settings.while_playing.label"),
                &[
                    (
                        WhilePlaying::KeepOpen,
                        kit.t("settings.while_playing.keep_open"),
                    ),
                    (
                        WhilePlaying::Minimize,
                        kit.t("settings.while_playing.minimize"),
                    ),
                ],
                &mut draft.while_playing,
            );
        });
        ui.add_space(th.space.lg);
        kit.card(ui, Surface::Card, |ui| {
            kit.section(ui, kit.t("settings.a11y.section"));
            if self.lib.as_ref().is_some_and(|m| m.caps().themes) {
                if kit
                    .button(ui, kit.t("nav.appearance"), ButtonKind::Secondary, true)
                    .clicked()
                {
                    self.page = Page::Appearance;
                }
                return;
            }
            // "Follow Windows" only when the backend can actually read the Windows setting.
            let follow = self
                .model
                .system
                .as_ref()
                .is_some_and(|s| s.system_reduced_motion.is_some());
            let mut motion_options = Vec::with_capacity(4);
            if follow {
                motion_options.push((
                    MotionPreference::FollowSystem,
                    kit.t("settings.a11y.motion.system"),
                ));
            }
            motion_options.extend([
                (MotionPreference::Full, kit.t("settings.a11y.motion.full")),
                (
                    MotionPreference::Reduced,
                    kit.t("settings.a11y.motion.reduced"),
                ),
                (MotionPreference::Off, kit.t("settings.a11y.motion.off")),
            ]);
            ui.add_space(th.space.sm);
            kit.segmented(
                ui,
                "motion",
                kit.t("settings.a11y.motion"),
                &motion_options,
                &mut draft.motion,
            );
            kit.caption(ui, kit.t("settings.a11y.motion_note"));
            ui.add_space(th.space.md);
            let labels: Vec<String> = UI_SCALES
                .iter()
                .map(|p| kit.tr.fmt("settings.a11y.scale_value", &[&p.to_string()]))
                .collect();
            let options: Vec<(u16, &str)> = UI_SCALES
                .iter()
                .copied()
                .zip(labels.iter().map(String::as_str))
                .collect();
            kit.segmented(
                ui,
                "scale",
                kit.t("settings.a11y.scale"),
                &options,
                &mut draft.ui_scale_percent,
            );
        });
        ui.add_space(th.space.lg);
        kit.card(ui, Surface::Card, |ui| self.about_section(ui, kit));
        ui.add_space(th.space.lg);

        let dirty = draft != saved;
        if dirty {
            let busy = !self
                .model
                .may_submit(&Command::UpdateSettings(draft.clone()));
            let mut save = false;
            let mut discard = false;
            kit.card(ui, Surface::Panel, |ui| {
                kit.body(
                    ui,
                    kit.t(if self.model.game_live() {
                        "settings.unsaved_game_running"
                    } else {
                        "settings.unsaved"
                    }),
                );
                ui.horizontal_wrapped(|ui| {
                    save = kit
                        .button(ui, kit.t("settings.save"), ButtonKind::Primary, !busy)
                        .clicked();
                    discard = kit
                        .button(ui, kit.t("settings.discard"), ButtonKind::Secondary, true)
                        .clicked();
                });
            });
            if save {
                self.submit(Command::UpdateSettings(draft.clone()));
            }
            if discard {
                draft = saved.clone();
                self.java_path.clear();
            }
        }
        self.draft = (draft != saved).then_some(draft);
    }

    fn account_section(&mut self, ui: &mut Ui, kit: &Kit<'_>) {
        let th = kit.theme;
        kit.section(ui, kit.t("account.section"));
        ui.add_space(th.space.sm);
        let busy = self.model.launch_op().is_some() || self.model.game_live();
        match self.model.account.clone() {
            Some(a) => {
                kit.body(ui, &kit.tr.fmt("account.signed_in_as", &[&a.username]));
                kit.caption(ui, &kit.tr.fmt("account.uuid", &[&a.uuid]));
                ui.add_space(th.space.sm);
                ui.horizontal_wrapped(|ui| {
                    let can = !busy && self.model.sign_in_op().is_none();
                    if kit
                        .button(ui, kit.t("account.switch"), ButtonKind::Secondary, can)
                        .clicked()
                    {
                        self.submit(Command::SignIn {
                            switch_account: true,
                        });
                        self.page = Page::Play;
                    }
                    if kit
                        .button(ui, kit.t("account.sign_out"), ButtonKind::Secondary, can)
                        .clicked()
                    {
                        self.submit(Command::SignOut);
                    }
                });
                kit.caption(ui, kit.t("account.storage_note"));
            }
            None => {
                kit.body(ui, kit.t("account.not_signed_in"));
                let can = self.model.may_submit(&Command::SignIn {
                    switch_account: false,
                });
                if kit
                    .button(ui, kit.t("play.action.sign_in"), ButtonKind::Secondary, can)
                    .clicked()
                {
                    self.submit(Command::SignIn {
                        switch_account: false,
                    });
                    self.page = Page::Play;
                }
            }
        }
    }

    fn install_section(&mut self, ui: &mut Ui, kit: &Kit<'_>, saved: &Settings) {
        let th = kit.theme;
        kit.section(ui, kit.t("install.section"));
        ui.add_space(th.space.sm);
        let path = saved
            .install_root
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_default();
        kit.text(
            ui,
            th.text.label,
            th.c.text_primary,
            kit.t("install.location"),
        );
        kit.secondary(ui, &path);
        ui.add_space(th.space.sm);
        let busy = self.model.prepare_op().is_some()
            || self.model.launch_op().is_some()
            || self.model.game_live()
            || self.selected_instance().is_some();
        ui.horizontal_wrapped(|ui| {
            if self.model.capabilities().open_folders
                && kit
                    .button(
                        ui,
                        kit.t("install.open_folder"),
                        ButtonKind::Secondary,
                        true,
                    )
                    .clicked()
            {
                self.submit(Command::OpenFolder(Folder::InstallRoot));
            }
            if kit
                .button(
                    ui,
                    kit.t("install.change_location"),
                    ButtonKind::Secondary,
                    !busy && self.pick.is_none(),
                )
                .clicked()
            {
                self.start_pick(PickTarget::SettingsInstallRoot);
            }
        });
        kit.caption(ui, kit.t("install.change_location_note"));
        ui.add_space(th.space.md);
        kit.text(
            ui,
            th.text.label,
            th.c.text_primary,
            kit.t("install.repair"),
        );
        kit.secondary(ui, kit.t("install.repair_note"));
        if kit
            .button(
                ui,
                kit.t("install.repair_action"),
                ButtonKind::Secondary,
                !busy && self.model.may_submit(&Command::Prepare { repair: true }),
            )
            .clicked()
        {
            self.submit(Command::Prepare { repair: true });
            self.page = Page::Play;
        }
    }

    fn java_section(&mut self, ui: &mut Ui, kit: &Kit<'_>, draft: &mut Settings) {
        let th = kit.theme;
        kit.section(ui, kit.t("settings.java.section"));
        ui.add_space(th.space.sm);
        let mut mode = match draft.java {
            JavaChoice::Managed => JavaMode::Managed,
            JavaChoice::Custom(_) => JavaMode::Custom,
        };
        let options = [
            (JavaMode::Managed, kit.t("settings.java.managed")),
            (JavaMode::Custom, kit.t("settings.java.custom")),
        ];
        if kit.segmented(
            ui,
            "java",
            kit.t("settings.java.label"),
            &options,
            &mut mode,
        ) {
            draft.java = match mode {
                JavaMode::Managed => JavaChoice::Managed,
                JavaMode::Custom => {
                    if let JavaChoice::Custom(p) = &self
                        .model
                        .settings
                        .as_ref()
                        .map_or(JavaChoice::Managed, |s| s.java.clone())
                    {
                        JavaChoice::Custom(p.clone())
                    } else {
                        JavaChoice::Custom(PathBuf::new())
                    }
                }
            };
        }
        match &draft.java {
            JavaChoice::Managed => {
                kit.caption(ui, kit.t("settings.java.managed_note"));
            }
            JavaChoice::Custom(current) => {
                if self.java_path.is_empty() && !current.as_os_str().is_empty() {
                    self.java_path = current.display().to_string();
                }
                ui.add_space(th.space.sm);
                kit.text_field(
                    ui,
                    kit.t("settings.java.path"),
                    &mut self.java_path,
                    kit.t("settings.java.path_hint"),
                );
                ui.horizontal_wrapped(|ui| {
                    let candidate = PathBuf::from(self.java_path.trim());
                    let check = Command::CheckJava {
                        path: candidate.clone(),
                    };
                    if kit
                        .button(
                            ui,
                            kit.t("common.browse"),
                            ButtonKind::Secondary,
                            self.pick.is_none(),
                        )
                        .clicked()
                    {
                        self.start_pick(PickTarget::Java);
                    }
                    if self.model.capabilities().check_java
                        && kit
                            .button(
                                ui,
                                kit.t("settings.java.check"),
                                ButtonKind::Secondary,
                                !self.java_path.trim().is_empty() && self.model.may_submit(&check),
                            )
                            .clicked()
                    {
                        self.submit(check);
                    }
                });
                if !self.model.capabilities().check_java {
                    // The backend validates Java (21, x64) when saving is used for a launch.
                    let typed = self.java_path.trim();
                    if !typed.is_empty() {
                        draft.java = JavaChoice::Custom(PathBuf::from(typed));
                    }
                    kit.caption(ui, kit.t("settings.java.checked_on_play"));
                    return;
                }
                match self.model.java_check.clone() {
                    Some(JavaCheckView::Checking(_)) => {
                        ui.horizontal_wrapped(|ui| {
                            kit.spinner(ui, kit.t("settings.java.checking"));
                            kit.secondary(ui, kit.t("settings.java.checking"));
                        });
                    }
                    Some(JavaCheckView::Valid(info)) => {
                        ui.horizontal_wrapped(|ui| {
                            kit.icon(ui, Icon::Check, th.c.success);
                            kit.secondary(
                                ui,
                                &kit.tr
                                    .fmt("settings.java.valid", &[&info.version, &info.vendor]),
                            );
                        });
                        draft.java = JavaChoice::Custom(info.path.clone());
                    }
                    Some(JavaCheckView::Invalid(e)) => {
                        ui.horizontal_wrapped(|ui| {
                            kit.icon(ui, Icon::Error, th.c.danger);
                            let (title, _) = error_keys(e.kind);
                            kit.secondary(ui, kit.t(title));
                        });
                    }
                    None => {
                        kit.caption(ui, kit.t("settings.java.custom_note"));
                    }
                }
            }
        }
    }

    fn about_section(&self, ui: &mut Ui, kit: &Kit<'_>) {
        let th = kit.theme;
        kit.section(ui, kit.t("about.section"));
        ui.add_space(th.space.sm);
        if let Some(s) = &self.model.system {
            kit.secondary(
                ui,
                &kit.tr.fmt("about.launcher_version", &[&s.launcher_version]),
            );
            kit.secondary(
                ui,
                &kit.tr.fmt(
                    "about.api_version",
                    &[&format!("{}.{}", s.api_version.0, s.api_version.1)],
                ),
            );
        }
        if let Some(p) = &self.model.profile {
            kit.secondary(
                ui,
                &kit.tr.fmt(
                    "about.profile",
                    &[&p.minecraft, &p.fabric_loader, &p.fabric_api, &p.ooze],
                ),
            );
        }
        kit.caption(ui, kit.t("about.fonts"));
        kit.caption(ui, kit.t("about.not_affiliated"));
    }

    // ----------------------------------------------------------------------------------------
    // Diagnostics

    fn diagnostics_page(&mut self, ui: &mut Ui, kit: &Kit<'_>) {
        let th = kit.theme;
        kit.title(ui, kit.t("nav.diagnostics"));
        ui.add_space(th.space.md);
        self.notice_area(ui, kit);
        kit.secondary(ui, kit.t("diag.redaction_note"));
        if self.model.capabilities().export_diagnostics {
            kit.caption(ui, kit.t("diag.export_note"));
        }
        ui.add_space(th.space.sm);
        ui.horizontal_wrapped(|ui| {
            if kit
                .button(
                    ui,
                    kit.t("diag.copy"),
                    ButtonKind::Secondary,
                    !self.model.logs.is_empty(),
                )
                .clicked()
            {
                let text: String = self
                    .model
                    .logs
                    .iter()
                    .map(|l| format!("{} {}\n", level_tag(l.level), redact(&l.text)))
                    .collect();
                ui.ctx().copy_text(text);
                self.live_text = kit.t("diag.copied").to_owned();
            }
            let caps = self.model.capabilities();
            if caps.export_diagnostics
                && kit
                    .button(
                        ui,
                        kit.t("diag.export"),
                        ButtonKind::Secondary,
                        self.pick.is_none()
                            && self.model.may_submit(&Command::ExportDiagnostics {
                                destination: PathBuf::new(),
                            }),
                    )
                    .clicked()
            {
                self.start_pick(PickTarget::DiagnosticsZip);
            }
            if caps.open_folders
                && kit
                    .button(ui, kit.t("diag.open_logs"), ButtonKind::Secondary, true)
                    .clicked()
            {
                self.submit(Command::OpenFolder(Folder::Logs));
            }
            if caps.open_folders
                && kit
                    .button(
                        ui,
                        kit.t("notice.action.open_crash_reports"),
                        ButtonKind::Secondary,
                        true,
                    )
                    .clicked()
            {
                self.submit(Command::OpenFolder(Folder::CrashReports));
            }
        });
        ui.add_space(th.space.md);
        let height = (ui.available_height() - th.space.lg).max(th.control.height_large * 4.0);
        let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
        paint_gel(
            ui.painter(),
            th,
            rect,
            Gel::new(Surface::Sunken, th.shape.radius_lg),
        );
        let mut child = ui.new_child(UiBuilder::new().max_rect(rect.shrink(th.space.md)));
        if self.model.logs.is_empty() {
            kit.secondary(&mut child, kit.t("diag.empty"));
            return;
        }
        let row_h = th.text.log.size * 1.45;
        let logs = &self.model.logs;
        ScrollArea::vertical()
            .stick_to_bottom(true)
            .auto_shrink([false, false])
            .show_rows(&mut child, row_h, logs.len(), |ui, range| {
                for line in logs.range(range) {
                    let color = match line.level {
                        LogLevel::Error => th.c.danger,
                        LogLevel::Warn => th.c.warning,
                        _ => th.c.text_primary,
                    };
                    let source = match line.source {
                        LogSource::Launcher => kit.t("diag.source.launcher"),
                        LogSource::Game => kit.t("diag.source.game"),
                    };
                    let text = format!("{} {} {}", level_tag(line.level), source, line.text);
                    ui.add(
                        egui::Label::new(RichText::new(text).font(font(th.text.log)).color(color))
                            .truncate(),
                    );
                }
            });
    }

    // ----------------------------------------------------------------------------------------
    // Folder picking (native dialog on a worker thread so the window stays responsive)

    fn start_pick(&mut self, target: PickTarget) {
        let (tx, rx) = std::sync::mpsc::channel();
        #[cfg(test)]
        if let Some(picked) = self.pick_for_tests.take() {
            let _ = tx.send(picked);
            self.pick = Some((target, rx));
            return;
        }
        let title = self
            .tr
            .get(match target {
                PickTarget::Java => "settings.java.pick_title",
                PickTarget::DiagnosticsZip => "diag.export_pick_title",
                _ => "first_run.pick_title",
            })
            .to_owned();
        let file_name = diagnostics_file_name();
        std::thread::spawn(move || {
            let dialog = rfd::FileDialog::new().set_title(title);
            let picked = match target {
                PickTarget::Java => dialog.add_filter("Java", &["exe", ""]).pick_file(),
                PickTarget::DiagnosticsZip => dialog
                    .add_filter("ZIP", &["zip"])
                    .set_file_name(file_name)
                    .save_file(),
                _ => dialog.pick_folder(),
            };
            let _ = tx.send(picked);
        });
        self.pick = Some((target, rx));
    }

    fn poll_pick(&mut self) {
        let Some((target, rx)) = &self.pick else {
            return;
        };
        match rx.try_recv() {
            Ok(picked) => {
                let target = *target;
                self.pick = None;
                if let Some(path) = picked {
                    match target {
                        PickTarget::InstallRoot => self.install_path = path.display().to_string(),
                        PickTarget::SettingsInstallRoot => {
                            self.submit(Command::SetInstallRoot { path })
                        }
                        PickTarget::Java => self.java_path = path.display().to_string(),
                        PickTarget::DiagnosticsZip => {
                            let destination = with_zip_extension(path);
                            if crate::core_backend::valid_export_path(&destination) {
                                self.submit(Command::ExportDiagnostics { destination });
                            } else {
                                self.live_text = self.tr.get("diag.export_bad_path").to_owned();
                            }
                        }
                    }
                }
            }
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => self.pick = None,
        }
    }

    // ----------------------------------------------------------------------------------------
    // Closing

    /// Decide whether a window close request may proceed. Returns `true` to allow it.
    pub fn on_close_requested(&mut self) -> bool {
        if self.allow_close {
            return true;
        }
        if let Some(library) = self.lib.as_ref().filter(|m| m.busy()) {
            self.close_prompt = Some(if library.cancellable() {
                CloseGuard::ConfirmLibrary
            } else {
                CloseGuard::WaitLibrary
            });
            return false;
        }
        match self.model.close_guard() {
            CloseGuard::Allow => true,
            guard => {
                self.close_prompt = Some(guard);
                false
            }
        }
    }

    fn close_dialog(&mut self, ctx: &Context, kit: &Kit<'_>) {
        let Some(guard) = self.close_prompt else {
            return;
        };
        let th = kit.theme;
        let (title, body, confirm) = match guard {
            CloseGuard::ConfirmLibrary => (
                "close.library.title",
                "close.library.body",
                "close.cancel_and_close",
            ),
            CloseGuard::WaitLibrary => (
                "close.library.title",
                "close.library.wait",
                "close.library.after",
            ),
            CloseGuard::ConfirmGameRunning => (
                "close.game_running.title",
                "close.game_running.body",
                "close.game_running.confirm",
            ),
            CloseGuard::ConfirmOperation(OperationKind::SignIn) => (
                "close.sign_in.title",
                "close.sign_in.body",
                "close.cancel_and_close",
            ),
            CloseGuard::ConfirmOperation(OperationKind::Launch) => (
                "close.launch.title",
                "close.launch.body",
                "close.close_anyway",
            ),
            CloseGuard::ConfirmOperation(_) => (
                "close.install.title",
                "close.install.body",
                "close.cancel_and_close",
            ),
            CloseGuard::Allow => return,
        };
        let mut decision = None;
        let modal = egui::Modal::new(egui::Id::unique("ooze-close-dialog"))
            .backdrop_color(th.c.scrim)
            .frame(Frame::new().fill(Color32::TRANSPARENT))
            .show(ctx, |ui| {
                ui.set_max_width(th.control.content_max_width * 0.7);
                kit.card(ui, Surface::Panel, |ui| {
                    kit.title(ui, kit.t(title));
                    kit.body(ui, kit.t(body));
                    ui.add_space(th.space.lg);
                    ui.horizontal_wrapped(|ui| {
                        let keep =
                            kit.button(ui, kit.t("close.keep_open"), ButtonKind::Primary, true);
                        if ui.memory(|m| m.focused().is_none()) {
                            keep.request_focus();
                        }
                        if keep.clicked() {
                            decision = Some(false);
                        }
                        if kit
                            .button(
                                ui,
                                kit.t(confirm),
                                ButtonKind::Secondary,
                                !self.close_after_library,
                            )
                            .clicked()
                        {
                            decision = Some(true);
                        }
                    });
                });
            });
        if modal.should_close() || ctx.input(|i| i.key_pressed(Key::Escape)) {
            decision.get_or_insert(false);
        }
        match decision {
            Some(true) => {
                if matches!(guard, CloseGuard::ConfirmLibrary | CloseGuard::WaitLibrary) {
                    if guard == CloseGuard::ConfirmLibrary {
                        self.lib_cancel();
                    }
                    self.close_after_library = true;
                    self.close_prompt = Some(CloseGuard::WaitLibrary);
                    return;
                }
                for op in self.model.cancellable_ops() {
                    self.submit(Command::Cancel { op });
                }
                self.close_prompt = None;
                self.allow_close = true;
                ctx.send_viewport_cmd(ViewportCommand::Close);
            }
            Some(false) => {
                self.close_prompt = None;
                self.close_after_library = false;
            }
            None => {}
        }
    }
}

impl eframe::App for LauncherApp {
    fn logic(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        self.pump(ctx);
    }

    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        if ui.ctx().input(|i| i.viewport().close_requested()) && !self.on_close_requested() {
            ui.ctx().send_viewport_cmd(ViewportCommand::CancelClose);
        }
        self.frame(ui);
    }
}

impl Kit<'_> {
    fn label_step(&self, ui: &mut Ui, step: Step, progress: Option<Progress>) {
        let th = self.theme;
        let label = self.t(step_key(step));
        ui.horizontal_wrapped(|ui| {
            self.text(ui, th.text.label, th.c.text_primary, label);
            if let Some(p) = progress
                && let Some(total) = p.total
            {
                let amount = match p.unit {
                    ProgressUnit::Bytes => self
                        .tr
                        .fmt("play.progress.bytes", &[&mib(p.done), &mib(total)]),
                    ProgressUnit::Files => self.tr.fmt(
                        "play.progress.files",
                        &[&p.done.to_string(), &total.to_string()],
                    ),
                    ProgressUnit::Steps => self.tr.fmt(
                        "play.progress.steps",
                        &[&p.done.to_string(), &total.to_string()],
                    ),
                };
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    self.caption(ui, &amount);
                });
            }
        });
        ui.add_space(th.space.xs);
        self.progress(ui, progress.and_then(|p| p.fraction()), label);
    }
}

fn mib(bytes: u64) -> String {
    format!("{:.1}", bytes as f64 / 1_048_576.0)
}

fn level_tag(level: LogLevel) -> &'static str {
    match level {
        LogLevel::Debug => "DEBUG",
        LogLevel::Info => "INFO ",
        LogLevel::Warn => "WARN ",
        LogLevel::Error => "ERROR",
    }
}

/// Localised reason for an engine state other than LOADED.
fn engine_off_key(state: &str) -> &'static str {
    match state {
        "MISSING_LIBRARY" => "game.engine_off.missing_library",
        "UNSUPPORTED_PLATFORM" => "game.engine_off.unsupported_platform",
        "INCOMPATIBLE" => "game.engine_off.incompatible",
        _ => "game.engine_off.load_failed",
    }
}

/// Suggested name for an exported diagnostics file: `ooze-diagnostics-<unix seconds>.zip`.
fn diagnostics_file_name() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    format!("ooze-diagnostics-{secs}.zip")
}

/// A save dialog may return a name without the extension; exports are always `.zip`.
fn with_zip_extension(path: PathBuf) -> PathBuf {
    if path
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("zip"))
    {
        path
    } else {
        let mut os = path.into_os_string();
        os.push(".zip");
        PathBuf::from(os)
    }
}

fn retry_command(during: OperationKind) -> Option<Command> {
    match during {
        OperationKind::SignIn => Some(Command::SignIn {
            switch_account: false,
        }),
        OperationKind::Prepare { repair } => Some(Command::Prepare { repair }),
        OperationKind::Launch => Some(Command::Play),
        // A retry needs a new destination: the player chooses it again with Export diagnostics.
        _ => None,
    }
}

pub fn notice_title(tr: &Strings, notice: &Notice) -> String {
    match notice {
        Notice::Error { error, .. } => tr.get(error_keys(error.kind).0).to_owned(),
        Notice::Cancelled(kind) => tr
            .get(match kind {
                OperationKind::SignIn => "notice.cancelled.sign_in",
                OperationKind::Prepare { .. } => "notice.cancelled.install",
                _ => "notice.cancelled.other",
            })
            .to_owned(),
        Notice::SignedOut => tr.get("notice.signed_out").to_owned(),
        Notice::InstallReady => tr.get("notice.install_ready").to_owned(),
        Notice::GameExited { code: 0 } => tr.get("notice.game_exited.title").to_owned(),
        Notice::GameExited { .. } => tr.get("notice.game_exited.title_code").to_owned(),
        Notice::GameCrashed { .. } => tr.get("notice.game_crashed.title").to_owned(),
        Notice::Exported(_) => tr.get("notice.exported").to_owned(),
    }
}

pub fn step_key(step: Step) -> &'static str {
    match step {
        Step::WaitingForBrowser => "step.waiting_for_browser",
        Step::XboxLive => "step.xbox_live",
        Step::MinecraftServices => "step.minecraft_services",
        Step::CheckingOwnership => "step.checking_ownership",
        Step::LoadingProfile => "step.loading_profile",
        Step::CheckingFiles => "step.checking_files",
        Step::DownloadingRuntime => "step.downloading_runtime",
        Step::DownloadingGame => "step.downloading_game",
        Step::DownloadingLibraries => "step.downloading_libraries",
        Step::DownloadingAssets => "step.downloading_assets",
        Step::InstallingFabric => "step.installing_fabric",
        Step::InstallingOoze => "step.installing_ooze",
        Step::Verifying => "step.verifying",
        Step::Authorizing => "step.authorizing",
        Step::Resolving => "step.resolving",
        Step::Downloading => "step.downloading",
        Step::ExtractingJava => "step.extracting_java",
        Step::CheckingJava => "step.checking_java",
        Step::RefreshingSession => "step.refreshing_session",
        Step::StartingJava => "step.starting_java",
        Step::WaitingForTitleScreen => "step.waiting_for_title_screen",
        Step::Working => "step.working",
    }
}

/// Localisation keys (title, body) for each typed error.
#[cfg(test)]
pub(crate) fn lib_error_keys_for_tests(
    kind: crate::library::LibErrorKind,
) -> (&'static str, &'static str) {
    library_pages::lib_error_keys(kind)
}

/// Live-region text for a library notice.
pub fn library_notice_text(tr: &Strings, n: &crate::library_model::LibNotice) -> String {
    use crate::library_model::LibNotice as N;
    match n {
        N::Error { error, .. } => tr
            .get(library_pages::lib_error_keys(error.kind).0)
            .to_owned(),
        N::Cancelled => tr.get("library.cancelled").to_owned(),
        N::Installed { .. } => tr.get("content.installed.title").to_owned(),
        N::ProfileSaved => tr.get("profile.saved").to_owned(),
        N::ThemeSaved => tr.get("appearance.saved").to_owned(),
        N::ContentChanged => tr.get("content.changed").to_owned(),
        N::IntegrityResolved => tr.get("integrity.resolved").to_owned(),
    }
}

pub fn error_keys(kind: ErrorKind) -> (&'static str, &'static str) {
    match kind {
        ErrorKind::UnsupportedPlatform => (
            "error.unsupported_platform.title",
            "error.unsupported_platform.body",
        ),
        ErrorKind::Configuration => ("error.configuration.title", "error.configuration.body"),
        ErrorKind::Io => ("error.io.title", "error.io.body"),
        ErrorKind::Network => ("error.network.title", "error.network.body"),
        ErrorKind::Authentication => ("error.authentication.title", "error.authentication.body"),
        ErrorKind::Metadata => ("error.metadata.title", "error.metadata.body"),
        ErrorKind::JavaRuntime => ("error.java_runtime.title", "error.java_runtime.body"),
        ErrorKind::ServiceDenied => ("error.service_denied.title", "error.service_denied.body"),
        ErrorKind::CredentialStore => (
            "error.credential_store.title",
            "error.credential_store.body",
        ),
        ErrorKind::AuthNotConfigured => (
            "error.auth_not_configured.title",
            "error.auth_not_configured.body",
        ),
        ErrorKind::AuthAppNotApproved => (
            "error.auth_app_not_approved.title",
            "error.auth_app_not_approved.body",
        ),
        ErrorKind::BrowserUnavailable => (
            "error.browser_unavailable.title",
            "error.browser_unavailable.body",
        ),
        ErrorKind::SignInTimedOut => (
            "error.sign_in_timed_out.title",
            "error.sign_in_timed_out.body",
        ),
        ErrorKind::ConsentDenied => ("error.consent_denied.title", "error.consent_denied.body"),
        ErrorKind::NoXboxProfile => ("error.no_xbox_profile.title", "error.no_xbox_profile.body"),
        ErrorKind::XboxChildAccount => ("error.xbox_child.title", "error.xbox_child.body"),
        ErrorKind::XboxRegionUnavailable => ("error.xbox_region.title", "error.xbox_region.body"),
        ErrorKind::XboxBanned => ("error.xbox_banned.title", "error.xbox_banned.body"),
        ErrorKind::NotEntitled => ("error.not_entitled.title", "error.not_entitled.body"),
        ErrorKind::NoMinecraftProfile => (
            "error.no_minecraft_profile.title",
            "error.no_minecraft_profile.body",
        ),
        ErrorKind::SessionExpired => ("error.session_expired.title", "error.session_expired.body"),
        ErrorKind::Offline => ("error.offline.title", "error.offline.body"),
        ErrorKind::ServiceUnavailable => (
            "error.service_unavailable.title",
            "error.service_unavailable.body",
        ),
        ErrorKind::RateLimited => ("error.rate_limited.title", "error.rate_limited.body"),
        ErrorKind::PathNotWritable => (
            "error.path_not_writable.title",
            "error.path_not_writable.body",
        ),
        ErrorKind::PathInvalid => ("error.path_invalid.title", "error.path_invalid.body"),
        ErrorKind::PathInsideAnotherInstall => (
            "error.path_inside_install.title",
            "error.path_inside_install.body",
        ),
        ErrorKind::DiskFull => ("error.disk_full.title", "error.disk_full.body"),
        ErrorKind::FilesInUse => ("error.files_in_use.title", "error.files_in_use.body"),
        ErrorKind::ChecksumMismatch => ("error.checksum.title", "error.checksum.body"),
        ErrorKind::JavaNotFound => ("error.java_not_found.title", "error.java_not_found.body"),
        ErrorKind::JavaWrongVersion => (
            "error.java_wrong_version.title",
            "error.java_wrong_version.body",
        ),
        ErrorKind::JavaNot64Bit => ("error.java_not_64bit.title", "error.java_not_64bit.body"),
        ErrorKind::NotSignedIn => ("error.not_signed_in.title", "error.not_signed_in.body"),
        ErrorKind::NotInstalled => ("error.not_installed.title", "error.not_installed.body"),
        ErrorKind::AlreadyRunning => ("error.already_running.title", "error.already_running.body"),
        ErrorKind::Busy => ("error.busy.title", "error.busy.body"),
        ErrorKind::ProcessStartFailed => ("error.process_start.title", "error.process_start.body"),
        ErrorKind::Internal => ("error.internal.title", "error.internal.body"),
    }
}
