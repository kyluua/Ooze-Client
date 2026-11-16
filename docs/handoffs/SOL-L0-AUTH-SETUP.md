# SOL L0 authentication setup supplement

Owner: Sol. Date: 2026-11-15, America/Los_Angeles.
Base: `f6b89e6` on main. Result: this documentation commit (resolve with
`git log -1 -- docs/handoffs/SOL-L0-AUTH-SETUP.md`). Launcher API 1 and JNI ABI 1
unchanged. No executable or authentication behavior changed.
Changed paths: `docs/launcher/AUTH_SETUP.md` and this handoff.

The owner confirmed there is no owned OAuth client ID yet and requested setup
documentation. AUTH_SETUP now contains registration/navigation, personal-account
selection, the exact native localhost callback and dynamic-port behavior,
public-client/PKCE requirements, separate Minecraft Services access verification,
CLI environment/build configuration, and the real-account acceptance sequence.

Checks: compared the documented constants/config precedence against the actual
core and CLI; reviewed current primary Microsoft registration/redirect/public
client docs and Minecraft support; `git diff --check`. No new app was registered,
no client ID was invented, and no live sign-in or new game/CI execution is claimed.
The current Minecraft approval route remains unverified because the official
support article did not expose its body. This is a documentation-only supplement;
the tested implementation/package checksums remain in SOL-L0.md.

Claude already combined the two agents' work on main through `f6b89e6`; this update
is based on that integrated main rather than the historical Sol branch. The
older SOL-L0.md is historical evidence; CLAUDE-L0.md records the subsequent UI and
fixture fix. A clean checkout at origin/main was used for this edit; unrelated
staged changes in another checkout were left untouched.

Integration observation for Claude before A2/A3/A5: the current committed UI
Cargo.toml declares `src/bin/ooze-launcher.rs` and
`src/bin/ooze-launcher-preview.rs`, but neither file is tracked in main or present
in the fresh checkout. The repository's broad `bin/` ignore is a likely cause.
Please commit the actual entrypoints with the appropriate ignore exception or
move them to tracked source paths, then verify their public-client-ID resolution.
Sol did not reconstruct or edit Claude-owned entrypoints. The guide's executable
examples therefore use the implemented CLI; desktop packaging remains pending.

Next: owner registers/configures the owned public app and confirms service
access; Claude/Sol finish the entrypoint/workspace/packaging requests and conduct
the joint entitled-account Gate L. No further owner confirmation is needed merely
to continue independent implementation. No gameplay modules were added.
