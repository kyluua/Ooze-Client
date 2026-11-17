//! Player appearance: applies an `ooze.theme/1` document ([`ThemeDoc`]) to the design tokens.
//!
//! Customisation changes tokens only, never component code: the accent (with its derived gel
//! top and on-accent text), contrast, text size and background style. The contrast guard is
//! `ooze_ui::theme_doc` (shared with Sol's library), so the interface cannot preview a theme the
//! library would refuse to persist.

use eframe::egui::Color32;
use ooze_ui::theme_doc::{
    AccentIssue, Background, BaseColors, Contrast, OnAccent, Rgb, ThemeDoc, ThemeError,
};

use crate::api::MotionPreference;
use crate::i18n::Strings;
use crate::theme::{Theme, oklab_shift};

/// OKLab lightness lift for the top of an Ink Gel body (DESIGN_SYSTEM §3; same as the base theme).
const ACCENT_TOP_LIFT: f32 = 0.08;

pub fn rgb(c: Rgb) -> Color32 {
    Color32::from_rgb(c.0, c.1, c.2)
}

pub fn to_rgb(c: Color32) -> Rgb {
    let [r, g, b, _] = c.to_srgba_unmultiplied();
    Rgb(r, g, b)
}

/// The tokens for `doc` over `base`. An accent that fails the guard is ignored (the document
/// validators already refuse it; this keeps a corrupt value from reaching the screen).
pub fn apply(base: &Theme, doc: &ThemeDoc, guard: &BaseColors) -> Theme {
    let mut t = base.clone();
    if let Some(accent) = doc.accent
        && let Ok(on) = guard.check_accent(accent)
    {
        let a = rgb(accent);
        t.c.accent = a;
        t.c.accent_top = oklab_shift(a, ACCENT_TOP_LIFT);
        t.c.text_on_accent = match on {
            OnAccent::Paper => base.c.text_on_accent,
            OnAccent::Ink => base.c.text_primary,
        };
    }
    if doc.contrast == Contrast::High {
        t.c.outline_soft = base.c.outline_strong;
        t.c.text_secondary = base.c.text_primary;
        t.c.text_disabled = base.c.text_secondary;
    }
    let k = f32::from(doc.text_scale) / 100.0;
    for role in [
        &mut t.text.display,
        &mut t.text.title,
        &mut t.text.subtitle,
        &mut t.text.section,
        &mut t.text.body,
        &mut t.text.body_small,
        &mut t.text.caption,
        &mut t.text.label,
        &mut t.text.button,
        &mut t.text.log,
    ] {
        role.size = (role.size * k).round();
    }
    if k > 1.0 {
        // Larger text keeps its padding: controls grow with it so labels never clip.
        t.control.height = (base.control.height * k).round();
        t.control.height_large = (base.control.height_large * k).round();
    }
    t
}

pub fn motion_pref(m: ooze_ui::theme_doc::Motion) -> MotionPreference {
    use ooze_ui::theme_doc::Motion as M;
    match m {
        M::System => MotionPreference::FollowSystem,
        M::Full => MotionPreference::Full,
        M::Reduced => MotionPreference::Reduced,
        M::Off => MotionPreference::Off,
    }
}

pub fn motion_doc(m: MotionPreference) -> ooze_ui::theme_doc::Motion {
    use ooze_ui::theme_doc::Motion as M;
    match m {
        MotionPreference::FollowSystem => M::System,
        MotionPreference::Full => M::Full,
        MotionPreference::Reduced => M::Reduced,
        MotionPreference::Off => M::Off,
    }
}

pub fn draws_drips(doc: &ThemeDoc) -> bool {
    doc.background == Background::Drips
}

pub fn draws_gradient(doc: &ThemeDoc) -> bool {
    doc.background != Background::Plain
}

/// A localised, specific reason for a refused accent.
pub fn accent_issue_text(tr: &Strings, issue: AccentIssue) -> String {
    match issue {
        AccentIssue::TextContrast(r) => tr.fmt("appearance.accent.low_text", &[&format!("{r:.1}")]),
        AccentIssue::CanvasContrast(r) => {
            tr.fmt("appearance.accent.low_canvas", &[&format!("{r:.1}")])
        }
    }
}

/// A localised reason for a refused theme import.
pub fn import_error_text(tr: &Strings, e: &ThemeError) -> String {
    match e {
        ThemeError::NotJson(_) | ThemeError::NotAnObject => {
            tr.get("appearance.import.not_json").to_owned()
        }
        ThemeError::WrongSchema => tr.get("appearance.import.wrong_schema").to_owned(),
        ThemeError::UnknownBase => tr.get("appearance.import.unknown_base").to_owned(),
        ThemeError::UnknownField(f) => tr.fmt("appearance.import.unknown_field", &[f]),
        ThemeError::BadValue(f) => tr.fmt("appearance.import.bad_value", &[f]),
        ThemeError::Accent(issue) => accent_issue_text(tr, *issue),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::contrast;

    #[test]
    fn accent_and_contrast_change_tokens_and_keep_text_readable() {
        let base = Theme::embedded();
        let guard = BaseColors::classic().unwrap();
        for (_, swatch) in &guard.swatches {
            let doc = ThemeDoc {
                accent: Some(*swatch),
                ..ThemeDoc::default()
            };
            let t = apply(&base, &doc, &guard);
            assert_eq!(t.c.accent, rgb(*swatch));
            assert!(contrast(t.c.accent, t.c.text_on_accent) >= 4.5);
            assert!(contrast(t.c.accent, t.c.canvas) >= 3.0);
        }
        let high = apply(
            &base,
            &ThemeDoc {
                contrast: Contrast::High,
                text_scale: 130,
                ..ThemeDoc::default()
            },
            &guard,
        );
        assert_eq!(high.c.outline_soft, base.c.outline_strong);
        assert_eq!(high.c.text_secondary, base.c.text_primary);
        assert!(high.text.body.size > base.text.body.size);
        assert!(high.control.height > base.control.height);
    }

    #[test]
    fn a_refused_accent_never_reaches_the_tokens() {
        let base = Theme::embedded();
        let guard = BaseColors::classic().unwrap();
        let doc = ThemeDoc {
            accent: Some(guard.canvas),
            ..ThemeDoc::default()
        };
        assert_eq!(apply(&base, &doc, &guard).c.accent, base.c.accent);
    }

    #[test]
    fn the_guard_agrees_with_the_launchers_own_contrast_and_tokens() {
        let base = Theme::embedded();
        let guard = BaseColors::classic().unwrap();
        assert_eq!(rgb(guard.canvas), base.c.canvas);
        assert_eq!(rgb(guard.accent), base.c.accent);
        let r1 = contrast(base.c.accent, base.c.canvas);
        let r2 = ooze_ui::theme_doc::contrast(guard.accent, guard.canvas);
        assert!((r1 - r2).abs() < 0.01, "{r1} vs {r2}");
    }
}
