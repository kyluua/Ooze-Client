# Ooze Client — Background Customisation Architecture

> Deliverable 6. Covers background types, the layer model, the image pipeline, procedural wallpapers, the legibility guard, the live editor and performance.

## 1. Requirements recap

Supported: solid colours, multi-colour gradients, custom local images, preset wallpapers, blur, animated gradients, moving particles, parallax, dimming, opacity and speed. Images can be scaled, positioned and cropped.

Every background must keep the white Ooze theme readable and attractive, and must have a live preview.

## 2. Contexts

A **context** is a place where a background is shown. Each context either has its own `BackgroundSpec` or `inherit`s from another context.

| Context | Default | Notes |
|---|---|---|
| `main_menu` | Wallpaper **Pearl Drift** | Also used by the intro and the out-of-world shell |
| `shell_menu` | `inherit: main_menu` | ClickGUI and other Ooze pages opened from the main menu |
| `shell_world` | **World view**: vanilla menu blur plus 30% `color.scrim` dim | ClickGUI and Ooze pages opened in a world. May be switched to any spec ("Hide world"), but the default keeps the world visible |
| `hud_editor` | World view with **no blur and no dim** | Outside a world: a preview of `main_menu` dimmed 20% |

## 3. Model

```jsonc
// config/oozeclient/backgrounds/backgrounds.json
{
  "schema": "oozeclient.backgrounds/1",
  "contexts": {
    "main_menu":   { "spec": "bg_7f3a" },
    "shell_menu":  { "inherit": "main_menu" },
    "shell_world": { "spec": "world_default" },
    "hud_editor":  { "spec": "world_clear" }
  },
  "specs": {
    "bg_7f3a": {
      "base": { "type": "image", "image": "lib:5e1b9c0a2f3d4e61", "fit": "fill",
                "focus": [0.42, 0.35], "zoom": 1.15, "letterbox": "#E9EDF5" },
      "effects": { "blur": 6, "dim": 0.15, "dimColor": "#171923", "opacity": 1.0,
                   "tint": null, "tintStrength": 0.0, "vignette": 0.25, "grain": 0.015 },
      "ambient": { "blobs": { "count": 3, "size": 1.0, "opacity": 0.25, "color": "{color.highlight.cool}", "attract": true },
                   "particles": { "density": 0.4, "size": 1.0, "speed": 1.0, "color": "#FFFFFF" },
                   "parallax": { "depth": 8 } },
      "speed": 1.0
    }
  },
  "library": [ { "id": "5e1b9c0a2f3d4e61", "name": "sunset.png", "file": "library/5e1b9c0a2f3d4e61.png",
                 "w": 3840, "h": 2160, "importedAt": "2026-11-08T12:00:00Z",
                 "stats": { "meanLuma": 0.61, "lumaGrid": "base64-8x8", "palette": ["#E7A86B", "…"] } } ]
}
```

### 3.1 Base types

| `type` | Parameters | Rendering |
|---|---|---|
| `solid` | `color` | One full-screen quad |
| `gradient` | `kind` (linear / radial), `angle` (deg, linear), `center` and `radius` (radial), `stops` [{pos, color}] (2–6). Interpolation is OKLab | A mesh grid, 24 columns × 14 rows. Colours are evaluated per vertex **on the CPU in OKLab**, so midpoints are not muddy. The grid adds split lines at stop positions for linear gradients |
| `image` | `image` (a library id or a bundled id), `fit` (fill / fit / stretch / center / tile), `focus` (fx, fy ∈ [0,1]), `zoom` (1–4), `tileScale`, `letterbox` colour | A textured quad with a computed source rect (§4.3) |
| `wallpaper` | `id`, `seed`, `params` (per wallpaper) | Procedural (§5) |
| `animated_gradient` | `stops` (2–6), `motion` (drift / rotate / breathe), `speed` | Mesh grid with time-varying per-vertex colours (MOTION_SPEC §7) |
| `world` | `blur` (uses vanilla menu blur), `dim` | Draws nothing over the world except the dim scrim. Blur goes through the platform adapter |

### 3.2 Effect layers (applied in order)

| Effect | Range | Notes |
|---|---|---|
| `blur` | 0–40 ou | For image and wallpaper bases: pre-blurred cached textures (§4.4). For `world`: vanilla menu blur via the adapter (ARCHITECTURE §11) |
| `opacity` | 0–1 | Base layer opacity over `color.canvas` |
| `tint` + `tintStrength` | colour, 0–1 | OKLab mix applied as an overlay quad at the given alpha |
| `dim` + `dimColor` | 0–0.85 | Overlay quad; the default colour is ink |
| `vignette` | 0–1 | A radial gradient ring from transparent (60% radius) to dimColor α `vignette × 0.5` at the corners |
| `grain` | 0–0.04 | A tiled blue-noise texture (64×64, generated at build time, MIT/CC0) drawn at 1 texel : 1 px. It also removes gradient banding (rule R7). Minimum 0.01 for gradients when Auto dither is on |

### 3.3 Ambient layers

| Layer | Parameters | Quality caps |
|---|---|---|
| `blobs` | count (0–6), size multiplier (0.5–2), opacity (0–0.6), colour, attract (bool) | Medium ≤ 4, High ≤ 6, Low = 0 |
| `particles` | density (0–1 → 0–140), size (0.5–2), speed (0.1–3), colour, direction (up / down / drift) | Medium ≤ 40, High ≤ 80, Ultra ≤ 140, Low = 0 |
| `parallax` | depth 0–24 ou | Off at Low. The image base gets depth × 0.5 and needs zoom ≥ 1 + 2·depth/viewport so edges never show. The editor auto-raises zoom and tells the user |

## 4. Image pipeline

### 4.1 Import paths

| Method | Behaviour |
|---|---|
| **Pick a file** button | A platform command (ARCHITECTURE §11.1): the Java adapter opens the native dialog through LWJGL **TinyFileDialogs** (not a graphics API) on a short-lived thread, and returns the path in the next snapshot (non-blocking UI; a spinner shows on the button). Filters: `*.png *.jpg *.jpeg *.bmp *.tga *.gif`. If tinyfd is unavailable on a target, the button is hidden (not disabled) |
| **Drag and drop** | Dropping files anywhere on the Background screen is handled via `Screen#onFilesDrop` through the adapter. A full-window drop zone overlay (dashed gel outline and "Drop to import") appears while files are dragged over the window, where the platform reports it; otherwise only on drop |
| **Backgrounds folder** | **Open folder** opens `config/oozeclient/backgrounds/inbox/`. Files placed there are imported automatically when the screen opens or **Rescan** is pressed |

### 4.2 Import processing (worker thread)

1. **Validate:** extension, size ≤ 40 MB, and the decoded header's dimensions ≤ 8192×8192.
2. **Hash:** SHA-256 of the bytes. The id is the first 16 hex characters. A duplicate import selects the existing entry.
3. **Copy:** the original bytes go to `config/oozeclient/backgrounds/library/<id>.<ext>`. Ooze always works from its own copy, so moving or deleting the original never breaks a background.
4. **Decode:** in the Rust engine (Sol, G5), to RGBA8. GIF: first frame only. The decoder crate is Sol's choice; supported formats must include PNG, JPEG, BMP, TGA and GIF.
5. **Normalise:** if the long edge > 4608 px, downscale with a high-quality box/Lanczos-2 filter to 4608. That covers 4K plus parallax and zoom headroom.
6. **Derive:**
   - **Mip chain:** box filter, down to 64 px.
   - **Thumbnail:** 320×180, cover-cropped, kept in memory and cached on disk in `thumbs/<id>.png`.
   - **Luminance stats:** mean luma and an 8×8 luma grid in Rec. 709 relative luminance.
   - **Palette:** 5 dominant colours (k-means++ in OKLab on a 64×36 downsample, 8 iterations). The editor offers them as *Suggested accent* and *Suggested tint*.
7. **Upload:** on the render thread via the platform renderer. A texture is created for the active mip levels only (§4.3).

Errors are reported inline in the import area, for example "Couldn't read *sunset.webp*. Ooze supports PNG, JPEG, BMP, TGA and GIF.". The library is not modified.

### 4.3 Fit, focus and crop

Given viewport size `V` (px), image size `I` and zoom `z`:

```
fill   : scale = max(Vw/Iw, Vh/Ih) · z
fit    : scale = min(Vw/Iw, Vh/Ih) · z      (letterbox colour fills the remainder)
stretch: independent x/y scale (zoom ignored)
center : scale = z (1 : 1 source pixels at zoom 1)
tile   : scale = tileScale, repeated (zoom ignored)

visible source size  S = V / scale
source origin        O = clamp(focus · I − focus · S, 0, I − S)   // keeps the focal point at the same relative screen position
```

- **Mip level:** the smallest level whose resolution is ≥ `S` in both axes, sampled with linear filtering. This avoids minification aliasing without trilinear sampling.
- **Upscaling notice (rule R6):** if `scale > 1.0` (source px per screen px < 1), the editor shows the chip **"Soft at this resolution: 1280×720 image on a 2560×1440 screen"**. Rendering stays linear. Ooze never claims a resolution it does not have.

### 4.4 Blur

- **Images and wallpapers:** a CPU separable blur approximating a Gaussian with **three box passes**, run on a worker. It works on a downscaled copy with factor `f = clamp(radius_px/4, 1, 8)` and radius `radius_px/f`. The result is cached per (image, blur bucket). Blur buckets are radius in ou rounded to 2, then converted to px with the current uiScale. It is drawn upscaled with linear filtering; blurred content does not reveal the upscale.
- While a new blur bucket is computing (dragging the slider), the nearest cached bucket is displayed. The slider never blocks.
- **World:** handled by the vanilla blur (ARCHITECTURE §11.3). If the target's blur radius cannot be controlled, Ooze uses Minecraft's *Menu Background Blur* option and the slider shows "Uses Minecraft's blur setting" with a link to the vanilla option.

### 4.5 Memory policy

- The **active** background plus at most one **transitioning** background stay resident. Anything else is released after its crossfade.
- **Budget: 67,108,864 bytes (64 MiB)** for all background textures: every resident mip level, blurred copies, thumbnails and the noise tile, including an outgoing crossfade.
  - It is enforced by the Rust image accountant and re-checked by the Java texture executor (PERFORMANCE_AND_ACCESSIBILITY §2.2).
  - Example: 3840×2160 with a full mip chain is ≈ 44,236,800 bytes; the half-resolution blurred copy with mips is ≈ 11,059,200; 9 thumbnails are 2,073,600; total ≈ 57.4 MiB.
  - During a crossfade, the outgoing image keeps only levels ≥ 1, so two 4K backgrounds never need 88 MiB.
- Thumbnails for the library grid load lazily as rows scroll into view.

## 5. Procedural wallpapers (built-in presets)

Wallpapers are **generated at the current resolution**, so they are always pixel-sharp. They cost nothing in JAR size and never upscale. Each is a function `(viewport, seed, params, ambientTime) → DrawList` built from mesh gradients, feathered blobs and particles.

| Id | Name | Description | Default params |
|---|---|---|---|
| `pearl_drift` | **Pearl Drift** (default) | A soft diagonal gradient `#F3F6FB → #DCE8F7 → #F8F9FF`, with 4 large white and cool blobs drifting slowly and sparse suspended droplets | blobs 4 @ 0.45 α, particles density 0.25 |
| `ink_tide` | Ink Tide | A deep ink gradient `#171923 → #262B3D` with pearl blobs glowing faintly (α 0.12). Pairs with Midnight Gel | blobs 3, particles 0.15 |
| `mint_melt` | Mint Melt | A pearl-to-mint wash `#F4FBF8 → #CDEFE2` with mint blobs | blobs 4 |
| `bubblegum_haze` | Bubblegum Haze | A pearl-to-pink wash `#FFF6FA → #F7D3E5` | blobs 4 |
| `frost_lattice` | Frost Lattice | A cool gradient with a faint hexagonal lattice of tiny droplets (r 2 ou, 56 ou pitch, α 0.18) that twinkle very slowly (period 8–20 s each, never in sync) | lattice on |
| `dripline` | Dripline | A gradient with an ooze frieze along the top edge: a continuous white gel band with 7–11 long drips that cycle through the drip lifecycle (MOTION_SPEC §4.11) at 0.5× speed | drips 9 |

Each wallpaper exposes 2–4 tunable params in the editor (for example the gradient colours, blob count and drip count), plus **Shuffle** (a new seed).

## 6. Rendering order

```
color.canvas  →  base (opacity)  →  tint  →  ambient blobs  →  particles (back half)
→  dim  →  vignette  →  grain  →  [UI content]  →  particles (front half, Ultra only, α ×0.5, never over text: clipped out of panel rects)
```

The whole background is drawn by a single `BackgroundRenderer` singleton. It persists across screens, so ambient motion continues seamlessly between the main menu, the shell and the editors.

## 7. Legibility guard

The guard guarantees readable UI on any background. It is **on** by default; turning it off requires confirming *Allow low-contrast backgrounds* in Advanced.

1. **Panels.** Panel opacity (`material.surfaceOpacity`) is clamped to ≥ 0.88 when the guard is on. Text on panels uses panel tokens, so contrast is set by the theme, not the background.
2. **Text directly on the background** (tagline, version string, copyright line, intro, HUD-editor hints) uses the **background-text treatment**:
   - Sample the background's luminance under the text's bounds:
     - images: the 8×8 luma grid, adjusted for fit, crop, dim, tint and blur;
     - gradients and solids: analytic;
     - wallpapers: their declared luminance profile;
     - world: unknown, so assume the worst case.
   - Choose the text colour: `text.primary` (ink) if the effective luma > 0.45, otherwise `text.inverse`.
   - Add a halo: an 8 ou blurred copy of the glyphs in the opposite colour at α 0.35 (TYPOGRAPHY §7).
   - If the computed contrast is still < 4.5 : 1 (busy images), add a local **gel plate**: a sunken-style pill behind the text with α 0.7, matching the surface class.
3. **Dim recommendation.** When the image's mean luma and contrast variance make UI-direct text fail, the editor shows "Busy image: text gets a backing plate" and offers **Add 20% dim**.
4. **Controls** always have outlines and elevation, so they stay distinguishable on any background.

## 8. Live editor (screen S06 summary)

- Full-screen live preview: the editor panel docks to the right (420 ou) over the actual background being edited. **Peek** (hold the button or hold `Space`) fades all UI out to 0 to see the whole background, and fades it back when released.
- A **context selector** (segmented: Main menu / Ooze menus / In-world / HUD editor) selects which context is being edited, plus the **Same as main menu** link toggle.
- A **mini-previews row** shows 160×90 frames of the other contexts, rendered at 1/8 scale with real components.
- **Changes are live**, with an undo/redo history (Ctrl+Z, Ctrl+Shift+Z or Ctrl+Y; 100 steps) and **Revert** (back to the state when the screen opened). Saving is automatic (debounced, ARCHITECTURE §9).
- **Focal point picker:** drag a crosshair on the thumbnail; the main preview updates live with `spring.lazy` smoothing.
- **Readability badge:** shows the legibility guard's status for the current spec ("Readable", "Readable with plate" or "Low contrast").

## 9. Modules (Rust: engine pieces by Sol, product pieces in `ooze-ui`; Java only executes texture work and file dialogs)

| Class | Responsibility |
|---|---|
| `BackgroundSpec` (record tree) | Immutable model plus codec |
| `BackgroundContext` (enum) | `MAIN_MENU`, `SHELL_MENU`, `SHELL_WORLD`, `HUD_EDITOR` |
| `BackgroundManager` | Resolves contexts, owns transitions (crossfade state) and saves through `ConfigStore` |
| `BackgroundRenderer` | Singleton. Paints the resolved spec into the DrawList each frame; owns ambient state |
| `BaseLayer` implementations | `SolidLayer`, `GradientLayer`, `ImageLayer`, `WallpaperLayer`, `AnimatedGradientLayer`, `WorldLayer` |
| `AmbientBlobs`, `ParticleField`, `Parallax` | Ambient layers; pure functions of time |
| `ImageLibrary` | Import, hash, copy, decode (through `PlatformImages`), mips, thumbnails, stats, palette, eviction |
| `BlurCache` | Worker-side box blur and bucketed cache |
| `WallpaperRegistry` | The built-in procedural wallpapers |
| `LegibilityGuard` | Luminance sampling, colour choice, plate decision, editor badges |
