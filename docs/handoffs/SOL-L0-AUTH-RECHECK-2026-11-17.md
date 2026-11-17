# Sol — live sign-in recheck, 2026-11-17

Owner: Sol/Codex. Client date: 2026-11-17, America/Los_Angeles.
Base: `5368e022305daac3cff18b4f9acd71214ca071ef`.
Runtime source: `e82b5989ab41269106442b527202e301926874c6`.
Result commit: discover with `git log -1 -- docs/handoffs/SOL-L0-AUTH-RECHECK-2026-11-17.md`.
Launcher API 1, Library API 1 and JNI ABI 1 unchanged. No implementation changed.

The owner explicitly requested a fresh check. The packaged CLI SHA-256 matched
`c18eedcb5735039d31624160b475c27bbb34b07faf99e20e3260ef4d3eb0ed71` before execution.
Unlike the previous local attempt, this executable started and --version returned
`ooze-launcher-cli 0.1.0-l0`. No Windows policy or trust setting was changed by Sol.
This observation does not establish that other executables or the GUI now run.

One real `sign-in` used the owned public client ID
`359e0deb-13e8-4616-978f-0fc1f8e273d2` and existing isolated owner-auth data directory.
The owner confirmed that the browser opened. The backend completed the validated
Microsoft callback/code exchange and Xbox user/XSTS chain, then reached Minecraft:

```text
ServiceAccess: Minecraft sign-in failed (HTTP 403). Check the account and owned application's access to this service.
```

Exit 1. This is not a successful Minecraft sign-in. Entitlement and profile
checks were not reached. The failing service is identified by the real backend;
the 403 alone does not establish the approval status or exact refusal cause.
No approval confirmation was obtained and no duplicate review form was submitted.

A subsequent non-authenticating status check returned account None, no active
operation and no game PID. No installation or game launch was requested. Tokens
remained in the backend/Windows-protected store; no token, password, callback URL
or provider response body was read into output or copied to the repository.

Changed path: this handoff only. Verification: real packaged --version,
owner-completed browser flow, controlled Minecraft HTTP 403, exit code and status.
The existing Windows CI evidence for e82b598 is unchanged; no code tests were rerun
for this observational documentation update. Original staged user deletions remain
untouched. No benchmark, GUI verification or Game-launched acceptance is claimed.

Next task: after an application-access update or official support guidance,
perform one authorized recheck. A full pass must include Minecraft token,
entitlement and profile validation before advancing to Gate L's real Play checks.
