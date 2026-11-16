//! Launcher configuration that is public by design.
//!
//! The Microsoft **public client** (Application) ID identifies the project's own Entra app
//! registration. It is not a secret: it appears in every sign-in URL. Its single source of truth
//! is `launcher/resources/auth/microsoft-public-client-id.txt` (owner-provided, 2026-11-16).
//! No client secret exists or may be added (public client, system browser + PKCE).

/// The committed, owner-provided public client ID (whitespace trimmed at use).
const COMMITTED_CLIENT_ID: &str =
    include_str!("../../../resources/auth/microsoft-public-client-id.txt");

/// Resolve the client ID: a run-time override (developers), then a build-time override
/// (release engineering), then the committed project ID. Anything that is not a GUID is
/// ignored, so a malformed value can never reach the backend; the backend then reports a
/// configuration error instead of signing in.
pub fn microsoft_client_id(run_time: Option<String>, build_time: Option<&str>) -> Option<String> {
    [run_time.as_deref(), build_time, Some(COMMITTED_CLIENT_ID)]
        .into_iter()
        .flatten()
        .map(str::trim)
        .find(|id| is_guid(id))
        .map(str::to_owned)
}

/// Same shape check as the backend's: 8-4-4-4-12 hexadecimal digits.
pub fn is_guid(id: &str) -> bool {
    id.len() == 36
        && id.bytes().enumerate().all(|(i, c)| {
            if [8, 13, 18, 23].contains(&i) {
                c == b'-'
            } else {
                c.is_ascii_hexdigit()
            }
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_committed_project_id_is_a_well_formed_guid() {
        assert!(
            is_guid(COMMITTED_CLIENT_ID.trim()),
            "{COMMITTED_CLIENT_ID:?}"
        );
        assert_eq!(
            microsoft_client_id(None, None).as_deref(),
            Some(COMMITTED_CLIENT_ID.trim())
        );
    }

    #[test]
    fn overrides_take_precedence_and_malformed_values_are_ignored() {
        let dev = "11111111-2222-4333-8444-555555555555".to_owned();
        assert_eq!(
            microsoft_client_id(Some(dev.clone()), None),
            Some(dev.clone())
        );
        let build = "aaaaaaaa-bbbb-4ccc-8ddd-eeeeeeeeeeee";
        assert_eq!(
            microsoft_client_id(None, Some(build)).as_deref(),
            Some(build)
        );
        assert_eq!(
            microsoft_client_id(Some("not-a-guid".into()), Some("  ")).as_deref(),
            Some(COMMITTED_CLIENT_ID.trim()),
            "garbage overrides fall back to the committed ID"
        );
        assert!(!is_guid("359e0deb13e84616978f0fc1f8e273d2"));
    }
}
