//! Defence-in-depth redaction for anything the interface displays, copies or exports.
//!
//! The backend owns redaction of logs and diagnostic bundles (Sol, LAUNCHER_API_V1). The
//! interface never receives credentials by design, but game output and backend text are passed
//! through this filter again before they reach the screen or the clipboard, so a backend
//! regression cannot leak a token into a screenshot or a pasted bug report.

use std::borrow::Cow;
use std::sync::OnceLock;

use regex::{Captures, Regex};

pub const REDACTED: &str = "[redacted]";

struct Rules {
    /// Minecraft launch arguments whose value is secret or personal: `--accessToken X`.
    launch_args: Regex,
    /// `key=value`, `key: value` and JSON `"key":"value"` for credential-like keys.
    key_values: Regex,
    /// `Authorization: Bearer x`, `XBL3.0 x=uhs;token`.
    auth_headers: Regex,
    /// JSON Web Tokens (Minecraft access tokens are JWTs).
    jwt: Regex,
    /// Microsoft refresh/authorization codes such as `M.C123_BAY.0.U.-Ab...`.
    ms_tokens: Regex,
    /// Legacy session string `token:<access>:<uuid>`.
    session_string: Regex,
    /// Personal path segments: `C:\Users\<name>\`, `/home/<name>/`, `/Users/<name>/`.
    user_paths: Regex,
}

fn rules() -> &'static Rules {
    static RULES: OnceLock<Rules> = OnceLock::new();
    RULES.get_or_init(|| Rules {
        launch_args: Regex::new(
            r"(?i)(--(?:accessToken|session|xuid|clientId|uuid-token|quickPlay\w*Token))(\s+|=)(\S+)",
        )
        .expect("static regex"),
        key_values: Regex::new(
            r#"(?i)("?(?:access_?token|refresh_?token|id_?token|session_?id|auth_?code|client_?secret|password|xuid|user_?hash|uhs|code_?verifier|device_?code)"?\s*[:=]\s*"?)([^"\s,;&}]+)"#,
        )
        .expect("static regex"),
        auth_headers: Regex::new(r"(?i)(authorization\s*[:=]\s*(?:bearer|xbl3\.0)\s+)(\S+)")
            .expect("static regex"),
        jwt: Regex::new(r"eyJ[A-Za-z0-9_-]{8,}\.[A-Za-z0-9_-]{8,}\.[A-Za-z0-9_-]{8,}")
            .expect("static regex"),
        ms_tokens: Regex::new(r"\bM\.[A-Z0-9]{1,6}_[A-Za-z0-9_.!*$-]{16,}").expect("static regex"),
        session_string: Regex::new(r"\btoken:[A-Za-z0-9_.-]{8,}:[0-9a-fA-F-]{32,36}")
            .expect("static regex"),
        user_paths: Regex::new(r"(?i)([A-Z]:[\\/]+Users[\\/]+|/home/|/Users/)([^\\/\r\n]+)([\\/])")
            .expect("static regex"),
    })
}

/// Redact one line of text. Returns the input unchanged (borrowed) when nothing matched.
pub fn redact(text: &str) -> Cow<'_, str> {
    let r = rules();
    let mut out = Cow::Borrowed(text);
    let mut apply = |re: &Regex, rep: &dyn Fn(&Captures<'_>) -> String| {
        if re.is_match(&out) {
            out = Cow::Owned(re.replace_all(&out, |c: &Captures<'_>| rep(c)).into_owned());
        }
    };
    apply(&r.jwt, &|_| REDACTED.to_owned());
    apply(&r.ms_tokens, &|_| REDACTED.to_owned());
    apply(&r.session_string, &|_| format!("token:{REDACTED}"));
    apply(&r.launch_args, &|c| format!("{}{}{REDACTED}", &c[1], &c[2]));
    apply(&r.auth_headers, &|c| format!("{}{REDACTED}", &c[1]));
    apply(&r.key_values, &|c| format!("{}{REDACTED}", &c[1]));
    apply(&r.user_paths, &|c| format!("{}<user>{}", &c[1], &c[3]));
    out
}

/// True when a string still looks like it contains a credential after redaction (used by tests
/// and by the export guard).
pub fn looks_secret(text: &str) -> bool {
    let r = rules();
    r.jwt.is_match(text)
        || r.ms_tokens.is_match(text)
        || r.session_string.is_match(text)
        || r.launch_args.captures_iter(text).any(|c| &c[3] != REDACTED)
        || r.key_values
            .captures_iter(text)
            .any(|c| &c[2] != REDACTED && !c[2].starts_with(REDACTED))
}

#[cfg(test)]
mod tests {
    use super::*;

    const JWT: &str = "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U";

    #[test]
    fn launch_arguments_are_redacted() {
        let line = format!(
            "Launching: java -cp x net.minecraft.client.main.Main --username Steve --accessToken {JWT} --xuid 2535400000000000 --clientId abc=="
        );
        let out = redact(&line);
        assert!(!out.contains(JWT), "{out}");
        assert!(out.contains("--accessToken [redacted]"));
        assert!(out.contains("--xuid [redacted]"));
        assert!(out.contains("--clientId [redacted]"));
        assert!(out.contains("--username Steve"), "display name stays");
        assert!(!looks_secret(&out));
    }

    #[test]
    fn equals_form_and_json_are_redacted() {
        let out = redact(
            r#"{"access_token":"abc.def-123","refresh_token": "M.C512_BAY.0.U.-AbCdEfGhIjKlMnOpQrSt", "username":"Alex"}"#,
        );
        assert!(!out.contains("abc.def-123"), "{out}");
        assert!(!out.contains("AbCdEfGhIjKlMnOpQrSt"), "{out}");
        assert!(out.contains("Alex"));
        assert!(!looks_secret(&out), "{out}");
        assert!(redact("--accessToken=xyz123").contains("--accessToken=[redacted]"));
    }

    #[test]
    fn headers_jwts_and_session_strings_are_redacted() {
        assert_eq!(
            redact("Authorization: Bearer abcdef123456"),
            "Authorization: Bearer [redacted]"
        );
        assert!(!redact(&format!("token {JWT} end")).contains("eyJ"));
        let s = redact("session token:abcdefghijkl:0123456789abcdef0123456789abcdef");
        assert!(!s.contains("abcdefghijkl"), "{s}");
        assert!(!redact("https://login.live.com/oauth20_desktop.srf?code=M.C540_SN1.2.U.abcdefghijklmnopqrstuv&state=1").contains("abcdefghijklmnop"));
    }

    #[test]
    fn personal_paths_are_shortened() {
        assert_eq!(
            redact(r"Saved to C:\Users\Alex Example\AppData\Roaming\.ooze\logs"),
            r"Saved to C:\Users\<user>\AppData\Roaming\.ooze\logs"
        );
        assert_eq!(redact("/home/alex/.ooze"), "/home/<user>/.ooze");
    }

    #[test]
    fn ordinary_lines_are_borrowed_unchanged() {
        let line = "[Render thread/INFO]: Ooze Client is OFF (UNSUPPORTED_PLATFORM)";
        assert!(matches!(redact(line), Cow::Borrowed(_)));
        assert!(!looks_secret(line));
    }
}
