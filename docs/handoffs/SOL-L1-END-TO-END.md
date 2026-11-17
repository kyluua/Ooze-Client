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

Result, checks, artifact and the remaining acceptance gates are recorded below once
the new Windows package is built and verified. Launcher API 1, Library API 1 and
JNI ABI 1 remain the contracts.
