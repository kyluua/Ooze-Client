# Ooze Client — Feature Backlog (real modules)

> The framework samples (Settings Showcase, Notification Playground, the FPS, Clock and Watermark widgets) prove the framework. **They are not a finished client.** Real modules come from this backlog, **one entry per milestone (F-xx)**, after C5 (modules framework) is complete.
>
> Every entry is **not started** until its milestone handoff says otherwise.

## Rules for every entry

- **Fair-play class** (ARCHITECTURE §8.1): `COSMETIC`, `INFORMATIONAL`, `QUALITY_OF_LIFE` or `GAMEPLAY`.
  - `GAMEPLAY` entries stay blocked until a server-policy design exists (servers must be able to disable them).
- **Split:** Java hooks (Claude) read or affect the game through snapshots and commands. Rust (`ooze-ui` plus Sol's engine algorithms) owns settings, state, UI and HUD composition.
- **Acceptance:**
  - Game-launched on Windows x64;
  - settings persist;
  - keyboard and narration;
  - reduced motion;
  - no allocation in steady-state HUD frames;
  - vanilla behaviour unchanged when the module is off.

## Excluded permanently

Anything that automates or assists combat, movement, building or mining. That includes reach, aim assist, auto-clickers, ESP, X-ray, flight and movement exploits, and anything that reads information the vanilla client does not show. These are not Ooze features.

## Entries

| Id | Module | Class | Java hooks needed | Rust (settings, UI, HUD) | Notes |
|---|---|---|---|---|---|
| F-01 | Zoom (hold key, smooth) | QUALITY_OF_LIFE | Modify FOV per frame (an event, or a minimal `require = 0` mixin), key hold from the input batch | Zoom factor, smoothing spring, scroll-to-adjust | Common on servers; verify that the FOV-change path respects the vanilla FOV Effects slider |
| F-02 | Coordinates and facing HUD | INFORMATIONAL | Snapshot: block position, facing, dimension | Widget layout and format | The same information as F3 |
| F-03 | Armour and durability HUD | INFORMATIONAL | Snapshot: equipped items and durability; **vanilla item-icon draw command** (Java draws icons with vanilla item rendering at positions Rust specifies) | Widget, thresholds, colours | Needs the item-icon command in the protocol (request to Sol) |
| F-04 | Status effects HUD | INFORMATIONAL | Snapshot: active effects and remaining time; effect icons drawn by Java | Widget, compact/gel style | Restyles vanilla information |
| F-05 | Keystrokes and CPS | INFORMATIONAL | Input batch only | Widget, CPS window | No input injection, ever |
| F-06 | Ping and server info | INFORMATIONAL | Snapshot: own latency from the player list, server address name | Widget | Hidden in singleplayer |
| F-07 | Screenshot manager | QUALITY_OF_LIFE | List screenshot files; open the folder; copy an image to the clipboard (platform command) | Gallery screen, thumbnails (budgeted) | No uploading anywhere |
| F-08 | Custom crosshair | COSMETIC | Replace the vanilla crosshair HUD element through Fabric's HUD registry when enabled | Shape editor, colours | Off → vanilla crosshair |
| F-09 | Chat timestamps and chat tweaks | QUALITY_OF_LIFE | Chat line decoration hook | Format settings | Must not alter message content or signing |
| F-10 | Freelook | GAMEPLAY | Camera hook | — | **Blocked** until the server-policy design exists |
| F-11 | Brightness override | GAMEPLAY | Gamma or lightmap hook | — | **Blocked** until the server-policy design exists |
