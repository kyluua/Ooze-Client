# SOL L0 — owned public OAuth application configuration

Owner: Sol. Date: 2026-11-15 (America/Los_Angeles).
Base: `15e46dc477e70a807b7544d3c2541fcce08bd160` on main.
Result: this commit; resolve with `git log -1 -- docs/handoffs/SOL-L0-AUTH-CONFIG.md`.
Launcher API 1, JNI ABI 1 and installation schema 1 are unchanged.

The owner supplied public client ID `359e0deb-13e8-4616-978f-0fc1f8e273d2` after
the guided Microsoft app-registration steps. The ID is public configuration and
does not establish approval, consent or a usable Minecraft session.

Changed paths: `launcher/packaging/public-auth.json`,
`.github/workflows/launcher-l0.yml`, `docs/launcher/AUTH_SETUP.md` and this handoff.
The source JSON holds schema 1 and the owned public ID. Launcher CI validates its
GUID and exports OOZE_MICROSOFT_CLIENT_ID before compilation. The existing CLI
already resolves a runtime override first, then the compile-time ID; therefore
the new CI build embeds this ID without needing each player to set it. No bearer
token, client secret or user credential was added. The core API and auth flow
were not changed. Developer commands now use this same config/ID.

Checks before publication: parsed the JSON, validated the exact nonzero GUID,
compared it with the owner's supplied ID, reviewed the existing CLI's precedence,
and `git diff --check`. CI build/package evidence will be recorded after its run.
This configuration check is not evidence of a real Microsoft sign-in.

Next: complete the outstanding UI entrypoint/workspace/packaging integration and
use this same public build ID for the desktop binary (Claude request A5).
The expected portal redirect remains the native `http://localhost/callback`.
Saved portal settings, browser behavior, Minecraft Services access, entitlement,
real refresh and authenticated Play require the live Gate L check. Local Windows
Application Control previously blocked the unsigned launcher executable; no
policy change or workaround was made. Never substitute another app ID or an
offline identity if real authentication is rejected.
