//! `ooze.theme/1`: a player's interface appearance, as portable, data-only JSON.
//!
//! A theme document never carries code, URLs, paths or fonts: only a base theme id and bounded
//! token choices. Validation is strict (unknown fields are rejected) because the library persists
//! and shares these documents. The same validator serves the launcher interface and Sol's library,
//! so a theme accepted in one is accepted in the other.
//!
//! Colours come from the canonical token file (`docs/tokens/ooze_classic.theme.json`); nothing
//! here hard-codes a colour value.

use crate::json::{self, Json};

pub const SCHEMA: &str = "ooze.theme/1";
/// The only base theme in v1.
pub const BASE_CLASSIC: &str = "ooze_classic";

const CLASSIC_TOKENS: &str = include_str!("../../../../docs/tokens/ooze_classic.theme.json");

/// Minimum contrast for text on the accent (WCAG 2.x AA, normal text).
pub const MIN_TEXT_CONTRAST: f32 = 4.5;
/// Minimum contrast between the accent and the canvas (WCAG 2.x non-text contrast).
pub const MIN_UI_CONTRAST: f32 = 3.0;

/// Palette entries offered as accent swatches, in display order. Values come from the token file.
pub const ACCENT_SWATCHES: [&str; 5] = ["ink", "signalBlue", "mint", "amber", "coral"];

pub const TEXT_SCALES: [u16; 5] = [90, 100, 110, 120, 130];
pub const UI_SCALES: [u16; 5] = [100, 125, 150, 175, 200];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Rgb(pub u8, pub u8, pub u8);

impl Rgb {
    pub fn hex(self) -> String {
        format!("#{:02X}{:02X}{:02X}", self.0, self.1, self.2)
    }

    /// `#RRGGBB` only (no alpha: an accent is always opaque).
    pub fn parse_hex(s: &str) -> Option<Self> {
        let h = s.trim().strip_prefix('#')?;
        if h.len() != 6 || !h.bytes().all(|b| b.is_ascii_hexdigit()) {
            return None;
        }
        let v = u32::from_str_radix(h, 16).ok()?;
        Some(Self((v >> 16) as u8, (v >> 8) as u8, v as u8))
    }

    fn luminance(self) -> f32 {
        let lin = |c: u8| {
            let c = f32::from(c) / 255.0;
            if c <= 0.040_45 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * lin(self.0) + 0.7152 * lin(self.1) + 0.0722 * lin(self.2)
    }
}

/// WCAG 2.x contrast ratio.
pub fn contrast(a: Rgb, b: Rgb) -> f32 {
    let (la, lb) = (a.luminance(), b.luminance());
    let (hi, lo) = if la > lb { (la, lb) } else { (lb, la) };
    (hi + 0.05) / (lo + 0.05)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Contrast {
    Standard,
    High,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Background {
    Drips,
    Gradient,
    Plain,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Motion {
    System,
    Full,
    Reduced,
    Off,
}

macro_rules! str_enum {
    ($t:ty { $($v:ident => $s:literal),+ $(,)? }) => {
        impl $t {
            pub const ALL: &'static [Self] = &[$(Self::$v),+];
            pub fn as_str(self) -> &'static str {
                match self { $(Self::$v => $s),+ }
            }
            pub fn parse(s: &str) -> Option<Self> {
                match s { $($s => Some(Self::$v),)+ _ => None }
            }
        }
    };
}
str_enum!(Contrast { Standard => "standard", High => "high" });
str_enum!(Background { Drips => "drips", Gradient => "gradient", Plain => "plain" });
str_enum!(Motion { System => "system", Full => "full", Reduced => "reduced", Off => "off" });

/// A complete appearance choice. `accent: None` keeps the base theme's accent (Ink Gel).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ThemeDoc {
    pub accent: Option<Rgb>,
    pub contrast: Contrast,
    pub text_scale: u16,
    pub background: Background,
    /// Motion and interface scale are also launcher settings (API 1); the document carries them so
    /// an exported theme restores the whole appearance.
    pub motion: Motion,
    pub ui_scale: u16,
}

impl Default for ThemeDoc {
    fn default() -> Self {
        Self {
            accent: None,
            contrast: Contrast::Standard,
            text_scale: 100,
            background: Background::Drips,
            motion: Motion::System,
            ui_scale: 100,
        }
    }
}

/// Which token colour sits on the accent: the base's on-accent paper or its ink.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OnAccent {
    Paper,
    Ink,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ThemeError {
    NotJson(json::JsonError),
    NotAnObject,
    WrongSchema,
    UnknownBase,
    UnknownField(String),
    BadValue(&'static str),
    Accent(AccentIssue),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AccentIssue {
    /// Neither paper nor ink text reaches 4.5:1 on this accent; the best ratio found.
    TextContrast(f32),
    /// The accent is too close to the canvas to see as a control (needs 3:1).
    CanvasContrast(f32),
}

/// The base theme colours the guard needs, read from the canonical token file.
#[derive(Clone, Debug)]
pub struct BaseColors {
    pub canvas: Rgb,
    pub ink: Rgb,
    pub paper: Rgb,
    pub accent: Rgb,
    pub focus: Rgb,
    pub swatches: Vec<(&'static str, Rgb)>,
}

impl BaseColors {
    /// Ooze Classic. The token file is compiled in and covered by tests, so this cannot fail in
    /// a tested build; a broken file falls back to `None` rather than panicking.
    pub fn classic() -> Option<Self> {
        let doc = json::parse(CLASSIC_TOKENS).ok()?;
        let palette = doc.get("palette")?;
        let colors = doc.get("color")?;
        let resolve = |v: &str| -> Option<Rgb> {
            let v = v.split('@').next()?;
            match v
                .strip_prefix("{palette.")
                .and_then(|r| r.strip_suffix('}'))
            {
                Some(name) => Rgb::parse_hex(palette.get(name)?.as_str()?),
                None => Rgb::parse_hex(v),
            }
        };
        let token = |k: &str| resolve(colors.get(k)?.as_str()?);
        let swatches = ACCENT_SWATCHES
            .iter()
            .map(|name| Some((*name, Rgb::parse_hex(palette.get(name)?.as_str()?)?)))
            .collect::<Option<Vec<_>>>()?;
        Some(Self {
            canvas: token("canvas")?,
            ink: token("text.primary")?,
            paper: token("text.onAccent")?,
            accent: token("accent.base")?,
            focus: token("outline.focus")?,
            swatches,
        })
    }

    /// The contrast guard. Returns which text colour to put on the accent.
    pub fn check_accent(&self, accent: Rgb) -> Result<OnAccent, AccentIssue> {
        let paper = contrast(accent, self.paper);
        let ink = contrast(accent, self.ink);
        let (on, best) = if paper >= ink {
            (OnAccent::Paper, paper)
        } else {
            (OnAccent::Ink, ink)
        };
        if best < MIN_TEXT_CONTRAST {
            return Err(AccentIssue::TextContrast(best));
        }
        let canvas = contrast(accent, self.canvas);
        if canvas < MIN_UI_CONTRAST {
            return Err(AccentIssue::CanvasContrast(canvas));
        }
        Ok(on)
    }
}

impl ThemeDoc {
    pub fn to_json(&self) -> String {
        Json::Object(vec![
            ("schema".into(), Json::String(SCHEMA.into())),
            ("base".into(), Json::String(BASE_CLASSIC.into())),
            (
                "accent".into(),
                self.accent.map_or(Json::Null, |c| Json::String(c.hex())),
            ),
            (
                "contrast".into(),
                Json::String(self.contrast.as_str().into()),
            ),
            ("text_scale".into(), Json::Number(self.text_scale.into())),
            (
                "background".into(),
                Json::String(self.background.as_str().into()),
            ),
            ("motion".into(), Json::String(self.motion.as_str().into())),
            ("ui_scale".into(), Json::Number(self.ui_scale.into())),
        ])
        .to_pretty()
    }

    /// Strict import: every field is checked, unknown fields are refused, and the accent must pass
    /// the contrast guard against `base`. Missing optional fields take their defaults.
    pub fn from_json(text: &str, base: &BaseColors) -> Result<Self, ThemeError> {
        let doc = json::parse(text).map_err(ThemeError::NotJson)?;
        let fields = doc.as_object().ok_or(ThemeError::NotAnObject)?;
        const KNOWN: [&str; 8] = [
            "schema",
            "base",
            "accent",
            "contrast",
            "text_scale",
            "background",
            "motion",
            "ui_scale",
        ];
        if let Some((k, _)) = fields.iter().find(|(k, _)| !KNOWN.contains(&k.as_str())) {
            return Err(ThemeError::UnknownField(k.clone()));
        }
        if doc.get("schema").and_then(Json::as_str) != Some(SCHEMA) {
            return Err(ThemeError::WrongSchema);
        }
        if doc.get("base").and_then(Json::as_str) != Some(BASE_CLASSIC) {
            return Err(ThemeError::UnknownBase);
        }
        let mut out = Self::default();
        match doc.get("accent") {
            None | Some(Json::Null) => {}
            Some(v) => {
                let c = v
                    .as_str()
                    .and_then(Rgb::parse_hex)
                    .ok_or(ThemeError::BadValue("accent"))?;
                base.check_accent(c).map_err(ThemeError::Accent)?;
                out.accent = Some(c);
            }
        }
        let text = |k: &'static str| -> Result<Option<&str>, ThemeError> {
            match doc.get(k) {
                None => Ok(None),
                Some(v) => v.as_str().map(Some).ok_or(ThemeError::BadValue(k)),
            }
        };
        let number = |k: &'static str, allowed: &[u16]| -> Result<Option<u16>, ThemeError> {
            match doc.get(k) {
                None => Ok(None),
                Some(v) => v
                    .as_f64()
                    .filter(|n| n.fract() == 0.0 && *n >= 0.0 && *n <= f64::from(u16::MAX))
                    .map(|n| n as u16)
                    .filter(|n| allowed.contains(n))
                    .map(Some)
                    .ok_or(ThemeError::BadValue(k)),
            }
        };
        if let Some(s) = text("contrast")? {
            out.contrast = Contrast::parse(s).ok_or(ThemeError::BadValue("contrast"))?;
        }
        if let Some(s) = text("background")? {
            out.background = Background::parse(s).ok_or(ThemeError::BadValue("background"))?;
        }
        if let Some(s) = text("motion")? {
            out.motion = Motion::parse(s).ok_or(ThemeError::BadValue("motion"))?;
        }
        if let Some(n) = number("text_scale", &TEXT_SCALES)? {
            out.text_scale = n;
        }
        if let Some(n) = number("ui_scale", &UI_SCALES)? {
            out.ui_scale = n;
        }
        Ok(out)
    }

    /// Re-check a document built in memory (for example by the interface) before persisting.
    pub fn validate(&self, base: &BaseColors) -> Result<(), ThemeError> {
        if let Some(c) = self.accent {
            base.check_accent(c).map_err(ThemeError::Accent)?;
        }
        if !TEXT_SCALES.contains(&self.text_scale) {
            return Err(ThemeError::BadValue("text_scale"));
        }
        if !UI_SCALES.contains(&self.ui_scale) {
            return Err(ThemeError::BadValue("ui_scale"));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> BaseColors {
        BaseColors::classic().unwrap()
    }

    #[test]
    fn classic_tokens_resolve_and_every_swatch_passes_the_guard() {
        let b = base();
        assert_eq!(b.ink, b.accent, "Ink Gel: the default accent is the ink");
        assert_eq!(b.swatches.len(), ACCENT_SWATCHES.len());
        for (name, c) in &b.swatches {
            assert!(b.check_accent(*c).is_ok(), "{name} {}", c.hex());
        }
        assert!(contrast(b.focus, b.canvas) >= MIN_UI_CONTRAST);
    }

    #[test]
    fn guard_rejects_low_contrast_accents_with_the_reason() {
        let b = base();
        assert!(matches!(
            b.check_accent(Rgb::parse_hex("#F2B33D").unwrap()),
            Err(AccentIssue::CanvasContrast(_))
        ));
        assert!(matches!(
            b.check_accent(Rgb::parse_hex("#808080").unwrap()),
            Err(AccentIssue::TextContrast(_)) | Err(AccentIssue::CanvasContrast(_))
        ));
        assert!(matches!(
            b.check_accent(b.canvas),
            Err(AccentIssue::CanvasContrast(_))
        ));
        assert_eq!(b.check_accent(b.ink), Ok(OnAccent::Paper));
    }

    #[test]
    fn round_trip_and_strict_import() {
        let b = base();
        let doc = ThemeDoc {
            accent: Some(b.swatches[1].1),
            contrast: Contrast::High,
            text_scale: 120,
            background: Background::Plain,
            motion: Motion::Reduced,
            ui_scale: 150,
        };
        assert_eq!(ThemeDoc::from_json(&doc.to_json(), &b), Ok(doc.clone()));
        assert_eq!(
            ThemeDoc::from_json(r#"{"schema":"ooze.theme/1","base":"ooze_classic"}"#, &b),
            Ok(ThemeDoc::default())
        );
        for (bad, why) in [
            ("[]", ThemeError::NotAnObject),
            (
                r#"{"schema":"ooze.theme/2","base":"ooze_classic"}"#,
                ThemeError::WrongSchema,
            ),
            (
                r#"{"schema":"ooze.theme/1","base":"other"}"#,
                ThemeError::UnknownBase,
            ),
            (
                r#"{"schema":"ooze.theme/1","base":"ooze_classic","font":"x.ttf"}"#,
                ThemeError::UnknownField("font".into()),
            ),
            (
                r#"{"schema":"ooze.theme/1","base":"ooze_classic","background":"https://x/y.png"}"#,
                ThemeError::BadValue("background"),
            ),
            (
                r#"{"schema":"ooze.theme/1","base":"ooze_classic","text_scale":115}"#,
                ThemeError::BadValue("text_scale"),
            ),
            (
                r##"{"schema":"ooze.theme/1","base":"ooze_classic","accent":"#FFF"}"##,
                ThemeError::BadValue("accent"),
            ),
        ] {
            assert_eq!(ThemeDoc::from_json(bad, &b), Err(why), "{bad}");
        }
        assert!(matches!(
            ThemeDoc::from_json(
                r##"{"schema":"ooze.theme/1","base":"ooze_classic","accent":"#F2B33D"}"##,
                &b
            ),
            Err(ThemeError::Accent(_))
        ));
        let bad = ThemeDoc {
            ui_scale: 110,
            ..ThemeDoc::default()
        };
        assert_eq!(bad.validate(&b), Err(ThemeError::BadValue("ui_scale")));
    }
}
