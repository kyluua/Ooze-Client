# Ooze Client — Typography System

> Deliverable 4. Covers the font registry, type roles, styles and fallbacks, rasterization, caching, measurement, user customisation and licensing.

## 1. Goals

1. **Bubblegum Sans by default.** It sets the playful-premium tone, set with careful spacing so it reads as deliberate rather than casual.
2. **Razor-sharp text at every resolution.** Glyphs are rasterized for the actual physical pixel size and drawn 1 : 1 at rest.
3. **Swappable fonts that keep the layout stable.** Size normalisation means switching from Bubblegum Sans to Inter does not break layouts.
4. **Graceful styles.** When a font lacks a weight or italic, Ooze picks the closest real face, synthesises the style only where that looks acceptable, and tells the user when it has done so.
5. **Licences honoured.** Fonts ship unmodified with their licence files and visible attribution.

## 2. Bundled fonts

All four families are licensed under the **SIL Open Font License 1.1**. Download **static TTF instances** from each project's upstream release or the Google Fonts family download (`static/` folder).

> **Do not subset, rename or otherwise modify the font files.** Under the OFL that counts as modification, which conflicts with any Reserved Font Name. Copy each family's `OFL.txt` verbatim from the downloaded package and record the exact copyright line in `fonts.json`.

| Family | Designer (verify against OFL.txt) | Faces to bundle | Category | Notes |
|---|---|---|---|---|
| **Bubblegum Sans** | Sudtipos | Regular 400 | display / rounded | The only weight; no italic. Latin coverage only |
| **Inter** | Rasmus Andersson | 400, 500, 600, 700, Italic 400, Italic 700 | sans | Broad Latin, Greek and Cyrillic coverage. **First fallback** for missing glyphs |
| **Caveat** | Impallari Type | 400, 500, 600, 700 | script | No italic; italic requests map to upright (never synthesised) |
| **Cormorant Garamond** | Christian Thalmann (Catharsis Fonts) | 400, 500, 600, 700, Italic 400, Italic 700 | serif | Elegant display serif; its small sizes need +4% size adjustment (handled by §4.3) |

**Why static instances?** Static faces make weights deterministic regardless of the Rust rasterizer's variable-font support (§5), and keep the OFL files unmodified.

**Jar budget:** about 3 MB of fonts in total. That is acceptable for v0.1.

### 2.1 Resource layout

```
assets/oozeclient/fonts/
├── fonts.json                       # registry manifest (below)
├── bubblegum_sans/ BubblegumSans-Regular.ttf, OFL.txt
├── inter/          Inter-Regular.ttf … Inter-BoldItalic.ttf, OFL.txt (LICENSE.txt)
├── caveat/         Caveat-Regular.ttf … Caveat-Bold.ttf, OFL.txt
└── cormorant_garamond/ CormorantGaramond-Regular.ttf … , OFL.txt
```

### 2.2 Registry manifest (`fonts.json`)

```jsonc
{
  "schema": "oozeclient.fonts/1",
  "families": [
    {
      "id": "bubblegum_sans",
      "name": "Bubblegum Sans",
      "category": "display",
      "license": { "id": "OFL-1.1", "file": "bubblegum_sans/OFL.txt",
                   "copyright": "<copied verbatim from OFL.txt>" },
      "faces": [ { "weight": 400, "italic": false, "file": "bubblegum_sans/BubblegumSans-Regular.ttf" } ],
      "synthesis": { "bold": true, "italic": true },   // allowed, but see §4.2 defaults
      "sizeAdjust": "auto",                            // or a number, e.g. 1.04
      "fallback": ["inter"]
    }
  ]
}
```

User fonts placed in `config/oozeclient/fonts/` (`.ttf`, `.otf` with TrueType or CFF outlines) are scanned at startup and when the user clicks **Rescan**. Faces are grouped into families by their name table (nameID 16/17, falling back to 1/2). Their licence is shown as "Provided by you".

## 3. Type roles

Default metrics are for Bubblegum Sans at text scale 1.0. Sizes and line heights are in ou. Tracking is in em.

| Role | Size | Line height | Tracking | Weight requested | Case | Default colour | Typical use |
|---|---|---|---|---|---|---|---|
| Display | 64 | 72 | −0.010 | 700 | as written | text.primary | Intro wordmark, About hero |
| Headline | 44 | 52 | −0.005 | 700 | | text.primary | Main-menu greeting, empty-state titles |
| Title | 30 | 36 | 0 | 600 | | text.primary | Page titles in the shell header |
| Subtitle | 22 | 28 | 0 | 500 | | text.secondary | Page subtitles, modal titles |
| Section Heading | 15 | 20 | +0.06 | 600 | UPPERCASE | text.secondary | Group headers in settings and sidebar |
| Body Large | 19 | 27 | +0.005 | 400 | | text.primary | Module names, prominent rows |
| Body Medium | 16 | 23 | +0.010 | 400 | | text.primary | Default text |
| Body Small | 14 | 20 | +0.015 | 400 | | text.secondary | Descriptions, helper text |
| Caption | 12 | 16 | +0.020 | 400 | | text.secondary | Metadata, version strings |
| Label | 14 | 18 | +0.020 | 500 | | text.primary | Control labels, chips |
| Button | 17 | 20 | +0.010 | 600 | | (component) | Button text |
| Tooltip | 14 | 19 | +0.010 | 400 | | text.inverse | Tooltips |
| Status | 12 | 15 | +0.030 | 600 | | (status colour) | Badges, toasts' meta, the SAMPLE badge |

**Weight mapping for Bubblegum Sans.** Bubblegum Sans has only a 400 face. When the family is Bubblegum Sans, role weights resolve to 400 **without synthesis**, because the face already reads as bold. Synthesis only applies when the user explicitly picks the **Bold** or **Italic** style for a role (§4.2).

**Minimum size.** No role renders below **11 ou** after text scale is applied. Sizes are clamped and the Typography editor shows a notice.

## 4. Styles, matching and fallbacks

### 4.1 Supported styles

| Style | Request |
|---|---|
| Regular | weight 400 |
| Medium | weight 500 |
| Semibold | weight 600 |
| Bold | weight 700 |
| Italic | weight 400, italic |
| Bold Italic | weight 700, italic |

### 4.2 Face matching (CSS-style)

For a request (family F, weight w, italic i):

1. **Italic:** if F has an italic face, choose among italic faces; otherwise use upright faces and mark italic for synthesis.
2. **Weight:** use an exact match if one exists. If w ≤ 500, look lighter first (descending), then heavier. If w > 500, look heavier first (ascending), then lighter.
3. **Synthetic bold:** applied if the chosen face is ≥ 200 weight units lighter than requested, F allows bold synthesis, and the global "Allow synthesized styles" setting is on (default **on**).
4. **Synthetic italic:** applied if italic was requested but not found, F allows italic synthesis, F is not in the `script` category, and synthesis is allowed.
5. The Font Selector and Typography editor show a "Synthesized" chip whenever 3 or 4 is in effect.

**Synthesis techniques**
- **Bold:** after rasterization, dilate the coverage bitmap horizontally by `r = max(0.35 px, 0.022 × size_px)` using a max filter blended at fractional radius. Widen the advance by `2r`. Cache it as a separate glyph variant.
- **Italic:** shear the glyph outline before rasterization, `x' = x + 0.2·y` (≈ 11.3°). Fallback: shear the quad's vertices (slightly softer).

### 4.3 Size normalisation ("optical size adjust")

Different families have different cap heights at the same nominal size. With **Normalize optical size** on (default):

```
capRatio(F)  = OS/2.sCapHeight / unitsPerEm       (fallback: measured bbox of 'H')
sizeAdjust(F)= capRatio(bubblegum_sans) / capRatio(F)
effectiveSize= roleSize × textScale × sizeAdjust(F)  (then clamped to ≥ 11 ou)
```

Line heights scale by the same factor. A `sizeAdjust` number in `fonts.json` overrides the computed value.

### 4.4 Glyph fallback chain

For each code point: requested face → other faces of the same family → the role's fallback list → **Inter** (matched weight) → **Minecraft's vanilla font renderer**.

Text is split into runs by the face that covers each code point. The vanilla fallback renders its runs through the platform adapter at the correct position and scale. Vanilla runs are bitmap or unifont-based and may look softer; that is accepted for rare scripts.

**Complex scripts and bidi.** Ooze's layout engine does not shape Arabic, Indic or other complex scripts, and it does not reorder bidi text. A string that contains such code points (detected with `Character.UnicodeScript` plus bidi class checks) is rendered **entirely** through the vanilla renderer, which supports them. This applies mainly to localised UI and user-entered names.

## 5. Rasterization and atlases

### 5.1 Rasterizer

- **Text rasterization lives in the Rust engine (Sol, G3).** The crate choice is Sol's. These are the product requirements:
  - grayscale, unhinted anti-aliased coverage at the exact physical pixel size;
  - TrueType and CFF outlines;
  - `kern` and GPOS pair kerning;
  - outline access for synthetic italic.
- Font files are loaded once per session (embedded or passed from the JAR per the resource contract) and freed at shutdown.
- Sub-pixel (LCD) AA is not used; it breaks under transforms and translucent backgrounds. No ligatures are applied, which is acceptable for UI text.
- **Java never rasterizes Ooze text.** It only uploads atlas regions the engine emits (ARCHITECTURE §5.7).

### 5.2 Glyph key and quantisation

```
GlyphKey = (faceId, glyphIndex, sizeQ, subpixelX, synthBold, synthItalic)

size_px   = effectiveSize_ou × uiScale
sizeQ     = size_px ≤ 32 ? round(size_px × 2) / 2 : round(size_px)
subpixelX = size_px ≤ 24 ? 4 buckets {0, .25, .5, .75} : (size_px ≤ 48 ? 2 buckets {0, .5} : 1 bucket)
```

The glyph is rasterized at the bucket's x shift, with 2 px padding on every side.

### 5.3 Atlas

- Pages are **1024×1024** RGBA. They grow to **2048×2048** when uiScale ≥ 1.75. RGB is white and A is coverage; the vertex colour tints the text.
- Packing uses a skyline bottom-left packer.
- A page fills up to 90%, then a new page is created. **Budget: 33,554,432 bytes (32 MiB) across all pages:** 8 pages of 1024², or 2 of 2048² (PERFORMANCE_AND_ACCESSIBILITY §2.2). Enforced by the Rust accountant and re-checked by the Java texture executor.
- **Eviction:** LRU per glyph, tracked by a frame counter. When the page limit is reached, the least-recently-used page with < 25% live glyphs is cleared and repacked.
- Filtering is **linear**, clamp to edge. At rest, text is drawn at integer pixel positions (plus the sub-pixel bucket baked into the bitmap), so linear filtering never blurs it. During scale animations the bitmap is resampled, which is imperceptible during motion and exact again when motion settles.
- **Uploads:** rasterization runs on Rust worker threads. Atlas regions reach Java as texture work in the frame output; Java uploads them on the render thread, at most 2 ms and 8 MiB per frame (the rest waits for the next frame).

### 5.4 Pre-warming

At startup (async) and after any change of scale or font, the client rasterizes ASCII 32–126 plus `…•·–—’“”` for each active (face, role size) pair. Work is spread across frames under the 2 ms budget. Until a glyph is ready, its run is drawn with the nearest cached size, scaled. This only ever lasts one or two frames.

## 6. Layout and measurement

```java
TextLayout layout = textEngine.layout(
    CharSequence text, TextStyle style,     // style = role + overrides + resolved face chain
    float maxWidthOu,                       // Float.POSITIVE_INFINITY = no wrapping
    WrapMode wrap,                          // NONE | WORD | CHAR
    int maxLines,                           // ellipsize the last visible line when exceeded
    Align align);                           // START | CENTER | END
// TextLayout: lines[], each with runs (face, glyph ids, x positions), width, ascent, descent, baseline.
```

- **Line breaking:** UAX #14-lite. Break after spaces and after `-`, `/`, `—`; allow breaks between any two CJK characters; otherwise break inside a word only if a single word is wider than the line (CHAR fallback).
- **Ellipsis:** U+2026 from the active face (falling back to "..."). Text is trimmed by binary search on run width.
- **Letter spacing:** `tracking × size` is added after every glyph except the last on each line.
- **Baseline snapping:** each line's baseline is snapped to a physical pixel row.
- **Measurement API:** `measure(text, style) → (width, height)` uses the same code path as layout, with no glyph rasterization.
- **Caching:** an LRU of 4096 `TextLayout` entries, keyed by (text hash plus identity check, style id, maxWidth quantised to 0.5 ou, wrap, maxLines, align, scale epoch). The cache is invalidated by font settings, theme typography or a scale change (new epoch).
- **Allocation discipline:** layouts are immutable and reused. Static labels hold a direct reference to their layout and rebuild it only when the text, style or epoch changes.

## 7. Rendering text

- Each glyph becomes one textured quad (`POSITION_TEX_COLOR`) in the DrawList, batched by atlas page.
- **Colour:** the vertex colour is the style colour × the component's opacity.
- **Coverage gamma:** Minecraft blends in non-linear sRGB. To keep stem weight consistent:
  - dark text on light backgrounds uses coverage as-is (γ = 1.0);
  - light text on dark backgrounds uses `coverage^(1/1.2)`, applied at rasterization time as a separate "light-on-dark" variant flag in the GlyphKey (chosen by text luminance > 0.5).
- **Text shadow** (used only by the legibility treatment, never decoratively): rendered from a blurred copy of the layout. Ooze rasterizes a "halo" variant of the glyph with an 8 ou Gaussian at reduced resolution (½), caches it, and draws it beneath the text.
- **Vanilla `Component` interop:** the adapter converts a vanilla text component (colour, bold, italic, underline, strikethrough) into Ooze styled runs, mapping bold and italic to the face matcher. Obfuscated (`§k`) text is rendered by the vanilla fallback.

## 8. User customisation

Settings live in `client.json → typography`:

| Setting | Range / values | Default |
|---|---|---|
| Interface font (global family) | any registered family | Bubblegum Sans |
| Per-role overrides | family, size (±40%), style, tracking (−0.05 … +0.15 em), line height (×0.9 … ×1.6) | none |
| Text scale | 0.80 – 1.40 (step 0.05), independent of UI scale | 1.00 |
| Normalize optical size | on / off | on |
| Allow synthesized styles | on / off | on |
| Bolder text (accessibility) | Every role's weight +100 (+200 for 400 → 600); Bubblegum uses synthetic bold | off |
| Body font override | "Use a different font for body text" with a family picker; affects Body L/M/S, Caption, Label, Tooltip | off |
| Text gamma for light text | 1.0 – 1.5 (Advanced) | 1.2 |

**Live preview and contrast check.** The Typography editor (SCREENS S07) shows each role in context and computes contrast for the role's colour on the surface it uses. Text smaller than 24 ou (or smaller than 18.66 ou when bold) requires 4.5 : 1. Larger text requires 3 : 1.

## 9. High-DPI requirements (checklist)

- [ ] Glyphs are rasterized at `size_ou × uiScale` px, never at a fixed size and then scaled.
- [ ] Changing the window size, Minecraft GUI scale, UI scale or text scale re-keys the glyph cache (old pages age out through LRU).
- [ ] At 3840×2160, Body Medium renders at 32 px with stems of at least 2 px in Inter Regular. Spot-check with the reference screenshots.
- [ ] Text positions: baseline y and line start x are snapped to whole pixels; the fractional x within a line is covered by sub-pixel buckets.
- [ ] No text quad is drawn with a scale other than 1.0 when no animation is in progress (asserted in dev builds).

## 10. Licensing and attribution

- Each bundled family's `OFL.txt` is shipped next to its font file, and also copied to `licenses/fonts/<family>-OFL.txt` in the JAR root.
- The About page (SCREENS S12) lists every bundled family with its designer, copyright line, licence name and a **View licence** button that opens the full text in a modal.
- The Font Selector shows a licence chip ("OFL 1.1", or "Provided by you" for user fonts).
- Fonts are never sold or distributed on their own. Their only distribution is inside the Ooze JAR, as the OFL permits.
- Lucide icons (ISC) and Ooze's own MIT licence are listed in the same About section.
