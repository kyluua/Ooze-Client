# Ooze Client — Design System

> Deliverables 2 and 3: visual identity specification, colour and theme tokens.
> The canonical default theme file is [`tokens/ooze_classic.theme.json`](tokens/ooze_classic.theme.json). When this document and that file disagree, the JSON file wins, and this document must be updated.

**Notation**
- **ou** (Ooze unit) is the layout unit. 1 ou equals 1 physical pixel at the 1920×1080 reference size; it scales with resolution (§10).
- `token.name` refers to a theme token. `{palette.ink}@0.18` means the palette colour at 18% alpha.
- All sizes are in ou unless marked **px** (physical pixels).

---

## 1. Identity summary

Ooze looks like **a soft, glossy white gel with a crisp ink outline**, lit from the upper left, resting on a customisable backdrop. Active things are made of **Ink Gel**, a dark, faintly glossy version of the same material. Large surfaces drip slowly from their bottom edge.

Signature details, in priority order:
1. **The outline.** A continuous, smooth ink stroke around every gel surface. It makes Ooze legible on any background.
2. **The gloss band and glint.** An asymmetric capsule highlight near the top edge plus a tiny round glint. This is what makes surfaces read as "liquid" rather than "flat".
3. **Organic corners.** Superellipse corners rather than circular ones. No sharp corners anywhere in the chrome.
4. **Drips.** A few slow drips on window and panel bottoms, and a single tiny drip under hovered primary buttons.
5. **Elastic motion.** See [MOTION_SPEC.md](MOTION_SPEC.md).

## 2. Brand marks

### 2.1 Droplet mark

The mark is built from vector primitives drawn by the Ooze renderer. It is never a bitmap. Geometry is relative to the mark radius `R`:

| Part | Geometry |
|---|---|
| Body | Superellipse blob centred at (0, 0), half-width `1.00R`, half-height `0.92R`, exponent 2.4. Fill uses the White Ooze material (§3) |
| Drip | Hangs from the body's bottom at x = `+0.24R`. Neck width `0.22R`, length `0.55R`, bulb radius `0.20R`, fillet `0.12R` (§4.5) |
| Outline | `0.09R` ink stroke around the body and drip as one continuous path |
| Gloss | Capsule `0.50R × 0.18R`, rotated −25°, centred at (−0.38R, −0.48R), white with α 0.9 fading to 0 |
| Glint | Circle of radius `0.07R` at (+0.05R, −0.62R), white α 0.95 |

### 2.2 Wordmark

- Lowercase **"ooze"** in Bubblegum Sans at the Display role, letter spacing −0.01 em, colour `color.text.primary`.
- The optional descriptor **"client"** uses the Caption role in uppercase, tracking +0.12 em, colour `color.text.secondary`. It is baseline-aligned to the wordmark and placed after a gap of 0.25 em.
- **Lockup:** mark height = 1.15 × wordmark cap height, with a gap of 0.35 × mark width between them.
- **Clear space:** at least 0.5 × mark height on every side.
- **Minimum size:** mark 16 ou tall; full lockup 96 ou wide.
- **Logo plate:** on the main menu the lockup sits on a White Ooze plate (radius `xl`) with two drips.
- **Do not:** recolour the outline to something other than ink (or the theme's outline token), add eyes or a face, stretch it, add drop shadows beyond the elevation tokens, or animate it except as described in MOTION_SPEC §6.

## 3. The Ooze Material

Every gel surface is drawn as an ordered layer stack. The renderer exposes a single `OozeMaterialPainter.paint(shape, MaterialSpec, state)` that composes these layers (see [ARCHITECTURE.md §5.5](ARCHITECTURE.md)).

| # | Layer | White Ooze (default) | Ink Gel |
|---|---|---|---|
| 1 | Ambient shadow | `elevation.eN.ambient` in `color.shadow` | same |
| 2 | Contact shadow | `elevation.eN.contact` | same |
| 3 | Body fill | Vertical 2-stop gradient: `color.surface.gradientTop` → `color.surface.gradientBottom` (for Raised use `surface.raised` → `surface.gradientBottom`) | `#2B2F3F` → `color.accent.base` (derived: accent at OKLab L+0.08 → accent) |
| 4 | Inner shade | Inside the bottom edge, a ring that fades from `color.highlight.cool` at α `material.innerShade.opacity` to 0 over `material.innerShade.feather`. Masked vertically: 0 at 45% height, 1 at the bottom | Accent at L+0.12, α 0.35 |
| 5 | Gloss band (controls ≤ 72 tall) | Capsule from 10% to 78% of the width. Top inset = outline width + 2. Height = clamp(0.30 h, 4, 22). Vertical gradient `color.highlight.gloss` α `material.gloss.opacity` → 0 | Same shape, α 0.28 |
| 5′ | Rim light (surfaces > 72 tall) | An inner stroke `material.rim.width` inset 1.5 from the outline. α `material.rim.opacity` at the top edge, fading to 0 at 40% of height. Plus a **corner glint**: a 2-wide arc along the top-left corner curve, spanning 60° of the corner | α 0.25 |
| 6 | Glint (controls 32–72 tall) | Circle, radius clamp(0.07 h, 1.5, 4), right of the gloss capsule's end with a gap equal to its radius, vertically centred on the capsule. White α 0.95 | α 0.6 |
| 7 | Outline | Stroke centred on the shape edge. Width from `shape.outline.*` × `shape.outlineScale`, colour `color.outline.strong` | `color.outline.strong` |
| 8 | Drips | Part of the same path as the body, so the fill and outline are continuous (§4.5) | same |

Surface classes:

| Class | Fill | Outline | Default elevation | Examples |
|---|---|---|---|---|
| `control` | White Ooze (base) | `shape.outline.control` | e1 | Secondary button, field, dropdown, chip |
| `card` | White Ooze (raised) | `shape.outline.control` | e1 | Module card, setting group card |
| `panel` | White Ooze (base) at α `material.surfaceOpacity` | `shape.outline.panel` | e4 (window), e3 (floating) | ClickGUI window, inspector, logo plate |
| `sunken` | Flat `color.surface.sunken`, no gloss, no shadow | none, or `outline.soft` hairline | e0 | Sidebar well, slider track, toggle track (off), search field background |
| `overlay` | White Ooze (raised), opaque | `shape.outline.panel` | e3 | Popover, dropdown list, modal |
| `inverse` | Ink Gel | `shape.outline.control`, same ink | e2 | Primary button, tooltip, active nav blob (when accent ≠ white) |

Rules:
- The gloss is **always** at the top and offset towards the upper-left light. It never sits at the bottom.
- Sunken areas never get gloss or shadow; they read as "pressed into" the gel.
- Translucent panels (α < 1) must stay above the minimum allowed by the legibility guard (BACKGROUNDS §7).
- Text never sits directly on a gloss band. Layout keeps the label baseline below the band; for very small controls the band is clipped to the label's top.

## 4. Shape language

### 4.1 Corner model

Corners are **superellipse quadrants**. The radius token is the corner's extent along each edge. The exponent is `shape.cornerExponent` (default **2.6**; 2.0 gives a circular arc). For a corner with centre `c` and extent `r`, sample θ ∈ [0, π/2]:

```
x = c.x + sign · r · cos(θ)^(2/n)
y = c.y + sign · r · sin(θ)^(2/n)
```

- **Pill shapes** (r = h/2) always use n = 2, so their ends are true semicircles.
- **Segment count:** `segments = clamp(ceil((π/2) / acos(1 − 0.15 / r_px)), 4, 64)`, giving a chord error ≤ 0.15 px.
- If r exceeds min(w, h)/2, it is clamped to that value.

### 4.2 Radius tokens

| Token | Value | Use |
|---|---|---|
| `shape.radius.xs` | 6 | Chips inside dense rows, checkbox |
| `shape.radius.sm` | 10 | Text fields, small buttons, tooltips |
| `shape.radius.md` | 14 | Buttons M/L, dropdowns, list rows |
| `shape.radius.lg` | 20 | Cards, sidebar well, inspector |
| `shape.radius.xl` | 28 | Windows, modals, logo plate |
| `pill` | h/2 | Toggles, segmented controls, badges, XL main menu buttons |

Nested surfaces use **concentric radii**: inner radius = max(outer radius − inset, `xs`).

### 4.3 Outlines

| Token | Width | Use |
|---|---|---|
| `shape.outline.hairline` | 1.0 | Dividers, sunken-region borders, focus halo |
| `shape.outline.control` | 1.5 | Controls, cards, chips |
| `shape.outline.panel` | 2.0 | Windows, panels, overlays, logo |
| `shape.outline.heavy` | 3.0 | High-contrast theme, selected card |

`shape.outlineScale` (0–1.5, default 1) multiplies all outline widths. At 0 the theme becomes outline-free ("Soft" style), and elevation e1 becomes the minimum for every non-sunken surface so separation is kept.

Rendered width in px = max(1, round(width × outlineScale × uiScale)). This keeps the outline crisp at every scale (rule R4).

### 4.4 Blobs

Decorative and morphing blobs (background, loaders, indicator transitions) are closed **centripetal Catmull-Rom splines** through `N = 8` radial control points:

```
r_i(t) = R · (1 + a · noise(seed + i, t · speed)),   a ∈ [0, 0.12]
```

Blobs are star-shaped about their centre, so the tessellator fills them with a fan.

### 4.5 Drips

A drip is a downward excursion of a surface's bottom edge, built into the surface's path so the fill and outline are seamless.

| Parameter | Token | Range |
|---|---|---|
| Neck width | `drip.neck` | 6 – 12 |
| Length (edge to bulb bottom) | `drip.length` | 8 – 26 (animated) |
| Bulb radius | `drip.bulb` | 4 – 8; always ≥ neck/2 + 1 |
| Fillet radius | `drip.fillet` | 6 |

Path construction, left to right along the bottom edge at drip centre `x0`:
1. Run along the edge to `x0 − neck/2 − fillet`.
2. Quadratic fillet curve into the neck's left side.
3. Go down to the bulb's tangent point.
4. Circular arc around the bulb, from the left tangent through the bottom to the right tangent.
5. Go up the neck's right side.
6. Mirror the fillet back to the edge.

The region below the edge is **y-monotone**, so it is triangulated as a monotone strip. It overlaps 1 ou into the body to avoid seams.

Placement rules:
- Bottom edges only, at least `2·fillet + neck` away from the corner curves, and at least 48 ou apart.
- At most `material.drips.maxPerSurface` per surface (default 3).
- Positions are seeded from a hash of the surface's stable id, so they are identical across frames and sessions.
- Drips are decorative. They are never hit-tested and never overlap interactive content below them (the layout reserves `drip.length.max` of bottom margin under dripping surfaces).

### 4.6 Breathing edges (Ultra quality only)

The edge is displaced along its normal by `0.6 · sin(2π·s/λ + t·ω)`, where `s` is perimeter distance, `λ` = 140 and the period is 4–7 s. Displacement is zero on corners and on edges that touch other surfaces.

## 5. Light and gloss

- There is one light source, at the upper left (30° from vertical). Everything about gloss placement follows from it.
- Shadows drop **straight down** (y offset only) to keep layouts symmetric.
- **Shimmer:** on hover-enter, a soft diagonal band (30% of the width, white α `material.shimmer.opacity`) sweeps across the gloss region (MOTION_SPEC §4.1).
- **Specular parallax (High and above):** the gloss band shifts up to 2 ou towards the cursor while the cursor is over the control.

## 6. Elevation and shadows

Each level has two shadows in `color.shadow`. Values are y-offset / blur / alpha.

| Token | Ambient | Contact | Use |
|---|---|---|---|
| `elevation.e0` | — | — | Sunken, inline |
| `elevation.e1` | 3 / 10 / 0.10 | 1 / 2 / 0.12 | Resting controls and cards |
| `elevation.e2` | 6 / 18 / 0.12 | 2 / 4 / 0.12 | Hovered controls, selected card, Ink Gel |
| `elevation.e3` | 12 / 30 / 0.16 | 3 / 8 / 0.12 | Popovers, dropdowns, tooltips, toasts, dragged items |
| `elevation.e4` | 24 / 56 / 0.22 | 6 / 14 / 0.12 | Windows, modals |

**Rendering.** A shadow is the shape offset by y, with a solid core inset by blur/2 and a fringe out to +blur/2. Five rings with α multipliers `1.0, 0.84, 0.5, 0.16, 0.0` approximate a Gaussian. Rounded-rect rings use radius `r ± offset`, clamped at 0. Quality tier Low draws only the ambient shadow, using three rings.

## 7. Colour

### 7.1 Palette (defaults, all editable)

| Token | Hex | Role |
|---|---|---|
| `palette.oozeWhite` | `#F8F9FF` | Base gel surface |
| `palette.pearlWhite` | `#E9EDF5` | Sunken surfaces, tracks |
| `palette.frostWhite` | `#FFFFFF` | Raised surfaces, gloss, gradient top |
| `palette.coolHighlight` | `#DCE8F7` | Inner shade, selection, soft accent |
| `palette.mist` | `#EEF2F8` | Body gradient bottom |
| `palette.ink` | `#171923` | Outline, primary text, Ink Gel |
| `palette.inkSecondary` | `#626977` | Secondary text |
| `palette.inkMuted` | `#9AA0AC` | Disabled text and icons |
| `palette.shadowInk` | `#1B2133` | Shadow colour |
| `palette.signalBlue` | `#2F6FE4` | Focus ring, links, info |
| `palette.mint` | `#1A7F5A` | Success |
| `palette.amber` | `#9A6100` | Warning text and icons |
| `palette.amberBright` | `#F2B33D` | Warning fill accents |
| `palette.coral` | `#C93A4E` | Danger |

### 7.2 Semantic tokens

| Token | Default |
|---|---|
| `color.canvas` | `{palette.pearlWhite}` (painted behind any background, and shown if a background fails to load) |
| `color.surface.base` | `{palette.oozeWhite}` |
| `color.surface.raised` | `{palette.frostWhite}` |
| `color.surface.sunken` | `{palette.pearlWhite}` |
| `color.surface.overlay` | `{palette.frostWhite}` |
| `color.surface.inverse` | `{palette.ink}` |
| `color.surface.gradientTop` | `{palette.frostWhite}` |
| `color.surface.gradientBottom` | `{palette.mist}` |
| `color.scrim` | `{palette.ink}@0.40` |
| `color.outline.strong` | `{palette.ink}@0.92` |
| `color.outline.soft` | `{palette.ink}@0.16` |
| `color.outline.focus` | `{palette.signalBlue}` |
| `color.outline.focusHalo` | `{palette.frostWhite}` |
| `color.text.primary` | `{palette.ink}` |
| `color.text.secondary` | `{palette.inkSecondary}` |
| `color.text.disabled` | `{palette.inkMuted}` |
| `color.text.onAccent` | `{palette.frostWhite}` |
| `color.text.inverse` | `{palette.oozeWhite}` |
| `color.text.link` | `{palette.signalBlue}` |
| `color.accent.base` | `{palette.ink}` (monochrome default: Ink Gel) |
| `color.accent.soft` | `{palette.coolHighlight}` |
| `color.highlight.gloss` | `{palette.frostWhite}` |
| `color.highlight.cool` | `{palette.coolHighlight}` |
| `color.selection.text` | `{palette.coolHighlight}` |
| `color.shadow` | `{palette.shadowInk}` |
| `color.status.success` / `.successSoft` | `{palette.mint}` / `#D8F0E6` |
| `color.status.warning` / `.warningSoft` | `{palette.amber}` / `#FBEBC8` |
| `color.status.danger` / `.dangerSoft` | `{palette.coral}` / `#F8DCE0` |
| `color.status.info` / `.infoSoft` | `{palette.signalBlue}` / `#DCE7FB` |

### 7.3 Verified contrast (default theme)

| Foreground | Background | Ratio | Requirement |
|---|---|---|---|
| `text.primary` #171923 | surface.base #F8F9FF | 16.7 : 1 | AA ✓ AAA ✓ |
| `text.secondary` #626977 | surface.base #F8F9FF | 5.25 : 1 | AA ✓ |
| `text.secondary` #626977 | surface.raised #FFFFFF | 5.5 : 1 | AA ✓ |
| `text.secondary` #626977 | surface.sunken #E9EDF5 | 4.70 : 1 | AA ✓ (minimum, do not darken the sunken surface further) |
| success #1A7F5A | #FFFFFF | 5.0 : 1 | AA ✓ |
| warning #9A6100 | #FFFFFF | 5.1 : 1 | AA ✓ |
| danger #C93A4E | #FFFFFF | 5.0 : 1 | AA ✓ |
| focus #2F6FE4 | #FFFFFF | 4.65 : 1 | Non-text ≥ 3 : 1 ✓ |
| `text.onAccent` #FFFFFF | accent #171923 | 17.4 : 1 | AA ✓ |

The Theme Studio recomputes these pairs live (TYPOGRAPHY §8, SCREENS S05).

### 7.4 State colour rules

States are expressed through the material first (inflation, elevation, gloss) and colour second. Derived colours are computed in **OKLab** (Björn Ottosson's transform) and cached when a theme loads.

| State | White Ooze | Ink Gel / accent |
|---|---|---|
| Hover | Body gradient top → `surface.raised`; elevation +1; gloss α ×1.1 | Body L +0.05 |
| Pressed | Body mixed 30% towards `surface.sunken`; elevation −1 (e1 → e0.5, interpolated) | Body L −0.04 |
| Selected | Outline → `shape.outline.heavy` in `accent.base`; elevation e2 | — |
| Disabled | Whole component α 0.45; outline → `outline.soft`; no gloss or shadow | same |
| Focus-visible | Focus ring (COMPONENTS §1.3) | same |

### 7.5 Accent presets

Changing the accent recolours Ink Gel everywhere. Outlines stay ink in light themes so the identity is kept.

| Preset | `color.accent.base` | `color.accent.soft` | `text.onAccent` |
|---|---|---|---|
| Ink (default) | `#171923` | `#DCE8F7` | `#FFFFFF` |
| Glacier | `#2F6FE4` | `#DCE7FB` | `#FFFFFF` |
| Mint | `#1A7F5A` | `#D5F2E8` | `#FFFFFF` |
| Bubblegum | `#C2407F` | `#FADCEB` | `#FFFFFF` |
| Lemon | `#F2C94C` | `#FBF0CC` | `#171923` (auto-picked, see below) |

`text.onAccent` is **auto-derived**: whichever of `#FFFFFF` or `palette.ink` has the higher contrast against the accent. A user can override it only if their choice still gives ≥ 4.5 : 1.

### 7.6 Built-in themes

| Id | Name | Summary |
|---|---|---|
| `ooze_classic` | Ooze Classic | Default. White Ooze, ink outline, Ink Gel accent |
| `midnight_gel` | Midnight Gel | Dark variant. Surfaces `#1C1F2B` / raised `#252938` / sunken `#151823`; outline `#F8F9FF@0.85`; text `#F3F5FB` / `#A3AABA`; accent `#F8F9FF` (white goo is the active substance); gloss α 0.35; shadows `#000000` with alpha ×1.6 |
| `mint_melt` | Mint Melt | Classic with the Mint accent and coolHighlight `#D5F2E8` |
| `bubblegum_pop` | Bubblegum Pop | Classic with the Bubblegum accent and coolHighlight `#FADCEB` |
| `high_contrast` | High Contrast | Outline `heavy` at ink 100%, `text.secondary` `#3D4350`, panels opaque, gloss α 0.4, no shimmer, focus ring 3 ou |

## 8. Theme model

### 8.1 File format

Themes are JSON (schema id `oozeclient.theme/1`). See the default: [`tokens/ooze_classic.theme.json`](tokens/ooze_classic.theme.json).

```jsonc
{
  "schema": "oozeclient.theme/1",
  "id": "mint_copy",                 // [a-z0-9_]{1,48}, unique
  "name": "Mint Copy",
  "author": "you",
  "mode": "light",                   // light | dark (used for the legibility guard and defaults)
  "extends": "ooze_classic",         // optional parent; missing tokens are inherited
  "palette":   { "coolHighlight": "#D5F2E8" },
  "color":     { "accent.base": "#1A7F5A" },
  "shape":     { "radius.lg": 20, "cornerExponent": 2.6, "outlineScale": 1.0 },
  "material":  { "gloss.opacity": 0.85 },
  "elevation": { },
  "typography":{ "family": null }    // optional font suggestion; user font settings win
}
```

**Value grammar**
- Colour: `#RRGGBB`, `#RRGGBBAA`, a reference `{group.token}`, or a reference with alpha `{palette.ink}@0.16`.
- Number: a JSON number.
- Elevation: `{ "ambient": [y, blur, alpha], "contact": [y, blur, alpha] }`.
- References resolve after inheritance. Cycles and unknown references are errors: log them and fall back to the parent theme's value.

### 8.2 Loading and runtime

1. Built-in themes come from `assets/oozeclient/themes/*.theme.json`. User themes come from `config/oozeclient/themes/*.theme.json`.
2. Each theme is resolved into an immutable `ResolvedTheme`. Colours are stored as a packed-ARGB `int[]` indexed by `ThemeToken` ordinal (no map lookups while rendering).
3. Validation produces warnings, which the Theme Studio shows: contrast failures for every pair in §7.3, out-of-range numbers, unknown keys. Unknown keys are kept on save for forward compatibility.
4. **Theme switch:** every colour token is interpolated in OKLab over 320 ms (`ease.standard`). Shape tokens switch instantly at the midpoint, under a crossfade.
5. Built-in themes are read-only. "Duplicate" creates a user theme with `extends` set.

## 9. Space, size and layout

### 9.1 Scale model

```
fbW, fbH          framebuffer size in physical px
autoScale       = clamp(min(fbW / 1920, fbH / 1080), 0.75, 4.0)
uiScale         = autoScale × userUiScale            (userUiScale 0.75–1.50, default 1.00)
1 ou            = uiScale px
viewport (ou)   = (fbW / uiScale) × (fbH / uiScale)
ou → MC GUI     = uiScale / mcGuiScale               (applied as a matrix scale by the screen host)
```

| Screen | autoScale | Viewport (ou) |
|---|---|---|
| 1920×1080 | 1.000 | 1920 × 1080 |
| 2560×1440 | 1.333 | 1920 × 1080 |
| 3840×2160 | 2.000 | 1920 × 1080 |
| 1280×720 window | 0.75 (clamped) | 1707 × 960 |
| 3440×1440 ultrawide | 1.333 | 2580 × 1080 |

Ooze layout never depends on Minecraft's GUI scale. The vanilla GUI scale applies only to vanilla elements.

### 9.2 Breakpoints (viewport width in ou)

| Class | Width | Behaviour |
|---|---|---|
| Compact | < 1360 | Sidebar collapsed to its rail by default; inspector becomes an overlay sheet; single-column main menu |
| Regular | 1360 – 2199 | Reference layouts |
| Wide | ≥ 2200 | Content max-width 1760, centred; extra columns in card grids |

### 9.3 Spacing tokens (4-ou grid)

| Token | ou | | Token | ou |
|---|---|---|---|---|
| `space.half` | 2 | | `space.6` | 24 |
| `space.1` | 4 | | `space.8` | 32 |
| `space.2` | 8 | | `space.10` | 40 |
| `space.3` | 12 | | `space.12` | 48 |
| `space.4` | 16 | | `space.16` | 64 |
| `space.5` | 20 | | `space.20` | 80 |

### 9.4 Size tokens

| Token | ou | Use |
|---|---|---|
| `size.control.sm` | 32 | Compact buttons, chips, small fields |
| `size.control.md` | 40 | Default controls |
| `size.control.lg` | 48 | Prominent buttons, search field |
| `size.control.xl` | 64 | Main menu buttons |
| `size.hit.min` | 32 | Minimum hit target (invisible padding is added if the visual is smaller) |
| `size.icon.sm/md/lg/xl` | 16 / 20 / 24 / 32 | Icons |

### 9.5 Layering (z-order)

`background (0) → content (100) → sticky headers (200) → inspector (300) → popover/dropdown (400) → modal (500) → toast (600) → tooltip (700) → drag ghost (800) → debug overlay (900)`

## 10. Iconography

- **Style:** rounded stroke icons on a 24-ou grid with 2 ou padding. Stroke is 1.75 at 24 ou and scales with size, with a minimum of 1.25 px. Caps and joins are round.
- **Source:** generic glyphs come from **Lucide** (ISC licence, attribution in About and `licenses/`). Brand glyphs (droplet, drip, blob, ooze-wordmark) are original Ooze artwork.
- **Pipeline:** SVGs in `assets-src/icons/` are compiled at build time by the `iconCompile` Gradle task. The task converts `circle`, `rect`, `line`, `polyline`, `polygon`, `ellipse` and arcs into absolute `M/L/C/Q/Z` path commands in `assets/oozeclient/icons/icons.json`. At runtime icons are flattened and stroked at the current physical scale (tolerance 0.15 px) and cached per (icon, size, scale).
- **Colour:** icons inherit the current text colour unless a component specifies otherwise.
- **Required set (v0.1):** search, x, arrow-left, chevron-left/right/up/down, check, plus, minus, settings, palette, image, type, waves, layout-dashboard, keyboard, info, star, star-filled, layout-grid, list, filter, arrow-up-down, undo-2, redo-2, rotate-ccw, trash-2, copy, download, upload, folder-open, eye, eye-off, lock, external-link, play, users, cloud, globe, accessibility, power, package, user, bell, triangle-alert, circle-check, circle-x, circle-alert, grip-vertical, maximize-2, minimize-2, pin, sun, moon, magnet, grid-3x3, crosshair, command, mouse, sparkles, droplet, history, panel-left, sliders-horizontal.

## 11. Rendering quality contract

These rules are non-negotiable. [TESTING.md §5](TESTING.md) verifies them.

| # | Rule |
|---|---|
| R1 | Every edge is anti-aliased with an analytic fringe exactly 1 px wide. No stair-stepped edges anywhere. |
| R2 | No bitmap UI chrome. Shapes, outlines, icons, the logo, drips and blobs are tessellated at the current physical scale. |
| R3 | At rest, text glyphs are rasterized at the exact physical pixel size and drawn 1 texel : 1 px (TYPOGRAPHY §5). |
| R4 | Axis-aligned edges and straight outline segments are snapped to the physical pixel grid. Stroke widths are whole pixels, at least 1 px. |
| R5 | Curve flattening tolerance is ≤ 0.15 px. |
| R6 | Images are never magnified above 1.0× their source density without the UI showing a quality notice. Minification uses mip selection plus linear filtering. |
| R7 | Large gradients are dithered (a blue-noise overlay at α ≤ 2.5%) so they show no visible banding. |
| R8 | Layout is computed in ou, then converted to px and snapped once at paint time. |
| R9 | A change of physical scale (resolution, UI scale or MC GUI scale) invalidates tessellation and glyph caches. Cached meshes are never scaled across scales. |
| R10 | Same-colour shapes that touch are drawn as one path, so no AA seam appears between them. |
| R11 | Ooze UI size is independent of Minecraft's GUI scale. |
| R12 | Quality is verified at 1920×1080, 2560×1440, 3840×2160 and a 1280×720 window. |

## 12. Do and don't

| Do | Don't |
|---|---|
| Keep text on gel surfaces, or protect it with the legibility treatment | Put body text directly on a busy wallpaper |
| Use one accent colour; express state through material | Introduce extra brand colours or rainbow gradients |
| Give large surfaces 1–3 subtle drips | Add drips to small controls (except the hover drip on primary buttons), or put drips on top or side edges |
| Use concentric radii for nested surfaces | Nest sharp-cornered elements inside gel |
| Let motion respond to input | Animate idle UI except through ambient background layers |
| Use spacing tokens and the 4-ou grid | Use arbitrary pixel offsets |
| Label sample content **Sample** | Show placeholders as if they were finished features |
