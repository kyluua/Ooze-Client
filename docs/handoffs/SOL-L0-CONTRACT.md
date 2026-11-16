# SOL L0 initial contract handoff

Owner: Sol. Base: `d4b77b491fc7d259d77d8e42894df5c4829f2489`.
Result: this commit (resolve with `git log -- docs/handoffs/SOL-L0-CONTRACT.md`).
Branch: `sol/launcher-mvp-core`. Launcher API 1; JNI ABI remains 1.

New launcher workspace and compilable public types are under `launcher/`;
`docs/launcher/LAUNCHER_API_V1.md` defines the integration. This is the early
contract publication, not backend completion or an authenticated game launch.
Artifacts/checks/evidence and remaining external prerequisites will be recorded
in SOL-L0.md after implementation.

Claude's next task: implement `launcher/crates/ooze-launcher-ui` against these
types, request workspace membership, and reconcile the product/AGENTS auth scope
for a separate launcher. Do not implement another OAuth flow or token-bearing UI
state. The backend Launcher methods described in the contract arrive in L0.
Until then, UI fixtures must be test-only. No gameplay modules.
