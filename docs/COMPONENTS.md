# Ooze Client — Component Inventory

> Deliverable 7. Every component's anatomy, sizes, tokens, states, motion, keyboard behaviour and API sketch.
> Visual tokens are defined in [DESIGN_SYSTEM.md](DESIGN_SYSTEM.md); motion in [MOTION_SPEC.md](MOTION_SPEC.md).

## 1. Shared rules

### 1.1 Interaction states

Every interactive component tracks these flags in an `InteractionState`:

| Flag | Set when |
|---|---|
| `hovered` | The pointer is inside the hit shape (the hit shape includes padding up to `size.hit.min`) and no other layer above captures it |
| `pressed` | The primary button went down inside the component, and pointer capture is held until release |
| `focused` | The component has keyboard focus |
| `focusVisible` | `focused` **and** the last input modality was the keyboard (focus-visible semantics) |
| `selected` | Component-specific: on, active or current |
| `disabled` | Not interactive. It never receives hover, press or focus, is skipped in tab order, and its tooltip still shows (explaining why it is disabled, when a reason is given) |
| `dragging` | Pointer capture with movement > 3 ou |

**Visual priority:** `disabled` > `pressed` > `hovered`. `selected` and `focusVisible` stack on top of any of those.

### 1.2 Baseline state visuals

| State | Visual |
|---|---|
| Normal | Material at resting elevation (usually e1) |
| Hover | Inflate, elevation +1, gloss α ×1.1, shimmer (buttons) |
| Pressed | Squash, elevation −1, body 30% towards sunken |
| Selected | Component-specific, usually Ink Gel (accent) fill or the heavy accent outline |
| Disabled | α 0.45, `outline.soft`, no gloss, no shadow, no motion |
| Focus-visible | Focus ring (below) |

### 1.3 Focus ring

- Two concentric strokes following the component's shape, offset 3 ou outside the outline:
  - inner stroke: 2 ou in `color.outline.focus`;
  - outer halo: 1.5 ou in `color.outline.focusHalo`.
- This works on light and dark backgrounds. High Contrast uses a 3 ou inner stroke.
- Appears with `spring.snappy` scale 1.06 → 1 and a 90 ms fade.
- Never clipped by a scroll container: it is drawn in the container's overlay pass.

### 1.4 Narration

Every component supplies a `NarrationInfo(role, label, value, state, hint)` for Minecraft's narrator (PERFORMANCE_AND_ACCESSIBILITY §6).

### 1.5 Component API conventions

- Components are retained nodes (`extends Node`) configured with builders. Values bind to `Property<T>` (two-way) so settings, themes and config stay in sync.
- No component hard-codes colours, sizes or timings. It reads `ThemeToken`, `SpaceToken`, `SizeToken` and `MotionToken`.
- Each component lists its theme tokens in a static `TOKENS` descriptor, which the Theme Studio uses to highlight what a token affects.

```java
Button.primary(Text.translatable("oozeclient.action.save"))
      .icon(Icons.CHECK).size(ControlSize.MD)
      .onActivate(this::save)
      .tooltip(Text.translatable("oozeclient.tooltip.save"))
      .build();
```

---

## 2. Inventory

| ID | Component | Section |
|---|---|---|
| C01 | Button (Primary / Secondary / Ghost / Danger) | §3 |
| C02 | Icon button | §3 |
| C03 | Toggle (Gel switch) | §4 |
| C04 | Checkbox (Gel drop) | §4 |
| C05 | Radio group | §4 |
| C06 | Segmented control | §4 |
| C07 | Slider | §5 |
| C08 | Number field / stepper | §5 |
| C09 | Text field / search field | §5 |
| C10 | Dropdown (select) | §6 |
| C11 | Color picker | §6 |
| C12 | Font selector | §6 |
| C13 | Keybind chip | §6 |
| C14 | Card (module / info / preview) | §7 |
| C15 | Panel / window | §7 |
| C16 | Sidebar | §8 |
| C17 | Nav item | §8 |
| C18 | Tabs | §8 |
| C19 | Scroll container and scrollbar | §8 |
| C20 | Modal dialog | §9 |
| C21 | Popover | §9 |
| C22 | Tooltip | §9 |
| C23 | Toast notification | §9 |
| C24 | Loading indicators (BlobSpinner, LiquidProgress, Skeleton) | §10 |
| C25 | Badge / chip / tag | §10 |
| C26 | Accordion / collapsible section | §10 |
| C27 | Context menu | §10 |
| C28 | Divider (straight / drip) | §10 |
| C29 | Empty state | §10 |
| C30 | Preview frame | §10 |
| C31 | Setting row | §11 |
| C32 | Command palette (search) | §11 |
| C33 | Decorations: drip, blob, droplet particle | §12 |
| C34 | Logo (mark, wordmark, plate) | §12 |

---

## 3. Buttons

### C01 Button

**Anatomy:** gel body → [leading icon] → label → [trailing icon or shortcut hint]. Gloss band and glint per DESIGN_SYSTEM §3.

| Size | Height | Padding x | Radius | Label role | Icon |
|---|---|---|---|---|---|
| SM | 32 | 14 | `md` (pill if `round`) | Label | 16 |
| MD | 40 | 18 | `md` | Button | 20 |
| LG | 48 | 22 | `md` | Button | 20 |
| XL | 64 | 28 | pill | Button (+2 ou) | 24 |

Minimum width is 2 × height. The label is centred, or left-aligned for XL menu buttons with the icon at the left padding.

| Variant | Body | Label | Outline | Notes |
|---|---|---|---|---|
| Primary | Ink Gel (accent) | `text.onAccent` | `outline.strong` | At most one per view region. LG and XL get the hover drip |
| Secondary | White Ooze (base) | `text.primary` | `outline.strong` | Default |
| Ghost | None at rest. On hover a White Ooze body fades in (α 0 → 1, 120 ms) and inflates from 0.92 | `text.primary` | none, then `outline.soft` on hover | Toolbars, low emphasis |
| Danger | White Ooze | `status.danger` | `status.danger` @ 0.9 | Hover: body tinted `status.dangerSoft`. Confirmation dialogs use Primary styling with the danger colour as accent |

**Additional states**
- **Loading:** the label is replaced by a BlobSpinner (C24); width stays fixed; the button is not interactive while loading.
- **Toggled** (for toggle buttons such as Grid/List): treated as `selected`, using Ink Gel.

**Keyboard:** Enter or Space activates; activation fires on key release for Space and on press for Enter.

**Narration:** "Button, {label}" (+ ", disabled").

### C02 Icon button

- Circle gel, sizes 32 / 40 / 48. The icon is 16 / 20 / 24.
- Variants: Secondary (default) and Ghost (toolbar).
- A tooltip is **required**; it doubles as the narration label.
- Selected (toggle) style: Ink Gel.

---

## 4. Choice controls

### C03 Toggle (Gel switch)

| Size | Track | Knob ⌀ | Travel |
|---|---|---|---|
| MD | 44 × 26 | 20 | 18 |
| LG | 52 × 30 | 24 | 22 |

| State | Track | Knob |
|---|---|---|
| Off | Sunken (`surface.sunken`), `outline.soft` hairline | White Ooze with `outline.control`, glint, e1 |
| On | Ink Gel (accent), gloss α 0.28 | White Ooze, e1 |
| Hover | + `outline.strong` hairline (off) / L+0.05 (on) | Inflate 1.06 |
| Pressed | — | Anticipation widen (MOTION_SPEC §4.2) |
| Disabled | α 0.45 | α 0.45 |
| Focus-visible | Ring around the track | — |

- **Motion:** knob spring, stretch-by-velocity, landing squash, track colour crossfade (MOTION_SPEC §4.2).
- **Keyboard:** Space or Enter toggles; Left or Right sets off or on.
- The **label is always outside** the toggle, in a Setting row (C31), and clicking the label toggles too.
- **Narration:** "Toggle, {label}, on/off".

### C04 Checkbox (Gel drop)

- A 20 × 20 box, radius `xs`, sunken with `outline.control`.
- **Checked:** an Ink Gel drop fills the box, with a white 2 ou check stroke.
- **Indeterminate:** a white 10 × 2 ou bar.
- Motion per MOTION_SPEC §4.3. Keyboard: Space.
- Use checkboxes for multi-select lists; use Toggles for settings that take effect immediately.

### C05 Radio group

- A 20 ou circle, sunken with an outline. The selected option shows an Ink Gel droplet (⌀ 10).
- Keyboard: arrow keys move the selection (roving tabindex; the group is one tab stop).
- Use only for 2–5 options where a segmented control does not fit (long labels). Otherwise prefer C06.

### C06 Segmented control

- A sunken pill container, height 36 (MD) or 32 (SM), with 3 ou inner padding. 2–5 segments of equal width, or content width when `fit`.
- **Indicator:** a White Ooze pill (raised, e1, gloss) on light themes, or Ink Gel when `emphasis = strong`. It moves with **liquid morph** (MOTION_SPEC §4.5).
- Labels use the Label role; an icon-only variant is allowed with tooltips.
- Keyboard: Left and Right move the selection (and apply it immediately); Home and End.
- **Narration:** "{group label}, {option}, {n} of {total}".

---

## 5. Value controls

### C07 Slider

**Anatomy:** track (sunken pill, 6 ou tall) → fill (Ink Gel, same height, rounded end with meniscus) → thumb (White Ooze circle ⌀ 20, glint, e1) → optional tick dots (⌀ 3, `outline.soft`) → value bubble (an Ink Gel tooltip showing the formatted value with its unit).

| Option | Behaviour |
|---|---|
| `min`, `max`, `step` | Values snap to step. The display format comes from the setting (e.g. `"%d%%"`, `"0.00×"`) |
| `showField` | Adds a Number field (C08, 72 wide) to the right, bound to the same property |
| `logarithmic` | Optional mapping for wide ranges (e.g. blur radius) |
| `defaultMarker` | A small notch at the default value. **Double-click the thumb** resets to the default |

- **Hit area:** the full row height (≥ 32), so it is easy to grab.
- **Motion:** MOTION_SPEC §4.4.
- **Keyboard:**
  - Left/Right or Down/Up: ±1 step.
  - Shift + arrow: ±10 steps.
  - PageUp/PageDown: ±10% of the range.
  - Home/End: min/max.
  - Typing a digit focuses the attached field (if any) with that digit.
- **Narration:** "Slider, {label}, {value}{unit}".

### C08 Number field / stepper

- A sunken field, height 32 or 40, radius `sm`, right-aligned numerals. The stepper variant adds − / + ghost icon buttons inside the field ends.
- Scrolling the wheel over the focused field steps the value.
- Typing an invalid value shows the outline in `status.danger`, plus a helper caption under the row ("Enter a number from 0 to 100"). The value is not committed until it is valid. Enter or blur commits; Esc reverts.

### C09 Text field / search field

**Anatomy:** a sunken well (height 40, radius `sm`, `outline.control` at rest) → [leading icon] → text (Body Medium) → placeholder (`text.secondary` @ 0.8) → [clear button] → [shortcut hint chip, e.g. `Ctrl K`].

| State | Visual |
|---|---|
| Focused | Outline 2 ou in `outline.focus` (no separate ring); well brightens 4% |
| Invalid | Outline `status.danger`; a helper text row below |
| Disabled | α 0.45 |

**Editing features**
- Caret with blink; selection highlight in `selection.text`.
- Shift + arrows select; Ctrl + arrows (Alt on macOS) move by word; Home/End.
- Ctrl+A/C/X/V and undo/redo within the field (Ctrl+Z / Ctrl+Shift+Z, 50 steps).
- Double-click selects a word; triple-click selects all.
- The text scrolls horizontally to keep the caret visible, with 8 ou of padding.

**Search variant**
- Leading search icon.
- The clear button (×) appears when the field is not empty.
- Esc clears first; a second Esc blurs.
- Optional `debounceMs` (default 0 for local filtering).

---

## 6. Pickers

### C10 Dropdown (select)

- **Trigger:** a Secondary-style field (height 40, radius `md`) showing the selected label, with a trailing chevron that rotates 180° when open.
- **Popover list:** an Overlay surface (e3, radius `md`), min width = trigger width, max height 320 with scrolling. Rows are 36 tall, with a leading check mark for the selected item and optional icons and descriptions (two-line rows, 52 tall).
- **Built-in search** appears automatically when there are > 8 options.
- **Keyboard:**
  - Enter, Space or Alt+Down opens.
  - Up/Down move the highlight; Enter selects; Esc closes.
  - Type-ahead: typing jumps to the first match (prefix, 700 ms buffer).
- **Placement:** below the trigger, or above if there is not enough space; aligned to the trigger's start edge, kept within the viewport.

### C11 Color picker

- **Trigger:** a swatch chip (32 × 32, radius `sm`, white outline ring plus outline; a checkerboard shows through the swatch to reveal alpha) plus the hex label.
- **Popover** (300 wide):
  1. A saturation/value square (268 × 180). Its gradient mesh uses 16×16 vertices so colours are accurate. The cursor is a 14 ⌀ gel ring.
  2. A hue strip (268 × 14).
  3. An alpha strip (268 × 14, checkerboard), when `alpha` is enabled.
  4. Inputs row: a hex field (`#RRGGBB` or `#RRGGBBAA`) plus a mode dropdown (RGB / HSV / OKLCH) with numeric fields.
  5. Swatches: the theme palette (a row of 8), recent colours (8, persisted) and the image palette (when editing a background with an image).
  6. **Contrast readout** when the picker edits a text or surface token: "On surface.base: 5.2 : 1 ✓ AA", or "2.9 : 1 ✕" with a **Fix** button that moves OKLab L until the pair passes.
- **Keyboard:**
  - Tab moves between regions.
  - In the square, arrow keys move by 1% (Shift: 10%).
  - In the strips, arrow keys adjust.
  - Enter closes.
- All edits apply live; Esc reverts to the value at open.

### C12 Font selector

- **Trigger:** a dropdown-style field showing the family name **rendered in that family**.
- **Popover** (420 wide, max 480 tall):
  - Search, plus category filter chips (All / Display / Sans / Serif / Script / Yours).
  - Rows 56 tall: the family name in its own font at 20 ou, a sample "Soft on the outside." in Body Small, a licence chip, a face-count caption ("6 styles" or "1 style · synthesized bold/italic available") and a coverage warning if the current UI language needs glyphs the font lacks.
- Bottom actions: **Open fonts folder** and **Rescan**.
- Keyboard: as for the dropdown.

### C13 Keybind chip

**Anatomy:** a pill chip (height 32, radius pill, White Ooze) containing the key label (Label role, monospace-like tabular figures), for example `R Shift`, `Ctrl + K` or `Mouse 4`. When unbound it shows "Not bound" in `text.secondary` with a dashed `outline.soft`.

| State | Visual |
|---|---|
| Listening | The chip morphs to Ink Gel and the label becomes "Press a key…", with a pulsing blob dot (BlobSpinner, small). Esc cancels; Backspace or Delete clears the binding; any other key or mouse button (except left and right click on the chip itself) is captured, together with held modifiers |
| Conflict | A warning badge (amber, `!`). The tooltip lists the conflicting actions, including vanilla key mappings |

- Clicking or pressing Enter/Space on the chip enters listening mode.
- **Narration:** "Keybind, {action}, {key or not bound}".

---

## 7. Containers

### C14 Card

| Variant | Size | Anatomy |
|---|---|---|
| **Module card** | min width 296, height 128, radius `lg`, padding 18 | Row 1: icon chip (36 ⌀ sunken circle; Ink Gel with a white icon when the module is enabled) + name (Body Large, one line, ellipsis) + spacer + Toggle (MD). Row 2: description (Body Small, max 2 lines). Row 3: keybind chip (SM, if bound), **Sample** badge (if sample), favourite star (Ghost icon button; visible on hover, focus or when favourited) |
| **Info card** | flexible, radius `lg`, padding 20 | Optional Section Heading, content and optional footer actions |
| **Preview card** | 16 : 9 or fixed, radius `lg` | A Preview frame (C30) on top plus a title and caption below; selectable (theme and wallpaper choosers) |

**Module card states**

| State | Visual |
|---|---|
| Hover | Inflate, e2 |
| Pressed | Squash (card body only; the toggle is independent) |
| Selected (its inspector is open) | `outline.heavy` in accent, e2 |
| Enabled module | A `highlight.cool` inner shade raised to α 0.9, plus the icon chip in Ink Gel |

**Card interactions**
- Clicking the body opens the inspector. Clicking the toggle toggles. Right-click opens the context menu (C27).
- Keyboard: the card is one focus stop. Enter opens the inspector; Space toggles; F favourites; the context-menu key or Shift+F10 opens the menu. Arrow keys move focus between cards in 2D (grid navigation).

### C15 Panel / window

- Surface class `panel`, radius `xl` (window) or `lg` (sub-panel), outline `panel`, e4 or e3. Bottom drips per DESIGN_SYSTEM §4.5.
- **Optional title bar** (height 56): title (Title role at 24 ou in windows), subtitle, trailing actions (Ghost icon buttons), close button.
- **Dragging:** floating panels can be dragged by the title bar. Position is clamped so at least 64 ou stays visible. Positions persist per panel id. The ClickGUI window itself is fixed and centred in v0.1.

---

## 8. Navigation and scrolling

### C16 Sidebar

| Mode | Width | Contents |
|---|---|---|
| Expanded | 264 | Logo lockup (small), search trigger, sections (Section Heading + nav items), footer (profile switcher) |
| Collapsed (rail) | 80 | Mark only, a search icon button, nav icons with tooltips, profile avatar |

- **Surface:** a sunken well inside the window, inset 12, radius `lg`.
- **Collapse toggle:** a Ghost icon button (`panel-left`) at the top. `Ctrl+B` toggles. The state persists.
- **Compact breakpoint:** collapsed by default. When expanded, it overlays the content as a floating panel (e3) and auto-collapses after navigation.

### C17 Nav item

- Height 44, radius `md`, icon 20 plus label (Body Medium) plus an optional count badge (C25, the enabled-module count).
- **Active:** the liquid-morph indicator (one shared indicator per sidebar) is a White Ooze raised blob (e1, gloss) under the item. Text `text.primary`, icon in accent.
- **Hover:** the pre-blob (MOTION_SPEC §4.5).
- **Keyboard:** Up/Down within the sidebar (roving focus); Enter activates; Ctrl+1…9 jump to the nth module category.

### C18 Tabs

- A row of text tabs (Label role at 15, height 40). The active tab has an underline-blob indicator: an Ink Gel capsule 4 ou tall, width = label width + 12, with liquid morph.
- Content crossfades over 160 ms. Keyboard: Left/Right (automatic activation).

### C19 Scroll container and scrollbar

- Clips its content to its rectangle (scissor). Edge-fade masks (MOTION_SPEC §4.7) hide hard clip edges, so the container never needs rounded clipping (ARCHITECTURE §5.6).
- **Scrollbar:** a gel pill thumb, 6 ou wide (10 on hover or drag), radius pill, White Ooze with `outline.soft`. The track is invisible unless hovered (then sunken at α 0.6). Minimum thumb length 32.
- **Virtualisation:** containers with > 50 homogeneous rows lay out and paint only the visible rows ± 4.
- **Keyboard:** PageUp/PageDown, Home/End, Space/Shift+Space when the container is focused, and arrow keys when no child consumes them.
- Focus moving into a partly hidden child scrolls it into view with `spring.lazy`.

---

## 9. Overlays

### C20 Modal dialog

- **Scrim:** `color.scrim`, plus a light blur of the window behind on High quality (a reuse of the backdrop's cached blur is acceptable). Width 480 (SM), 640 (MD) or 880 (LG); radius `xl`; padding 28; e4; one or two bottom drips.
- **Structure:** title (Subtitle role at 24, `text.primary`), body (Body Medium), optional content, and actions right-aligned (Secondary "Cancel", Primary or Danger confirm). The destructive button never has initial focus; Cancel does.
- **Focus trap:** Tab cycles within the dialog. Esc equals Cancel unless the dialog is `blocking`. Focus returns to the invoking element on close.
- **Kinds:** Confirm, Alert, Form (with validation), and Licence viewer (a scrollable monospace-free Body Small text with an LG width).

### C21 Popover

- An Overlay surface (e3, radius `md`) anchored to a trigger, with an optional 8 ou drip-shaped pointer tail.
- Closes on outside click, Esc or focus leaving it (unless it is `sticky`).
- Placement uses a flip-and-shift algorithm to stay within the viewport with 8 ou margins.

### C22 Tooltip

- **Surface:** inverse (Ink Gel, α 0.96), radius `sm`, padding 8 × 12, max width 320, Tooltip role in `text.inverse`.
- **Tail:** a 6 ou drip-shaped pointer towards the target.
- **Content:** a title line plus an optional secondary line (`text.inverse` @ 0.75) plus an optional shortcut chip.
- **Trigger:** hover delay 450 ms (300 ms warm), or immediately on keyboard focus-visible after 600 ms. Hidden on press and scroll.
- **Placement:** above by default, then below, right or left (flip). Tooltips never cover the cursor.

### C23 Toast notification

- **Surface:** a White Ooze card (raised, e3, radius `lg`), width 360, min height 64, padding 14 × 16.
- **Anatomy:** a status glyph (24, in its status colour on its soft circle) → title (Label role at 15) → body (Body Small, max 3 lines) → actions (up to 2 Ghost buttons) → a close icon button → an auto-dismiss progress hairline at the bottom (Ink Gel, draining).

| Type | Glyph | Default duration |
|---|---|---|
| info | info | 4.5 s |
| success | circle-check | 4.5 s |
| warning | triangle-alert | 7 s |
| error | circle-x | Persistent until dismissed |
| progress | BlobSpinner, then LiquidProgress | Until complete, then +2 s |

- **Stack:** up to 4 visible; the rest queue. Placement: a corner set in Settings (default bottom-right, 24 ou from the edges).
- Hovering or focusing a toast pauses its timer.
- **Keyboard:** `Ctrl+Shift+N` focuses the newest toast; Esc dismisses the focused toast.
- **In-world compact toast** (used by module keybind toggles): a pill 36 tall, "{Module} on/off", top-centre, 1.6 s, drawn by the HUD layer. It can be turned off in Settings.

---

## 10. Feedback and structure

### C24 Loading indicators

| Indicator | Size | Spec |
|---|---|---|
| BlobSpinner | 16 / 24 / 40 | Three merging droplets (MOTION_SPEC §4.10), in accent on White Ooze surfaces or white on Ink Gel |
| LiquidProgress | Height 10, radius pill | A sunken track and an Ink Gel fill with a wave surface. **Indeterminate**: a 30%-wide blob slides with `ease.viscous` (1.4 s loop) |
| Skeleton | Matches the content shape | Pearl blocks (`surface.sunken`) with a shimmer sweep every 1.4 s. Used for the library grid, server list and world list while loading |

### C25 Badge / chip / tag

| Kind | Look |
|---|---|
| Count badge | min 20 × 20, pill, Ink Gel, Status role, white text |
| Status badge | pill, soft status colour background, status-coloured text, Status role |
| **Sample** badge | pill, height 20, fill `accent.soft`, outline `outline.soft`, text "SAMPLE" in the Status role, `text.secondary`. **Mandatory on all sample modules, widgets and data** |
| Synthesized chip | as Status badge (info) |
| Filter chip | height 32, White Ooze, toggleable (selected = Ink Gel). Removable variant with an × |

### C26 Accordion / collapsible section

- **Header row** (height 48): chevron, title (Body Large) and optional summary text (Body Small, secondary) right-aligned. Clicking anywhere on the header toggles.
- Content height animates with `spring.soft` (MOTION_SPEC §4.6).
- Keyboard: Enter/Space toggles; Right expands; Left collapses.
- Used for setting groups, licence texts and advanced sections.

### C27 Context menu

- An Overlay popover at the pointer (or at the focused item for keyboard). Rows are 36 tall: icon, label, shortcut hint. Separators are hairline. Destructive rows use `status.danger`.
- Submenus are not used in v0.1.
- Keyboard: arrows, Enter, Esc, and type-ahead.

### C28 Divider

- **Straight:** a 1 ou `outline.soft` hairline (snapped to the pixel grid).
- **Drip divider:** a hairline with a single tiny static drip (neck 4, length 6) at 15% from the start. Used once per page at most, between major sections.

### C29 Empty state

- Centred in its container. A vector blob illustration (120 ou, a slowly breathing blob plus 2 droplets) → Headline at 28 → Body Medium explanation (max 420 wide) → an optional primary action.
- Example copy: "No favourites yet" / "Star a module to keep it here."

### C30 Preview frame

- A rounded (`lg`) frame with `outline.control`, content rendered at a reduced scale. It reuses real components with `interactive = false` and the same theme tokens.
- Used by theme presets, wallpapers, background context mini-previews and the Motion lab.
- It renders into the parent DrawList with a transform plus scissor; there is no offscreen render target.

---

## 11. Composite rows

### C31 Setting row

The universal row used in every settings page and in module inspectors.

```
┌────────────────────────────────────────────────────────────────────────┐
│ Label (Body Medium)                    [ control, right-aligned      ] │  min height 56 (64 with description)
│ Description (Body Small, secondary)    [ reset (~) when not default  ] │  padding 16 × 12
└────────────────────────────────────────────────────────────────────────┘
```

- **Mapping from setting types to controls:**

| Setting type | Control |
|---|---|
| `BooleanSetting` | Toggle (C03) |
| `IntSetting` / `FloatSetting` | Slider (C07), with a field when `precise` |
| `EnumSetting` | ≤ 4 options and short labels → Segmented (C06); otherwise Dropdown (C10) |
| `ColorSetting` | Color picker trigger (C11) |
| `KeybindSetting` | Keybind chip (C13) |
| `StringSetting` | Text field (C09) |
| `StringListSetting` | Chip editor: removable chips plus an input |
| `FontSetting` | Font selector (C12) |
| `ActionSetting` | Button (Secondary SM) |
| `SettingGroup` | Accordion (C26) or a section card |

- The row is one tab stop group. Tab goes to the control; the reset button is reachable by Tab when visible.
- Hover lightens the row (a sunken-α 0.4 pill behind it).
- **Search highlight:** when navigated to from search, the row pulses twice (a `highlight.cool` glow ring, 2 × 600 ms).
- Disabled rows (setting visibility predicate false, or locked by the environment) show the reason in a tooltip.

### C32 Command palette

The main navigation search; see [SCREENS.md S11](SCREENS.md).

- An Overlay surface, 720 wide, at y = 140, radius `xl`, e4.
- **Input:** 56 tall, Body Large.
- **Results:** grouped sections (Section Heading) of rows 48 tall: icon, title with matched characters in Ink Gel weight (bold or synth bold), subtitle (path, e.g. "Showcase › Settings Showcase › Accent strength") and a right-aligned type chip.
- **Footer hints:** `↑↓ Navigate · Enter Open · Tab Scope · Esc Close`.

---

## 12. Decorations and brand

### C33 Decorations

| Decoration | Notes |
|---|---|
| Drip | Part of a surface path (DESIGN_SYSTEM §4.5). Never a standalone sprite |
| Blob | A standalone filled blob (DESIGN_SYSTEM §4.4) used in empty states, loaders and the intro |
| Droplet particle | A circle (⌀ 3–8) with a gloss dot. Used by background particles and the drip detach |

None of these is interactive, focusable or narrated.

### C34 Logo

- **Mark**, **Wordmark** and **Plate** variants per DESIGN_SYSTEM §2.
- The plate variant has two drips and supports the intro's morph parameters (`morph: 0 = droplet … 1 = plate`).

---

## 13. State coverage matrix

✓ = has a defined visual. — = not applicable.

| Component | Normal | Hover | Pressed | Selected | Disabled | Focus | Other |
|---|---|---|---|---|---|---|---|
| C01 Button | ✓ | ✓ | ✓ | ✓ (toggle) | ✓ | ✓ | loading |
| C02 Icon button | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | — |
| C03 Toggle | ✓ | ✓ | ✓ | ✓ (on) | ✓ | ✓ | dragging |
| C04 Checkbox | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | indeterminate |
| C05 Radio | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | — |
| C06 Segmented | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | — |
| C07 Slider | ✓ | ✓ | ✓ | — | ✓ | ✓ | dragging |
| C08 Number field | ✓ | ✓ | — | — | ✓ | ✓ | invalid |
| C09 Text field | ✓ | ✓ | — | — | ✓ | ✓ | invalid, selection |
| C10 Dropdown | ✓ | ✓ | ✓ | ✓ (open) | ✓ | ✓ | — |
| C11 Color picker | ✓ | ✓ | ✓ | ✓ (open) | ✓ | ✓ | contrast fail |
| C12 Font selector | ✓ | ✓ | ✓ | ✓ (open) | ✓ | ✓ | synthesized |
| C13 Keybind chip | ✓ | ✓ | ✓ | ✓ (listening) | ✓ | ✓ | conflict, unbound |
| C14 Card | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | enabled-module |
| C17 Nav item | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | — |
| C18 Tabs | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | — |
| C23 Toast | ✓ | ✓ (pause) | — | — | — | ✓ | per type |
| C25 Filter chip | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | — |
| C26 Accordion | ✓ | ✓ | ✓ | ✓ (open) | ✓ | ✓ | — |
| C31 Setting row | ✓ | ✓ | — | — | ✓ | ✓ (child) | modified, search highlight |
