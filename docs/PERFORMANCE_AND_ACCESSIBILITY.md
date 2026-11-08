# Ooze Client — Performance & Accessibility Requirements

> Deliverable 13 (plus the accessibility requirements from the brief). Budgets are **requirements**: a milestone is not complete if its scenes exceed them ([TESTING.md §6](TESTING.md)).

## 1. Reference hardware and method

| Tier | CPU | GPU | Display |
|---|---|---|---|
| **Reference (mid)** | 6-core desktop CPU, 2019-class (e.g. Ryzen 5 3600 / Core i5-10400) | GTX 1660-class or integrated Radeon 680M-class | 2560×1440 at 144 Hz |
| Low | 4-core laptop CPU, 2018-class | Intel UHD 620-class iGPU | 1920×1080 at 60 Hz |
| High | 8-core 2023-class | RTX 4070-class | 3840×2160 at 120 Hz |

**Measurement:** Ooze's `FrameProfiler` (ARCHITECTURE §15), 60-second runs after a 10 s warm-up, reporting p50, p95 and p99 of Ooze CPU time per frame. GPU time is sampled with an external profiler (RenderDoc or Nsight) on the reference tier at milestone gates.

## 2. Budgets

### 2.1 Frame time and allocation

| Scene | Ooze CPU p95 (reference) | Ooze CPU p95 (low) | Steady-state heap allocation per frame |
|---|---|---|---|
| HUD with 3 sample widgets (in world) | ≤ 0.25 ms | ≤ 0.5 ms | **0 bytes** (asserted by test) |
| Main menu, High quality, ambient on | ≤ 1.5 ms | ≤ 3.0 ms | ≤ 2 KB |
| Shell with 40 module cards (virtualised), inspector open | ≤ 2.0 ms | ≤ 4.0 ms | ≤ 8 KB |
| Theme Studio while dragging a colour | ≤ 3.0 ms | ≤ 6.0 ms | ≤ 32 KB |
| HUD editor while dragging | ≤ 1.5 ms | ≤ 3.0 ms | ≤ 4 KB |

- **GPU:** Ooze UI ≤ 1.0 ms at 2560×1440 (reference) and ≤ 1.5 ms at 3840×2160 (high), excluding the world and vanilla blur.
- **Vertices:** ≤ 60k per frame for any shell page at 4K. ≤ 6k for the HUD.
- **Batches:** ≤ 40 per frame in the shell. ≤ 6 for the HUD.

### 2.2 Memory

| Resource | Budget |
|---|---|
| Glyph atlases | ≤ 24 MB GPU (6 pages × 1024² × 4 B; larger pages only at high scale, with the page count reduced) |
| Backgrounds | ≤ 64 MB GPU (BACKGROUNDS §4.5) |
| Mesh cache | ≤ 512 meshes, ≤ 8 MB heap |
| Text layout cache | 4096 entries, ≤ 6 MB heap |
| Total Ooze retained heap | ≤ 64 MB, excluding background image CPU copies (released after upload unless they are needed for blur rebuilds) |

### 2.3 Start-up and I/O

| Item | Budget |
|---|---|
| Main-thread bootstrap | ≤ 150 ms. Fonts, images and themes load asynchronously |
| Time to an interactive main menu after the vanilla overlay | ≤ 300 ms, intro excluded |
| Config write | Never on the render thread except bounded flushes (≤ 500 ms) at close and stop |
| Image import (4K PNG) | Off-thread. The UI stays at the target FPS throughout |

## 3. Effects quality tiers and the adaptive governor

| Feature | Low | Medium | **High** (default) | Ultra |
|---|---|---|---|---|
| Shadows | Ambient only, 3 rings | 2 layers, 5 rings | 2 layers, 5 rings | same |
| Gloss band, glint, rim | ✓ | ✓ | ✓ | ✓ |
| Shimmer | — | ✓ | ✓ | ✓ |
| Specular parallax | — | — | ✓ | ✓ |
| Drips | Static | Animated (≤ 4 on screen) | Animated (≤ 8) | Animated (≤ 8) |
| Breathing edges | — | — | — | ✓ |
| Surface tension (cursor) | — | — | — | opt-in |
| Ambient blobs | — | ≤ 4 | ≤ 6 | ≤ 6 |
| Particles | — | ≤ 40 | ≤ 80 | ≤ 140 |
| Parallax | — | ✓ | ✓ | ✓ |
| World blur | Off (dim only) | ✓ | ✓ | ✓ |
| Blur for image backgrounds | Cached, at ½ resolution | Cached | Cached | Cached, at full bucket resolution |

### Adaptive governor (on by default)

Inputs, sampled each frame:
- `oozeMs`: Ooze CPU time, EMA with α = 0.1;
- `frameMs`: total frame time, EMA;
- the target refresh rate (the vanilla FPS limit, or 60 if unlimited).

```
demote (one feature step, following MOTION_SPEC §9 shedding order) when, for 2 consecutive seconds:
    oozeMs > budget(scene) × 1.25   OR   (frameMs > 1000/targetFps × 1.10 AND oozeMs > 0.15 × frameMs)
promote (one step back) when, for 10 consecutive seconds:
    oozeMs < budget(scene) × 0.6    AND   frameMs < 1000/targetFps × 0.85
never promote above the user-selected tier; never demote below Low.
```

- The effective tier is shown in S08 as "High (adaptive: Medium)".
- Demotions are silent, except for one diagnostic log line per session.
- When the window is unfocused, ambient work is suspended entirely (MOTION_SPEC §2).

## 4. Engineering techniques (required)

1. **Retained tree plus dirty flags.** Layout runs only for dirty subtrees. Paint uses cached meshes (ARCHITECTURE §5.4).
2. **Colour evaluated at emission.** Animating a colour or opacity never re-tessellates.
3. **No per-frame allocation in hot paths:**
   - Reusable primitive arrays for the DrawList.
   - No lambdas or iterators in paint loops over children (indexed loops).
   - No boxing; no `String.format` / `String.valueOf` per frame (cache formatted strings and rebuild them only when values change).
   - fastutil collections where maps are needed.
4. **Virtualisation** for lists over 50 rows (C19). Off-screen nodes are culled before paint.
5. **Text:** layout cache, glyph atlases, uploads budgeted to 2 ms per frame, pre-warming (TYPOGRAPHY §5–6).
6. **Textures:** only the mip level needed is resident. Background images are released after crossfades.
7. **Sleeping animations** (MOTION_SPEC §3.2) and parametric ambient motion.
8. **Worker threads** for decode, blur, rasterization and font scanning. The render-thread queue is time-budgeted.
9. **HUD fast path:** HUD widgets update their text at most `updateHz` (default 4 Hz for FPS). Their meshes are fully cached, and no layout runs while the content is unchanged.
10. **Input latency:** input events are dispatched as they arrive. Visual feedback is applied in the same frame (pressed state set before paint). Nothing waits for I/O.

## 5. Reliability

- Atomic config writes, `.bak` fallbacks, corruption quarantine and start-up backups (ARCHITECTURE §9).
- Read-only mode for configs from newer versions.
- Listener failure isolation and module auto-disable.
- Optional mixins degrade gracefully.
- **Crash test** (TESTING §7): killing the process while saving must never lose more than the last 5 seconds of changes and must never produce an unreadable config.

## 6. Accessibility requirements

### 6.1 Keyboard

- [ ] Every screen and every control works with the keyboard only (INTERACTION_FLOWS §3).
- [ ] Logical tab order; focus scopes with `F6`; roving focus in composites.
- [ ] A visible focus ring (COMPONENTS §1.3) for keyboard modality. It is never hidden by clipping.
- [ ] Every drag interaction has a keyboard equivalent: slider arrow keys; HUD nudge and scale keys; colour picker numeric fields and arrows; gradient stop editor arrows and `+`/`Delete`; focal point arrows.
- [ ] Shortcuts are listed in tooltips and in the Keybinds page (fixed shortcuts section).

### 6.2 Motion

- [ ] Reduced motion (Off / On / Auto from Minecraft's *Distortion Effects* = 0%) follows the mapping in MOTION_SPEC §8.
- [ ] Animations Off leaves no element in an intermediate state. Every state is reachable and visible instantly.
- [ ] No flashing above 3 Hz; no full-screen flashes.

### 6.3 Vision and text

- [ ] UI scale 75–150% and text scale 0.80–1.40×, independent of each other and of Minecraft's GUI scale.
- [ ] Minimum rendered text size of 11 ou. Minimum hit target of 32 ou (40 recommended).
- [ ] Contrast: body text ≥ 4.5 : 1; large text ≥ 3 : 1; UI component boundaries and focus indicators ≥ 3 : 1. Enforced by validators in the Theme Studio and Typography page.
- [ ] State is never conveyed by colour alone: toggle knob position, check glyph, selected indicator shape, and warning icons alongside colour.
- [ ] A High Contrast theme is built in. Bolder text option.
- [ ] Legibility guard for text on backgrounds (BACKGROUNDS §7).

### 6.4 Narration (screen reader)

- [ ] When Minecraft's narrator is active, focus changes narrate `NarrationInfo`: role, label, value, state and hint. For example: "Toggle, Gloss shimmer, on", or "Slider, Intensity, 80 percent, use arrow keys to adjust".
- [ ] Page changes narrate the page title. Toasts narrate their title and body (polite, not interrupting). Modals narrate their title and body on open.
- [ ] Narration is routed through `PlatformNarrator`, and it is silent when the narrator is off.

### 6.5 Time and attention

- [ ] Toasts pause on hover or focus. Their duration is adjustable (0.5–3×). Errors persist until dismissed.
- [ ] No time limits on any interaction. Keybind listening mode has no timeout.
- [ ] Ambient motion can be turned off independently of interaction animations.
