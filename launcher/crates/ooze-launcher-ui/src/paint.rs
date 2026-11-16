//! The White Ooze / Ink Gel material for the desktop window (DESIGN_SYSTEM §3–§6), painted with
//! egui shapes and meshes. This is ordinary desktop GUI painting through the toolkit; the
//! "no custom renderer" rule applies to the in-game adapter only (ARCHITECTURE §19).

use std::f32::consts::FRAC_PI_2;

use eframe::egui::{
    Color32, CornerRadius, Mesh, Painter, Pos2, Rect, Shape, Stroke, epaint::PathShape,
    epaint::Shadow, pos2,
};

use crate::theme::{Theme, oklab_shift, with_alpha};

/// OKLab lightness lift for disabled Ink Gel (checked by the contrast test in `theme`).
pub const DISABLED_INK_LIFT: f32 = 0.22;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Surface {
    /// Large window-level surface (rim light instead of a gloss band).
    Panel,
    /// Raised card.
    Card,
    /// Secondary button, chip, field.
    Control,
    /// Pressed-in well: flat, no gloss or shadow.
    Sunken,
    /// Ink Gel: primary button, selected navigation item.
    Inverse,
}

#[derive(Clone, Copy, Debug)]
pub struct Gel {
    pub surface: Surface,
    pub radius: f32,
    /// Elevation token index 0–4.
    pub elevation: usize,
    /// 0..1 hover amount (already eased).
    pub hover: f32,
    pub pressed: bool,
    pub disabled: bool,
    /// The gloss band must end above this y (keeps text off the gloss).
    pub gloss_limit: Option<f32>,
}

impl Gel {
    pub fn new(surface: Surface, radius: f32) -> Self {
        let elevation = match surface {
            Surface::Panel => 3,
            Surface::Card | Surface::Control => 1,
            Surface::Inverse => 2,
            Surface::Sunken => 0,
        };
        Self {
            surface,
            radius,
            elevation,
            hover: 0.0,
            pressed: false,
            disabled: false,
            gloss_limit: None,
        }
    }
}

/// Points of a superellipse-cornered rectangle, clockwise from the top-left corner.
pub fn rounded_outline(rect: Rect, radius: f32, exponent: f32, pixels_per_point: f32) -> Vec<Pos2> {
    let r = radius
        .min(rect.width() / 2.0)
        .min(rect.height() / 2.0)
        .max(0.0);
    if r <= 0.0 {
        return vec![
            rect.left_top(),
            rect.right_top(),
            rect.right_bottom(),
            rect.left_bottom(),
        ];
    }
    // Pills are true semicircles (DESIGN_SYSTEM §4.1).
    let n = if (r - rect.height() / 2.0).abs() < 0.01 {
        2.0
    } else {
        exponent
    };
    let r_px = (r * pixels_per_point).max(1.0);
    let segments =
        ((FRAC_PI_2 / (1.0 - 0.15 / r_px).clamp(-1.0, 1.0).acos()).ceil() as usize).clamp(4, 64);
    let e = 2.0 / n;
    let corner = |c: Pos2, sx: f32, sy: f32, theta: f32| {
        pos2(
            c.x + sx * r * theta.cos().max(0.0).powf(e),
            c.y + sy * r * theta.sin().max(0.0).powf(e),
        )
    };
    let corners = [
        (pos2(rect.left() + r, rect.top() + r), -1.0, -1.0, false),
        (pos2(rect.right() - r, rect.top() + r), 1.0, -1.0, true),
        (pos2(rect.right() - r, rect.bottom() - r), 1.0, 1.0, false),
        (pos2(rect.left() + r, rect.bottom() - r), -1.0, 1.0, true),
    ];
    let mut points = Vec::with_capacity(4 * (segments + 1));
    for (c, sx, sy, reverse) in corners {
        for i in 0..=segments {
            let k = if reverse { segments - i } else { i };
            let theta = FRAC_PI_2 * k as f32 / segments as f32;
            points.push(corner(c, sx, sy, theta));
        }
    }
    points
}

/// Fill a convex outline with a vertical two-stop gradient.
fn gradient_fill(points: &[Pos2], rect: Rect, top: Color32, bottom: Color32) -> Mesh {
    let mut mesh = Mesh::default();
    let color_at = |y: f32| {
        let t = ((y - rect.top()) / rect.height().max(1.0)).clamp(0.0, 1.0);
        lerp_color(top, bottom, t)
    };
    let centre = rect.center();
    mesh.colored_vertex(centre, color_at(centre.y));
    for p in points {
        mesh.colored_vertex(*p, color_at(p.y));
    }
    let n = points.len() as u32;
    for i in 0..n {
        mesh.add_triangle(0, 1 + i, 1 + (i + 1) % n);
    }
    mesh
}

pub fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let [ar, ag, ab, aa] = a.to_srgba_unmultiplied();
    let [br, bg, bb, ba] = b.to_srgba_unmultiplied();
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgba_unmultiplied(l(ar, br), l(ag, bg), l(ab, bb), l(aa, ba))
}

fn shadow(out: &mut Vec<Shape>, theme: &Theme, rect: Rect, radius: f32, elevation: usize) {
    let e = theme.elevation[elevation.min(4)];
    let cr = CornerRadius::same(radius.round().clamp(0.0, 255.0) as u8);
    for [y, blur, alpha] in [e.ambient, e.contact] {
        if alpha <= 0.0 {
            continue;
        }
        let s = Shadow {
            offset: [0, y.round().clamp(-127.0, 127.0) as i8],
            blur: blur.round().clamp(0.0, 255.0) as u8,
            spread: 0,
            color: with_alpha(theme.c.shadow, alpha),
        };
        out.push(s.as_shape(rect, cr).into());
    }
}

/// Paint one gel surface: shadows, body, gloss or rim light, glint, outline.
pub fn paint_gel(painter: &Painter, theme: &Theme, rect: Rect, gel: Gel) {
    painter.extend(gel_shapes(theme, rect, gel, painter.pixels_per_point()));
}

/// The shapes of one gel surface, for containers that paint their background after layout.
pub fn gel_shapes(theme: &Theme, rect: Rect, gel: Gel, ppp: f32) -> Vec<Shape> {
    let mut out = Vec::with_capacity(8);
    let c = &theme.c;
    let sunken = gel.surface == Surface::Sunken;
    let inverse = gel.surface == Surface::Inverse;
    let elevation = if gel.disabled {
        gel.elevation.min(1)
    } else {
        gel.elevation + usize::from(gel.hover > 0.5)
    };
    if !sunken && elevation > 0 {
        shadow(&mut out, theme, rect, gel.radius, elevation);
    }

    let points = rounded_outline(rect, gel.radius, theme.shape.corner_exponent, ppp);
    let (mut top, mut bottom) = match gel.surface {
        Surface::Panel | Surface::Control => (c.gradient_top, c.gradient_bottom),
        Surface::Card => (c.surface_raised, c.gradient_bottom),
        Surface::Sunken => (c.surface_sunken, c.surface_sunken),
        Surface::Inverse => (c.accent_top, c.accent),
    };
    if gel.disabled && inverse {
        // Busy/disabled Ink Gel stays legible (its label is a status such as "Installing…"):
        // a lighter, flat ink that still holds white text above 4.5:1.
        top = oklab_shift(c.accent, DISABLED_INK_LIFT);
        bottom = top;
    } else if gel.pressed {
        let d = if inverse { 0.04 } else { -0.03 };
        top = oklab_shift(top, d);
        bottom = oklab_shift(bottom, d);
    }
    out.push(Shape::mesh(gradient_fill(&points, rect, top, bottom)));

    let outline_w = match gel.surface {
        Surface::Panel => theme.shape.outline_panel,
        Surface::Sunken => theme.shape.outline_hairline,
        _ => theme.shape.outline_control,
    };
    let m = &theme.material;
    if !sunken && !gel.disabled {
        if rect.height() <= 72.0 {
            // Gloss band + glint (DESIGN_SYSTEM §3 layers 5 and 6).
            let inset = outline_w + 2.0;
            let mut band_h = (rect.height() * m.gloss_height_ratio).clamp(4.0, 22.0);
            if let Some(limit) = gel.gloss_limit {
                band_h = band_h.min(limit - (rect.top() + inset)).max(0.0);
            }
            if band_h >= 2.0 {
                let band = Rect::from_min_max(
                    pos2(
                        rect.left() + rect.width() * m.gloss_start,
                        rect.top() + inset,
                    ),
                    pos2(
                        rect.left() + rect.width() * m.gloss_end,
                        rect.top() + inset + band_h,
                    ),
                );
                let alpha = if inverse {
                    m.ink_gloss_opacity
                } else {
                    m.gloss_opacity
                };
                let alpha = alpha * (1.0 - 0.2 * f32::from(u8::from(gel.pressed)));
                let pill = rounded_outline(band, band.height() / 2.0, 2.0, ppp);
                out.push(Shape::mesh(gradient_fill(
                    &pill,
                    band,
                    with_alpha(c.gloss, alpha),
                    with_alpha(c.gloss, 0.0),
                )));
                if rect.height() >= 32.0 {
                    let gr = (rect.height() * 0.07).clamp(1.5, 4.0);
                    let glint_alpha = if inverse {
                        m.ink_glint_opacity
                    } else {
                        m.glint_opacity
                    };
                    out.push(Shape::circle_filled(
                        pos2(
                            band.right() + 2.0 * gr,
                            band.top() + band_h.min(gr * 3.0) / 2.0,
                        ),
                        gr,
                        with_alpha(c.gloss, glint_alpha),
                    ));
                }
            }
        } else {
            // Rim light along the top edge for large surfaces (layer 5′).
            let y = rect.top() + outline_w + 1.5;
            let r = gel.radius.min(rect.height() / 2.0);
            out.push(Shape::line_segment(
                [pos2(rect.left() + r, y), pos2(rect.right() - r, y)],
                Stroke::new(1.5, with_alpha(c.gloss, m.rim_opacity)),
            ));
        }
    }

    let outline_color = match gel.surface {
        Surface::Sunken => c.outline_soft,
        _ if gel.disabled => with_alpha(c.text_disabled, 0.8),
        _ => c.outline_strong,
    };
    out.push(PathShape::closed_line(points, Stroke::new(outline_w, outline_color)).into());
    out
}

/// The keyboard focus ring: a `focus` stroke with a thin halo, outside the control (COMPONENTS §1.3).
pub fn paint_focus_ring(painter: &Painter, theme: &Theme, rect: Rect, radius: f32) {
    let gap = theme.control.focus_ring_gap;
    let w = theme.control.focus_ring_width;
    let ring = rect.expand(gap + w / 2.0);
    let ppp = painter.pixels_per_point();
    let points = rounded_outline(
        ring,
        radius + gap + w / 2.0,
        theme.shape.corner_exponent,
        ppp,
    );
    painter.add(PathShape::closed_line(
        points.clone(),
        Stroke::new(w + 2.0 * theme.shape.outline_hairline, theme.c.focus_halo),
    ));
    painter.add(PathShape::closed_line(
        points,
        Stroke::new(w, theme.c.focus),
    ));
}

/// One decorative drip hanging from the bottom edge of `rect` at x (DESIGN_SYSTEM §4.5).
/// Drawn after the surface: the neck overlaps the body by one point so the joint is seamless.
pub fn paint_drip(
    painter: &Painter,
    theme: &Theme,
    rect: Rect,
    x: f32,
    neck: f32,
    length: f32,
    bulb: f32,
) {
    let c = &theme.c;
    let top = rect.bottom() - theme.shape.outline_panel;
    let bulb = bulb.max(neck / 2.0 + 1.0);
    let centre = pos2(x, rect.bottom() + length - bulb);
    let fill = c.gradient_bottom;
    let neck_rect = Rect::from_min_max(
        pos2(x - neck / 2.0, top - 1.0),
        pos2(x + neck / 2.0, centre.y),
    );
    painter.rect_filled(neck_rect, CornerRadius::ZERO, fill);
    painter.circle_filled(centre, bulb, fill);
    let stroke = Stroke::new(theme.shape.outline_control, c.outline_strong);
    // Neck sides and the bulb outline (the arc between the neck lines is hidden by the fill).
    let side_bottom = centre.y - (bulb * bulb - (neck / 2.0) * (neck / 2.0)).max(0.0).sqrt();
    painter.line_segment(
        [
            pos2(x - neck / 2.0, rect.bottom()),
            pos2(x - neck / 2.0, side_bottom),
        ],
        stroke,
    );
    painter.line_segment(
        [
            pos2(x + neck / 2.0, rect.bottom()),
            pos2(x + neck / 2.0, side_bottom),
        ],
        stroke,
    );
    let start = ((neck / 2.0) / bulb).asin();
    let steps = 24;
    let arc: Vec<Pos2> = (0..=steps)
        .map(|i| {
            let a = -FRAC_PI_2
                + start
                + (2.0 * std::f32::consts::PI - 2.0 * start) * i as f32 / steps as f32;
            pos2(centre.x + bulb * a.cos(), centre.y + bulb * a.sin())
        })
        .collect();
    painter.add(PathShape::line(arc, stroke));
    // Cover the body outline where the neck joins it.
    painter.line_segment(
        [
            pos2(x - neck / 2.0 + stroke.width / 2.0, rect.bottom()),
            pos2(x + neck / 2.0 - stroke.width / 2.0, rect.bottom()),
        ],
        Stroke::new(theme.shape.outline_panel + 1.0, fill),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::vec2;

    #[test]
    fn outline_is_closed_convex_and_inside_the_rect() {
        let rect = Rect::from_min_size(pos2(10.0, 20.0), vec2(200.0, 48.0));
        let pts = rounded_outline(rect, 14.0, 2.6, 2.0);
        assert!(pts.len() >= 16);
        for p in &pts {
            assert!(rect.expand(0.01).contains(*p), "{p:?}");
        }
        // Convex and clockwise (screen coordinates): every cross product has the same sign.
        let n = pts.len();
        let mut sign = 0.0f32;
        for i in 0..n {
            let (a, b, c) = (pts[i], pts[(i + 1) % n], pts[(i + 2) % n]);
            let cross = (b - a).x * (c - b).y - (b - a).y * (c - b).x;
            if cross.abs() > 1e-4 {
                if sign == 0.0 {
                    sign = cross.signum();
                }
                assert_eq!(cross.signum(), sign, "concave at {i}");
            }
        }
    }

    #[test]
    fn segment_count_follows_the_chord_error_rule() {
        let rect = Rect::from_min_size(Pos2::ZERO, vec2(400.0, 400.0));
        let small = rounded_outline(rect, 4.0, 2.6, 1.0).len();
        let large = rounded_outline(rect, 28.0, 2.6, 2.0).len();
        assert!(large > small);
        assert!(rounded_outline(rect, 28.0, 2.6, 100.0).len() <= 4 * 65);
    }
}
