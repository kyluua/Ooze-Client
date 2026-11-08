# Ooze Client — Screen-by-Screen Specification

> Deliverable 8. Layouts are given for the **reference viewport of 1920 × 1080 ou** (DESIGN_SYSTEM §9). Component ids (Cxx) refer to [COMPONENTS.md](COMPONENTS.md).

## 0. Screen architecture

### 0.1 Hosts and routes

Ooze has **four full-screen hosts** (each is one Minecraft `Screen` subclass hosting an Ooze view tree) and one **shell** that hosts many pages.

| Id | Screen | Host / route | Opened from |
|---|---|---|---|
| S01 | Startup / branding (Intro + Welcome) | `IntroScreen` | First title-screen show per launch (setting) |
| S02 | Main menu | `MainMenuScreen` | Replaces vanilla `TitleScreen` when enabled |
| S03 | ClickGUI shell, Modules page | `ShellScreen` → `shell/modules/{category}` | `Right Shift` (in-world), **Ooze** button on the main menu, Mod Menu config button |
| S04 | Settings | `shell/settings` | Sidebar, search |
| S05 | Theme Studio | `shell/theme` | Sidebar, search, Welcome |
| S06 | Background Studio | `shell/background` (docked mode) | Sidebar, main menu quick action, search |
| S07 | Typography | `shell/typography` | Sidebar, search |
| S08 | Motion & effects | `shell/motion` | Sidebar, search, Welcome |
| S09 | HUD editor | `HudEditorScreen` | Sidebar (CUSTOMIZE › HUD editor), keybind (unbound by default), search |
| S10 | Keybinds | `shell/keybinds` | Sidebar, search |
| S11 | Search and navigation | Command palette overlay on any shell page, plus the sidebar | `Ctrl K`, `/`, the sidebar search field |
| S12 | About | `shell/about` | Sidebar, main menu (version string click), search |

### 0.2 Layer stack (inside every host)

`Background → Content → Inspector → Popovers → Modals → Toasts → Tooltips → Drag ghost → Debug`

### 0.3 Router and back behaviour

- The shell keeps a **history stack**, max 32 entries. Back is triggered by `Esc` (after the Esc precedence chain, [INTERACTION_FLOWS.md §3](INTERACTION_FLOWS.md)), `Mouse 4`, or `Alt+Left`. `Alt+Right` or `Mouse 5` goes forward.
- Back from the root page closes the shell and returns to the screen that opened it (`parent`): the world, the main menu or the vanilla screen.
- The last visited shell page is remembered per context (in-world vs. menu) when **Remember last page** is on (default on).

---

## S01 — Startup / branding screen

**Purpose:** a short, premium brand moment that never delays play, plus a first-run quick setup.

**When it shows**
1. After Minecraft's own loading overlay finishes, the first time the title screen would open in this session.
2. Setting: *Intro*: Every launch / **First launch after an update** (default) / Never.
3. The vanilla first-launch accessibility onboarding screen, when Minecraft shows it, **always comes first** and is never suppressed.

### Intro layout

Full-bleed background (`main_menu` context). The logo animates to its final main-menu slot. Timeline: [MOTION_SPEC §6](MOTION_SPEC.md).

```
┌──────────────────────────────────────────────────────────────────────────────┐
│                                                                              │
│                                  (droplet falls)                             │
│                                        ()                                    │
│                            ╭──────────────────────╮                          │
│                            │  (o)  ooze   CLIENT  │    plate morph + wipe    │
│                            ╰──────────╮╭──────────╯                          │
│                                       ╰╯   ·  (detaching droplet)            │
│                                                                              │
│                                                       Press any key to skip  │  Caption, fades in at 600 ms
└──────────────────────────────────────────────────────────────────────────────┘
```

- **Loading fallback:** if Ooze's own async start-up (fonts, theme, background decode) has not finished when the intro would end, the plate stays centred and a small BlobSpinner (C24, 24 ou) appears under it until everything is ready. If start-up takes more than 5 s, the spinner gets a caption: "Preparing fonts…" or "Preparing background…".
- **Errors:** if the background fails, `color.canvas` is used, and a toast appears after the main menu is shown.

### Welcome sheet (first Ooze launch only)

A modal-style card (LG, 880 × 560) over the main menu. It has three steps, a progress dot row and **Skip** in the top-right.

```
╭──────────────────────────────────────────────────────────────────────────╮
│  Welcome to Ooze                                             Skip   x    │
│  Make it yours in three quick steps.                                     │
│                                                                          │
│  1 Pick a look        2 Pick a background        3 Comfort               │
│  ╭─────────╮╭─────────╮╭─────────╮╭─────────╮╭─────────╮                 │
│  │ Classic ││ Midnight││   Mint  ││Bubblegum││   High  │  Preview cards  │
│  │    *    ││         ││         ││         ││ Contrast│  (C14)          │
│  ╰─────────╯╰─────────╯╰─────────╯╰─────────╯╰─────────╯                 │
│                                                                          │
│  * o o                                           [ Back ]  [ Next > ]    │
╰──────────────────────────────────────────────────────────────────────────╯
```

| Step | Content |
|---|---|
| ① Pick a look | Theme preset cards. Selecting one applies it live behind the sheet |
| ② Pick a background | Wallpaper preset cards (live) and **Use my own image…** (opens the import flow from S06 inline) |
| ③ Comfort | ClickGUI key (C13, default `Right Shift`); Motion: Full / Reduced / Off (C06); UI size: Auto plus a slider (C07) |

Every choice is saved immediately. **Finish** closes the sheet with the modal close animation, and the main menu buttons stagger in.

---

## S02 — Main menu

**Purpose:** a beautiful, fast launch pad that keeps 100% of vanilla title-screen functionality.

### Layout (Regular)

```
┌──────────────────────────────────────────────────────────────────────────────────────────────┐
│                                                                     ╭─────────────────────╮  │
│                                                                     │ [#] Kyluua      (*) │  │  profile chip (y 40)
│   ╭─────────────────────────────╮                                   ╰─────────────────────╯  │
│   │ (o)  ooze   CLIENT          │                                                            │  logo plate 360×120 at (120, 112)
│   ╰────╮╭────────────────╮╭─────╯                                                            │
│        ╰╯                ╰╯                                                                  │
│                                                                                              │
│   Soft on the outside.                tagline, y 268                                         │
│                                                                                              │
│   ╭────────────────────────────────────╮               ╭──────────────────────────────────╮  │
│   │ >   Singleplayer                   │  y 340        │ CONTINUE                         │  │  right column: x 1360, w 440
│   ╰────────────────────────────────────╯               │ Cozy Valley              [Play]  │  │
│   ╭────────────────────────────────────╮               │ Survival · 2 hours ago           │  │
│   │ =   Multiplayer                    │  y 420        ╰──────────────────────────────────╯  │
│   ╰────────────────────────────────────╯               ╭──────────────────────────────────╮  │
│   ╭────────────────────────────────────╮               │ RECENT SERVERS                   │  │
│   │ ~   Minecraft Realms               │  y 500        │ [#] Friends SMP          [Join]  │  │
│   ╰────────────────────────────────────╯               │ [#] Creative Build       [Join]  │  │
│   (Opt) (Ooze) (Mods) (Lang) (Access) (+2)  y 596      ╰──────────────────────────────────╯  │
│   ╭────────────╮                                       ╭──────────────────────────────────╮  │
│   │ x  Quit    │  y 668                                │ WHAT'S NEW IN 0.1.0              │  │
│   ╰────────────╯                                       │ • Ooze main menu  • HUD editor…  │  │
│                                                        ╰──────────────────────────────────╯  │
│ Minecraft 1.21.11 · Fabric · Ooze 0.1.0              Copyright Mojang AB. Do not distribute! │
└──────────────────────────────────────────────────────────────────────────────────────────────┘

```

### Regions

| Region | Spec |
|---|---|
| Logo plate | C34 plate variant, 360 × 120, two drips. Hover: inflate plus shimmer. Click: plays the logo jiggle (`spring.wobble`) as a hidden delight. It does nothing else |
| Tagline | Body Large with the legibility treatment (BACKGROUNDS §7). A localisable string |
| Primary stack | XL buttons (C01), 440 × 64, gap 16. **Singleplayer** = Primary (Ink Gel); **Multiplayer** and **Realms** = Secondary. They open the vanilla `SelectWorldScreen`, `JoinMultiplayerScreen` and `RealmsMainScreen` (through the adapter) |
| Utility row | Icon buttons (C02, 48) with tooltips: **Options** (vanilla `OptionsScreen`), **Ooze** (opens the shell), **Mods** (only if Mod Menu is installed; opens its screen through the compat adapter), **Language** (vanilla), **Accessibility** (vanilla), and **More from your mods** (only if other mods add buttons to the title screen; a count badge plus a popover listing them; see below) |
| Quit | Secondary LG with a power icon. Calls the vanilla stop |
| Profile chip | The player's face (taken from the skin texture via the adapter; pixel art is drawn with **nearest** filtering at an integer scale), username, and a gear icon button that opens shell Settings |
| Continue card | C14 info card. The most recently played world (async world list), its game mode and relative time. **Play** loads it directly. A skeleton shows while loading; the card is hidden if there are no worlds |
| Recent servers card | Up to 3 entries from the saved server list: icon (cached favicon), name, **Join** (connects). Pinging is **off** by default (setting *Ping servers on main menu*); when on, it adds latency and a player count. Hidden if the list is empty |
| What's new card | From the bundled `changelog.json` (local, no network). **See all** opens About › Changelog |
| Footer | Left: version info (Caption). Clicking it opens About. Right: the Mojang copyright line (Caption), which opens the vanilla credits like the vanilla title screen does. Both use the legibility treatment |
| Background | `main_menu` context with ambient attraction to the cursor |

**"More from your mods" compatibility tray**
- The adapter builds an off-screen vanilla `TitleScreen`, runs `init`, and collects buttons that are not vanilla (Fabric's `Screens.getButtons`).
- Each is shown as a Secondary SM button with its label. Pressing it invokes the original button's press handler.
- The tray always contains **Open vanilla title screen** as the last row.
- Failures are caught: the tray shows only the vanilla fallback and logs a warning.

### Responsive

- **Compact:** a single centred column, 440 wide. The right column is removed; **Continue** becomes a Secondary XL button "Continue · Cozy Valley" above Singleplayer. Recent servers and What's new are hidden.
- **Wide:** the columns keep their positions relative to a centred 1680-wide frame.

### Motion

- **Entry (after the intro or on return):** stagger the logo → tagline → buttons → utility row → right cards (24 ms steps).
- Buttons follow MOTION_SPEC §4.1, including the hover drip on XL buttons.

### Keyboard

- Tab order: primary stack → utility row → Quit → right cards → profile chip.
- Arrow keys move within the primary stack and the utility row.
- `Ctrl K` opens the shell with the command palette open.

### Settings that affect S02

- Use Ooze main menu (if off, the vanilla `TitleScreen` is used and gets a small **Ooze** gel button in the top-right).
- Show Continue card; Show Recent servers; Ping servers; Show What's new; Tagline (on/off).

---

## S03 — ClickGUI shell and Modules page

**Purpose:** the premium hub for modules and every Ooze customisation page.

### Shell layout (Regular)

```
 ╭───────────────────────────────────────────────────────────────────────────────────────────────╮  window 1520×880
 │ ╭───────────────────────╮  Showcase                          [All|Enabled|*]  [#|=]  [Sort v] │  header h 76
 │ │ (o) ooze          [<] │  2 modules · 1 enabled                                              │
 │ │ ╭───────────────────╮ │  ────────────────────────────────────────────────────────────────── │
 │ │ │ / Search…   Ctrl K│ │  ╭─────────────────────╮  ╭─────────────────────╮  ╭──────────────╮ │
 │ │ ╰───────────────────╯ │  │ (o) Settings Sho. * │  │ (o) Notificatio… o  │  │ …            │ │  module cards, 3 columns
 │ │ MODULES               │  │ Demonstrates every  │  │ Posts each kind of  │  │              │ │
 │ │ (* Showcase      2)   │  │ control. No game…   │  │ toast…              │  │              │ │  (inspector slides in from
 │ │   # HUD          3    │  │ [SAMPLE]          * │  │ [SAMPLE] [R Ctrl] * │  │              │ │   the right; see below)
 │ │ CUSTOMIZE             │  ╰─────────────────────╯  ╰─────────────────────╯  ╰──────────────╯ │
 │ │   o Theme             │                                                                     │
 │ │   # Background        │                                                                     │
 │ │   T Typography        │                                                                     │
 │ │   ~ Motion & effects  │                                                                     │
 │ │   + HUD editor      ^ │                                                                     │
 │ │ CLIENT                │                                                                     │
 │ │   k Keybinds          │                                                                     │
 │ │   * Settings          │                                                                     │
 │ │   i About             │                                                                     │
 │ │ ╭───────────────────╮ │                                                                     │
 │ │ │ [#] Default    v  │ │                                                                     │
 │ │ ╰───────────────────╯ │                                                                     │
 │ ╰───────────────────────╯                                                                     │
 ╰────────╮╭────────────────────────────────╮╭───────────────────────────────────────────────────╯
          ╰╯                                ╰╯  (drips)
```

| Region | Geometry | Spec |
|---|---|---|
| Window | w = min(1520, vw − 128), h = min(880, vh − 112), centred | C15, radius `xl`, e4, 2–3 drips |
| Sidebar | Expanded 264 / rail 80; inset 12; full height | C16. Sections: **MODULES** (categories that have ≥ 1 module, with a count badge of enabled modules), **CUSTOMIZE** (Theme, Background, Typography, Motion & effects, HUD editor ↗ = opens the full-screen editor), **CLIENT** (Keybinds, Settings, About). Footer: profile switcher (Dropdown C10 with Manage profiles…) |
| Header | Height 76, padding x 24 | Title (Title role) plus a summary line (Caption). Trailing controls depend on the page |
| Content | Scrolls (C19); padding 24 | Page content |
| Inspector | Width 408, right edge | Module details (below) |

### Modules page content

- **Header controls:**
  - Filter segmented control (C06): All / Enabled / Favourites.
  - View segmented control (icon-only): Grid / List.
  - Sort dropdown (C10): Name / Recently used / Enabled first.
- **Grid:** auto-fill columns with min card width 296 and gap 16. 3 columns at the reference size; 2 with the inspector open.
- **List view:** rows 64 tall (icon, name, description truncated to one line, keybind chip, toggle).
- **Cards:** module card (C14), with all states and interactions defined there.
- **Empty states (C29):**

| Case | Copy | Action |
|---|---|---|
| Favourites empty | "No favourites yet. Star a module to keep it here." | — |
| Enabled empty | "Nothing is on right now." | **Show all** |
| Category empty (dev only; empty categories are hidden in release builds) | — | — |

### Inspector

```
╭──────────────────────────────────────╮
│ (o) Settings Showcase    [SAMPLE]  x │  header 64
│ Demonstrates every control type.     │
│ It has no effect on gameplay.        │
│ ──────────────────────────────────── │
│ Enabled                        [on ] │  C31 rows
│ Keybind                [ Not bound ] │
│ Mode               [ Toggle | Hold ] │
│ v APPEARANCE                         │  C26 group
│ Accent strength        ───*────  60% │
│ Highlight colour         [# #DCE8F7] │
│ Style          [ Soft | Bold | Ink ] │
│ > BEHAVIOUR                          │
│ ──────────────────────────────────── │
│ ~ Reset to defaults     id: showcase │  footer
╰──────────────────────────────────────╯
```

- **Wide and Regular:** the inspector pushes the content (the grid reflows with `spring.soft`), as long as ≥ 2 columns remain. Otherwise it overlays the content with e3.
- **Compact:** it is a bottom sheet (full width, 70% height) with a drag handle.
- Closed by ✕, Esc (precedence chain), or by clicking the same card again.

### Modal states and notifications

- **Reset to defaults:** a Confirm modal (C20), "Reset Settings Showcase to its default settings?", with **Reset** (Danger) and **Cancel**.
- **Toggling a module:** no toast inside the shell (the card change is the feedback). Via keybind in-world: the compact in-world toast (C23).

### Shell responsive

| Class | Behaviour |
|---|---|
| Compact | Window full-bleed with 24 margins; sidebar rail; inspector as a sheet; cards 1–2 columns |
| Wide | Window max 1760 × 960; 4–5 columns |

**Studio pages** (S05, S07) ask for the sidebar **rail** in Regular, and restore it on leaving. S06 uses docked mode.

### Keyboard (shell-wide)

| Keys | Action |
|---|---|
| `Ctrl K` or `/` | Command palette |
| `Ctrl B` | Toggle the sidebar |
| `Ctrl 1…9` | Jump to module category n |
| `Ctrl ,` | Settings |
| `Alt ←` / `Alt →`, `Mouse 4` / `Mouse 5` | Back / forward |
| `F6` | Cycle focus regions: sidebar → header → content → inspector |
| `Esc` | Precedence chain |

---

## S04 — Settings

**Route:** `shell/settings`.

**Layout:** a sticky mini table of contents (200 wide, Body Small links with an active-section indicator dot that morphs) on the left; a sections column (max 760) on the right. Each section is an Info card (C14) containing Setting rows (C31).

```
 Settings                                                    [ / Filter settings ]
 ┌───────────────┐ ╭──────────────────────────────────────────────────────────────╮
 │ • Interface   │ │ INTERFACE                                                    │
 │   Main menu   │ │ UI size                      [Auto|Custom]  ──*──── 100%     │
 │   ClickGUI    │ │ Text size                                   ───*─── 1.00×    │
 │   Notifica…   │ │ Remember last page                                   (*  )   │
 │   Profiles    │ │ Show tooltips · Tooltip delay              (*  ) ──*── 450 ms│
 │   Advanced    │ ╰──────────────────────────────────────────────────────────────╯
 └───────────────┘ ╭──────────────────────────────────────────────────────────────╮
                   │ MAIN MENU …                                                  │
```

| Section | Settings |
|---|---|
| **Interface** | UI size (Auto / Custom 75–150%); Text size (0.80–1.40×); Language → opens the vanilla language screen; Remember last page; Show tooltips; Tooltip delay (200–1000 ms); UI sounds (Off / On, vanilla click with Ooze pitch); UI sound volume |
| **Main menu** | Use Ooze main menu; Intro (Every launch / After updates / Never); Show Continue card; Show Recent servers; Ping servers on main menu; Show What's new; Show tagline; Window title shows "Ooze Client" (default off) |
| **ClickGUI** | Open key (C13); Pause singleplayer while open (default off); World blur (uses the vanilla blur, with a link to its option); World dim (0–80%); Open on last page |
| **Notifications** | Enabled; Position (four corners, C06 with icons); Duration multiplier (0.5–3×); In-world module toasts |
| **Profiles** | Active profile (C10); New, Duplicate, Rename, Delete (modals); Import settings… / Export settings… (file dialog or folder fallback); Open config folder |
| **Advanced** | Effects quality is linked to S08; Allow low-contrast backgrounds (confirm modal); Debug overlay (frame stats); Developer tools (Render Lab, Component Gallery, Type Lab); Reset all Ooze settings (a Danger modal that asks you to type `RESET`) |

The filter field narrows the visible rows. Matching text is highlighted, and empty sections collapse.

---

## S05 — Theme Studio

**Route:** `shell/theme`. **Layout:** three columns with the sidebar in rail mode.

```
 Theme                                     [Undo][Redo]  [Compare o hold]  [Export]  [Duplicate]
 ╭────────────────────╮ ╭──────────────────────────────────────────╮ ╭──────────────────────────╮
 │ THEMES             │ │                LIVE PREVIEW              │ │ TOKENS          [/]      │
 │ ╭────────────────╮ │ │  ╭────────────────────────────────────╮  │ │ v PALETTE                │
 │ │# Ooze Classic v│ │ │  │ Card title              (*  )      │  │ │ Ooze white   [# #F8F9FF] │
 │ ╰────────────────╯ │ │  │ Body text sample, secondary text   │  │ │ Pearl white  [# #E9EDF5] │
 │ ╭────────────────╮ │ │  │ [ Primary ] [ Secondary ]  ──*──   │  │ │ Ink          [# #171923] │
 │ │# Midnight Gel  │ │ │  ╰────────────────────────────────────╯  │ │ v ACCENT                 │
 │ ╰────────────────╯ │ │   [Tab A|Tab B]  ( tooltip )  i toast    │ │ Accent  [Ink|Glacier|…]  │
 │ … Mint · Bubblegum │ │   Text roles: Display … Caption          │ │ > SURFACES  > OUTLINE    │
 │ · High Contrast    │ │                                          │ │ > MATERIAL  > SHAPE      │
 │ MY THEMES          │ │   Contrast: 6 checks v  1 warning !      │ │ > ELEVATION > STATUS     │
 │ # Mint Copy   :    │ ╰──────────────────────────────────────────╯ ╰──────────────────────────╯
 │ [+ New] [Import]   │
 ╰────────────────────╯
```

| Column | Width | Content |
|---|---|---|
| Themes | 280 | Built-in presets (read-only; tagged "Built-in") and My themes (: menu: Rename, Duplicate, Export, Delete). Each card shows a mini preview swatch (C30). Clicking a card applies the theme live (with the theme-switch animation) |
| Preview | flex | A real-component sandbox (C30 with `interactive = true`): card, buttons in every variant, toggle, slider, segmented control, tabs, field, tooltip pinned open, toast sample, type ramp. **Hovering a token** in the right column pulses the elements that use it (from each component's `TOKENS` descriptor) |
| Tokens | 360 | Grouped Accordions: Palette, Accent (preset segmented control plus a custom picker), Surfaces, Outline (weight scale slider 0–1.5), Material (gloss, rim, shade, shimmer, surface opacity, drips on/off and count), Shape (radius scale slider 0.5–1.5×, corner exponent 2.0–3.5), Elevation (shadow strength 0–2×), Status colours. Every row has a reset (↺) when it differs from the parent theme |

**Rules**
- Editing a built-in theme first prompts: "Built-in themes can't be changed. Duplicate it as 'Ooze Classic Copy'?" (**Duplicate and edit** / **Cancel**).
- **Contrast panel:** the live checks from DESIGN_SYSTEM §7.3. Warnings name the failing pair and offer **Fix** (adjust L in OKLab).
- **Compare:** hold to view the parent or original theme (crossfade 160 ms).
- **Undo/redo:** 100 steps; the history persists while the page is open.
- **Export:** `.theme.json` via file dialog or folder. **Import:** validates the file, shows warnings, and resolves id conflicts by renaming.

---

## S06 — Background Studio

**Route:** `shell/background`. **Docked mode:** on entry, the shell window **liquid-morphs** into a right-docked panel (420 × (vh − 48), at 24 margins; `spring.soft` on the rect, with the radius wobbling as it settles). The sidebar hides and the actual background fills the rest of the screen.

```
┌─────────────────────────────────────────────────────────────┬────────────────────────────────────┐
│                                                             │ < Background            [Peek]     │
│                                                             │ [Main menu|Ooze menus|World|HUD]   │
│                                                             │ [ ] Same as main menu              │
│               ( live background, full screen )              │ TYPE                               │
│                                                             │ [Solid|Gradient|Image|Wallpaper|   │
│                                                             │  Animated]                         │
│     ╭──────╮  ╭──────╮  ╭──────╮                            │ ╭─────╮╭─────╮╭─────╮              │
│     │ menu │  │ ooze │  │ hud  │  < context mini-previews   │ │ img ││ img ││  +  │  library     │
│     ╰──────╯  ╰──────╯  ╰──────╯    (C30, 160×90)           │ ╰─────╯╰─────╯╰─────╯              │
│                                                             │ FIT  Fill|Fit|Stretch|Center|Tile  │
│                                                             │ Zoom ──*──── 1.15×   Focus [+]     │
│                                                             │ EFFECTS  Blur · Dim · Opacity ·    │
│                                                             │   Tint · Vignette · Grain          │
│                                                             │ AMBIENT  Blobs · Particles ·       │
│                                                             │   Parallax · Speed                 │
│                                                             │ Readability: v Readable            │
│                                                             │ [Undo][Redo]           [Revert]    │
└─────────────────────────────────────────────────────────────┴────────────────────────────────────┘
```

| Section | Content |
|---|---|
| Context | Segmented control (BACKGROUNDS §2) plus the inherit checkbox |
| Type | Segmented control. Each type swaps in its parameter block with a 160 ms crossfade |
| Solid | Color picker |
| Gradient | Kind (Linear / Radial); an angle dial (a draggable gel knob on a ring; arrow keys ±5°); a stop editor (a horizontal gradient bar with draggable gel stop handles; click to add, drag off or Delete to remove; 2–6 stops) |
| Image | Library grid (3 columns of 16:9 thumbnails, skeletons while loading; : menu: Rename, Delete with confirm, Reveal file); an **Import** tile with **Pick a file…**, **Open folder** and the drop hint; Fit; Zoom; focal point picker; quality notice chip; suggested-colour swatches |
| Wallpaper | Preset cards (C14 preview variant) plus their params and **Shuffle** |
| Animated | Stops editor, motion (Drift / Rotate / Breathe), speed |
| Effects | Sliders: Blur 0–40, Dim 0–85%, Opacity, Tint (colour plus strength), Vignette, Grain |
| Ambient | Blobs (count, size, opacity, colour, attract), Particles (density, size, speed, direction), Parallax depth, Ambient speed |
| Readability | The legibility guard badge (BACKGROUNDS §7). When a plate is used, **Add 20% dim** is offered |

- **Peek:** hold the button or `Space` (when no text field is focused) to fade all UI out.
- **Drop zone:** dragging files over the window shows a full-screen dashed gel outline with "Drop images to import".
- **Leaving:** the panel morphs back into the shell window.

---

## S07 — Typography

**Route:** `shell/typography`. Sidebar rail. Three columns.

```
 Typography                                                     [Reset all]  [Open fonts folder]
 ╭────────────────────╮ ╭────────────────────────────────────────────╮ ╭────────────────────────╮
 │ INTERFACE FONT     │ │ PREVIEW         [In context|Type ramp]     │ │ ROLE: Body Medium      │
 │ [Bubblegum Sans v] │ │                                            │ │ Font   [Inherit v]     │
 │ Text size ─*─ 1.0  │ │ Display — Soft on the outside.             │ │ Size   ─*─  16 ou      │
 │ [x] Normalize size │ │ Headline — Squishy, glossy, crisp.         │ │ Style  [Regular v]     │
 │ [x] Allow synth.   │ │ Title — Settings Showcase                  │ │ Tracking ─*─ +0.010em  │
 │ [ ] Bolder text    │ │ Body — The quick brown fox jumps over…     │ │ Line height ─*─ 1.44×  │
 │ [ ] Body font      │ │ Caption — Minecraft 1.21.11 · Ooze 0.1.0   │ │ Contrast 5.3:1 v AA    │
 │ ROLES              │ │                                            │ │ [Synthesized] (if any) │
 │ • Display          │ │                                            │ │ ~ Reset role           │
 │ • Headline  …      │ │                                            │ │                        │
 │ • Status           │ ╰────────────────────────────────────────────╯ │ LICENCE                │
 │                    │                                                │ Bubblegum · OFL 1.1    │
 │                    │                                                │ [View licence]         │
 ╰────────────────────╯                                                ╰────────────────────────╯
```

- **Left:** global settings (TYPOGRAPHY §8) and the roles list. Selecting a role highlights it in the preview.
- **Preview modes:**
  - **In context:** a mini shell made of real components, with role usages outlined on hover.
  - **Type ramp:** every role stacked, with its size and line metrics shown in Caption.
- **Right:** the selected role's properties, a contrast check, synthesis status and the licence of the active family.
- All changes are live and invalidate text caches (one frame of re-layout). Undo and redo are supported.

---

## S08 — Motion & effects

**Route:** `shell/motion`. Two columns: settings (max 640) and a sticky **Motion Lab** (480).

```
 Motion & effects
 ╭──────────────────────────────────────────────╮  ╭────────────────────────────────────╮
 │ MOTION                                       │  │ MOTION LAB                [Replay] │
 │ Animations                          (*  )    │  │  [ Button ]   (*  )   ──*────      │
 │ Reduced motion          [Off|On|Auto]        │  │  [A|B|C]      ╭ panel ╮            │
 │   Auto follows Minecraft's Distortion Effects│  │  spring curve: gel ~ (live plot)   │
 │ Intensity                ──────*──  100%     │  │  ┌───────────────────────────────┐ │
 │ Speed                    ───*─────  1.0×     │  │  │ Ooze UI: 0.84 ms  · 21k verts │ │
 │ EFFECTS QUALITY                              │  │  │ Quality: High (adaptive)      │ │
 │ [Low|Medium|High|Ultra|Custom]  [x] Adaptive │  │  └───────────────────────────────┘ │
 │ > Custom effects (shown when Custom)         │  ╰────────────────────────────────────╯
 │ AMBIENT                                      │
 │ Ambient motion (*  )   Ambient speed ─*─ 1×  │
 ╰──────────────────────────────────────────────╯
```

| Group | Settings |
|---|---|
| Motion | Animations; Reduced motion (Off / On / Auto); Intensity 0–150%; Speed 0.5–2.0× (MOTION_SPEC §8) |
| Effects quality | Low / Medium / High / Ultra / Custom (PERFORMANCE_AND_ACCESSIBILITY §3); Adaptive quality toggle |
| Custom effects | Shadows (1 or 2 layers); Gloss shimmer; Drips (Static / Animated / Off); Breathing edges; Surface tension (cursor-reactive); Ambient blobs; Particles; Parallax; Background blur |
| Ambient | Ambient motion; Ambient speed |

**Motion Lab**
- Live samples of a button, toggle, segmented control, slider and a mini panel (open/close). **Replay** re-runs the entry animations.
- A **spring plot** draws x(t) for the spring the user last interacted with, recomputed when Intensity or Speed changes. It is a 2D line built from the closed-form solution.
- **Frame stats readout:** Ooze CPU ms (average and p95 over 2 s), vertices, batches, and the current effective quality with an "(adaptive)" note. This is real data from the frame profiler.

---

## S09 — HUD editor

**Route:** `HudEditorScreen` (full screen).
- **In-world:** the world is visible with no blur and no dim. The vanilla HUD is drawn for reference.
- **From the menu:** the `hud_editor` background is shown with vanilla HUD **silhouettes** (dashed outlines of the hotbar, health and hunger rows, crosshair, chat area, boss bar and scoreboard) labelled "Layout preview".

```
 ┌──────────────────────────────────────────────────────────────────────────────────────────┐
 │      ╭────────────────────────────────────────────────────────────────────────────╮      │
 │      │ [+ Widgets] │ [# Grid] [o Snap] [# Safe zones] │ [<][>] [Reset] │ [ Done ] │      │  floating toolbar pill (top centre;
 │      ╰────────────────────────────────────────────────────────────────────────────╯      │  fades while dragging)
 │  ┌ ─ ─ ─ ─ ─ ┐                                                                           │
 │    120 FPS     < selected: accent outline, 4 scale handles, * chip                       │
 │  └ ─ ─ ─ ─ ─ ┘   x 24 · y 24 · TOP_LEFT  (coordinate chip while dragging)                │
 │ ────────────────────────── centre guide (appears when near) ──────────────────────────── │
 │                                            +                                             │
 │                                                                    ╭──────────╮          │
 │                                                                    │ 14:32    │          │
 │                                                                    ╰──────────╯          │
 │ %%% chat zone (safe zone) %%%                                                            │
 │                        %%%%%%%%%% hotbar zone %%%%%%%%%%                                 │
 │                                                                                          │
 └──────────────────────────────────────────────────────────────────────────────────────────┘
```

| Feature | Spec |
|---|---|
| Widgets | Every enabled HUD widget is rendered live and is draggable. Clicking selects; Shift+click multi-selects; dragging on empty space draws a marquee selection |
| Snapping | Magnetic (6 ou) to screen edges (with an 8 ou safe margin), the centre lines, other widgets' edges and centres, and the 8-ou grid (when the grid is on). Guides are drawn as 1 ou `outline.focus` lines with α fades. Hold `Alt` to suspend snapping |
| Anchors | On drop, the anchor is the nearest of 9 regions. The offset is stored relative to that anchor, so layouts adapt to resolution changes (ARCHITECTURE §10) |
| Scaling | Corner handles scale 0.5–3.0× (Shift preserves the anchor corner); the scale snaps to 0.05 steps |
| Settings | The ⚙ chip or a double-click opens a popover (C21) with the widget's settings (C31 rows) |
| Widget library | **+ Widgets** opens a drawer (left side, 320 wide) listing available widgets (with the Sample badge) with toggles and descriptions. Enabling one places it at its default position |
| Safe zones | Hatched overlays (α 0.12) of the vanilla HUD areas, computed from the vanilla layout at the current Minecraft GUI scale. A widget overlapping a zone gets an amber outline and the tooltip "Overlaps the hotbar" |
| Toolbar | A floating pill (C15 mini) with Ghost buttons. It auto-hides (fades to 0.2 α) while dragging and reappears on hover |
| History | Undo/redo for move, scale, add, remove and setting changes (100 steps) |
| Exit | **Done** or `Esc` saves and exits (auto-save). **Reset** opens a confirm modal that restores the default layout |

**Keyboard**

| Keys | Action |
|---|---|
| Tab / Shift+Tab | Cycle widgets |
| Arrows / Shift+Arrows | Nudge 1 ou / 8 ou |
| `+` / `−` | Scale ±0.05 |
| Delete | Disable the widget |
| Enter | Open the settings popover |
| `G` | Toggle the grid |
| `S` | Toggle snapping |
| `Z` | Toggle safe zones |

**Rendering note:** while the editor is open, the normal in-game Ooze HUD pass is suppressed. The editor draws the widgets itself, so nothing is drawn twice.

---

## S10 — Keybinds

**Route:** `shell/keybinds`.

```
 Keybinds                          [/ Search actions]   [All|Ooze|Modules|Conflicts !1]
 ╭─────────────────────────────────────────────────────────────────────────────────────────╮
 │ OOZE                                                                                    │
 │ / Open ClickGUI           Opens the Ooze menu        [ R Shift ]   [Toggle]       ~     │
 │ + Open HUD editor         …                          [ Not bound ] [Toggle]             │
 │ / Command palette (in GUI)…                          [ Ctrl K ]    fixed                │
 │ MODULES                                                                                 │
 │ o Settings Showcase [SAMPLE]                         [ Not bound ] [Toggle|Hold]        │
 │ o Notification Playground [SAMPLE]                   [ R Ctrl ] !  [Toggle|Hold]   ~    │
 ╰─────────────────────────────────────────────────────────────────────────────────────────╯
 Minecraft's own controls live in Options › Controls.                        [ Open controls ]
```

- **Rows:** icon, action name (Body Medium), description (Body Small), keybind chip (C13), Mode (Toggle / Hold; Hold only applies to modules), and reset (↺) when the binding differs from its default.
- **Conflict detection:** across all Ooze binds, plus vanilla `KeyMapping`s (read through the adapter). The conflict chip's tooltip lists the conflicting actions, with **Unbind other** and **Keep both** buttons in a popover.
- Ooze's core actions are also registered as vanilla key mappings (category "Ooze Client"), so they appear in vanilla Controls too. Both UIs edit the same binding.
- **Fixed in-GUI shortcuts** (`Ctrl K`, `Esc`, …) are listed as read-only with the "fixed" label.

---

## S11 — Search and navigation

Navigation is provided by two cooperating parts: the **sidebar** (S03) and the **command palette**.

### Command palette (C32)

```
            ╭────────────────────────────────────────────────────────────────────────╮
            │ /  acc                                                       [ Esc ]   │  input 56
            │ ────────────────────────────────────────────────────────────────────── │
            │ SETTINGS                                                               │
            │ o Accent strength   Showcase › Settings Showcase › Appearance  Setting │  < highlighted
            │ o Accent            Theme › Accent                             Setting │
            │ PAGES                                                                  │
            │ A Accessibility     Minecraft › Options                         Page   │
            │ ACTIONS                                                                │
            │ @ Reset accent      Theme                                      Action  │
            │ ────────────────────────────────────────────────────────────────────── │
            │ ^v Navigate · Enter Open · Tab Scope: All · Esc Close                  │
            ╰────────────────────────────────────────────────────────────────────────╯
```

- **Opening:** `Ctrl K` or `/`, or clicking the sidebar search field. The field **morphs** into the palette (liquid morph of its rect, MOTION_SPEC §3.3). With the sidebar in rail mode, the palette appears centred.
- **Scopes:** cycle with Tab through All → Modules → Settings → Pages → Actions. The current scope shows in the footer and as a chip in the input.
- **Result types:**

| Type | Content | Activation |
|---|---|---|
| Module | Module name, description, tags | Opens its category and inspector |
| Setting | Setting name and description; path in the subtitle | Navigates, scrolls to the row, and pulses it twice (C31) |
| Page | Every shell page plus selected vanilla screens (Options, Controls, Language, Accessibility, Resource packs, Video settings) | Opens the page or screen |
| Action | Toggle HUD, Open HUD editor, Reload fonts, Open config folder, Export settings, Switch profile…, Switch theme… | Runs the action (with a confirm modal if destructive) |
| Theme / Background preset | Name | Applies it, with an undo toast |

- **Matching:** fuzzy matching per [ARCHITECTURE.md §12](ARCHITECTURE.md). Up to 8 results per group and 30 in total. With an empty query: **Recent** (last 8 picks) then **Suggested** (pages).
- **Keyboard:** Up/Down (with wrap), Enter opens, `Ctrl Enter` opens without closing the palette (for toggling several modules), Esc closes.
- **No results:** "No matches for 'xyz'. Try a module name, setting or page."

---

## S12 — About

**Route:** `shell/about`. A centred column, max 880 wide.

```
                        ╭─────────────────────────────╮
                        │  (o)  ooze   CLIENT         │   hero plate (animated drip)
                        ╰──────╮╭──────────────╮╭─────╯
                               ╰╯              ╰╯
                         Soft on the outside. v0.1.0 "Gel"
 ╭────────────────────────────────────────────────────────────────────────────────╮
 │ VERSION                                                                        │
 │ Ooze Client      0.1.0+mc26.1        Minecraft     26.1.2                      │
 │ Fabric Loader    0.x.y               Fabric API    0.x.y+26.1                  │
 │ Java             25.0.1 (vendor)     Renderer      Vulkan · <GPU name>         │
 │ [Copy debug info]  [Open config folder]  [Open logs folder]                    │
 ╰────────────────────────────────────────────────────────────────────────────────╯
 ╭─────────────╮ ╭───────────╮  ╭─ LICENCES v ────────────────────────────────────╮
 │ CHANGELOG > │ │ CREDITS > │  │ Ooze Client — MIT              [View licence]   │
 ╰─────────────╯ ╰───────────╯  │ Bubblegum Sans — OFL 1.1       [View licence]   │
                                │ Inter — OFL 1.1                [View licence]   │
                                │ Caveat · Cormorant Garamond · Lucide (ISC)      │
                                ╰─────────────────────────────────────────────────╯
 [ GitHub ^ ]  opens the repository URL through Minecraft's confirm-link dialog
```

- **Version data** is real, from the platform info adapter. The renderer row shows the active backend (OpenGL or Vulkan) and the GPU renderer string.
- **Copy debug info** copies a Markdown block (versions, OS, renderer, quality tier, UI scale, active theme, installed mods count) to the clipboard and shows a success toast.
- **Changelog** comes from the bundled `changelog.json`. **Credits** lists contributors from `credits.json`.
- **Licences** use Accordion (C26) rows. **View licence** opens the Licence viewer modal (C20) with the full text.
- External links always go through Minecraft's confirm-link dialog (the vanilla `ConfirmLinkScreen`, via the adapter).
