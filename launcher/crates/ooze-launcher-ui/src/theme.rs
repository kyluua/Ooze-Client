//! Design tokens for the launcher. Colours, radii, outlines, elevation and material come from the
//! canonical Ooze theme (`docs/tokens/ooze_classic.theme.json`, shared with the in-game UI);
//! layout, type roles, control metrics and motion come from `launcher/resources/tokens/`.
//! Components read only this struct: no literal colours, sizes or durations elsewhere.

use eframe::egui::Color32;
use serde_json::Value;

const THEME_JSON: &str = include_str!("../../../../docs/tokens/ooze_classic.theme.json");
const LAUNCHER_TOKENS_JSON: &str = include_str!("../../../resources/tokens/launcher.tokens.json");

#[derive(Clone, Debug)]
pub struct Colors {
    pub canvas: Color32,
    pub surface_base: Color32,
    pub surface_raised: Color32,
    pub surface_sunken: Color32,
    pub gradient_top: Color32,
    pub gradient_bottom: Color32,
    pub scrim: Color32,
    pub outline_strong: Color32,
    pub outline_soft: Color32,
    pub focus: Color32,
    pub focus_halo: Color32,
    pub text_primary: Color32,
    pub text_secondary: Color32,
    pub text_disabled: Color32,
    pub text_on_accent: Color32,
    pub text_link: Color32,
    pub accent: Color32,
    /// Top of the Ink Gel body gradient: accent at OKLab L + 0.08 (DESIGN_SYSTEM §3).
    pub accent_top: Color32,
    pub accent_soft: Color32,
    pub gloss: Color32,
    pub cool: Color32,
    pub selection: Color32,
    pub shadow: Color32,
    pub success: Color32,
    pub success_soft: Color32,
    pub warning: Color32,
    pub warning_soft: Color32,
    pub danger: Color32,
    pub danger_soft: Color32,
    pub info: Color32,
    pub info_soft: Color32,
}

#[derive(Clone, Copy, Debug)]
pub struct Shape {
    pub radius_xs: f32,
    pub radius_sm: f32,
    pub radius_md: f32,
    pub radius_lg: f32,
    pub radius_xl: f32,
    pub corner_exponent: f32,
    pub outline_hairline: f32,
    pub outline_control: f32,
    pub outline_panel: f32,
    pub outline_heavy: f32,
    /// (min, max) drip neck width, length and bulb radius (DESIGN_SYSTEM §4.5).
    pub drip_neck: [f32; 2],
    pub drip_length: [f32; 2],
    pub drip_bulb: [f32; 2],
}

#[derive(Clone, Copy, Debug)]
pub struct Material {
    pub gloss_opacity: f32,
    pub gloss_height_ratio: f32,
    pub gloss_start: f32,
    pub gloss_end: f32,
    pub glint_opacity: f32,
    pub rim_opacity: f32,
    pub ink_gloss_opacity: f32,
    pub ink_glint_opacity: f32,
}

/// (y offset, blur, alpha) for the ambient and contact shadows.
#[derive(Clone, Copy, Debug, Default)]
pub struct Elevation {
    pub ambient: [f32; 3],
    pub contact: [f32; 3],
}

#[derive(Clone, Copy, Debug)]
pub struct Space {
    pub xxs: f32,
    pub xs: f32,
    pub sm: f32,
    pub md: f32,
    pub lg: f32,
    pub xl: f32,
    pub xxl: f32,
    pub xxxl: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Family {
    Display,
    Strong,
    Body,
}

#[derive(Clone, Copy, Debug)]
pub struct TypeRole {
    pub family: Family,
    pub size: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct TypeRoles {
    pub display: TypeRole,
    pub title: TypeRole,
    pub subtitle: TypeRole,
    pub section: TypeRole,
    pub body: TypeRole,
    pub body_small: TypeRole,
    pub caption: TypeRole,
    pub label: TypeRole,
    pub button: TypeRole,
    pub log: TypeRole,
}

#[derive(Clone, Copy, Debug)]
pub struct Control {
    pub height: f32,
    pub height_large: f32,
    pub min_width: f32,
    pub min_hit: f32,
    pub pad_x: f32,
    pub focus_ring_width: f32,
    pub focus_ring_gap: f32,
    pub progress_height: f32,
    pub nav_width: f32,
    pub content_max_width: f32,
    pub window_min: [f32; 2],
    pub window_default: [f32; 2],
    pub icon_size: f32,
    /// Horizontal positions (fractions of the width) of the play card's drips.
    pub drip_positions: [f32; 2],
}

/// Durations in seconds at speed 1.0.
#[derive(Clone, Copy, Debug)]
pub struct MotionTokens {
    pub micro: f32,
    pub short: f32,
    pub medium: f32,
    pub long: f32,
    pub reduced_fade: f32,
    pub spinner_period: f32,
    pub progress_sweep: f32,
    pub drip_period: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub log_lines: usize,
    pub memory_step_mib: u32,
}

#[derive(Clone, Debug)]
pub struct Theme {
    pub c: Colors,
    pub shape: Shape,
    pub material: Material,
    pub elevation: [Elevation; 5],
    pub space: Space,
    pub text: TypeRoles,
    pub control: Control,
    pub motion: MotionTokens,
    pub limits: Limits,
}

#[derive(Debug)]
pub struct TokenError(pub String);

impl std::fmt::Display for TokenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "design token error: {}", self.0)
    }
}

impl Theme {
    /// The embedded default theme. Both token files are compiled in and covered by the
    /// `embedded_tokens_parse` test, so this cannot fail in a tested build.
    pub fn embedded() -> Self {
        Self::parse(THEME_JSON, LAUNCHER_TOKENS_JSON)
            .expect("embedded design tokens are valid (tested)")
    }

    pub fn parse(theme_json: &str, launcher_json: &str) -> Result<Self, TokenError> {
        let theme: Value =
            serde_json::from_str(theme_json).map_err(|e| TokenError(e.to_string()))?;
        let tokens: Value =
            serde_json::from_str(launcher_json).map_err(|e| TokenError(e.to_string()))?;
        let r = Resolver { theme: &theme };
        let color = |k: &str| r.color(k);
        let accent = color("accent.base")?;
        let c = Colors {
            canvas: color("canvas")?,
            surface_base: color("surface.base")?,
            surface_raised: color("surface.raised")?,
            surface_sunken: color("surface.sunken")?,
            gradient_top: color("surface.gradientTop")?,
            gradient_bottom: color("surface.gradientBottom")?,
            scrim: color("scrim")?,
            outline_strong: color("outline.strong")?,
            outline_soft: color("outline.soft")?,
            focus: color("outline.focus")?,
            focus_halo: color("outline.focusHalo")?,
            text_primary: color("text.primary")?,
            text_secondary: color("text.secondary")?,
            text_disabled: color("text.disabled")?,
            text_on_accent: color("text.onAccent")?,
            text_link: color("text.link")?,
            accent,
            accent_top: oklab_shift(accent, 0.08),
            accent_soft: color("accent.soft")?,
            gloss: color("highlight.gloss")?,
            cool: color("highlight.cool")?,
            selection: color("selection.text")?,
            shadow: color("shadow")?,
            success: color("status.success")?,
            success_soft: color("status.successSoft")?,
            warning: color("status.warning")?,
            warning_soft: color("status.warningSoft")?,
            danger: color("status.danger")?,
            danger_soft: color("status.dangerSoft")?,
            info: color("status.info")?,
            info_soft: color("status.infoSoft")?,
        };
        let num = |v: &Value, path: &str| -> Result<f32, TokenError> {
            v.pointer(path)
                .and_then(Value::as_f64)
                .map(|n| n as f32)
                .ok_or_else(|| TokenError(format!("missing number {path}")))
        };
        let shape = Shape {
            radius_xs: num(&theme, "/shape/radius.xs")?,
            radius_sm: num(&theme, "/shape/radius.sm")?,
            radius_md: num(&theme, "/shape/radius.md")?,
            radius_lg: num(&theme, "/shape/radius.lg")?,
            radius_xl: num(&theme, "/shape/radius.xl")?,
            corner_exponent: num(&theme, "/shape/cornerExponent")?,
            outline_hairline: num(&theme, "/shape/outline.hairline")?,
            outline_control: num(&theme, "/shape/outline.control")?,
            outline_panel: num(&theme, "/shape/outline.panel")?,
            outline_heavy: num(&theme, "/shape/outline.heavy")?,
            drip_neck: [
                num(&theme, "/shape/drip.neck.min")?,
                num(&theme, "/shape/drip.neck.max")?,
            ],
            drip_length: [
                num(&theme, "/shape/drip.length.min")?,
                num(&theme, "/shape/drip.length.max")?,
            ],
            drip_bulb: [
                num(&theme, "/shape/drip.bulb.min")?,
                num(&theme, "/shape/drip.bulb.max")?,
            ],
        };
        let material = Material {
            gloss_opacity: num(&theme, "/material/gloss.opacity")?,
            gloss_height_ratio: num(&theme, "/material/gloss.heightRatio")?,
            gloss_start: num(&theme, "/material/gloss.startRatio")?,
            gloss_end: num(&theme, "/material/gloss.endRatio")?,
            glint_opacity: num(&theme, "/material/glint.opacity")?,
            rim_opacity: num(&theme, "/material/rim.opacity")?,
            ink_gloss_opacity: num(&theme, "/material/inkGel.glossOpacity")?,
            ink_glint_opacity: num(&theme, "/material/inkGel.glintOpacity")?,
        };
        let mut elevation = [Elevation::default(); 5];
        for (i, e) in elevation.iter_mut().enumerate() {
            let triple = |part: &str| -> Result<[f32; 3], TokenError> {
                let base = format!("/elevation/e{i}/{part}");
                Ok([
                    num(&theme, &format!("{base}/0"))?,
                    num(&theme, &format!("{base}/1"))?,
                    num(&theme, &format!("{base}/2"))?,
                ])
            };
            e.ambient = triple("ambient")?;
            e.contact = triple("contact")?;
        }
        let t = &tokens;
        let space = Space {
            xxs: num(t, "/space/xxs")?,
            xs: num(t, "/space/xs")?,
            sm: num(t, "/space/sm")?,
            md: num(t, "/space/md")?,
            lg: num(t, "/space/lg")?,
            xl: num(t, "/space/xl")?,
            xxl: num(t, "/space/xxl")?,
            xxxl: num(t, "/space/xxxl")?,
        };
        let role = |name: &str| -> Result<TypeRole, TokenError> {
            let family = match t
                .pointer(&format!("/type/{name}/family"))
                .and_then(Value::as_str)
            {
                Some("display") => Family::Display,
                Some("strong") => Family::Strong,
                Some("body") => Family::Body,
                other => return Err(TokenError(format!("type {name}: bad family {other:?}"))),
            };
            Ok(TypeRole {
                family,
                size: num(t, &format!("/type/{name}/size"))?,
            })
        };
        let text = TypeRoles {
            display: role("display")?,
            title: role("title")?,
            subtitle: role("subtitle")?,
            section: role("section")?,
            body: role("body")?,
            body_small: role("bodySmall")?,
            caption: role("caption")?,
            label: role("label")?,
            button: role("button")?,
            log: role("log")?,
        };
        let control = Control {
            height: num(t, "/control/height")?,
            height_large: num(t, "/control/heightLarge")?,
            min_width: num(t, "/control/minWidth")?,
            min_hit: num(t, "/control/minHit")?,
            pad_x: num(t, "/control/padX")?,
            focus_ring_width: num(t, "/control/focusRingWidth")?,
            focus_ring_gap: num(t, "/control/focusRingGap")?,
            progress_height: num(t, "/control/progressHeight")?,
            nav_width: num(t, "/control/navWidth")?,
            content_max_width: num(t, "/control/contentMaxWidth")?,
            window_min: [
                num(t, "/control/windowMin/0")?,
                num(t, "/control/windowMin/1")?,
            ],
            window_default: [
                num(t, "/control/windowDefault/0")?,
                num(t, "/control/windowDefault/1")?,
            ],
            icon_size: num(t, "/control/iconSize")?,
            drip_positions: [
                num(t, "/control/dripPositions/0")?,
                num(t, "/control/dripPositions/1")?,
            ],
        };
        let ms =
            |k: &str| -> Result<f32, TokenError> { Ok(num(t, &format!("/motion/{k}"))? / 1000.0) };
        let motion = MotionTokens {
            micro: ms("dur.micro")?,
            short: ms("dur.short")?,
            medium: ms("dur.medium")?,
            long: ms("dur.long")?,
            reduced_fade: ms("reduced.fade")?,
            spinner_period: ms("spinner.period")?,
            progress_sweep: ms("progress.sweep")?,
            drip_period: ms("drip.period")?,
        };
        let limits = Limits {
            log_lines: num(t, "/limits/logLines")? as usize,
            memory_step_mib: num(t, "/limits/memoryStepMib")? as u32,
        };
        Ok(Self {
            c,
            shape,
            material,
            elevation,
            space,
            text,
            control,
            motion,
            limits,
        })
    }
}

struct Resolver<'a> {
    theme: &'a Value,
}

impl Resolver<'_> {
    fn color(&self, key: &str) -> Result<Color32, TokenError> {
        let raw = self
            .theme
            .pointer(&format!("/color/{}", key.replace('/', "~1")))
            .and_then(Value::as_str)
            .ok_or_else(|| TokenError(format!("missing color.{key}")))?;
        self.resolve(raw, 0)
    }

    fn resolve(&self, raw: &str, depth: u8) -> Result<Color32, TokenError> {
        if depth > 4 {
            return Err(TokenError(format!("reference loop at {raw}")));
        }
        let (base, alpha) = match raw.split_once('@') {
            Some((b, a)) => (
                b,
                a.parse::<f32>()
                    .map_err(|e| TokenError(format!("{raw}: {e}")))?,
            ),
            None => (raw, 1.0),
        };
        let color =
            if let Some(reference) = base.strip_prefix('{').and_then(|s| s.strip_suffix('}')) {
                let mut parts = reference.splitn(2, '.');
                let (group, name) = (
                    parts.next().unwrap_or_default(),
                    parts.next().unwrap_or_default(),
                );
                let target = self
                    .theme
                    .get(group)
                    .and_then(|g| g.get(name))
                    .and_then(Value::as_str)
                    .ok_or_else(|| TokenError(format!("unresolved {reference}")))?;
                self.resolve(target, depth + 1)?
            } else {
                parse_hex(base).ok_or_else(|| TokenError(format!("bad colour {base}")))?
            };
        Ok(with_alpha(color, alpha * (color.a() as f32 / 255.0)))
    }
}

fn parse_hex(s: &str) -> Option<Color32> {
    let h = s.strip_prefix('#')?;
    if h.len() != 6 {
        return None;
    }
    let v = u32::from_str_radix(h, 16).ok()?;
    Some(Color32::from_rgb((v >> 16) as u8, (v >> 8) as u8, v as u8))
}

/// Non-premultiplied colour with the given alpha (0–1).
pub fn with_alpha(c: Color32, alpha: f32) -> Color32 {
    let [r, g, b, _] = c.to_srgba_unmultiplied();
    Color32::from_rgba_unmultiplied(r, g, b, (alpha.clamp(0.0, 1.0) * 255.0).round() as u8)
}

fn srgb_to_linear(c: u8) -> f32 {
    let c = c as f32 / 255.0;
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn linear_to_srgb(c: f32) -> u8 {
    let c = c.clamp(0.0, 1.0);
    let s = if c <= 0.003_130_8 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    };
    (s * 255.0).round() as u8
}

/// Shift a colour's OKLab lightness by `dl` (Björn Ottosson's transform), keeping hue and chroma.
pub fn oklab_shift(c: Color32, dl: f32) -> Color32 {
    let [r8, g8, b8, a] = c.to_srgba_unmultiplied();
    let (r, g, b) = (srgb_to_linear(r8), srgb_to_linear(g8), srgb_to_linear(b8));
    let l = (0.412_221_46 * r + 0.536_332_55 * g + 0.051_445_995 * b).cbrt();
    let m = (0.211_903_5 * r + 0.680_699_5 * g + 0.107_396_96 * b).cbrt();
    let s = (0.088_302_46 * r + 0.281_718_85 * g + 0.629_978_7 * b).cbrt();
    let ll = 0.210_454_26 * l + 0.793_617_8 * m - 0.004_072_047 * s + dl;
    let aa = 1.977_998_5 * l - 2.428_592_2 * m + 0.450_593_7 * s;
    let bb = 0.025_904_037 * l + 0.782_771_77 * m - 0.808_675_77 * s;
    let l2 = (ll + 0.396_337_78 * aa + 0.215_803_76 * bb).powi(3);
    let m2 = (ll - 0.105_561_346 * aa - 0.063_854_17 * bb).powi(3);
    let s2 = (ll - 0.089_484_18 * aa - 1.291_485_5 * bb).powi(3);
    let r = 4.076_741_7 * l2 - 3.307_711_6 * m2 + 0.230_969_94 * s2;
    let g = -1.268_438 * l2 + 2.609_757_4 * m2 - 0.341_319_38 * s2;
    let b = -0.004_196_086_3 * l2 - 0.703_418_6 * m2 + 1.707_614_7 * s2;
    Color32::from_rgba_unmultiplied(linear_to_srgb(r), linear_to_srgb(g), linear_to_srgb(b), a)
}

/// WCAG 2.x contrast ratio between two opaque colours.
pub fn contrast(a: Color32, b: Color32) -> f32 {
    let lum = |c: Color32| {
        let [r, g, b, _] = c.to_srgba_unmultiplied();
        0.2126 * srgb_to_linear(r) + 0.7152 * srgb_to_linear(g) + 0.0722 * srgb_to_linear(b)
    };
    let (la, lb) = (lum(a), lum(b));
    let (hi, lo) = if la > lb { (la, lb) } else { (lb, la) };
    (hi + 0.05) / (lo + 0.05)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_tokens_parse() {
        let t = Theme::embedded();
        assert_eq!(t.c.text_primary, Color32::from_rgb(0x17, 0x19, 0x23));
        assert_eq!(t.c.outline_strong.a(), (0.92f32 * 255.0).round() as u8);
        assert!(t.motion.short > 0.15 && t.motion.short < 0.17);
        assert!(
            t.text.caption.size >= 11.0,
            "PERFORMANCE_AND_ACCESSIBILITY §6.3 minimum text size"
        );
        assert!(t.control.min_hit >= 32.0);
        assert!(t.control.height >= t.control.min_hit);
    }

    #[test]
    fn ink_gel_top_is_lighter_than_the_accent() {
        let t = Theme::embedded();
        let [r1, ..] = t.c.accent.to_srgba_unmultiplied();
        let [r2, ..] = t.c.accent_top.to_srgba_unmultiplied();
        assert!(r2 > r1);
        assert_eq!(
            oklab_shift(t.c.accent, 0.0),
            t.c.accent,
            "round trip is lossless"
        );
    }

    /// The pairs the launcher actually renders (PERFORMANCE_AND_ACCESSIBILITY §6.3).
    #[test]
    fn rendered_pairs_meet_contrast_requirements() {
        let t = Theme::embedded();
        let c = &t.c;
        let body = [
            (c.text_primary, c.surface_base),
            (c.text_primary, c.canvas),
            (c.text_secondary, c.surface_base),
            (c.text_secondary, c.surface_raised),
            (c.text_secondary, c.surface_sunken),
            (c.text_on_accent, c.accent),
            (c.text_on_accent, c.accent_top),
            (c.text_primary, c.danger_soft),
            (c.text_primary, c.warning_soft),
            (c.text_primary, c.info_soft),
            (c.text_primary, c.success_soft),
            (
                c.text_on_accent,
                oklab_shift(c.accent, crate::paint::DISABLED_INK_LIFT),
            ),
            (c.danger, c.surface_raised),
            (c.text_link, c.surface_raised),
        ];
        for (fg, bg) in body {
            assert!(
                contrast(fg, bg) >= 4.5,
                "{fg:?} on {bg:?}: {}",
                contrast(fg, bg)
            );
        }
        // Non-text: focus ring and the disabled outline against the surfaces they sit on.
        assert!(contrast(c.focus, c.surface_base) >= 3.0);
        assert!(contrast(c.focus, c.canvas) >= 3.0);
    }
}
