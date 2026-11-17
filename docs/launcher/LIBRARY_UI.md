# Ooze Launcher — Library Interface Specification (P0/L1–L3)

> Owner: Claude (interface).
> Backend contract: Sol's [`LIBRARY_API_V1.md`](LIBRARY_API_V1.md) (Library API 1).
> Module definitions: [`../MODULES.md`](../MODULES.md).
> Scope: the owner's 2026-11-17 expansion: instances, profiles, themes, Modrinth content and integrity modes.
> The L0 interface (`LAUNCHER_UI.md`) is unchanged and still governs sign-in, Play, Settings and Diagnostics.

## 1. Rules

1. **Three distinct things:**
   - an **instance** is game version + loader + runtime + files + content;
   - a **profile** is module, HUD and visual configuration;
   - a **theme** is interface appearance.

   Each has its own page, its own words and its own reset. One never silently changes another.
2. **Backend decides, interface shows.**
   - The interface never decides compatibility, resolves dependencies, builds paths, downloads files or checks hashes.
   - It renders the library's typed snapshot and results, and submits typed commands.
   - It never invents a supported version, loader, content type or action.
3. **Capabilities gate everything.**
   - A page, tab or button appears only when the library reports that capability.
   - With no library (today's release), the launcher is exactly the L0 launcher: no empty pages, no "coming soon".
4. **Real or absent.**
   - Every result shown comes from the backend.
   - The fixture library exists only for tests and the developer preview (`dev-fixtures`), and is labelled as such.
5. **Honest outcomes.**
   - "Installed" means the files were verified and recorded. It never means the content works.
   - The interface says so for shader packs, resource packs and data packs.
   - Play readiness is still only `OOZE_READY` (`LAUNCHER_UI.md §6`).
6. **Fair play.**
   - Integrity modes are **Managed Fair Play** and **Custom**.
   - Never "cheat-proof", "anticheat", "safe on every server" or "server-approved".
   - Server-sensitive modules carry their notice wherever they are switched on.
7. **Keyboard, focus and narration** for every control, and stale-result handling for every async operation, exactly as in L0 (`LAUNCHER_UI.md §7–8`).

## 2. Navigation

The left navigation gains, in this order and only when capable:

| Item | Shown when |
|---|---|
| **Instances** | library `instances` capability |
| **Profile** | the selected instance has Ooze available and the library reports `profiles` |
| **Content** | the library reports at least one content kind for the selected instance |
| **Appearance** | the library reports `themes` |

Play, Settings and Diagnostics stay as in L0.
- **Selected-instance chip:** the header shows the selected instance ("Fabric 1.21.11 · PvP"). It is activatable and opens Instances.
- **Play** plays the selected instance through API 1: the interface saves the library's instance directory with `SaveSettings`, then sends `Launch`.

## 3. Instances

**List.** One row per instance:
- name, game version, loader and loader version;
- "Ooze modules available" or "Ooze modules are not available for this instance";
- profile (preset name or "Custom");
- integrity mode;
- install state (`Ready`, `Needs install`, `Needs repair`, `Archived`).

Row actions are Select, Rename, Duplicate, Install / Repair and Archive. Each appears only if the backend's per-instance action list contains it. Archive moves the instance into the library archive; the confirmation says it is not deleted.

**Create.**
- A dialog lists **only** the combinations from `capabilities.create` (today Fabric 0.20.1 / 1.21.11 and, if reported, Vanilla 1.21.11). There are no free-text version fields.
- Each option shows the Java requirement and whether Ooze is available.
- Name: 1–48 characters, trimmed. A duplicate name is allowed; the ID is the identity.

**Duplicate.** The confirmation states what is copied: configuration and registered content. It also states what is not copied: worlds, screenshots and options.

**Busy and stale.**
- Mutations are disabled while that instance's game runs. The reason is shown, not hidden.
- One library operation runs at a time: other actions show "Waiting for the current task" and remain focusable with that description.

## 4. Profile (per instance, Ooze instances only)

**Preset picker.**
- Three cards: **FPS**, **PvP**, **Minimal**. Each has one sentence and a "What this changes" list.
- The list is computed from `ooze_ui::profile::diff(catalog defaults, preset)`, so it cannot drift from the data.
- The FPS card says it **does not install performance mods or promise a frame-rate gain**.

**Switching preset.**
- The confirmation lists the differences between the current effective profile and the new preset.
- **User overrides are kept.** The dialog says how many, with an option to also reset them.

**Modules.**
- One row per catalog module (`ooze_ui::catalog::MODULES`): name, description, fair-play class tag, and an enable switch.
- Server-sensitive modules show the notice "Some servers do not allow this. Check a server's rules before using it." Blocked (GAMEPLAY) modules are not listed.
- Expanding a row shows typed editors built from `SettingDef`:
  - switch (Bool);
  - stepper with range and step (Int/Float);
  - segmented control (Choice ≤ 4) or list (more);
  - colour field with hex entry and contrast hint (Color);
  - key capture (Key).

  Every edit goes through `SettingDef::validate`, and invalid input is shown inline, never saved.
- Changed values carry a "Changed" mark and a per-setting reset.

**Profile actions.**
- Reset all overrides.
- Duplicate into another instance (when there is one).
- **Export:** copies `ooze.profile/1` JSON to the clipboard.
- **Import:** paste, validate, then show what will change before applying. Warnings list dropped unknown modules or settings.
- The library persists and revalidates. The interface never writes files.

**HUD layout.** The launcher shows anchors and scale as text and offers "Reset HUD layout". Positioning happens in game (M1 HUD editor); the launcher shows no fake preview of the game screen.

## 5. Content (Modrinth)

**Tabs.** Mods, Resource packs, Data packs, Shader packs. Only kinds the library reports for the selected instance are shown.

**Search.**
- Query field, sort (relevance, downloads, updated), category filters from the library, and paging.
- Results are filtered by the backend to the instance's game version and loader. Every card says so: "Compatible with Fabric 1.21.11, as reported by Modrinth".
- Cards show icon (when provided), title, author, summary, downloads, updated date and the project type.
- Text is plain: no markup is interpreted.

**Details.** Summary, licence, links (opened in the system browser after showing the URL host), gallery off by default, and versions. Each version shows its number, release channel (release/beta/alpha), date, game versions, loaders and file size. Incompatible versions are listed but cannot be chosen, with the backend's reason.

**Install flow.**
1. Choose a version, defaulting to the newest compatible release.
2. **Review:** the backend's dependency plan lists required items to install, optional items (not installed; listed with a link to install separately), embedded items (informational), and conflicts (blocking, with the conflicting installed item).
3. **Install:** progress per file. Cancel is available until the commit boundary; the interface then says "Finishing — can't cancel now".
4. **Result:** "Installed and verified (SHA-512)". It is never "Ready to use" or "Working".
   - Shader packs add: "Enable it in the shader loader's settings in game."
   - Resource packs add: "Enable it in Options → Resource Packs."
   - Data packs add: "Takes effect the next time this world is loaded."

**Data packs.**
- The tab first asks for a world, listed from the instance's existing local worlds by the backend.
- With no worlds: "Data packs install into a world. Create a world in this instance first."
- Installing is refused while the game is running.
- There is no server installation.

**Shader packs.**
- The tab explains that a shader loader (for example Iris) is needed and shows the backend's loader check.
- If the loader is missing, the install review shows it as a blocking requirement with an "Install the loader" action, but only when the backend offers it.

**Installed.** A list per kind shows name, version, file and state (Enabled, Disabled, Modified, Missing). Available actions:
- Update, when the backend reports a newer compatible version;
- Disable / Enable;
- Remove recorded files: the confirmation names the files and states that unrelated files are untouched;
- Repair, for a modified registered file.

## 6. Appearance (themes)

**Controls** (token-based, live preview):
- **Accent:** a palette of six Ooze accents plus a custom hex value.
- **Contrast:** Standard / High.
- **Text size:** 90–130 % in 10 % steps.
- **Interface scale:** the existing L0 setting.
- **Background:** Drips / Gradient / Plain.
- **Motion:** System / Full / Reduced / Off (as in L0).

**Live preview.**
- A preview panel built from the real components (a button, a card, a banner, a focus ring, body text) shows the draft.
- **Apply** persists through the library; **Revert** discards the draft; **Reset to Ooze Classic** restores defaults.

**Contrast guard.** An accent is accepted only if all three hold:
- accent text (white or ink, whichever is better) reaches **4.5:1** on the accent;
- the accent reaches **3:1** against the canvas;
- the focus ring stays at **3:1** or more.

Rejected accents say why and keep the last valid value. High contrast replaces soft outlines and secondary text with stronger tokens.

**Import / export.**
- Format: `ooze.theme/1` JSON via the clipboard.
- Data only: base theme ID, accent hex, contrast, text size, background, motion.
- Unknown fields, URLs, paths and out-of-range values are rejected with a reason.
- The library validates again before persisting.

## 7. Integrity

The **Integrity** section in each instance's details shows the mode and the last report.

**Custom.** "Content downloads are hash-checked. Ooze makes no fair-play statement about this instance."

**Managed Fair Play.**
- "This instance is checked against Ooze's release manifest: only approved files and settings. This describes your installation; servers decide what they allow."
- **Report:** each failure is concrete: file, expected vs found, or "unexpected mod", "unapproved pack" or "prohibited launch setting".
- **Actions:**
  - Repair (re-download a managed file);
  - Move to a quarantine folder (the backend moves it; never deletes);
  - Open folder;
  - Switch to Custom (explicit confirmation).

  Nothing is deleted automatically.

**Wording ban.** "Cheat-proof", "undetectable", "anticheat", "server-approved", "safe on all servers". A test scans the language file for these words.

## 8. Errors

Library errors map to the L0 banner pattern (`LAUNCHER_UI.md §5`), with the backend's safe detail. Some have their own copy:

| Error | Copy |
|---|---|
| Busy | "Another library task is running." |
| GameRunning | "Close Minecraft for this instance first." |
| Incompatible | Shows the backend's reason, for example "Needs Minecraft 1.21.10". |
| HashMismatch | "The download didn't match Modrinth's checksum, so it wasn't installed." |
| RateLimited | "Modrinth asked us to slow down. Try again in N seconds." |
| Offline | "Couldn't reach Modrinth." |
| Config | "Library data couldn't be read. Your files were not changed." Shows the backend's recovery path. |

## 9. Tests

- **Headless (egui_kittest, fixture library):**
  - every page hidden without its capability;
  - create offers only reported combinations;
  - a stale search result is ignored;
  - cancel during download;
  - the commit-boundary message;
  - dependency review with a conflict blocks Install;
  - data packs require a world, and are refused while the game runs;
  - the shader loader explanation;
  - "Installed" never reads as "working";
  - preset switch keeps overrides;
  - profile import shows the diff and rejects invalid documents;
  - module setting validation;
  - the server-sensitive notice;
  - theme contrast guard, live preview, revert, reset and import/export round trip;
  - integrity report actions never include delete;
  - the wording-ban scan;
  - keyboard reachability and AccessKit roles on each page.
- **Needs the real library (L1 gate):** Built, Headless-tested on Windows CI, Game-launched on Windows x64 for an installed mod, and Manually verified with Narrator.
