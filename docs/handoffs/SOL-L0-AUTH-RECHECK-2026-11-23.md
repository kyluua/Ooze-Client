# Sol live launcher sign-in result, 2026-11-23

Owner Sol/Codex. Tested on 2026-11-23, America/Los_Angeles.
Base commit `984cafb7492ecdd91077bb03d2c43d87dbb161f4`.
Result commit can be found with `git log -1 -- docs/handoffs/SOL-L0-AUTH-RECHECK-2026-11-23.md`.
This handoff and `docs/launcher/AUTH_SETUP.md` record the result. Launcher API 1,
Library API 1 and JNI ABI 1 remain the applicable contracts.

The Windows x64 CLI from `launcher/dist/ooze-launcher-e82b598/` matched the
verified SHA-256 `c18eedcb5735039d31624160b475c27bbb34b07faf99e20e3260ef4d3eb0ed71`.
Runtime source commit `e82b5989ab41269106442b527202e301926874c6`.

One live `sign-in` attempt used the Ooze-owned public application ID and the
existing isolated owner-auth directory. The CLI started, reached the browser
authorization stage, emitted an account summary, completed the operation and
exited with code 0. The source confirms that the account event follows the
Microsoft, Xbox/XSTS, Minecraft token, Java Edition entitlement and Minecraft
profile checks. This verifies independent launcher sign-in for the tested
account and application. The owner’s account identifiers are omitted here.

The prior Minecraft HTTP 403 and Windows execution block did not recur in this
run. The test did not establish the reason for either earlier result or confirm
the AppID review disposition. No Windows policy setting was changed by Codex.
The desktop UI, direct Play, title readiness, and post-Play session refresh
remain separate acceptance checks. This was a live CLI authentication check
without a new build, game launch or benchmark. Documentation diff checks passed.
There were no new peer commits to review before publishing this result.

Next launcher acceptance task is to install the verified Fabric instance, run
authenticated Play through the title readiness marker `OOZE_READY`, then verify
saved-session reuse and sign-out. Use the existing launcher handoff and record
the actual build and runtime evidence. The temporary Prism modpack remains
the separate distribution route defined by the owner in the MP0 handoff.
