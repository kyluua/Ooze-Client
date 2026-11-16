//! Launcher components. Every interactive element:
//! - reports role, label, state and value to AccessKit (Windows UI Automation via egui-winit);
//! - is reachable with Tab / Shift+Tab and activated with Enter or Space;
//! - draws a visible focus ring in keyboard modality;
//! - reads every colour, size and duration from [`Theme`].

use std::sync::Arc;

use eframe::egui::{
    self, Align, Color32, FontFamily, FontId, Key, Layout, Rect, Response, RichText, Sense, Shape,
    Stroke, Ui, UiBuilder, Vec2, WidgetInfo, accesskit, pos2, vec2,
};

use crate::i18n::Strings;
use crate::motion::{MotionMode, ease_standard};
use crate::paint::{Gel, Surface, gel_shapes, paint_focus_ring, paint_gel};
use crate::theme::{Family, Theme, TypeRole, with_alpha};

pub const FAMILY_DISPLAY: &str = "ooze-display";
pub const FAMILY_STRONG: &str = "ooze-strong";

pub fn font(role: TypeRole) -> FontId {
    let family = match role.family {
        Family::Display => FontFamily::Name(Arc::from(FAMILY_DISPLAY)),
        Family::Strong => FontFamily::Name(Arc::from(FAMILY_STRONG)),
        Family::Body => FontFamily::Proportional,
    };
    FontId::new(role.size, family)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ButtonKind {
    /// Ink Gel: the one main action of a view.
    Primary,
    /// White Ooze control.
    Secondary,
    /// Text-only action for low-emphasis commands (still 32+ points tall).
    Quiet,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tone {
    Info,
    Success,
    Warning,
    Danger,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Icon {
    Check,
    Info,
    Warning,
    Error,
}

/// Shared component context for one frame.
pub struct Kit<'a> {
    pub theme: &'a Theme,
    pub tr: &'a Strings,
    pub motion: MotionMode,
    /// Focus-visible modality: the last input was the keyboard.
    pub keyboard: bool,
}

impl Kit<'_> {
    pub fn t<'k>(&'k self, key: &'k str) -> &'k str {
        self.tr.get(key)
    }

    pub fn text(
        &self,
        ui: &mut Ui,
        role: TypeRole,
        color: Color32,
        text: impl Into<String>,
    ) -> Response {
        ui.add(egui::Label::new(RichText::new(text.into()).font(font(role)).color(color)).wrap())
    }

    pub fn title(&self, ui: &mut Ui, text: &str) -> Response {
        let th = self.theme;
        let r = self.text(ui, th.text.title, th.c.text_primary, text);
        ui.ctx().accesskit_node_builder(r.id, |n| {
            n.set_role(accesskit::Role::Heading);
            n.set_label(text);
            n.set_level(1);
        });
        r
    }

    pub fn section(&self, ui: &mut Ui, text: &str) -> Response {
        let th = self.theme;
        let r = self.text(
            ui,
            th.text.section,
            th.c.text_secondary,
            text.to_uppercase(),
        );
        ui.ctx().accesskit_node_builder(r.id, |n| {
            n.set_role(accesskit::Role::Heading);
            n.set_label(text);
            n.set_level(2);
        });
        r
    }

    pub fn body(&self, ui: &mut Ui, text: &str) -> Response {
        self.text(ui, self.theme.text.body, self.theme.c.text_primary, text)
    }

    pub fn secondary(&self, ui: &mut Ui, text: &str) -> Response {
        self.text(
            ui,
            self.theme.text.body_small,
            self.theme.c.text_secondary,
            text,
        )
    }

    pub fn caption(&self, ui: &mut Ui, text: &str) -> Response {
        self.text(
            ui,
            self.theme.text.caption,
            self.theme.c.text_secondary,
            text,
        )
    }

    fn hover_amount(&self, ui: &Ui, resp: &Response, active: bool) -> f32 {
        let dur = self
            .motion
            .tween(&self.theme.motion, self.theme.motion.short);
        ease_standard(
            ui.ctx()
                .animate_bool_with_time(resp.id.with("hover"), active, dur),
        )
    }

    pub fn button(&self, ui: &mut Ui, label: &str, kind: ButtonKind, enabled: bool) -> Response {
        self.button_sized(ui, label, kind, enabled, false)
    }

    pub fn button_large(&self, ui: &mut Ui, label: &str, enabled: bool) -> Response {
        self.button_sized(ui, label, ButtonKind::Primary, enabled, true)
    }

    fn button_sized(
        &self,
        ui: &mut Ui,
        label: &str,
        kind: ButtonKind,
        enabled: bool,
        large: bool,
    ) -> Response {
        let th = self.theme;
        let text_color = match (kind, enabled) {
            (_, false) if kind == ButtonKind::Primary => th.c.text_on_accent,
            (_, false) => th.c.text_disabled,
            (ButtonKind::Primary, true) => th.c.text_on_accent,
            (ButtonKind::Secondary, true) => th.c.text_primary,
            // Quiet actions sit on tinted banners: ink text plus a permanent underline keeps them
            // above 4.5:1 and not distinguished by colour alone.
            (ButtonKind::Quiet, true) => th.c.text_primary,
        };
        let galley =
            ui.painter()
                .layout_no_wrap(label.to_owned(), font(th.text.button), text_color);
        let height = if large {
            th.control.height_large
        } else {
            th.control.height
        };
        let pad = if large {
            th.control.pad_x * 1.5
        } else {
            th.control.pad_x
        };
        let min_w = if kind == ButtonKind::Quiet {
            th.control.min_hit
        } else {
            th.control.min_width
        };
        let width = (galley.size().x + 2.0 * pad).max(min_w);
        let sense = if enabled {
            Sense::click()
        } else {
            Sense::hover()
        };
        let (rect, resp) = ui.allocate_exact_size(vec2(width, height), sense);
        resp.widget_info(|| WidgetInfo::labeled(accesskit::Role::Button, enabled, label));
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let hover = self.hover_amount(ui, &resp, enabled && resp.hovered());
        let pressed = enabled && resp.is_pointer_button_down_on();
        let radius = if large {
            th.shape.radius_lg
        } else {
            th.shape.radius_md
        };
        let grow = if self.motion == MotionMode::Full {
            hover * th.space.xxs * 0.5
        } else {
            0.0
        };
        let draw = rect.expand(grow);
        let text_pos = pos2(
            rect.center().x - galley.size().x / 2.0,
            rect.center().y - galley.size().y / 2.0,
        );
        let painter = ui.painter();
        match kind {
            ButtonKind::Quiet => {
                let y = text_pos.y + galley.size().y;
                let width = th.shape.outline_hairline
                    + hover * (th.shape.outline_control - th.shape.outline_hairline);
                painter.line_segment(
                    [pos2(text_pos.x, y), pos2(text_pos.x + galley.size().x, y)],
                    Stroke::new(width, text_color),
                );
            }
            _ => {
                let surface = if kind == ButtonKind::Primary {
                    Surface::Inverse
                } else {
                    Surface::Control
                };
                let mut gel = Gel::new(surface, radius);
                gel.hover = hover;
                gel.pressed = pressed;
                gel.disabled = !enabled;
                gel.gloss_limit = Some(text_pos.y - 1.0);
                paint_gel(painter, th, draw, gel);
            }
        }
        painter.galley(text_pos, galley, text_color);
        if resp.has_focus() && self.keyboard {
            paint_focus_ring(painter, th, draw, radius);
        }
        resp
    }

    /// A single-choice control (radio group). Only the selected option is a Tab stop; the arrow
    /// keys move the selection (roving focus, INTERACTION_FLOWS §3).
    pub fn segmented<T: Copy + PartialEq>(
        &self,
        ui: &mut Ui,
        id_salt: &str,
        label: &str,
        options: &[(T, &str)],
        value: &mut T,
    ) -> bool {
        let th = self.theme;
        let mut changed = false;
        ui.vertical(|ui| {
            let label_resp = self.text(ui, th.text.label, th.c.text_primary, label);
            ui.add_space(th.space.xs);
            let ids: Vec<egui::Id> = (0..options.len())
                .map(|i| ui.scope_id().with((id_salt, i)))
                .collect();
            let galleys: Vec<_> = options
                .iter()
                .map(|(_, l)| {
                    ui.painter().layout_no_wrap(
                        (*l).to_owned(),
                        font(th.text.label),
                        th.c.text_primary,
                    )
                })
                .collect();
            let seg_w =
                galleys.iter().map(|g| g.size().x).fold(0.0f32, f32::max) + 2.0 * th.control.pad_x;
            let height = th.control.height;
            let pad = th.space.xxs * 1.5;
            let total = vec2(seg_w * options.len() as f32 + 2.0 * pad, height + 2.0 * pad);
            let (track, _) = ui.allocate_exact_size(total, Sense::hover());
            let group = ui.new_child(UiBuilder::new().max_rect(track).id_salt((id_salt, "group")));
            ui.ctx().accesskit_node_builder(group.unique_id(), |n| {
                n.set_role(accesskit::Role::RadioGroup);
                n.push_labelled_by(label_resp.id.accesskit_id());
            });
            paint_gel(
                ui.painter(),
                th,
                track,
                Gel::new(Surface::Sunken, th.shape.radius_md + pad),
            );
            let ui = &group;
            let selected_index = options.iter().position(|(v, _)| *v == *value);
            let mut move_to: Option<usize> = None;
            for (i, ((v, l), galley)) in options.iter().zip(galleys).enumerate() {
                let rect = Rect::from_min_size(
                    pos2(track.left() + pad + seg_w * i as f32, track.top() + pad),
                    vec2(seg_w, height),
                );
                let selected = Some(i) == selected_index;
                let tab_stop = selected || (selected_index.is_none() && i == 0);
                let sense = if tab_stop {
                    Sense::click()
                } else {
                    Sense::CLICK
                };
                let resp = ui.interact(rect, ids[i], sense);
                resp.widget_info(|| {
                    WidgetInfo::selected(accesskit::Role::RadioButton, true, selected, *l)
                });
                if resp.clicked() && !selected {
                    *value = *v;
                    changed = true;
                }
                if resp.has_focus() {
                    let (prev, next) = ui.ctx().input_mut(|inp| {
                        (
                            inp.consume_key(egui::Modifiers::NONE, Key::ArrowLeft)
                                || inp.consume_key(egui::Modifiers::NONE, Key::ArrowUp),
                            inp.consume_key(egui::Modifiers::NONE, Key::ArrowRight)
                                || inp.consume_key(egui::Modifiers::NONE, Key::ArrowDown),
                        )
                    });
                    if prev {
                        move_to = Some((i + options.len() - 1) % options.len());
                    } else if next {
                        move_to = Some((i + 1) % options.len());
                    }
                }
                let hover = self.hover_amount(ui, &resp, resp.hovered() && !selected);
                let painter = ui.painter();
                let text_color = if selected {
                    th.c.text_on_accent
                } else {
                    th.c.text_primary
                };
                if selected {
                    paint_gel(
                        painter,
                        th,
                        rect,
                        Gel::new(Surface::Inverse, th.shape.radius_md),
                    );
                } else if hover > 0.0 {
                    painter.rect_filled(
                        rect,
                        th.shape.radius_md,
                        with_alpha(th.c.surface_raised, hover),
                    );
                }
                let pos = rect.center() - galley.size() / 2.0;
                painter.galley(pos, galley, text_color);
                if resp.has_focus() && self.keyboard {
                    paint_focus_ring(painter, th, rect, th.shape.radius_md);
                }
            }
            if let Some(j) = move_to {
                *value = options[j].0;
                changed = true;
                ui.ctx().memory_mut(|m| m.request_focus(ids[j]));
            }
        });
        changed
    }

    /// Real progress: `Some(fraction)` from backend counters, or `None` for indeterminate work.
    pub fn progress(&self, ui: &mut Ui, fraction: Option<f32>, label: &str) -> Response {
        let th = self.theme;
        let width = ui.available_width();
        let (rect, resp) =
            ui.allocate_exact_size(vec2(width, th.control.progress_height), Sense::hover());
        resp.widget_info(|| WidgetInfo::labeled(accesskit::Role::ProgressIndicator, true, label));
        if let Some(f) = fraction {
            ui.ctx().accesskit_node_builder(resp.id, |n| {
                n.set_min_numeric_value(0.0);
                n.set_max_numeric_value(100.0);
                n.set_numeric_value(f64::from((f * 100.0).round()));
            });
        }
        let painter = ui.painter();
        let radius = rect.height() / 2.0;
        paint_gel(painter, th, rect, Gel::new(Surface::Sunken, radius));
        let inner = rect.shrink(th.shape.outline_control);
        match fraction {
            Some(f) => {
                let w = (inner.width() * f.clamp(0.0, 1.0)).max(if f > 0.0 {
                    inner.height()
                } else {
                    0.0
                });
                if w > 0.0 {
                    let fill = Rect::from_min_size(inner.min, vec2(w, inner.height()));
                    painter.rect_filled(fill, inner.height() / 2.0, th.c.accent);
                }
            }
            None if self.motion.ambient() => {
                let period = th.motion.progress_sweep as f64;
                let t = ((ui.input(|i| i.time) % period) / period) as f32;
                let seg = inner.width() * 0.3;
                let x = inner.left() - seg + (inner.width() + seg) * ease_standard(t);
                let fill = Rect::from_min_max(
                    pos2(x.max(inner.left()), inner.top()),
                    pos2((x + seg).min(inner.right()), inner.bottom()),
                );
                if fill.width() > 0.0 {
                    painter.rect_filled(fill, inner.height() / 2.0, th.c.accent);
                }
                ui.ctx().request_repaint();
            }
            None => {
                // Static "working" pattern: evenly spaced dots, no implied amount.
                let r = inner.height() * 0.22;
                let mut x = inner.left() + inner.height();
                while x < inner.right() - inner.height() / 2.0 {
                    painter.circle_filled(pos2(x, inner.center().y), r, th.c.text_secondary);
                    x += inner.height() * 1.5;
                }
            }
        }
        resp
    }

    /// Small working indicator; static when motion is reduced or off.
    pub fn spinner(&self, ui: &mut Ui, label: &str) -> Response {
        let th = self.theme;
        let size = th.control.icon_size * 1.5;
        let (rect, resp) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());
        resp.widget_info(|| WidgetInfo::labeled(accesskit::Role::ProgressIndicator, true, label));
        let painter = ui.painter();
        let c = rect.center();
        let r = size * 0.4;
        let stroke = Stroke::new(th.shape.outline_panel, th.c.accent);
        painter.circle_stroke(c, r, Stroke::new(th.shape.outline_panel, th.c.outline_soft));
        let start = if self.motion.ambient() {
            ui.ctx().request_repaint();
            (ui.input(|i| i.time) / th.motion.spinner_period as f64 * std::f64::consts::TAU) as f32
        } else {
            -std::f32::consts::FRAC_PI_2
        };
        let points: Vec<_> = (0..=16)
            .map(|i| {
                let a = start + std::f32::consts::FRAC_PI_2 * 1.5 * i as f32 / 16.0;
                pos2(c.x + r * a.cos(), c.y + r * a.sin())
            })
            .collect();
        painter.add(Shape::line(points, stroke));
        resp
    }

    pub fn icon(&self, ui: &mut Ui, icon: Icon, color: Color32) {
        let th = self.theme;
        let (rect, _) = ui.allocate_exact_size(Vec2::splat(th.control.icon_size), Sense::hover());
        let p = ui.painter();
        let w = th.shape.outline_panel;
        let s = Stroke::new(w, color);
        let c = rect.center();
        let r = rect.width() / 2.0 - w / 2.0;
        match icon {
            Icon::Check => {
                p.circle_filled(c, r + w / 2.0, color);
                let k = Stroke::new(w, th.c.text_on_accent);
                p.add(Shape::line(
                    vec![
                        pos2(c.x - r * 0.45, c.y),
                        pos2(c.x - r * 0.1, c.y + r * 0.38),
                        pos2(c.x + r * 0.5, c.y - r * 0.38),
                    ],
                    k,
                ));
            }
            Icon::Info => {
                p.circle_stroke(c, r, s);
                p.line_segment([pos2(c.x, c.y - r * 0.05), pos2(c.x, c.y + r * 0.5)], s);
                p.circle_filled(pos2(c.x, c.y - r * 0.42), w * 0.6, color);
            }
            Icon::Warning => {
                let pts = vec![
                    pos2(c.x, rect.top() + w),
                    pos2(rect.right() - w / 2.0, rect.bottom() - w),
                    pos2(rect.left() + w / 2.0, rect.bottom() - w),
                ];
                p.add(Shape::closed_line(pts, s));
                p.line_segment([pos2(c.x, c.y - r * 0.25), pos2(c.x, c.y + r * 0.3)], s);
                p.circle_filled(pos2(c.x, c.y + r * 0.62), w * 0.6, color);
            }
            Icon::Error => {
                p.circle_stroke(c, r, s);
                let d = r * 0.42;
                p.line_segment([pos2(c.x - d, c.y - d), pos2(c.x + d, c.y + d)], s);
                p.line_segment([pos2(c.x + d, c.y - d), pos2(c.x - d, c.y + d)], s);
            }
        }
    }

    /// A gel card that wraps its contents; returns the closure's result.
    pub fn card<R>(&self, ui: &mut Ui, surface: Surface, add: impl FnOnce(&mut Ui) -> R) -> R {
        let th = self.theme;
        let slot = ui.painter().add(Shape::Noop);
        let pad = th.space.xl;
        let outer = ui.available_rect_before_wrap();
        let inner_rect = outer.shrink(pad);
        let mut child = ui.new_child(
            UiBuilder::new()
                .max_rect(inner_rect)
                .layout(Layout::top_down(Align::Min)),
        );
        let r = add(&mut child);
        let used = child.min_rect();
        let rect = Rect::from_min_max(outer.min, pos2(outer.right(), used.bottom() + pad));
        let radius = if surface == Surface::Panel {
            th.shape.radius_xl
        } else {
            th.shape.radius_lg
        };
        ui.painter().set(
            slot,
            Shape::Vec(gel_shapes(
                th,
                rect,
                Gel::new(surface, radius),
                ui.pixels_per_point(),
            )),
        );
        ui.allocate_rect(rect, Sense::hover());
        r
    }

    /// A tone-coloured notice. Returns the index of the action pressed, if any.
    pub fn banner(
        &self,
        ui: &mut Ui,
        tone: Tone,
        title: &str,
        body: &str,
        actions: &[(&str, ButtonKind)],
    ) -> Option<usize> {
        let th = self.theme;
        let (bg, fg, icon) = match tone {
            Tone::Info => (th.c.info_soft, th.c.info, Icon::Info),
            Tone::Success => (th.c.success_soft, th.c.success, Icon::Check),
            Tone::Warning => (th.c.warning_soft, th.c.warning, Icon::Warning),
            Tone::Danger => (th.c.danger_soft, th.c.danger, Icon::Error),
        };
        let mut pressed = None;
        egui::Frame::new()
            .fill(bg)
            .stroke(Stroke::new(th.shape.outline_control, fg))
            .corner_radius(th.shape.radius_md)
            .inner_margin(th.space.lg)
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal_top(|ui| {
                    self.icon(ui, icon, fg);
                    ui.add_space(th.space.sm);
                    ui.vertical(|ui| {
                        self.text(ui, th.text.label, th.c.text_primary, title);
                        if !body.is_empty() {
                            ui.add_space(th.space.xxs);
                            self.text(ui, th.text.body_small, th.c.text_primary, body);
                        }
                        if !actions.is_empty() {
                            ui.add_space(th.space.sm);
                            ui.horizontal_wrapped(|ui| {
                                for (i, (label, kind)) in actions.iter().enumerate() {
                                    if self.button(ui, label, *kind, true).clicked() {
                                        pressed = Some(i);
                                    }
                                }
                            });
                        }
                    });
                });
            });
        pressed
    }

    /// A labelled single-line text field. The label is linked for screen readers.
    pub fn text_field(&self, ui: &mut Ui, label: &str, value: &mut String, hint: &str) -> Response {
        let th = self.theme;
        let label_resp = self.text(ui, th.text.label, th.c.text_primary, label);
        ui.add_space(th.space.xs);
        let resp = ui
            .add(
                egui::TextEdit::singleline(value)
                    .font(font(th.text.body))
                    .hint_text(hint)
                    .desired_width(ui.available_width())
                    .margin(vec2(th.space.md, th.space.sm)),
            )
            .labelled_by(label_resp.id);
        if resp.has_focus() && self.keyboard {
            paint_focus_ring(ui.painter(), th, resp.rect, th.shape.radius_sm);
        }
        resp
    }
}
