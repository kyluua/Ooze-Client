//! HUD layout: nine anchors, offsets in Ooze units (ou), per-widget scale, clamping, snapping to
//! screen edges, centre lines, other widgets and a grid, and keyboard nudging. Pure geometry; the
//! renderer and the editor screen consume the rectangles.

/// Where a widget is pinned. Offsets point inward from that anchor.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Anchor {
    TopLeft,
    Top,
    TopRight,
    Left,
    Center,
    Right,
    BottomLeft,
    Bottom,
    BottomRight,
}

impl Anchor {
    pub const ALL: [Self; 9] = [
        Self::TopLeft,
        Self::Top,
        Self::TopRight,
        Self::Left,
        Self::Center,
        Self::Right,
        Self::BottomLeft,
        Self::Bottom,
        Self::BottomRight,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::TopLeft => "top_left",
            Self::Top => "top",
            Self::TopRight => "top_right",
            Self::Left => "left",
            Self::Center => "center",
            Self::Right => "right",
            Self::BottomLeft => "bottom_left",
            Self::Bottom => "bottom",
            Self::BottomRight => "bottom_right",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|a| a.as_str() == s)
    }

    /// (horizontal, vertical) factor: 0 = start, 0.5 = centre, 1 = end.
    fn factors(self) -> (f32, f32) {
        let h = match self {
            Self::TopLeft | Self::Left | Self::BottomLeft => 0.0,
            Self::Top | Self::Center | Self::Bottom => 0.5,
            Self::TopRight | Self::Right | Self::BottomRight => 1.0,
        };
        let v = match self {
            Self::TopLeft | Self::Top | Self::TopRight => 0.0,
            Self::Left | Self::Center | Self::Right => 0.5,
            Self::BottomLeft | Self::Bottom | Self::BottomRight => 1.0,
        };
        (h, v)
    }

    fn from_factors(h: f32, v: f32) -> Self {
        let pick = |f: f32| {
            if f < 0.25 {
                0
            } else if f < 0.75 {
                1
            } else {
                2
            }
        };
        Self::ALL[pick(v) * 3 + pick(h)]
    }
}

pub const MIN_SCALE: f32 = 0.5;
pub const MAX_SCALE: f32 = 2.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Placement {
    pub anchor: Anchor,
    /// Inward offset from the anchor in ou (x from the anchored side, y likewise).
    pub x: f32,
    pub y: f32,
    pub scale: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub fn right(&self) -> f32 {
        self.x + self.w
    }
    pub fn bottom(&self) -> f32 {
        self.y + self.h
    }
}

impl Placement {
    pub fn valid(&self) -> bool {
        self.x.is_finite() && self.y.is_finite() && (MIN_SCALE..=MAX_SCALE).contains(&self.scale)
    }

    /// The widget's rectangle on a `screen` (w, h) for an unscaled widget `size`, clamped so the
    /// widget never leaves the screen (a widget larger than the screen pins to the top-left).
    pub fn rect(&self, size: (f32, f32), screen: (f32, f32)) -> Rect {
        let (w, h) = (size.0 * self.scale, size.1 * self.scale);
        let (fh, fv) = self.anchor.factors();
        let sign = |f: f32| if f >= 1.0 { -1.0 } else { 1.0 };
        let x = fh * (screen.0 - w) + sign(fh) * if fh == 0.5 { self.x } else { self.x.max(0.0) };
        let y = fv * (screen.1 - h) + sign(fv) * if fv == 0.5 { self.y } else { self.y.max(0.0) };
        Rect {
            x: x.clamp(0.0, (screen.0 - w).max(0.0)),
            y: y.clamp(0.0, (screen.1 - h).max(0.0)),
            w,
            h,
        }
    }

    /// The placement that reproduces `rect` on `screen`, anchored to the nearest third so
    /// widgets keep their relative position when the window is resized.
    pub fn from_rect(rect: Rect, screen: (f32, f32), scale: f32) -> Self {
        let cx = (rect.x + rect.w / 2.0) / screen.0.max(1.0);
        let cy = (rect.y + rect.h / 2.0) / screen.1.max(1.0);
        let anchor = Anchor::from_factors(cx, cy);
        let (fh, fv) = anchor.factors();
        let off = |pos: f32, size: f32, extent: f32, f: f32| {
            if f == 0.0 {
                pos
            } else if f >= 1.0 {
                extent - (pos + size)
            } else {
                pos - (extent - size) / 2.0
            }
        };
        Self {
            anchor,
            x: off(rect.x, rect.w, screen.0, fh),
            y: off(rect.y, rect.h, screen.1, fv),
            scale,
        }
    }

    /// Keyboard nudge by (dx, dy) screen units, staying on screen.
    pub fn nudge(&self, dx: f32, dy: f32, size: (f32, f32), screen: (f32, f32)) -> Self {
        let r = self.rect(size, screen);
        let moved = Rect {
            x: r.x + dx,
            y: r.y + dy,
            ..r
        };
        let clamped = Rect {
            x: moved.x.clamp(0.0, (screen.0 - r.w).max(0.0)),
            y: moved.y.clamp(0.0, (screen.1 - r.h).max(0.0)),
            ..moved
        };
        Self::from_rect(clamped, screen, self.scale)
    }

    pub fn with_scale(&self, scale: f32) -> Self {
        Self {
            scale: scale.clamp(MIN_SCALE, MAX_SCALE),
            ..*self
        }
    }
}

/// Snap a dragged rectangle: to screen edges and centre lines, then to other widgets' edges,
/// within `threshold`; otherwise to `grid` (0 disables the grid). Returns the snapped rectangle.
pub fn snap(rect: Rect, others: &[Rect], screen: (f32, f32), grid: f32, threshold: f32) -> Rect {
    let snap_axis =
        |pos: f32, size: f32, extent: f32, lines: &mut dyn Iterator<Item = f32>| -> f32 {
            let mut best: Option<(f32, f32)> = None; // (distance, new position)
            let mut consider = |target: f32, edge_offset: f32| {
                let candidate = target - edge_offset;
                let d = (pos - candidate).abs();
                if d <= threshold && best.is_none_or(|(bd, _)| d < bd) {
                    best = Some((d, candidate));
                }
            };
            for line in [0.0, extent / 2.0, extent].into_iter().chain(lines) {
                consider(line, 0.0); // leading edge
                consider(line, size); // trailing edge
                consider(line, size / 2.0); // centre
            }
            match best {
                Some((_, p)) => p,
                None if grid > 0.0 => (pos / grid).round() * grid,
                None => pos,
            }
        };
    let x = snap_axis(
        rect.x,
        rect.w,
        screen.0,
        &mut others.iter().flat_map(|o| [o.x, o.right()]),
    );
    let y = snap_axis(
        rect.y,
        rect.h,
        screen.1,
        &mut others.iter().flat_map(|o| [o.y, o.bottom()]),
    );
    Rect {
        x: x.clamp(0.0, (screen.0 - rect.w).max(0.0)),
        y: y.clamp(0.0, (screen.1 - rect.h).max(0.0)),
        ..rect
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCREEN: (f32, f32) = (1920.0, 1080.0);
    const SIZE: (f32, f32) = (100.0, 20.0);

    fn p(anchor: Anchor, x: f32, y: f32) -> Placement {
        Placement {
            anchor,
            x,
            y,
            scale: 1.0,
        }
    }

    #[test]
    fn anchors_place_widgets_inward() {
        assert_eq!(
            p(Anchor::TopLeft, 4.0, 4.0).rect(SIZE, SCREEN),
            Rect {
                x: 4.0,
                y: 4.0,
                w: 100.0,
                h: 20.0
            }
        );
        assert_eq!(
            p(Anchor::BottomRight, 4.0, 4.0).rect(SIZE, SCREEN),
            Rect {
                x: 1816.0,
                y: 1056.0,
                w: 100.0,
                h: 20.0
            }
        );
        assert_eq!(
            p(Anchor::Center, 0.0, 0.0).rect(SIZE, SCREEN),
            Rect {
                x: 910.0,
                y: 530.0,
                w: 100.0,
                h: 20.0
            }
        );
        let scaled = Placement {
            scale: 2.0,
            ..p(Anchor::TopRight, 0.0, 0.0)
        }
        .rect(SIZE, SCREEN);
        assert_eq!((scaled.x, scaled.w), (1720.0, 200.0));
    }

    #[test]
    fn widgets_never_leave_the_screen() {
        let r = p(Anchor::TopLeft, 5000.0, 5000.0).rect(SIZE, SCREEN);
        assert!(r.right() <= SCREEN.0 && r.bottom() <= SCREEN.1);
        let tiny = p(Anchor::BottomRight, 0.0, 0.0).rect(SIZE, (50.0, 10.0));
        assert_eq!((tiny.x, tiny.y), (0.0, 0.0));
    }

    #[test]
    fn rect_and_placement_round_trip_through_the_nearest_anchor() {
        for anchor in Anchor::ALL {
            let original = p(anchor, 12.0, 8.0);
            let r = original.rect(SIZE, SCREEN);
            let back = Placement::from_rect(r, SCREEN, 1.0);
            assert_eq!(back.rect(SIZE, SCREEN), r, "{anchor:?}");
        }
        // Resizing keeps a bottom-right widget at the bottom-right.
        let br = Placement::from_rect(
            Rect {
                x: 1800.0,
                y: 1050.0,
                w: 100.0,
                h: 20.0,
            },
            SCREEN,
            1.0,
        );
        assert_eq!(br.anchor, Anchor::BottomRight);
        let small = br.rect(SIZE, (1280.0, 720.0));
        assert_eq!((small.right(), small.bottom()), (1260.0, 710.0));
    }

    #[test]
    fn snapping_prefers_edges_then_widgets_then_grid() {
        let near_edge = snap(
            Rect {
                x: 3.0,
                y: 500.0,
                w: 100.0,
                h: 20.0,
            },
            &[],
            SCREEN,
            8.0,
            6.0,
        );
        assert_eq!(near_edge.x, 0.0);
        let near_centre = snap(
            Rect {
                x: 908.0,
                y: 300.0,
                w: 100.0,
                h: 20.0,
            },
            &[],
            SCREEN,
            8.0,
            6.0,
        );
        assert_eq!(near_centre.x, 910.0, "centred on the vertical centre line");
        let other = Rect {
            x: 400.0,
            y: 400.0,
            w: 50.0,
            h: 50.0,
        };
        let below = snap(
            Rect {
                x: 403.0,
                y: 453.0,
                w: 30.0,
                h: 10.0,
            },
            &[other],
            SCREEN,
            8.0,
            6.0,
        );
        assert_eq!(
            (below.x, below.y),
            (400.0, 450.0),
            "aligned to the other widget's edges"
        );
        let free = snap(
            Rect {
                x: 211.0,
                y: 333.0,
                w: 30.0,
                h: 10.0,
            },
            &[],
            SCREEN,
            8.0,
            2.0,
        );
        assert_eq!((free.x, free.y), (208.0, 336.0), "grid");
    }

    #[test]
    fn nudging_and_scaling_stay_bounded() {
        let at_edge = p(Anchor::TopLeft, 0.0, 0.0).nudge(-10.0, -10.0, SIZE, SCREEN);
        assert_eq!(at_edge.rect(SIZE, SCREEN).x, 0.0);
        let moved = p(Anchor::TopLeft, 10.0, 10.0).nudge(1.0, 0.0, SIZE, SCREEN);
        assert_eq!(moved.rect(SIZE, SCREEN).x, 11.0);
        assert_eq!(p(Anchor::Top, 0.0, 0.0).with_scale(9.0).scale, MAX_SCALE);
        assert!(
            !Placement {
                scale: 0.1,
                ..p(Anchor::Top, 0.0, 0.0)
            }
            .valid()
        );
    }
}
