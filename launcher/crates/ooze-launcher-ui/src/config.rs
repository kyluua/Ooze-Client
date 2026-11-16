//! Launcher configuration that is public by design.
//!
//! The Microsoft **public client** (Application) ID identifies the project's own Entra app
//! registration. It is not a secret: it appears in every sign-in URL. Its single source of truth
//! is Sol's `launcher/packaging/public-auth.json` (owner-provided, 2026-11-16), which Sol's CI
//! also exports as `OOZE_MICROSOFT_CLIENT_ID` at build time. No client secret exists or may be
//! added (public client, system browser + PKCE).

/// The committed public build configuration, compiled in.
const PUBLIC_AUTH_JSON: &str = include_str!("../../../packaging/public-auth.json");

/// The committed, owner-provided public client ID, if the file is well formed (schema 1).
fn committed_client_id() -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(PUBLIC_AUTH_JSON).ok()?;
    if value.get("schema")?.as_u64()? != 1 {
        return None;
    }
    Some(value.get("microsoft_client_id")?.as_str()?.to_owned())
}

/// Resolve the client ID: a run-time override (developers), then a build-time override
/// (release engineering, set by CI from the same file), then the committed project ID.
/// Anything that is not a non-nil GUID is ignored, so a malformed value can never reach the
/// backend; the backend then reports a configuration error instead of signing in.
pub fn microsoft_client_id(run_time: Option<String>, build_time: Option<&str>) -> Option<String> {
    let committed = committed_client_id();
    [run_time.as_deref(), build_time, committed.as_deref()]
        .into_iter()
        .flatten()
        .map(str::trim)
        .find(|id| is_guid(id) && *id != "00000000-0000-0000-0000-000000000000")
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
        let id = committed_client_id().expect("launcher/packaging/public-auth.json, schema 1");
        assert!(is_guid(&id), "{id:?}");
        assert_eq!(microsoft_client_id(None, None), Some(id));
    }

    #[test]
    fn overrides_take_precedence_and_malformed_values_are_ignored() {
        let dev = "11111111-2222-4333-8444-555555555555".to_owned();
        assert_eq!(microsoft_client_id(Some(dev.clone()), None), Some(dev));
        let build = "aaaaaaaa-bbbb-4ccc-8ddd-eeeeeeeeeeee";
        assert_eq!(
            microsoft_client_id(None, Some(build)).as_deref(),
            Some(build)
        );
        assert_eq!(
            microsoft_client_id(
                Some("not-a-guid".into()),
                Some("00000000-0000-0000-0000-000000000000")
            ),
            committed_client_id(),
            "garbage and nil overrides fall back to the committed ID"
        );
        assert!(!is_guid("359e0deb13e84616978f0fc1f8e273d2"));
    }
}
