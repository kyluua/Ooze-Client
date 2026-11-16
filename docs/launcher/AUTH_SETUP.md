# Ooze launcher authentication setup (L0)

The independent launcher is a new owner-authorized layer. The older rule that
Ooze never handles credentials still applies to the Fabric adapter/JNI engine,
not to the standalone launcher's private authentication backend. No token enters
the public Rust UI API. Minecraft itself receives its required game session.

## Developer prerequisites

Register an application owned by Ooze with Microsoft identity, supporting
personal Microsoft accounts. Configure it as a native/public desktop client with
a system-browser loopback redirect of `http://localhost/callback`; the backend
uses an ephemeral loopback port. Confirm the current redirect/port rules in the
Microsoft documentation for the actual registration. Never create or embed a
client secret in this public app. The client ID is public configuration.

The backend requests `XboxLive.signin` and `offline_access` using authorization
code + PKCE S256, random state and the user's system browser. It listens only on
127.0.0.1 and validates the callback path, Host, state, unique code and bounds.
Cancellation and a three-minute deadline close the callback operation. OAuth
requests use a maintained oauth2 implementation and reject HTTP redirects.

Set `OOZE_MICROSOFT_CLIENT_ID` for development. The CLI also accepts a client ID
embedded at compile time from that environment variable, so final players should
receive an appropriately configured distribution. The UI creates LauncherConfig
with the same public ID. Do not ask every player to register their own app.

Microsoft app registration and Minecraft Services application access are distinct
checks. The Minecraft support application-registration page returned only a
support shell when inspected on 2026-11-15; its current approval route could not
be verified here. Do not claim registration alone grants Minecraft access, or
promise an approval-free route. Confirm access with Microsoft/Minecraft for the
owned client ID. Do not borrow another launcher's client ID or credentials.
This implementation does not require an Azure-hosted server or Ooze login backend.

## Exchanges and storage

After Microsoft OAuth, the backend requests an Xbox Live user token, an XSTS
token for `rp://api.minecraftservices.com/`, then a Minecraft game token through
`/authentication/login_with_xbox`. It checks `/entitlements/mcstore` and
`/minecraft/profile`. Empty or unrelated entitlements fail; no offline/demo
identity is substituted. Some Xbox relying-party claims omit xid; an empty xuid
argument is then passed rather than inventing an identifier. These protocol
choices require a real entitled account test with the owned app before release.

Refresh tokens live in Windows credential storage under service
`io.github.kyluua.ooze.launcher.v1`, keyed by owned app ID and a hash of the
canonical launcher data directory. Different Ooze data roots do not overwrite
each other's credentials. A rotated refresh token is saved before later service
requests so a temporary entitlement outage does not discard rotation. Settings
contain only the remembered Minecraft username/UUID. Sign-out removes the vault
entry and remembered summary; it does not revoke the browser's Microsoft cookies.
Switch account through sign-out/sign-in; the browser requests account selection.

Bearer-bearing structs are private, have no Debug/Serialize implementation and
never cross the JNI bridge. Game output is bounded and redacted before events or
launcher log files; raw HTTP errors/provider bodies and launch argv are never
exported. Java's non-secret classpath argument file contains no credentials. The
game session is necessarily visible to the launched game and potentially to
software running as the same Windows user; L0 does not claim OS-level isolation
from that user or erase every temporary library allocation. No plaintext vault
fallback exists. The launcher owns its redacted logs, not arbitrary third-party
mod log files.

## Actual validation boundary

No owned client ID was configured at implementation start. OAuth callback/state,
denial, cancellation, refresh-error handling, entitlement parsing and redaction
have controlled tests; these do not prove Microsoft consent, Minecraft service
access, a real token refresh or an online launcher game session. SOL-L0 records
the actual executed checks. ServiceAccess diagnoses an access denial without
exposing the response body; it does not assert every 403 is an approval problem.

Primary sources inspected:

- [Microsoft app registration](https://learn.microsoft.com/en-us/entra/identity-platform/quickstart-register-app)
- [Authorization code and PKCE](https://learn.microsoft.com/en-us/entra/identity-platform/v2-oauth2-auth-code-flow)
- [Public clients and secrets](https://learn.microsoft.com/en-us/entra/identity-platform/msal-client-applications)
- [Xbox website sign-in and XSTS](https://learn.microsoft.com/en-us/gaming/gdk/docs/services/fundamentals/s2s-auth-calls/service-authentication/live-website-authentication)
- [Minecraft application access support entry](https://help.minecraft.net/hc/en-us/articles/16254801392141) (content unavailable; not approval evidence)
- [oauth2 crate](https://docs.rs/oauth2/5.0.0/oauth2/) (PKCE and redirect guidance)
