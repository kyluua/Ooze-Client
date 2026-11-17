# SOL L1 — independent review of Claude P0

Owner: Sol. Date: 2026-11-16 (America/Los_Angeles). Reviewed main 6d72250 and
the adapter sources merged into 210b900. No peer-owned UI behavior was edited.
This focused review is not a declaration that every interface path was audited.

## Findings and exact next changes

1. **Selection and launch settings can diverge.**
   `app/library_pages.rs::select_instance` submits library Select and API 1
   UpdateSettings independently, without awaiting successful selection first.
   It skips UpdateSettings while a game is live, while the Select button is
   guarded only by the library model's busy flag. A rejected Select or a selection
   made during a live game can leave the highlighted instance different from the
   API 1 game directory. Chain successful Select → successful SaveSettings and
   gate Play throughout. Reconcile the selected root again before every Play or
   reject selection during a live API 1 session. Add tests for failed Select,
   settings failure, and select-B-while-A-runs followed by A exit and Play.

2. **Project → Versions follow-up race, fixed in backend.**
   `core_library.rs::on_finished` immediately submits the next operation. Sol now
   clears active state before emitting Finished; the UI can chain immediately.

3. **Consume the newly fulfilled contract requests.**
   G1: `Snapshot.appearance.document` and SaveAppearance/ImportAppearance/
   ExportAppearance use the global canonical ooze.theme/1. G2: ListWorlds returns
   opaque local-world keys accepted by Plan. G3: ProgressStage/Progress reports
   Contacting, Downloading, Verifying, Committing; Cancel is closed before the
   reliable Committing event. G8: set_notify exists; its closure schedules repaint.
   The current adapter hides Appearance/data packs and ignores progress, so these
   backend capabilities are not yet claimed integrated into the release UI.

4. **Keep data-pack management scoped when enabling G2.**
   The current UI uses project ID as record_id; backend RemoveContent/SetEnabled
   intentionally affect that project's registered receipts across the instance.
   Multiple worlds can contain the same data-pack project. Do not present that
   as a one-world operation. Request a record/world-scoped backend command before
   exposing per-world removal/disable, or explain the all-records scope explicitly.

5. **Availability remains honest.**
   Hiding module switches while runtime_modules is empty is correct. The module,
   HUD, statistics and profile models are not game-facing hooks. Keep those
   features gated until G1/G4/M0/M1 bridge/runtime work and actual game validation.

## Positive checks and evidence limits

The adapter uses the real library facade, keeps credentials out of its types,
separates install from enabled state, and leaves missing capabilities hidden.
Global appearance validation is shared with the product model; there is no copied
preset table. Library workers own network/filesystem operations, and instance
locks/PID guards reject edits to active games even while the UI's running field
is still conservative/incomplete.

Windows CI is compiling/testing the combined tree. CI UI tests and previews do
not establish Microsoft/Minecraft authorization, Narrator behavior on the owner's
machine, or an actual launched game. G4 richer catalog data, G5 conflict review,
G6 typed remedies/mode promotion and G7 install/running capability snapshots
remain explicitly requested follow-ups. Other versions/loaders are L3, not
silently advertised by enum variants in L1.
