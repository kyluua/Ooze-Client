# Sol — launcher integration continuation

Owner: Sol/Codex. Base: `22d0e2067b578bffa95d9f71a51960e15d2d156b`.

The owner explicitly reassigned the remaining launcher interface to Codex in chat:
“Yes—finish the launcher end to end.” This authorizes this milestone's changes to
`launcher/crates/ooze-launcher-ui/**` and launcher resources, superseding that path's
normal Claude ownership in AGENTS.md for this work. In-game Fabric/ooze-ui ownership
stays with Claude. All integration is published to main without rewriting history.

The milestone closes the launcher interface mappings and selected-instance Play
sequencing. It does not implement the separate M-series in-game module runtime.
Authentication approval and actual title-screen readiness must still be verified;
no offline-auth fallback or borrowed application ID is introduced.

Status: **Built** and **Headless-tested** on Windows x64. The full L1/L2 acceptance
gates remain open. Launcher API 1, Library API 1 and JNI ABI 1 remain the contracts.
The source result is e82b598; this documentation result is discoverable with
`git log -1 -- docs/handoffs/SOL-L1-END-TO-END.md`.

The owner confirmed in this continuation that no Microsoft/Minecraft application
approval has been received. No new live authentication retry is authorized by
that status answer, and none is made. Microsoft sign-in UI, credentials and game
launch remain real backend operations; acceptance of the full authenticated flow
is pending the external review and an owner-completed entitled-account test.

## Implemented changes

Source candidate: `e82b5989ab41269106442b527202e301926874c6` on main.
All source commits were fetched/merged before publication with checked leases and
no force push. The original main checkout's staged LICENSE/README deletions remain
untouched. The existing Sol checkout was used; no account data was imported.

- The real desktop adapter now exposes global Appearance, existing local-world
  data packs, all four progress stages and the library repaint notifier (P0
  requests G1/G2/G3/G8). Global `ooze.theme/1` owns motion and scale too; legacy
  UI preferences no longer override it. Standard toolkit controls receive the
  updated tokens and animation duration along with custom gel widgets.
  Valid legacy motion/scale preferences migrate once before the window opens,
  preserving their original bytes and existing Off/Reduced choices. An existing
  global theme is never replaced by the migration. “Follow Windows” appears only
  when API 1 supplies the system preference; an unsupported System choice has the
  explicit effective Full fallback rather than a non-working control.
- Selection is awaited before API 1 settings change. Play/install require the
  selected directory to match completed settings. Failed saves can be retried;
  selection is blocked while API 1 is busy or the selected game runs. Empty or
  archived selections cannot launch the previous directory. Startup reconciles
  the remembered selection once. API 1 and library-reported running state are
  combined for display without overwriting the library snapshot.
- Closing during a library operation waits for cancellation or commit completion;
  a cancellation/commit race does not prematurely close the process. Completion
  rechecks API 1's game/operation guard, so a simultaneous search cannot bypass
  the live-game close warning.
- A new exact-receipt content command prevents a data pack's disable/remove from
  affecting a copy in another world. Data-pack dependency checks share that scope.
  The project-wide CLI commands preserve their older, explicit all-records scope.
- Search has real Relevance/Downloads/Updated ordering and bounded category facets.
  Author, download count, modified date, channel, published date and primary file
  size are retained. Categories are discovered from real page hits (not advertised
  as a complete global taxonomy). Mod version queries retain game/loader filtering.
- Installed-content lists now hash-check on a worker. Same-size modified files
  and missing files have typed states. Successful file actions refresh the list.
  Integrity offers exact-record pinned-hash repair in Custom mode. Managed
  conversion verifies the pinned baseline and refuses extra content without
  changing/deleting files. Quarantine is not offered by this backend.
- The instance label says “Ooze Client included”, not “modules available” while
  the game runtime module capability is empty. Module/profile switches remain
  hidden under Real or absent. The Play card describes the actual selected
  Vanilla/Fabric target rather than the release's fixed Fabric heading.
- The portable package includes player instructions for the desktop, instances,
  content and global appearance. Players use the configured public application
  ID; they do not set up Azure registrations. No proprietary game files are bundled.

Changed paths: launcher core `src/library/{model,modrinth,content,mod}.rs`;
launcher UI `src/{app,library,core_library,library_ui_tests,state}.rs` and
`src/app/library_pages.rs`; launcher `resources/lang/en_us.json`;
`packaging/prepare-release.ps1`; launcher CI; AGENTS.md; Library API contract;
this handoff. Cargo dependencies/toolchain, game Java, native bridge and JNI bytes
are unchanged. Source API additions are explicit in LIBRARY_API_V1.md.

## Review and acceptance boundaries

SOL-L1-CLAUDE-P0-REVIEW findings 1/3/4 are addressed by this continuation. The
project→versions race in finding 2 was already fixed in the backend. Finding 5's
module capability gate remains deliberate. P0's G4 catalog request is implemented;
G6 now has checked content, exact-record repair and safe Managed promotion, while
quarantine and richer structured policy findings are still follow-ups. G7 now
has the API 1 running overlay; an install-state capability snapshot remains absent.
G5 conflicts still fail closed with a safe error instead of a reviewable conflict
plan; no UI control pretends otherwise. Installed updates use selecting/planning
another compatible project version; no automatic update discovery is claimed.
Complex mixed-kind/world dependency plans remain an L2/G5 follow-up; conservative
resolver refusals preserve existing content rather than adopting unknown files.

The backend and GUI are not a declaration that L1/L2 acceptance passed. Pending
external gates: entitled-account sign-in after application approval, actual Play
to OOZE_READY on Windows, per-content-type in-game validation, and a Narrator pass.
Only Vanilla/Fabric 1.21.11 on Windows x64 is offered. Quilt/Forge/NeoForge and other
versions remain L3. In-game modules and HUD hooks remain G1/G4/M0–M3 with Claude.
No FPS improvement, GUI frame-time budget, allocator-memory budget or JNI/copy
performance result is claimed by this integration. Prior CLI measurements remain
in SOL-L1.md and are not extrapolated to the GUI.

Local fmt/diff checks ran. A fresh local compile was attempted and blocked by
Windows Application Control 4551 on a Rust dependency build script; no policy was
changed or bypassed. CI supplies actual Windows compilation and test execution.
No downloaded GUI was opened locally: the earlier automatic approval denial
(“blocked by policy”) remains unresolved. CI renderer screenshots establish
appearance, not a manually verified Windows desktop/Narrator session.

## Actual verification and delivery

[Windows run 44026232354](https://github.com/kyluua/Ooze-Client/actions/runs/44026232354)
passed both jobs (133847130307, 133848674232) on the exact e82b598 source:

- fmt/clippy and **49 backend tests**;
- **95 UI tests passed**, two live-UI network tests intentionally ignored;
- real-library UI theme save and local-world listing, legacy preference migration,
  system-motion capability gating, failed selection/settings save, live-game
  selection, empty/archive selection and cancellation/commit/close regressions;
- 4 native core + 1 JNI + 24 product-model Rust tests; actual Java
  **21.0.13+9-LTS / amd64**, G0 **20,323 assertions** in both the script and Gradle
  harness; Fabric build/tests passed;
- real CLI Modrinth searches for all four kinds (10 hits each), pinned Iris with
  required Sodium, disable/enable, modified-file detection and explicit repair,
  canonical profile/appearance and local-world results;
- matching pinned package repeat and actual managed-Java installation/repair for
  Fabric and Vanilla 1.21.11 with developer tools/JDK removed from PATH, baseline
  verification and options preservation. No game or authentication was launched.

The review ZIP is GitHub artifact **14303633163**, `ooze-launcher-windows-x64`,
**13,978,979 bytes**. It includes both executables, the real native-bearing mod,
pinned metadata, 332 dependency-notice entries and unmodified fonts/OFL files.
After downloading it, the archive digest and all **598 files in SHA256SUMS** were
verified locally, and unlisted extra package files were checked. Nothing was
executed before verification. Local delivery (ignored build output):
`launcher/dist/ooze-launcher-e82b598.zip` and `launcher/dist/ooze-launcher-e82b598/`.

| Artifact | SHA-256 |
|---|---|
| ZIP | `731361d9759f31c43d4c379e70ca73d43717f8dcf5293281142ce5eec3e5a543` |
| GUI | `d7db14db5c7bc34a39976e46d97492adce9b02159cbb92e0903f6f17c090a5ba` |
| CLI | `c18eedcb5735039d31624160b475c27bbb34b07faf99e20e3260ef4d3eb0ed71` |
| Ooze JAR | `30a48105d552fa9e1e408615ce999e37fb7f3d14958743cae813da5a830ad6d2` |

UI evidence is artifact **14303488225**, 28 renderer PNGs, ZIP SHA-256
`0bd861edae3eff6ae912a34b5d2836bad0a8953c8d0febdb6dde79da8f154e01`.
The archive was verified and the final real-library Appearance/world and instance
creation images were inspected. These are headless renderer captures. The
real-library UI regression uses a test API 1 provider for platform/account state;
its theme persistence and world listing use the real library/filesystem. No
desktop window, live Microsoft sign-in or Narrator pass is implied.

A local packaged CLI --version and isolated library smoke attempt was made after
verification. **Windows Application Control blocked the CLI before execution**;
the subsequent script invocation also failed before creating the dedicated test
directory. There is no local runtime-pass claim for this candidate. CI supplied
the real executable checks above. No GUI launch or policy bypass was attempted.
The earlier automatic approval rejection of opening the GUI (“blocked by policy”)
also remains unresolved. Local use needs a legitimate administrator-approved
execution-trust/signing path, separate from Microsoft application approval.

This remains an unsigned review package. Existing workspace version branding is
0.1.0-l0; the source commit and hashes identify the candidate. Build/package and
content instructions are in docs/launcher/BUILD_AND_TEST.md and the ZIP's README.
Machine-readable evidence is `evidence/SOL-L1/verification-e82b598.json`.

## Exact next tasks

Sol/owner: once execution trust and Microsoft app access are resolved, run the
entitled-account Gate L and Narrator checks. Record actual title-screen readiness
before a player release; do not infer it from installation or process spawning.
Claude: independently review e82b598 against the base and continue game frame/
configuration contracts and M0+ runtime in owned game paths. Do not enable launcher
module controls until those hooks are real and verified. No concurrent edits to
the launcher interface are needed during this owner-authorized continuation.
