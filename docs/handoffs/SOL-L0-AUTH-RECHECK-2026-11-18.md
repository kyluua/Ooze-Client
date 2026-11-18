# Sol: live sign-in recheck, 2026-11-18

Owner: Sol/Codex. Client date: 2026-11-18, America/Los_Angeles.
Base: `539cc2c6855f9f98ea765718645f9997ed676c93`.
Runtime source: `e82b5989ab41269106442b527202e301926874c6`.
Result commit: `git log -1 -- docs/handoffs/SOL-L0-AUTH-RECHECK-2026-11-18.md`.
Changed path: this handoff only. JNI ABI 1, Launcher API 1 and Library API 1
are unchanged. No implementation or authentication configuration changed.

The owner asked whether sign-in works now. Fetching `origin/main` found no newer
commits. Before execution, the packaged Windows x64 CLI matched the previously
verified SHA-256
`c18eedcb5735039d31624160b475c27bbb34b07faf99e20e3260ef4d3eb0ed71`.

One real `sign-in` used the owned public client ID
`359e0deb-13e8-4616-978f-0fc1f8e273d2` and the existing isolated owner-auth data
directory. The operation reached Authorizing and AwaitingBrowser, completed the
Microsoft callback/code exchange and Xbox user/XSTS chain, then returned:

```text
ServiceAccess: Minecraft sign-in failed (HTTP 403). Check the account and owned application's access to this service.
```

Exit code: 1. No additional owner input was needed during this attempt. No new
manual browser verification is claimed. A subsequent non-authenticating status
check showed account None, no active operation and no game PID. Full Minecraft
sign-in remains unsuccessful; entitlement/profile checks were not reached.
The HTTP status alone does not establish app approval status or the exact cause.

Tokens stayed within the backend and Windows-protected storage. No password,
token, callback URL or provider response body was read into output or committed.
No game installation/launch, policy change, duplicate review submission or
additional retry was performed. No new benchmark, GUI, Game-launched or
Manually verified acceptance is claimed. Code tests were not rerun for this
observational documentation update; `git diff --check` passed.
Original staged user deletions in the other worktree remain untouched.

Next task: continue the temporary Prism modpack route described in
`SOL-MP0-CLAUDE-REQUEST.md`. Preserve the independent launcher; after an
application/account-access update, an authorized recheck must complete Minecraft
token, entitlement and profile validation before claiming sign-in works.
