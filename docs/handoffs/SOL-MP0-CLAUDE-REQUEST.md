# Sol to Claude: temporary modpack distribution, MP0

Owner: Sol/Codex. Owner decision: 2026-11-17, America/Los_Angeles.
Base: `fd665ae9c3b1f45418d77291b63023b65b2315be`.
Result commit: `git log -1 -- docs/handoffs/SOL-MP0-CLAUDE-REQUEST.md`.
Changed paths: this handoff and the owner decision in `AGENTS.md`.
JNI ABI 1, Launcher API 1 and Library API 1 are unchanged.

## Decision and current state

The owner accepted a modpack **for now**. Deliver a Prism-importable `.mrpack`
so an existing launcher can authenticate the player and launch Ooze as a Fabric
mod. Preserve the independent Ooze launcher and its tests. This temporary route
does not satisfy its sign-in or Gate L acceptance.

The latest real independent sign-in completed Microsoft OAuth and Xbox/XSTS,
then Minecraft sign-in returned HTTP 403. Entitlements and profile were not
reached. The exact refusal cause and app approval status remain unconfirmed.
See `SOL-L0-AUTH-RECHECK-2026-11-17.md`; do not describe login as fixed.

Prism performs its own sign-in and launch. Never copy its client ID, extract
credentials or add an offline entitlement bypass. The Fabric mod receives only
non-secret display metadata through the existing native contract.

## MP0 scope and ownership

The first pack is a working baseline: Minecraft 1.21.11, Java 21, Fabric,
Fabric API and the production Ooze mod including its Windows x64 native artifact.
Use the current pinned versions from the Fabric producer; the reviewed pins are
Fabric Loader 0.20.1 and Fabric API 0.141.6+1.21.11. Record the actual inputs,
sizes and hashes when producing the pack. Additional platforms require their own
native builds and validation before support is declared.

- **Sol/Codex:** `.mrpack` export, packaging/manifests, dependency and license
  checks, artifact checksums, CI and native/bridge fixes within Sol ownership.
- **Claude:** Fabric production JAR/producer, Java integration, in-game Rust UI,
  product documentation, installation instructions and actual game validation.
  Review the resulting pack and Sol's changes independently.

Reuse `versions/mc1_21_11/build.gradle.kts`'s `remapJar` and `launcherProfile`
producer. Its `build/dist/ooze-launcher-profile.json` uses
`oozelauncher.profile/1` and records pinned dependencies and artifact hashes.
Coordinate any producer changes in a handoff before changing Sol's consumer.
Do not edit Sol's packaging/export files concurrently.

Start with required dependencies only. Optional performance mods need exact
game/loader compatibility, provenance and distribution permission checks before
inclusion. Do not bundle proprietary Minecraft files, another launcher's runtime
or account data. Keep test fixtures and sample content out of production.

FPS/PvP/Minimal profile models are present; real in-game modules are not delivered.
Keep unfinished controls absent. Module implementation remains the staged M-series
work with the required frame/config contracts and Java game hooks. No FPS gain or
cheat-proof integrity guarantee is established by producing this pack.

## Exact next task for Claude

1. Fetch/merge `origin/main`, preserve user changes, read `AGENTS.md`, this handoff,
   `SOL-L1-END-TO-END.md`, the latest auth recheck and the relevant product specs.
   Continue on `main`; do not reset another worktree or discard staged changes.
2. Verify the real production JAR and profile producer for the MP0 inputs. Confirm
   native resources are included and the developer fixture is not the release UI.
   Send any required bridge/packaging changes to Sol through a committed request.
3. Reconcile the installation/product specs with this temporary distribution route
   while retaining the independent-launcher requirements. Document Prism import,
   its own account sign-in, Java 21 and this pack's Windows x64 native limitation.
4. Once Sol provides the actual pack artifact/checksum, import it in Prism and
   validate an entitled-account game launch on Windows x64. Record the exact pack,
   JAR/native hashes, Java version, game/loader versions and observed
   `OOZE_READY v1 screen=title engine=LOADED`. Verify the actual title/options
   behavior and the native-load failure fallback. Installation or a spawned Java
   process alone is not game readiness. If interactive account access is needed,
   report the precise remaining check instead of inventing evidence.
5. Publish `docs/handoffs/CLAUDE-MP0.md` with base/result commits, changed paths,
   producer inputs, contract versions, actual checks, artifact/checksums,
   limitations and the exact next task. Distinguish Built, Headless-tested,
   Game-launched and Manually verified evidence.

Sol's corresponding next task is to implement the exporter using the real
producer inputs, validate the pack format and contents, and publish an importable
artifact with checksums and reproducible instructions for Claude's review.

## Existing evidence and current limits

The last verified launcher/runtime source is
`e82b5989ab41269106442b527202e301926874c6`; its evidence is recorded in
`SOL-L1-END-TO-END.md` and `evidence/SOL-L1/verification-e82b598.json`.
That Windows package's production `ooze-client.jar` SHA-256 is
`30a48105d552fa9e1e408615ce999e37fb7f3d14958743cae813da5a830ad6d2`.
Recheck the actual packaging input; do not treat this historical hash as a newly
built artifact or claim the previous game checks validate a new pack.

This handoff changes documentation only. No `.mrpack` artifact, Prism import,
new game launch, new benchmark or module implementation is delivered here.
Verification for this change: documentation review and `git diff --check`.
Original staged user deletions in the other worktree remain untouched.
