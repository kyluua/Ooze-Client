# Ooze launcher authentication setup (L0)

The independent launcher is a new owner-authorized layer. The older rule that
Ooze never handles credentials still applies to the Fabric adapter/JNI engine,
not to the standalone launcher's private authentication backend. No token enters
the public Rust UI API. Minecraft itself receives its required game session.

## Current status

On 2026-11-15 the owner supplied public application ID
`359e0deb-13e8-4616-978f-0fc1f8e273d2` after creating the Microsoft registration.
It is recorded in `launcher/packaging/public-auth.json`; Windows launcher CI reads
that file before compiling so the CLI embeds the owned ID. This ID is public
configuration, not a secret. On 2026-11-16 the owner corrected the single-tenant
registration and its access-token version. A fresh browser sign-in returned to
Ooze and completed the Microsoft code exchange and protected refresh-token save,
then a later authentication-service request returned HTTP 403. The old CLI's
coarse diagnostic cannot identify which service refused it. Full Minecraft
sign-in, entitlement, profile, refresh and Play remain **unverified**. See
`SOL-L0-AUTH-SERVICE-DIAGNOSTICS.md` for the follow-up. No external application
was created or approved by Sol.

## 1. Register the developer-owned application

Use an identity directory controlled by the Ooze project owner. Microsoft's
current registration guide lists an Azure account with an active subscription,
a directory, and permission to register apps as prerequisites. This desktop
design creates no hosted Azure server; Microsoft account/portal prerequisites
still apply. [Registration guide](https://learn.microsoft.com/en-us/entra/identity-platform/quickstart-register-app).

1. Open the [Microsoft Entra admin center](https://entra.microsoft.com/) and select
   the directory where the project will own its registration.
2. Navigate through **Entra ID → App registrations → New registration**.
3. Name it **Ooze Launcher**. Choose **Personal accounts only**; the backend uses
   the consumers authority for Minecraft players' Microsoft accounts.
4. Create the registration. Copy its **Application (client) ID** from the overview.
   That public GUID is the value Ooze needs, rather than the tenant or object ID.

For an existing single-tenant registration, open **Manifest**, set
`signInAudience` to `PersonalMicrosoftAccount` and set the existing
`api.requestedAccessTokenVersion` to integer `2` in the same save. Personal-account
apps require token version 2; a null value defaults to 1 and prevents that audience
change from saving. Older manifest formats name the latter field
`accessTokenAcceptedVersion`. Preserve the rest of the manifest.
[Account-type settings](https://learn.microsoft.com/en-us/entra/identity-platform/howto-modify-supported-accounts),
[token-version requirement](https://learn.microsoft.com/en-us/graph/api/resources/apiapplication?view=graph-rest-1.0).

The project developer does this once. Players sign into the resulting app;
they do not create their own registration. [Supported account types and client ID](https://learn.microsoft.com/en-us/entra/identity-platform/quickstart-register-app).

## 2. Configure the desktop callback

In the app's authentication settings, add a **Mobile and desktop applications**
platform with this custom redirect URI, then save:

```text
http://localhost/callback
```

Keep the path's spelling/case and omit a trailing slash or wildcard. Ooze opens
an ephemeral port and sends `http://localhost:<port>/callback`; Microsoft ignores
the port when matching localhost redirects, while the path must match. One
registration of the URI above covers those ports. Select the native desktop
platform rather than Web or SPA. [Desktop platform selection and localhost matching](https://learn.microsoft.com/en-us/entra/identity-platform/reply-url#localhost-exceptions).

The listener currently binds IPv4 `127.0.0.1`; the browser URL remains `localhost`.
Do not change just the registered hostname to `127.0.0.1`, use a different callback
path, or open a public firewall port. A change in hostname/binding belongs in a
coordinated code/contract change and browser test. IPv6/browser compatibility is
a pending review item in [Claude's B5 request](../handoffs/CLAUDE-L0-request.md).

Create **no client secret**. Ooze uses authorization code with PKCE S256 in the
system browser, not a confidential server or password flow. Its requested scopes
are `XboxLive.signin` and `offline_access`. The advanced public-client fallback
toggle is separate from adding the native redirect; this guide does not enable
device-code/password flows that Ooze does not implement. Follow Microsoft's
flow-specific guidance if changing that setting. [Public-client guidance](https://learn.microsoft.com/en-us/entra/identity-platform/msal-client-applications),
[PKCE flow](https://learn.microsoft.com/en-us/entra/identity-platform/v2-oauth2-auth-code-flow).

## 3. Confirm Minecraft Services access separately

An Entra client ID is not proof of permission to use Minecraft's services.
The official [Minecraft application-access support entry](https://help.minecraft.net/hc/en-us/articles/16254801392141)
returned only a support shell when inspected on 2026-11-15, so the current
onboarding/approval route could not be verified. No particular form, approval
timeline or approval-free workaround is promised here.

Use official Minecraft/Microsoft support to confirm which application access
process applies to this third-party Java launcher. Have the public client ID,
project repository/contact, desktop/PKCE design and intended Minecraft use ready.
Record their instructions and any granted access against this exact app ID.
Never borrow another launcher's ID or read its credentials.

Treat Microsoft consent, Xbox/XSTS authorization and Minecraft entitlement/profile
requests as separate checks. A successful Microsoft browser login alone does not
prove the game token is usable. Current coarse `ServiceAccess` errors also cover
some Xbox failures; a 403 is not conclusive proof of missing approval. Typed
service/account errors are requested in [Claude's B1 review](../handoffs/CLAUDE-L0-request.md).

## 4. Configure the current CLI for development

For an existing backend package without an embedded ID, extract it, open
PowerShell in its directory and set the owner-supplied public application GUID:

```powershell
$env:OOZE_MICROSOFT_CLIENT_ID = '359e0deb-13e8-4616-978f-0fc1f8e273d2'
./ooze-launcher-cli.exe status
./ooze-launcher-cli.exe sign-in
```

This variable applies to the current shell and child processes. Do not paste a
secret or token into it. `status` does not prove sign-in; `sign-in` must complete
Microsoft, Xbox and Minecraft checks before reporting success. After authorized
sign-in, the remaining CLI flow is:

```powershell
./ooze-launcher-cli.exe install
./ooze-launcher-cli.exe play
# Close Minecraft normally, restart the launcher, then test saved-session refresh:
./ooze-launcher-cli.exe play
./ooze-launcher-cli.exe sign-out
```

If `release.json` is elsewhere, supply `--release <absolute-path>` before the
subcommand. Use `--data-dir <absolute-test-directory>` on every command to keep
test credentials/settings separate from the normal installation. See
[build/package instructions](BUILD_AND_TEST.md) for obtaining/verifying the
package and the observed Windows Application Control execution blocker. These
commands describe the full acceptance path. The live attempt recorded above
has not yet passed the service exchanges.

| If setup fails | Check |
|---|---|
| Ooze reports missing configuration | Set the public ID in the same shell that starts the CLI, or use a build compiled with it |
| Microsoft reports `unauthorized_client` / not enabled for consumers | Check the exact application ID and that the owned app supports personal Microsoft accounts. Microsoft documents changing `signInAudience` to `PersonalMicrosoftAccount` in the app manifest: [account-type settings](https://learn.microsoft.com/en-us/entra/identity-platform/howto-modify-supported-accounts). Do not treat this pre-login error as a Minecraft entitlement/approval result |
| Microsoft reports `AADSTS50011` | Match the native registration's `/callback` path and case; see [redirect mismatch guidance](https://learn.microsoft.com/en-us/entra/identity-platform/reply-url) |
| Authentication asks for a client secret | Recheck the desktop/public platform configuration; do not embed a secret |
| Microsoft login succeeds but services refuse access | Updated diagnostics identify the fixed service name and HTTP status. Xbox user/XSTS failures use Authentication; only Minecraft HTTP 403 uses ServiceAccess. Neither proves a specific approval cause. Confirm account/app access for that service; browser consent alone is insufficient |

## 5. Configure the player-facing release

The current CLI resolves the environment variable first and otherwise uses an ID
embedded at compile time. Windows launcher CI sets this public build variable
from `launcher/packaging/public-auth.json`. For a local developer build, run the
following from the repository root before compiling and packaging:

```powershell
$publicAuth = Get-Content './launcher/packaging/public-auth.json' -Raw | ConvertFrom-Json
$env:OOZE_MICROSOFT_CLIENT_ID = $publicAuth.microsoft_client_id
cargo build --manifest-path launcher/Cargo.toml --locked --release -p ooze-launcher-cli
```

For the desktop binary, Claude's tracked entrypoint/config module now reads the
same public-auth.json as a fallback after runtime/build overrides (2c96d2b).
Complete the outstanding workspace, Windows tests and packaging integration.
Both executables must use the owned configured ID. Final players should receive
a configured build; they should not need a developer environment variable.
The ID is public build configuration, not a GitHub Actions secret/token.

## 6. Record the real acceptance result

Once the app ID and service access exist, use an entitled personal Microsoft
account on Windows x64. Check cancellation and successful sign-in, direct Fabric
1.21.11 launch on actual Java 21, title-screen readiness, normal exit, restart and
refresh without another browser login, then sign-out. Verify redacted logs and
world/options preservation during repair. Record the build/app ID reference,
platform and outcomes in a new handoff, with no credentials attached. The full
joint sequence is [Gate L](../handoffs/CLAUDE-L0-request.md#d-joint-windows-acceptance-gate-l).

Only the public client ID is needed for subsequent development coordination.
Never send passwords, authorization callback URLs, authorization codes, access or
refresh tokens, browser cookies or a credential-store export.

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

No owned client ID was configured at implementation start; the owner has since
supplied the public ID above. OAuth callback/state,
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
