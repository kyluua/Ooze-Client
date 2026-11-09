# Ooze Client — Motion & Animation Specification

> Deliverable 5. The motion language that makes Ooze feel like a soft, viscous, elastic substance. Every number here is a default; the user settings in §8 scale it.

## 1. Principles

1. **Viscous, not bouncy.** Motion starts with a little resistance, carries momentum, and settles with a small, quickly damped wobble. Overshoot is usually ≤ 6%. Only deformation (squash and stretch) is allowed to wobble more visibly.
2. **Volume is conserved.** When gel stretches in one axis it thins in the other: `sx · sy ≈ 1`.
3. **Response before decoration.** Input feedback starts within the same frame as the input. Decorative follow-ups such as shimmer, ripple and drips come after.
4. **Time-based, frame-rate independent.** All animation uses real elapsed time from a monotonic clock. Springs use the closed-form solution (§3.2), so 30 FPS and 240 FPS produce the same trajectory.
5. **Interruptible.** Every animation can be retargeted mid-flight. Springs keep their velocity; tweens restart from the current value.
6. **Quiet when idle.** When nothing is happening, the only movement is the optional ambient background layer.

## 2. Clock

- `FrameClock` samples `System.nanoTime()` once per rendered frame:
  - `now` is monotonic, in seconds as a double.
  - `dt = clamp(now − last, 0, 0.050)` s.
  - `rawDt` is unclamped and used only for statistics.
- **Hitch handling:** if `rawDt > 0.25 s` (window drag, world load, alt-tab), one-shot effects that would have started during the gap (shimmers, ripples) are skipped. Springs continue from their current state with the clamped dt. Ambient layers are pure functions of absolute time (§7), so they jump seamlessly.
- **Pausing:** while the window is unfocused or minimised, ambient time is frozen. `ambientTime` advances only while the window is focused or `continueAmbientWhenUnfocused` is set.
- One clock instance exists per client. Screens and the HUD read from it and never sample time themselves.

## 3. Motion primitives

### 3.1 Duration and easing tokens

Durations are at speed 1.0.

| Token | ms | Use |
|---|---|---|
| `dur.instant` | 0 | Off mode, programmatic resets |
| `dur.micro` | 90 | Colour state changes, focus-ring fade, chevron rotation |
| `dur.short` | 160 | Hover fades, tooltip in, toggle track colour |
| `dur.medium` | 240 | Popover and dropdown in, tab content crossfade |
| `dur.long` | 360 | Page transitions, modal in |
| `dur.extended` | 520 | Shimmer sweep, intro sub-steps |

| Token | Curve | Use |
|---|---|---|
| `ease.standard` | cubic-bezier(0.20, 0.00, 0.00, 1.00) | Default for tweens |
| `ease.enter` | cubic-bezier(0.05, 0.70, 0.10, 1.00) | Things appearing |
| `ease.exit` | cubic-bezier(0.30, 0.00, 0.80, 0.15) | Things leaving (shorter than enter) |
| `ease.viscous` | cubic-bezier(0.65, 0.00, 0.35, 1.00) | Drip growth, ambient interpolation, syrupy moves |
| `ease.drop` | cubic-bezier(0.55, 0.00, 1.00, 0.45) | Gravity-like acceleration (falling droplets) |
| `ease.linear` | linear | Progress bars, continuous rotation |

**Cubic-bezier evaluation:** a pre-sampled 11-entry table, then Newton–Raphson (up to 8 iterations, tolerance 1e-6) with a bisection fallback. Instances are immutable and shared.

### 3.2 Springs

Mass m = 1. Each spring is defined by stiffness `k` and damping ratio `ζ`.

| Token | k | ζ | ω₀ (rad/s) | Overshoot | ~Settle (2%) | Use |
|---|---|---|---|---|---|---|
| `spring.snappy` | 700 | 0.82 | 26.5 | 1.1% | 0.18 s | Pointer-follow (slider thumb), focus ring, small indicators |
| `spring.gel` | 380 | 0.68 | 19.5 | 5.4% | 0.30 s | **Default component spring**: toggle knob, nav indicator, hover inflate, popover scale |
| `spring.wobble` | 260 | 0.45 | 16.1 | 20.5% | 0.55 s | **Deformation only**: squash recovery, landing jiggle, blob settle |
| `spring.soft` | 160 | 0.90 | 12.6 | 0.2% | 0.35 s | Panels, inspector slide, sidebar width, toasts |
| `spring.lazy` | 60 | 1.00 | 7.7 | 0% | 0.75 s | Parallax, cursor-follow ambient, scroll-to |

Damping coefficient `c = 2ζ√(k·m)`.

**Closed-form update.** Let `x₀ = value − target` and `v₀ = velocity`. For a step `dt`:

```
Underdamped (ζ < 1):   a = ζω₀,  ω_d = ω₀√(1−ζ²)
  x(t) = e^(−a t) · ( x₀·cos(ω_d t) + ((v₀ + a·x₀)/ω_d)·sin(ω_d t) )
  v(t) = e^(−a t) · ( v₀·cos(ω_d t) − ((a·v₀ + ω₀²·x₀)/ω_d)·sin(ω_d t) )

Critical (|ζ − 1| < 1e-4):
  x(t) = (x₀ + (v₀ + ω₀x₀)·t) · e^(−ω₀ t)
  v(t) = (v₀ − ω₀·(v₀ + ω₀x₀)·t) · e^(−ω₀ t)

Overdamped (ζ > 1):   r₁,₂ = −ω₀(ζ ∓ √(ζ²−1)),  C₁ = (v₀ − r₂x₀)/(r₁ − r₂),  C₂ = x₀ − C₁
  x(t) = C₁e^(r₁t) + C₂e^(r₂t),   v(t) = C₁r₁e^(r₁t) + C₂r₂e^(r₂t)
```

**Sleep.** A spring sleeps when `|x| < ε_x` and `|v| < ε_v`, then snaps exactly to the target and stops requesting frames. Thresholds per value type:

| Value type | ε_x | ε_v |
|---|---|---|
| Position (ou) | 0.01 | 0.05 /s |
| Scale | 0.0005 | 0.005 /s |
| Opacity | 0.002 | 0.01 /s |
| Colour (OKLab) | 0.001 per channel | 0.005 /s |

**Animated value types:** `AnimatedFloat`, `AnimatedVec2`, `AnimatedColor` (interpolated in OKLab, stored as packed ARGB) and `AnimatedRect`. Each can be driven by a tween or a spring and exposes `value()`, `velocity()`, `isSettled()`, `animateTo(target, spec)` and `snapTo(target)`.

### 3.3 Composite behaviours

| Behaviour | Definition |
|---|---|
| **Inflate** | scale → 1 + 0.025·I, lift y → −1 ou, elevation e1 → e2. Driven by `spring.gel` |
| **Squash** (press) | sx → 1 + 0.04·I, sy → 1 − 0.05·I, anchored at bottom centre for buttons on surfaces and at centre for floating items. Driven by a 90 ms `ease.standard` tween (pressing must feel immediate) |
| **Release** | sx, sy → 1 with `spring.wobble`, starting from the squashed state with velocity 0 (visible jiggle about 3 cycles, amplitude decaying) |
| **Stretch-by-velocity** | Along the direction of travel: s∥ = 1 + s_max·clamp(\|v\| / v_ref, 0, 1), s⊥ = 1/s∥. Here v_ref = 8 × travel distance per second; s_max is defined per component |
| **Landing** | On arrival (velocity sign flips, or \|x\| < 10% of travel): one-shot squash 0.92/1.08 perpendicular to travel, recovered with `spring.wobble` |
| **Ripple** | A ring centred on the interaction point. Radius 0 → 2.2 × source radius, α 0.35·I → 0, stroke 2 → 0.5 ou, 420 ms `ease.enter`. Colour `highlight.cool` (accent at L+0.2 on Ink Gel) |
| **Shimmer** | A diagonal band 30% of the width, at a 20° angle, white α `material.shimmer.opacity`·I, sweeping −30% → 130% across the gloss region in 520 ms `ease.standard`. Clipped to the gloss capsule. Cooldown 1.2 s per component |
| **Liquid morph** (indicator move) | Two springs: the **leading edge** uses `spring.snappy`, the **trailing edge** uses `spring.gel`. The indicator spans between them. Its thickness is h = h₀·√(w₀/w), clamped ≥ 0.75·h₀ (volume conservation). Result: it stretches towards the target, then contracts |
| **Stagger** | 24 ms between siblings. At most 8 staggered items; any after the 8th appear with the 8th. Total stagger ≤ 200 ms |

`I` = the user's animation intensity (§8). Deformation amplitudes multiply by `I`; timings do not.

## 4. Component motion

All timings are at speed 1.0, intensity 1.0, quality High.

### 4.1 Buttons (C01, C02)

| Event | Motion |
|---|---|
| Hover enter | Inflate (`spring.gel`). Shimmer once (cooldown applies). Gloss shifts up to 2 ou towards the cursor (`spring.lazy`). Primary L/XL buttons: hover drip grows 0 → 5 ou over 600 ms (`ease.viscous`) at 70% of the width |
| Hover exit | De-inflate (`spring.gel`). Hover drip retracts (`spring.gel`) |
| Press | Squash (90 ms). Elevation → e0.5. Body darkens 30% towards sunken (tween 90 ms) |
| Release (activates) | Release wobble (`spring.wobble`). Ripple from the press point. Activation fires on release, inside the bounds |
| Release outside | Release wobble only; no activation and no ripple |
| Focus-visible | Focus ring scales in from 1.06 → 1.0 (`spring.snappy`) and fades in over 90 ms |
| Disabled ↔ enabled | Opacity tween over 160 ms; no deformation |
| Loading | Label fades out over 90 ms; BlobSpinner (C24) fades in; width stays constant |

### 4.2 Toggle (C03)

| Event | Motion |
|---|---|
| Click | The knob x is driven by `spring.gel` towards the opposite end. Stretch-by-velocity with s_max = 0.35. Track colour crossfades over 180 ms (OKLab) |
| Arrival | Landing squash on the knob (sx 0.92 / sy 1.08), recovered with `spring.wobble` |
| Press (before release) | Knob widens by 12% towards the travel direction (anticipation) over 90 ms |
| Drag | Knob follows the pointer within the track, using `spring.snappy`. On release it commits to the nearest side (or the side the velocity points to if \|v\| > 300 ou/s) |
| Hover | Knob inflates (scale 1.06, `spring.gel`); the track gains the soft outline |
| Keyboard (Space/Enter) | Same as click, with the ripple at the knob centre |

### 4.3 Checkbox / radio (C04, C05)

- **Check:** the box squashes; an ink drop fills the box from the centre (radius 0 → full in 200 ms with `spring.gel`, a slight overshoot clipped by the box); then the check glyph strokes in from 0 → 100% path length over 160 ms (`ease.enter`, starting at 60 ms).
- **Uncheck:** the drop shrinks to the centre in 140 ms (`ease.exit`); the check fades out over 90 ms.
- **Radio:** the inner droplet morphs from the previously selected option to the new one using a two-blob merge. The old droplet shrinks to 0 and the new one grows from 0 with `spring.gel`, overlapping by 60 ms.

### 4.4 Slider (C07)

| Event | Motion |
|---|---|
| Press on thumb | Thumb inflates to 1.15 (`spring.gel`). Ripple. The value bubble (tooltip) appears above the thumb (scale 0.9 → 1, 120 ms, `ease.enter`) |
| Press on track | The value jumps to the click point; the thumb travels there with `spring.gel` plus stretch-by-velocity (s_max 0.25) |
| Drag | **The value updates immediately.** The visual thumb follows the pointer with `spring.snappy` (≤ 40 ms of visual lag). Stretch-by-velocity, s_max 0.25 |
| Fill | The fill end follows the visual thumb. A meniscus bulge (fill end radius +1.5 ou) appears while dragging and settles after release with `spring.wobble` |
| Stepped values | The thumb snaps between steps with `spring.gel`. Tick dots scale to 1.4 when passed (90 ms) |
| Release | Thumb de-inflates with `spring.wobble`; the value bubble fades over 160 ms after a 400 ms hold; ripple |
| Keyboard | Arrow keys step by 1 step (Shift ×10, Ctrl to the minimum step); the thumb animates with `spring.gel`; the bubble shows for 800 ms |

### 4.5 Segmented control, tabs and navigation (C06, C17, C18)

- The **active indicator** uses **liquid morph** (§3.3) between segments or items. Sidebar nav uses a vertical morph; tabs use a horizontal underline-blob morph (height 4 ou).
- **Label colour** crossfades between `text.primary` and `text.onAccent` over 160 ms while the indicator passes under it. The crossfade is timed by overlap: a label is "selected-coloured" when the indicator covers ≥ 50% of it.
- **Hover** on inactive items: a sunken "pre-blob" fades in (α 0 → 0.6, 120 ms) and inflates 0.9 → 1.0 (`spring.gel`).
- **Category change content:** the outgoing grid fades out and moves 8 ou up (140 ms `ease.exit`). The incoming grid fades in from 12 ou below (260 ms `ease.enter`, start +80 ms), with cards staggered.

### 4.6 Panels, windows, modals and popovers (C15, C20, C21)

| Element | Open | Close |
|---|---|---|
| Window (ClickGUI) | scale 0.94 → 1 (`spring.gel`), opacity 0 → 1 (160 ms), y +16 → 0 (`spring.soft`). Corner radius starts at +12 ou and settles (`spring.wobble`) to read as "a blob becoming a panel". Drips grow from 0 after 300 ms (`ease.viscous`, 1.6–2.4 s) | 140 ms `ease.exit`: scale → 0.97, opacity → 0; drips retract instantly |
| Inspector | x from +40 ou → 0 (`spring.soft`), opacity over 160 ms. Content staggered | x → +24, opacity → 0, 160 ms `ease.exit` |
| Modal | Scrim α 0 → 1 over 200 ms. Dialog scale 0.90 → 1 with `spring.gel`, plus a landing squash (sy 0.97) on arrival | Scrim fades over 160 ms; dialog scale → 0.96 and opacity → 0 over 140 ms |
| Popover / dropdown | Scale 0.92 → 1 from the anchor edge (`spring.gel`), opacity over 120 ms. The list's items stagger (12 ms) | 100 ms `ease.exit` |
| Tooltip | Delay 450 ms (300 ms "warm" if another tooltip closed < 1 s ago). Scale 0.96 → 1 and fade over 120 ms (`ease.enter`). The pointer tail grows 0 → 6 ou | Fade over 80 ms |
| Toast | Slides from the screen edge (x +48 → 0) with `spring.soft` plus a landing squash. Progress (auto-dismiss) bar drains linearly | x → +32, opacity → 0, 180 ms `ease.exit`; the stack below closes the gap with `spring.soft` |
| Sidebar collapse | Labels fade out over 90 ms first, then the width animates with `spring.soft`. Expand reverses (width first, labels fade in at 70% width) | — |
| Accordion | Height with `spring.soft` (content clipped, with an edge-fade mask); chevron rotates 90° over 160 ms | same, reversed |

### 4.7 Scrolling (C19)

| Aspect | Spec |
|---|---|
| Wheel | Target += notches × 72 ou. Visual position uses exponential smoothing `pos += (target − pos)·(1 − e^(−dt/τ))` with τ = 80 ms |
| Precision devices | Fractional deltas: τ = 40 ms |
| Drag (scrollbar thumb) | Direct (1 : 1), no smoothing |
| Overscroll | Rubber band: displayed offset `= L·d/(L + d)` with L = 48 ou. Returns with `spring.gel` |
| Programmatic scroll-to | `spring.lazy` (search results, keyboard focus into view). Focus scrolls keep 24 ou of margin |
| Scrollbar | A gel pill 6 ou wide (10 on hover) that fades in on scroll or hover and auto-hides after 900 ms idle (fade 240 ms) |
| Edge fades | 16 ou gradient masks in the container surface colour at the scrolled edges; α tied to scroll distance from the edge (0 → 1 over 24 ou) |

### 4.8 Drag and drop (HUD widgets, list reorder)

- **Pick up:** scale 1.03 (`spring.gel`), elevation → e3, opacity 0.96.
- **While dragging:** position follows the pointer **exactly** (no lag). Liquid feel comes from deformation: stretch-by-velocity with s_max 0.04, and tilt `θ = clamp(vx / 4000, −3°, +3°)` with `spring.gel`.
- **Snapping:** magnetic within 6 ou of a guide. The position eases into the guide with `spring.snappy` and the guide line fades in over 90 ms.
- **Drop:** a landing squash (0.97 / 1.03), recovered with `spring.wobble`; elevation back to e1 with `spring.soft`.

### 4.9 Text field (C09)

- **Focus:** outline animates from `outline.strong` 1.5 → focus colour 2.0 (90 ms); the sunken well brightens slightly.
- **Caret:** blinks with a 1.06 s period (530 ms on, 530 ms off, with 60 ms fades). The blink restarts after each keystroke.
- **Typing:** each inserted glyph scales from 0.85 → 1 over 90 ms (`ease.enter`). This is disabled in Reduced motion.

### 4.10 Loading indicators (C24)

- **BlobSpinner:** three droplets orbit a centre (period 1.2 s). Their radii pulse with phase offsets of 120°. Where they overlap they are drawn as a single merged silhouette: a smooth-min of circles, sampled into a polygon on the CPU each frame (64 points).
- **LiquidProgress:** the fill level follows progress with `spring.soft`. The fill surface has a sine wave (amplitude 1.5 ou, wavelength 40 ou, 2 s period); amplitude goes to 0 at 100%.
- **Skeleton:** a pearl block with a shimmer sweep every 1.4 s.

### 4.11 Drips (decorative)

Each drip runs its own lifecycle state machine. All parameters are seeded per drip.

```
IDLE (hidden or at rest length L_rest)
 → GROW:    L: L_rest → L_max over 2.4–4.0 s (ease.viscous)
 → HOLD:    1–3 s; bulb radius ±6% sine wobble (1.6 s period)
 → either   RETRACT: L → L_rest with spring.gel                       (70%)
     or     DETACH:  the bulb pinches off (the neck thins to 0 over 220 ms);
                     a droplet falls with gravity g = 1800 ou/s², fading after 120 ou;
                     the neck retracts with spring.wobble                (30%)
 → IDLE for 6–14 s
```

- Window-open drips grow from 0 to L_rest = 0.4·L_max, then begin the cycle.
- In **Reduced motion**, drips are static at L_rest with no cycle. With **Animations off**, the same.
- At most 3 drips per surface, and at most 8 animating drips on screen. Extra drips stay static.

### 4.12 Cursor-reactive ooze

- **Surface tension (Ultra, opt-in):** if the cursor is outside a panel but within 60 ou of an edge, that edge bulges towards the cursor. The displacement is `3·I·(1 − d/60)²` ou, with a Gaussian falloff along the edge (σ = 40 ou), smoothed by `spring.lazy`. It never applies to edges with drips or to small controls.
- **Ambient attraction (High and above, on by default on the main menu):** background blobs within 300 ou of the cursor are pulled towards it by up to 24 ou (`spring.lazy`).

## 5. Screen transitions

| Transition | Spec |
|---|---|
| Shell page → page | Outgoing content: opacity → 0 and y → −8 over 140 ms `ease.exit`. Incoming: opacity 0 → 1 and y +12 → 0 over 260 ms `ease.enter`, starting at +80 ms. Children are staggered. The sidebar and header persist without animating; only the header title crossfades |
| Vanilla screen → Ooze screen | Ooze background: crossfade from the previous frame's scrim colour over 200 ms. Window: open animation (§4.6) |
| Ooze screen → vanilla screen | Window close animation (140 ms), then `setScreen`. The handoff happens **after** the close animation completes, unless the user pressed a key that needs an immediate response (then it switches instantly) |
| Main menu → ClickGUI shell (out of world) | The background persists. Menu buttons stagger-collapse (scale 0.96, fade, 20 ms stagger, 120 ms each); the shell window opens |
| Background change | Crossfade over 600 ms `ease.standard`. Ambient layers persist across screens because the background renderer is a singleton |
| Theme change | Colour tokens interpolate in OKLab over 320 ms (DESIGN_SYSTEM §8.2) |

## 6. Intro sequence (S01)

Total about 1.6 s. Skippable with any key or click (fast-forward: 150 ms crossfade to the final state).

| t (ms) | Beat |
|---|---|
| 0–300 | The background fades in from the vanilla loading overlay colour to the Ooze background |
| 200–700 | A white droplet (the logo body) falls from above the screen to the centre with `ease.drop` |
| 700 | Impact: squash sx 1.25 / sy 0.78, recovered with `spring.wobble`. A ripple ring goes out to 3 × R |
| 750–1100 | The droplet morphs into the logo plate (rounded rect, `spring.gel` on width and height). The wordmark is revealed with a left-to-right liquid wipe: a clip edge with a sine-wave profile, amplitude 4 ou |
| 1000–1200 | A drip forms under the plate and detaches one droplet |
| 1100–1600 | Shared-element move: the plate translates and scales to the main menu logo slot (`spring.soft`) while the menu buttons stagger in |

- **Reduced motion:** a 400 ms crossfade from the background to the final main menu with the logo in place.
- **Animations off:** skipped.

## 7. Ambient background motion

Ambient layers are **pure functions of `ambientTime`**, so they are deterministic, frame-rate independent and never accumulate error. Each is seeded.

| Layer | Motion |
|---|---|
| Ambient blobs | Lissajous drift: `p(t) = c + (Aₓ·sin(ωₓt + φₓ), A_y·sin(ω_yt + φ_y))`, with A of 40–140 ou and periods of 40–90 s. Shape noise (DESIGN_SYSTEM §4.4) with speed 0.05 |
| Particles ("suspended droplets") | Rise at 4–14 ou/s with sway `x += 6·sin(t·0.7 + φ)` ou; wrap at the edges; α varies by depth |
| Animated gradient | Stops drift along their path with periods of 30–60 s; the angle rotates ±12° (period 80 s) |
| Parallax | Layer offset = cursorOffsetFromCentre × depth × 12 ou, smoothed with `spring.lazy` |

`speed` (the user's ambient speed, 0.1–3×) multiplies `ambientTime`. Ambient layers stop updating (they freeze, but stay visible) when the adaptive quality governor demotes to Low, or when the user turns off **Ambient motion**.

## 8. User controls

Settings live in `client.json → motion`:

| Setting | Values | Default | Effect |
|---|---|---|---|
| Animations | On / Off | On | **Off:** every transition is instant (`dur.instant`), springs snap, no deformation, no shimmer, ripple or drips cycle, no ambient motion. State changes remain visible through colour, position and shape |
| Reduced motion | Off / On / Auto | Auto | **Auto** turns on when Minecraft's *Distortion Effects* accessibility option is 0%. See the mapping table below |
| Intensity (`I`) | 0 – 150% | 100% | Scales deformation amplitudes (squash, stretch, landing, wobble), ripple and shimmer alpha, hover drip length and surface tension. Durations are not affected |
| Speed (`S`) | 0.5× – 2.0× | 1.0× | Tween durations ÷ S. Spring stiffness × S² (ω₀ × S); ζ unchanged |
| Ambient motion | On / Off | On | Background blobs, particles, animated gradient, parallax, drip cycles |
| Ambient speed | 0.1× – 3× | 1× | Multiplies ambientTime |

**Reduced-motion mapping**

| Pattern | Reduced alternative |
|---|---|
| Spring moves (indicator, knob, panels) | `spring.soft` without overshoot (ζ forced to 1.0); no stretch |
| Squash, stretch, landing, wobble | Disabled |
| Shimmer, ripple | Disabled. Pressed state is shown by darkening only |
| Page transitions | Crossfade only, 160 ms; no translation |
| Panel and modal open | Fade over 160 ms, scale 0.98 → 1 (tween) |
| Drips | Static, no cycle |
| Ambient layers | Frozen, unless the user explicitly turns Ambient motion on again |
| Scrolling smoothing | Kept (it improves comprehension), but overscroll bounce is disabled |
| Intro | 400 ms crossfade |
| Text field glyph pop | Disabled |

## 9. Performance safeguards

1. **Only awake animations are updated.** Each component registers its animated values with the frame's `Animator`; sleeping values cost nothing. If no animated value is awake and no input arrived, the screen paint still runs (Minecraft redraws every frame), but layout is skipped (cached).
2. **No allocation per frame in animation code.** Springs and tweens are mutable fields owned by components and reused when retargeted.
3. **Budgets per frame:** ≤ 300 awake animated values; ≤ 8 animating drips; ≤ 140 particles (Ultra); ≤ 6 ambient blobs.
4. **Adaptive quality** (PERFORMANCE_AND_ACCESSIBILITY §3) demotes effects before frame rate suffers. Order of shedding: surface tension → edge breathing → particles → shimmer → ambient blobs → drip cycles → secondary shadows.
5. **Photosensitivity:** nothing flashes faster than 3 Hz. Shimmer and ripple are low contrast (≤ 0.35 α) and never full-screen.
6. **Settled frames:** when every value is settled, Ooze skips re-tessellating animated shapes and reuses cached meshes (ARCHITECTURE §5.4).

## 10. Implementation reference

Animation runs in **Rust**: springs and tweens in Sol's engine (G4), motion tokens and their use in `ooze-ui`. Illustrative shape only; the engine API is Sol's:

```rust
pub enum MotionSpec { Spring { stiffness: f32, damping_ratio: f32 }, Tween { duration_ms: f32, easing: Easing } }

pub struct Animated<T> { /* value, velocity, target, spec, sleeping */ }
impl Animated<f32> {
    pub fn animate_to(&mut self, target: f32, spec: MotionSpec); // retargets, keeps velocity for springs
    pub fn snap_to(&mut self, target: f32);
    pub fn is_settled(&self) -> bool;
}

/// Resolves user settings once per frame: Off / Reduced / Speed, and intensity I (0 when reduced or off).
pub fn resolve(token: MotionToken, settings: &MotionSettings) -> MotionSpec;
```

Time comes from the frame header that Java sends (monotonic nanoseconds). Rust never reads wall-clock time for animation.

Components never hard-code numbers. They reference `MotionToken`s (for example `MotionToken::ToggleKnob` → `spring.gel`, `MotionToken::PressSquash` → tween 90 ms). `resolve` applies the user settings, so behaviour stays consistent and centrally tunable.
