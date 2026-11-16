//! Time-based motion that honours the Motion setting (MOTION_SPEC §8, launcher subset).
//!
//! - **Full:** token durations, ambient drips and the indeterminate sweep.
//! - **Reduced:** transitions become short fades (`reduced.fade`), no ambient motion, the
//!   indeterminate indicator is static.
//! - **Off:** every transition is instant and nothing animates; state stays visible through
//!   text, shape and colour. No repaint loop runs, so an idle window uses no CPU.

use crate::api::MotionPreference;
use crate::theme::MotionTokens;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MotionMode {
    Full,
    Reduced,
    Off,
}

impl MotionMode {
    pub fn resolve(pref: MotionPreference, system_reduced: Option<bool>) -> Self {
        match pref {
            MotionPreference::Full => Self::Full,
            MotionPreference::Reduced => Self::Reduced,
            MotionPreference::Off => Self::Off,
            MotionPreference::FollowSystem => {
                if system_reduced == Some(true) {
                    Self::Reduced
                } else {
                    Self::Full
                }
            }
        }
    }

    /// Duration (seconds) to use for a transition whose token duration is `dur`.
    pub fn tween(self, tokens: &MotionTokens, dur: f32) -> f32 {
        match self {
            Self::Full => dur,
            Self::Reduced => dur.min(tokens.reduced_fade),
            Self::Off => 0.0,
        }
    }

    /// Continuous ambient motion (drips, indeterminate sweep).
    pub fn ambient(self) -> bool {
        self == Self::Full
    }
}

/// `ease.standard` from MOTION_SPEC §3.1: cubic-bezier(0.20, 0.00, 0.00, 1.00).
pub fn ease_standard(t: f32) -> f32 {
    cubic_bezier(0.20, 0.00, 0.00, 1.00, t)
}

/// `ease.viscous`: cubic-bezier(0.65, 0.00, 0.35, 1.00), for drip growth.
pub fn ease_viscous(t: f32) -> f32 {
    cubic_bezier(0.65, 0.00, 0.35, 1.00, t)
}

/// Evaluate a CSS cubic-bezier at `x` (Newton–Raphson with bisection fallback, MOTION_SPEC §3.1).
pub fn cubic_bezier(x1: f32, y1: f32, x2: f32, y2: f32, x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    let bez = |a: f32, b: f32, t: f32| {
        let u = 1.0 - t;
        3.0 * u * u * t * a + 3.0 * u * t * t * b + t * t * t
    };
    let d = |a: f32, b: f32, t: f32| {
        let u = 1.0 - t;
        3.0 * u * u * a + 6.0 * u * t * (b - a) + 3.0 * t * t * (1.0 - b)
    };
    let mut t = x;
    for _ in 0..8 {
        let err = bez(x1, x2, t) - x;
        if err.abs() < 1e-6 {
            return bez(y1, y2, t);
        }
        let slope = d(x1, x2, t);
        if slope.abs() < 1e-6 {
            break;
        }
        t = (t - err / slope).clamp(0.0, 1.0);
    }
    let (mut lo, mut hi) = (0.0f32, 1.0f32);
    t = x;
    for _ in 0..32 {
        let v = bez(x1, x2, t);
        if (v - x).abs() < 1e-6 {
            break;
        }
        if v < x {
            lo = t;
        } else {
            hi = t;
        }
        t = 0.5 * (lo + hi);
    }
    bez(y1, y2, t)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::Theme;

    #[test]
    fn modes_resolve_from_preference_and_system() {
        assert_eq!(
            MotionMode::resolve(MotionPreference::FollowSystem, None),
            MotionMode::Full
        );
        assert_eq!(
            MotionMode::resolve(MotionPreference::FollowSystem, Some(true)),
            MotionMode::Reduced
        );
        assert_eq!(
            MotionMode::resolve(MotionPreference::Off, Some(false)),
            MotionMode::Off
        );
    }

    #[test]
    fn durations_follow_the_mode() {
        let t = Theme::embedded().motion;
        assert_eq!(MotionMode::Full.tween(&t, t.long), t.long);
        assert_eq!(MotionMode::Reduced.tween(&t, t.long), t.reduced_fade);
        assert_eq!(MotionMode::Off.tween(&t, t.long), 0.0);
        assert!(!MotionMode::Reduced.ambient() && !MotionMode::Off.ambient());
    }

    #[test]
    fn easing_endpoints_and_monotonic() {
        for f in [ease_standard, ease_viscous] {
            assert!(f(0.0).abs() < 1e-4 && (f(1.0) - 1.0).abs() < 1e-4);
            let mut prev = 0.0;
            for i in 1..=100 {
                let v = f(i as f32 / 100.0);
                assert!(v + 1e-4 >= prev);
                prev = v;
            }
        }
        assert!(ease_standard(0.5) > 0.5, "standard decelerates");
    }
}
