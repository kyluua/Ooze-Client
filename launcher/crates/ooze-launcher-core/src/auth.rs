//! Credentials are private to this module and the process-launch boundary.
use crate::{
    api::{AccountSummary, ErrorCode},
    net::{self, Http},
    util::{Result, error},
};
use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, CsrfToken, PkceCodeChallenge, RedirectUrl, RefreshToken,
    Scope, TokenResponse, TokenUrl, basic::BasicClient,
};
use serde_json::{Value, json};
use std::{
    io::{BufRead, BufReader, Write},
    net::{TcpListener, TcpStream},
    time::{Duration, Instant},
};
use zeroize::Zeroizing;

pub(crate) struct Session {
    pub token: Zeroizing<String>,
    pub account: AccountSummary,
    pub xuid: String,
}
fn auth_error() -> crate::api::LauncherError {
    error(
        ErrorCode::Authentication,
        "Microsoft/Xbox sign-in failed. Sign in again and check your account's Xbox setup.",
    )
}
fn access_error() -> crate::api::LauncherError {
    error(
        ErrorCode::ServiceAccess,
        "Minecraft Services denied this application. Verify the owned app registration and Minecraft access authorization.",
    )
}
fn credential(id: &str) -> Result<keyring::Entry> {
    keyring::Entry::new("io.github.kyluua.ooze.launcher.v1", id).map_err(|_| {
        error(
            ErrorCode::CredentialStore,
            "Couldn't open Windows credential storage.",
        )
    })
}
pub(crate) fn forget(id: &str) -> Result<()> {
    match credential(id)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(_) => Err(error(
            ErrorCode::CredentialStore,
            "Couldn't remove the stored credential. Retry sign-out.",
        )),
    }
}
pub(crate) fn authenticate(
    id: &str,
    interactive: bool,
    http: &Http,
    check: &impl Fn() -> Result<()>,
    browser_wait: &impl Fn(),
) -> Result<Session> {
    if id.len() != 36 || !id.chars().all(|c| c.is_ascii_hexdigit() || c == '-') {
        return Err(error(
            ErrorCode::Configuration,
            "Configure Ooze's owned public Microsoft client ID before signing in.",
        ));
    }
    let client = BasicClient::new(ClientId::new(id.to_owned()))
        .set_auth_uri(
            AuthUrl::new(
                "https://login.microsoftonline.com/consumers/oauth2/v2.0/authorize".into(),
            )
            .map_err(|_| auth_error())?,
        )
        .set_token_uri(
            TokenUrl::new("https://login.microsoftonline.com/consumers/oauth2/v2.0/token".into())
                .map_err(|_| auth_error())?,
        );
    check()?;
    let token = if interactive {
        let listener = TcpListener::bind(("127.0.0.1", 0)).map_err(|_| {
            error(
                ErrorCode::Authentication,
                "Couldn't open a local sign-in callback. Check your firewall.",
            )
        })?;
        listener.set_nonblocking(true).map_err(|_| auth_error())?;
        let port = listener.local_addr().map_err(|_| auth_error())?.port();
        let redirect = format!("http://localhost:{port}/callback");
        let client = client.set_redirect_uri(RedirectUrl::new(redirect).map_err(|_| auth_error())?);
        let (challenge, verifier) = PkceCodeChallenge::new_random_sha256();
        let (url, state) = client
            .authorize_url(CsrfToken::new_random)
            .add_scope(Scope::new("XboxLive.signin".into()))
            .add_scope(Scope::new("offline_access".into()))
            .add_extra_param("prompt", "select_account")
            .set_pkce_challenge(challenge)
            .url();
        webbrowser::open(url.as_str()).map_err(|_| {
            error(
                ErrorCode::Authentication,
                "Couldn't open your default browser. Configure one and retry sign-in.",
            )
        })?;
        browser_wait();
        let code = wait_callback(&listener, port, &state, check)?;
        check()?;
        client
            .exchange_code(AuthorizationCode::new(code))
            .set_pkce_verifier(verifier)
            .request(&http.client)
            .map_err(|_| auth_error())?
    } else {
        let refresh = Zeroizing::new(credential(id)?.get_password().map_err(|_| {
            error(
                ErrorCode::Authentication,
                "No usable saved session. Sign in through Microsoft.",
            )
        })?);
        client
            .exchange_refresh_token(&RefreshToken::new(refresh.to_string()))
            .request(&http.client)
            .map_err(|_| {
                error(
                    ErrorCode::Authentication,
                    "The saved session could not be refreshed. Sign in again.",
                )
            })?
    };
    check()?;
    // Persist rotation immediately, even if a later entitlement request is temporarily unavailable.
    if let Some(refresh) = token.refresh_token() {
        credential(id)?
            .set_password(refresh.secret())
            .map_err(|_| {
                error(
                    ErrorCode::CredentialStore,
                    "Couldn't save the refreshed session in Windows credential storage.",
                )
            })?;
    }
    exchange(http, token.access_token().secret(), check)
}
fn wait_callback(
    listener: &TcpListener,
    port: u16,
    state: &CsrfToken,
    check: &impl Fn() -> Result<()>,
) -> Result<String> {
    let deadline = Instant::now() + Duration::from_secs(180);
    while Instant::now() < deadline {
        check()?;
        match listener.accept() {
            Ok((mut stream, remote)) => {
                if !remote.ip().is_loopback() {
                    continue;
                }
                let result = callback(&mut stream, port, state);
                let body = if result.is_ok() {
                    "Sign-in received. You can return to Ooze."
                } else {
                    "Ooze could not validate this sign-in. Return to Ooze and retry."
                };
                let reply = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/plain; charset=utf-8\r\nCache-Control: no-store\r\nContent-Security-Policy: default-src 'none'\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{body}",
                    body.len()
                );
                let _ = stream.write_all(reply.as_bytes());
                return result;
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(40))
            }
            Err(_) => return Err(auth_error()),
        }
    }
    Err(error(
        ErrorCode::Authentication,
        "Sign-in timed out after three minutes. Retry when ready.",
    ))
}
fn callback(stream: &mut TcpStream, port: u16, state: &CsrfToken) -> Result<String> {
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .map_err(|_| auth_error())?;
    let mut reader = BufReader::new(stream);
    let mut bytes = Vec::new();
    loop {
        let before = bytes.len();
        let n = reader
            .read_until(b'\n', &mut bytes)
            .map_err(|_| auth_error())?;
        if n == 0 || bytes.len() > 8192 {
            return Err(auth_error());
        }
        if &bytes[before..] == b"\r\n" {
            break;
        }
    }
    parse_callback(
        &String::from_utf8(bytes).map_err(|_| auth_error())?,
        port,
        state,
    )
}
fn parse_callback(request: &str, port: u16, state: &CsrfToken) -> Result<String> {
    let mut lines = request.split("\r\n");
    let words = lines
        .next()
        .unwrap_or("")
        .split_whitespace()
        .collect::<Vec<_>>();
    if words.len() != 3
        || words[0] != "GET"
        || !words[1].starts_with("/callback?")
        || words[2] != "HTTP/1.1"
    {
        return Err(auth_error());
    }
    let host = format!("localhost:{port}");
    if !lines.any(|line| {
        line.split_once(':')
            .is_some_and(|(k, v)| k.eq_ignore_ascii_case("host") && v.trim() == host)
    }) {
        return Err(auth_error());
    }
    let url =
        url::Url::parse(&format!("http://localhost{}", words[1])).map_err(|_| auth_error())?;
    let params = url.query_pairs().collect::<Vec<_>>();
    let value = |key: &str| {
        let values = params.iter().filter(|(k, _)| k == key).collect::<Vec<_>>();
        if values.len() == 1 {
            Some(values[0].1.to_string())
        } else {
            None
        }
    };
    let returned = CsrfToken::new(value("state").ok_or_else(auth_error)?);
    if returned != *state || params.iter().any(|(k, _)| k == "error") {
        return Err(auth_error());
    }
    value("code")
        .filter(|v| !v.is_empty() && v.len() < 4096)
        .ok_or_else(auth_error)
}
fn post(http: &Http, url: &str, body: &Value) -> Result<Value> {
    let response = http
        .client
        .post(url)
        .json(body)
        .send()
        .map_err(|_| net::network_error())?;
    if response.status() == reqwest::StatusCode::FORBIDDEN {
        return Err(access_error());
    }
    if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS
        || response.status().is_server_error()
    {
        return Err(net::network_error());
    }
    if !response.status().is_success() {
        return Err(auth_error());
    }
    net::decode(response).map_err(|_| auth_error())
}
fn required<'a>(value: &'a Value, key: &str) -> Result<&'a str> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty() && s.len() < 16384)
        .ok_or_else(auth_error)
}
fn exchange(http: &Http, msa: &str, check: &impl Fn() -> Result<()>) -> Result<Session> {
    let xbox = post(
        http,
        "https://user.auth.xboxlive.com/user/authenticate",
        &json!({"Properties":{"AuthMethod":"RPS","SiteName":"user.auth.xboxlive.com","RpsTicket":format!("d={msa}")},"RelyingParty":"http://auth.xboxlive.com","TokenType":"JWT"}),
    )?;
    check()?;
    let xsts = post(
        http,
        "https://xsts.auth.xboxlive.com/xsts/authorize",
        &json!({"Properties":{"SandboxId":"RETAIL","UserTokens":[required(&xbox,"Token")?]},"RelyingParty":"rp://api.minecraftservices.com/","TokenType":"JWT"}),
    )?;
    let user = xsts
        .pointer("/DisplayClaims/xui/0")
        .ok_or_else(auth_error)?;
    let uhs = required(user, "uhs")?;
    // Some successful Minecraft relying-party tokens omit xid; the game's xuid argument accepts empty.
    let xuid = user
        .get("xid")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_owned();
    check()?;
    let minecraft = post(
        http,
        "https://api.minecraftservices.com/authentication/login_with_xbox",
        &json!({"identityToken":format!("XBL3.0 x={uhs};{}",required(&xsts,"Token")?)}),
    )?;
    let token = Zeroizing::new(required(&minecraft, "access_token")?.to_owned());
    check()?;
    let entitlements = authorized_get(
        http,
        "https://api.minecraftservices.com/entitlements/mcstore",
        &token,
    )?;
    check_entitlements(&entitlements)?;
    let profile = authorized_get(
        http,
        "https://api.minecraftservices.com/minecraft/profile",
        &token,
    )?;
    let account = profile_summary(&profile)?;
    check()?;
    Ok(Session {
        token,
        account,
        xuid,
    })
}
fn authorized_get(http: &Http, url: &str, token: &str) -> Result<Value> {
    let response = http
        .client
        .get(url)
        .bearer_auth(token)
        .send()
        .map_err(|_| net::network_error())?;
    match response.status().as_u16() {
        403 => return Err(access_error()),
        404 => {
            return Err(error(
                ErrorCode::Entitlement,
                "No Java Edition profile was found. Check ownership and create your Minecraft profile.",
            ));
        }
        401 => return Err(auth_error()),
        429 | 500..=599 => return Err(net::network_error()),
        _ => {}
    }
    net::decode(response).map_err(|_| auth_error())
}
fn check_entitlements(value: &Value) -> Result<()> {
    let owns = value
        .get("items")
        .and_then(Value::as_array)
        .is_some_and(|items| {
            items.iter().any(|v| {
                v.get("name")
                    .and_then(Value::as_str)
                    .is_some_and(|n| matches!(n, "game_minecraft" | "product_minecraft"))
            })
        });
    if !owns {
        return Err(error(
            ErrorCode::Entitlement,
            "This account has no verified Minecraft Java Edition entitlement. Check your account and subscription.",
        ));
    }
    Ok(())
}
fn profile_summary(value: &Value) -> Result<AccountSummary> {
    let uuid = required(value, "id")?;
    let username = required(value, "name")?;
    if uuid.len() != 32
        || !uuid.chars().all(|c| c.is_ascii_hexdigit())
        || !(3..=16).contains(&username.len())
        || !username
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        return Err(auth_error());
    }
    Ok(AccountSummary {
        uuid: uuid.to_owned(),
        username: username.to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn callback_rejects_state_mismatch_duplicates_and_denial() {
        let state = CsrfToken::new("expected".into());
        let request =
            |query: &str| format!("GET /callback?{query} HTTP/1.1\r\nHost: localhost:1234\r\n\r\n");
        assert_eq!(
            parse_callback(&request("state=expected&code=code"), 1234, &state).unwrap(),
            "code"
        );
        for query in [
            "state=wrong&code=x",
            "state=expected&state=expected&code=x",
            "state=expected&error=access_denied",
            "state=expected&code=x&code=y",
        ] {
            assert!(parse_callback(&request(query), 1234, &state).is_err());
        }
        assert!(parse_callback(&request("state=expected&code=x"), 2222, &state).is_err());
    }
    #[test]
    fn entitlement_and_profile_require_real_fields() {
        assert!(check_entitlements(&json!({"items":[]})).is_err());
        assert!(check_entitlements(&json!({"items":[{"name":"unrelated"}]})).is_err());
        assert!(check_entitlements(&json!({"items":[{"name":"game_minecraft"}]})).is_ok());
        assert!(profile_summary(&json!({"id":"invalid","name":"Player"})).is_err());
    }
}
