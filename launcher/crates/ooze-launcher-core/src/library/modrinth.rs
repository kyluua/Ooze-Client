use super::model::*;
use crate::{
    api::ErrorCode,
    util::{Result, error, metadata_error},
};
use serde::{Deserialize, de::DeserializeOwned};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::Read,
    sync::Mutex,
    time::{Duration, Instant},
};

const BASE: &str = "https://api.modrinth.com/v2/";
const MAX_JSON: u64 = 8 * 1024 * 1024;
type Cache = BTreeMap<String, (Instant, Vec<u8>)>;

pub(super) struct Modrinth {
    pub client: reqwest::blocking::Client,
    base: url::Url,
    cache: Mutex<Cache>,
    rate_reset: Mutex<Option<Instant>>,
}
pub(super) trait Source {
    fn project(&self, id: &str, check: &dyn Fn() -> Result<()>) -> Result<Project>;
    fn versions(
        &self,
        project: &str,
        target: &Target,
        check: &dyn Fn() -> Result<()>,
    ) -> Result<Vec<Version>>;
    fn version(&self, id: &str, check: &dyn Fn() -> Result<()>) -> Result<Version>;
}
#[derive(Clone, Debug, Deserialize)]
pub(super) struct Version {
    #[serde(flatten)]
    pub summary: VersionSummary,
    pub files: Vec<VersionFile>,
    pub dependencies: Vec<Dependency>,
}
#[derive(Clone, Debug, Deserialize)]
pub(super) struct VersionFile {
    pub url: String,
    pub filename: String,
    pub primary: bool,
    pub size: u64,
    pub hashes: Hashes,
}
#[derive(Clone, Debug, Deserialize)]
pub(super) struct Hashes {
    pub sha512: String,
}
#[derive(Clone, Debug, Deserialize)]
pub(super) struct Dependency {
    pub version_id: Option<String>,
    pub project_id: Option<String>,
    pub dependency_type: String,
}
#[derive(Clone)]
pub(super) struct ResolvedFile {
    pub project: String,
    pub version: String,
    pub kind: ContentKind,
    pub file: VersionFile,
    pub required: Vec<String>,
}
pub(super) struct Resolution {
    pub files: Vec<ResolvedFile>,
    pub optional: Vec<String>,
    pub embedded: Vec<String>,
}

pub(super) fn identifier(value: &str) -> Result<()> {
    if value.is_empty()
        || value.len() > 128
        || !value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
    {
        return Err(metadata_error());
    }
    Ok(())
}
impl Modrinth {
    pub fn new() -> Result<Self> {
        let client = reqwest::blocking::Client::builder()
            .user_agent("OozeLauncher/0.1.0 (https://github.com/kyluua/Ooze-Client)")
            .connect_timeout(Duration::from_secs(8))
            .timeout(Duration::from_secs(20))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|_| network())?;
        Ok(Self {
            client,
            base: url::Url::parse(BASE).map_err(|_| metadata_error())?,
            cache: Mutex::new(BTreeMap::new()),
            rate_reset: Mutex::new(None),
        })
    }
    fn get<T: DeserializeOwned>(
        &self,
        path: &str,
        query: &[(&str, String)],
        check: &dyn Fn() -> Result<()>,
    ) -> Result<T> {
        let mut url = self.base.join(path).map_err(|_| metadata_error())?;
        url.query_pairs_mut()
            .extend_pairs(query.iter().map(|(k, v)| (*k, v)));
        let key = url.as_str().to_owned();
        check()?;
        if let Some((at, bytes)) = self.cache.lock().map_err(|_| network())?.get(&key)
            && at.elapsed() < Duration::from_secs(60)
        {
            return serde_json::from_slice(bytes).map_err(|_| metadata_error());
        }
        for attempt in 0..3 {
            check()?;
            if let Some(deadline) = *self.rate_reset.lock().map_err(|_| network())? {
                pause(deadline.saturating_duration_since(Instant::now()), check)?;
            }
            let response = self.client.get(url.clone()).send().map_err(|_| network())?;
            let status = response.status();
            let seconds = |name| {
                response
                    .headers()
                    .get(name)
                    .and_then(|v| v.to_str().ok())
                    .and_then(|v| v.parse::<u64>().ok())
            };
            if seconds("x-ratelimit-remaining") == Some(0) {
                *self.rate_reset.lock().map_err(|_| network())? = Some(
                    Instant::now()
                        + Duration::from_secs(seconds("x-ratelimit-reset").unwrap_or(60).min(3600)),
                );
            }
            if (status.as_u16() == 429 || status.is_server_error()) && attempt < 2 {
                let delay = seconds("retry-after").unwrap_or(1u64 << attempt);
                pause(Duration::from_secs(delay.min(3600)), check)?;
                continue;
            }
            if !status.is_success() {
                return Err(error(
                    if status.as_u16() == 429 || status.is_server_error() {
                        ErrorCode::Network
                    } else {
                        ErrorCode::Metadata
                    },
                    format!("Modrinth request failed (HTTP {}).", status.as_u16()),
                ));
            }
            if response.content_length().is_some_and(|n| n > MAX_JSON) {
                return Err(metadata_error());
            }
            let mut bytes = Vec::new();
            response
                .take(MAX_JSON + 1)
                .read_to_end(&mut bytes)
                .map_err(|_| network())?;
            check()?;
            if bytes.len() as u64 > MAX_JSON {
                return Err(metadata_error());
            }
            let value = serde_json::from_slice(&bytes).map_err(|_| metadata_error())?;
            let mut cache = self.cache.lock().map_err(|_| network())?;
            if cache.len() >= 64 {
                cache.clear();
            }
            cache.insert(key, (Instant::now(), bytes));
            return Ok(value);
        }
        Err(network())
    }
    pub fn search(&self, request: &Search, check: &dyn Fn() -> Result<()>) -> Result<SearchPage> {
        request.target.validate()?;
        if request.query.len() > 256
            || request.query.chars().any(char::is_control)
            || !(1..=50).contains(&request.limit)
            || request.offset > 10000
        {
            return Err(invalid("Invalid content search bounds."));
        }
        let mut facets = vec![
            vec![format!("project_type:{}", request.kind.api_name())],
            vec![format!("versions:{}", request.target.minecraft)],
        ];
        if request.kind == ContentKind::Mod {
            let loader = request
                .target
                .modrinth_loader()
                .ok_or_else(|| invalid("Vanilla instances do not load mod JARs."))?;
            facets.push(vec![format!("categories:{loader}")]);
            facets.push(vec![
                "client_side:required".into(),
                "client_side:optional".into(),
            ]);
        }
        let facets = serde_json::to_string(&facets).map_err(|_| metadata_error())?;
        self.get(
            "search",
            &[
                ("query", request.query.clone()),
                ("facets", facets),
                ("offset", request.offset.to_string()),
                ("limit", request.limit.to_string()),
            ],
            check,
        )
    }
}
impl Source for Modrinth {
    fn project(&self, id: &str, check: &dyn Fn() -> Result<()>) -> Result<Project> {
        identifier(id)?;
        self.get(&format!("project/{id}"), &[], check)
    }
    fn versions(
        &self,
        project: &str,
        target: &Target,
        check: &dyn Fn() -> Result<()>,
    ) -> Result<Vec<Version>> {
        identifier(project)?;
        target.validate()?;
        self.get(
            &format!("project/{project}/version"),
            &[(
                "game_versions",
                serde_json::to_string(&[&target.minecraft]).map_err(|_| metadata_error())?,
            )],
            check,
        )
    }
    fn version(&self, id: &str, check: &dyn Fn() -> Result<()>) -> Result<Version> {
        identifier(id)?;
        self.get(&format!("version/{id}"), &[], check)
    }
}
fn pause(delay: Duration, check: &dyn Fn() -> Result<()>) -> Result<()> {
    // Long server deadlines are surfaced as retryable rather than retried early.
    if delay > Duration::from_secs(10) {
        return Err(network());
    }
    let deadline = Instant::now() + delay;
    while Instant::now() < deadline {
        check()?;
        std::thread::sleep(
            deadline
                .saturating_duration_since(Instant::now())
                .min(Duration::from_millis(50)),
        );
    }
    check()
}
fn network() -> crate::api::LauncherError {
    error(
        ErrorCode::Network,
        "Modrinth is unavailable or rate limited. Retry later.",
    )
}

pub(super) fn compatible(project: &Project, version: &Version, target: &Target) -> bool {
    let v = &version.summary;
    if !v.game_versions.contains(&target.minecraft) || v.project_id != project.id {
        return false;
    }
    match project.project_type {
        ContentKind::Mod => {
            project.client_side != "unsupported"
                && target
                    .modrinth_loader()
                    .is_some_and(|l| v.loaders.iter().any(|s| s == l))
        }
        ContentKind::Datapack => v.loaders.iter().any(|l| l == "datapack"),
        ContentKind::Shader | ContentKind::Resourcepack => project.client_side != "unsupported",
    }
}

pub(super) fn resolve(
    source: &dyn Source,
    instance: &Instance,
    project: &str,
    version: &str,
    check: &dyn Fn() -> Result<()>,
) -> Result<Resolution> {
    struct Graph<'a> {
        source: &'a dyn Source,
        instance: &'a Instance,
        check: &'a dyn Fn() -> Result<()>,
        selected: BTreeMap<String, String>,
        visiting: BTreeSet<String>,
        done: BTreeSet<String>,
        forbidden: Vec<(String, Option<String>)>,
        result: Resolution,
    }
    impl Graph<'_> {
        fn visit(&mut self, version: Version, depth: usize) -> Result<()> {
            (self.check)()?;
            if depth > 32 || self.selected.len() >= 64 {
                return Err(invalid("Dependency graph exceeds the supported limit."));
            }
            let summary = &version.summary;
            identifier(&summary.id)?;
            identifier(&summary.project_id)?;
            if !self.visiting.insert(summary.id.clone()) {
                return Err(invalid("Content dependencies contain a cycle."));
            }
            if self.done.contains(&summary.id) {
                self.visiting.remove(&summary.id);
                return Ok(());
            }
            if self
                .selected
                .get(&summary.project_id)
                .is_some_and(|id| id != &summary.id)
            {
                return Err(invalid(
                    "Dependencies require conflicting versions of one project.",
                ));
            }
            self.selected
                .insert(summary.project_id.clone(), summary.id.clone());
            let project = self.source.project(&summary.project_id, self.check)?;
            if !compatible(&project, &version, &self.instance.target) {
                return Err(invalid(
                    "Selected content is incompatible with this instance or client side.",
                ));
            }
            if project.project_type == ContentKind::Mod
                && self.instance.target.ooze
                && project.slug == "fabric-api"
            {
                if summary.version_number != "0.141.6+1.21.11" {
                    return Err(invalid(
                        "This dependency requires a different Fabric API than the pinned Ooze release. Use a compatible content version or a Fabric instance without Ooze.",
                    ));
                }
                self.result
                    .embedded
                    .push(format!("{} (provided by pinned Ooze release)", project.id));
                self.visiting.remove(&summary.id);
                self.done.insert(summary.id.clone());
                return Ok(());
            }
            let mut required = Vec::new();
            for dep in &version.dependencies {
                let label = dep
                    .project_id
                    .clone()
                    .or_else(|| dep.version_id.clone())
                    .unwrap_or_else(|| "external dependency".into());
                match dep.dependency_type.as_str() {
                    "optional" => self.result.optional.push(label),
                    "embedded" => self.result.embedded.push(label),
                    "incompatible" => {
                        let project = match (&dep.project_id, &dep.version_id) {
                            (Some(p), _) => p.clone(),
                            (_, Some(v)) => self.source.version(v, self.check)?.summary.project_id,
                            _ => {
                                return Err(invalid(
                                    "An incompatible external dependency cannot be resolved.",
                                ));
                            }
                        };
                        self.forbidden.push((project, dep.version_id.clone()));
                    }
                    "required" => {
                        let candidate = if let Some(v) = &dep.version_id {
                            self.source.version(v, self.check)?
                        } else if let Some(p) = &dep.project_id {
                            let project = self.source.project(p, self.check)?;
                            self.source
                                .versions(p, &self.instance.target, self.check)?
                                .into_iter()
                                .find(|v| compatible(&project, v, &self.instance.target))
                                .ok_or_else(|| {
                                    invalid("No compatible required dependency is available.")
                                })?
                        } else {
                            return Err(invalid(
                                "A required external dependency needs manual resolution.",
                            ));
                        };
                        if dep
                            .project_id
                            .as_ref()
                            .is_some_and(|p| p != &candidate.summary.project_id)
                        {
                            return Err(metadata_error());
                        }
                        required.push(candidate.summary.project_id.clone());
                        self.visit(candidate, depth + 1)?;
                    }
                    _ => return Err(metadata_error()),
                }
            }
            let primary: Vec<_> = version.files.iter().filter(|f| f.primary).collect();
            let file = match primary.as_slice() {
                [file] => *file,
                [] if version.files.len() == 1 => &version.files[0],
                _ => {
                    return Err(invalid(
                        "Choose a version with one unambiguous primary file.",
                    ));
                }
            };
            validate_file(file)?;
            self.result.files.push(ResolvedFile {
                project: project.id,
                version: summary.id.clone(),
                kind: project.project_type,
                file: file.clone(),
                required,
            });
            self.visiting.remove(&summary.id);
            self.done.insert(summary.id.clone());
            Ok(())
        }
    }
    let root = source.version(version, check)?;
    let root_project = source.project(project, check)?;
    if root.summary.project_id != root_project.id {
        return Err(metadata_error());
    }
    let mut graph = Graph {
        source,
        instance,
        check,
        selected: BTreeMap::new(),
        visiting: BTreeSet::new(),
        done: BTreeSet::new(),
        forbidden: Vec::new(),
        result: Resolution {
            files: Vec::new(),
            optional: Vec::new(),
            embedded: Vec::new(),
        },
    };
    graph.visit(root, 0)?;
    for (project, version) in graph.forbidden {
        if graph
            .selected
            .get(&project)
            .is_some_and(|v| version.as_ref().is_none_or(|want| v == want))
            || instance.content.iter().any(|r| {
                r.enabled
                    && r.project == project
                    && version.as_ref().is_none_or(|v| &r.version == v)
            })
        {
            return Err(invalid(
                "Content has an incompatible installed or selected dependency.",
            ));
        }
    }
    for f in &graph.result.files {
        if f.kind == ContentKind::Shader
            && !instance
                .content
                .iter()
                .any(|r| r.enabled && r.kind == ContentKind::Mod && r.project == "YL57xq9U")
            && !graph
                .result
                .files
                .iter()
                .any(|r| r.kind == ContentKind::Mod && r.project == "YL57xq9U")
        {
            return Err(invalid(
                "Install a compatible Iris shader loader before installing shader packs.",
            ));
        }
    }
    if graph.result.files.iter().map(|f| f.file.size).sum::<u64>() > 2 * 1024 * 1024 * 1024 {
        return Err(invalid("Content plan is too large."));
    }
    Ok(graph.result)
}
pub(super) fn validate_file(file: &VersionFile) -> Result<()> {
    crate::util::relative(&file.filename)?;
    if file.filename.contains('/')
        || file.filename.len() > 180
        || !(file.filename.ends_with(".jar") || file.filename.ends_with(".zip"))
        || file.size == 0
        || file.size > 1024 * 1024 * 1024
        || file.hashes.sha512.len() != 128
        || !file.hashes.sha512.bytes().all(|b| b.is_ascii_hexdigit())
    {
        return Err(metadata_error());
    }
    cdn_url(&file.url)?;
    Ok(())
}
pub(super) fn cdn_url(raw: &str) -> Result<url::Url> {
    let url = url::Url::parse(raw).map_err(|_| metadata_error())?;
    if url.scheme() != "https"
        || url.host_str() != Some("cdn.modrinth.com")
        || !url.path().starts_with("/data/")
        || !url.username().is_empty()
        || url.password().is_some()
        || url.port().is_some_and(|p| p != 443)
        || url.fragment().is_some()
    {
        return Err(metadata_error());
    }
    Ok(url)
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Fixture(BTreeMap<String, Version>);
    impl Source for Fixture {
        fn project(&self, id: &str, _: &dyn Fn() -> Result<()>) -> Result<Project> {
            Ok(Project {
                id: id.into(),
                slug: id.into(),
                title: id.into(),
                description: String::new(),
                project_type: ContentKind::Mod,
                client_side: "required".into(),
                license: License {
                    id: "MIT".into(),
                    name: "MIT".into(),
                    url: None,
                },
            })
        }
        fn version(&self, id: &str, _: &dyn Fn() -> Result<()>) -> Result<Version> {
            self.0.get(id).cloned().ok_or_else(metadata_error)
        }
        fn versions(
            &self,
            project: &str,
            _: &Target,
            _: &dyn Fn() -> Result<()>,
        ) -> Result<Vec<Version>> {
            Ok(self
                .0
                .values()
                .filter(|v| v.summary.project_id == project)
                .cloned()
                .collect())
        }
    }
    fn version(id: &str, project: &str, deps: Vec<Dependency>) -> Version {
        Version {
            summary: VersionSummary {
                id: id.into(),
                project_id: project.into(),
                name: id.into(),
                version_number: "1".into(),
                game_versions: vec!["1.21.11".into()],
                loaders: vec!["fabric".into()],
            },
            files: vec![VersionFile {
                url: format!("https://cdn.modrinth.com/data/{project}/versions/{id}/a.jar"),
                filename: format!("{id}.jar"),
                primary: true,
                size: 10,
                hashes: Hashes {
                    sha512: "a".repeat(128),
                },
            }],
            dependencies: deps,
        }
    }
    fn dep(id: &str, kind: &str) -> Dependency {
        Dependency {
            version_id: Some(id.into()),
            project_id: None,
            dependency_type: kind.into(),
        }
    }
    #[test]
    fn dependency_graph_rejects_cycles_conflicts_and_wrong_loaders() {
        let dir = tempfile::tempdir().unwrap();
        let store = super::super::store::Store::open(dir.path()).unwrap();
        let id = store
            .create(
                "Graph".into(),
                Target::fabric(),
                Preset::Minimal,
                Mode::Custom,
            )
            .unwrap();
        let instance = store.instance(&id).unwrap();
        let fixture = Fixture(BTreeMap::from([
            ("a".into(), version("a", "A", vec![dep("b", "required")])),
            ("b".into(), version("b", "B", vec![dep("a", "required")])),
        ]));
        assert!(resolve(&fixture, &instance, "A", "a", &|| Ok(())).is_err());
        let mut fixture = fixture;
        fixture.0.insert(
            "a".into(),
            version("a", "A", vec![dep("b", "required"), dep("c", "required")]),
        );
        fixture.0.insert("b".into(), version("b", "B", vec![]));
        fixture.0.insert("c".into(), version("c", "B", vec![]));
        assert!(resolve(&fixture, &instance, "A", "a", &|| Ok(())).is_err());
        fixture.0.insert(
            "a".into(),
            version("a", "A", vec![dep("b", "required"), dep("c", "optional")]),
        );
        let result = resolve(&fixture, &instance, "A", "a", &|| Ok(())).unwrap();
        assert_eq!(result.files.len(), 2);
        assert_eq!(result.optional, vec!["c"]);
        fixture.0.get_mut("b").unwrap().summary.loaders = vec!["forge".into()];
        assert!(resolve(&fixture, &instance, "A", "a", &|| Ok(())).is_err());
    }
    #[test]
    fn search_uses_facets_user_agent_and_cache_for_all_four_types() {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let worker = std::thread::spawn(move || {
            for _ in 0..4 {
                let (mut stream, _) = listener.accept().unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .unwrap();
                let mut buffer = [0; 8192];
                let n = stream.read(&mut buffer).unwrap();
                let request = String::from_utf8_lossy(&buffer[..n]).to_ascii_lowercase();
                assert!(request.contains("facets="));
                assert!(request.contains("oozelauncher/0.1.0"));
                let body = r#"{"hits":[],"offset":0,"limit":10,"total_hits":0}"#;
                write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                )
                .unwrap();
            }
        });
        let mut client = Modrinth::new().unwrap();
        client.base = url::Url::parse(&format!("http://{address}/v2/")).unwrap();
        for kind in [
            ContentKind::Mod,
            ContentKind::Resourcepack,
            ContentKind::Datapack,
            ContentKind::Shader,
        ] {
            let request = Search {
                query: String::new(),
                kind,
                target: Target::fabric(),
                offset: 0,
                limit: 10,
            };
            client.search(&request, &|| Ok(())).unwrap();
            client.search(&request, &|| Ok(())).unwrap();
        }
        worker.join().unwrap();
    }
    #[test]
    fn rejects_untrusted_content_urls_and_paths() {
        for raw in [
            "http://cdn.modrinth.com/data/a",
            "https://cdn.modrinth.com.evil.test/data/a",
            "https://user@cdn.modrinth.com/data/a",
            "https://localhost/data/a",
        ] {
            assert!(cdn_url(raw).is_err());
        }
        let mut file = VersionFile {
            url: "https://cdn.modrinth.com/data/a/v/x.jar".into(),
            filename: "ok.jar".into(),
            primary: true,
            size: 10,
            hashes: Hashes {
                sha512: "a".repeat(128),
            },
        };
        validate_file(&file).unwrap();
        file.filename = "../evil.jar".into();
        assert!(validate_file(&file).is_err());
    }
    #[test]
    fn cancellation_and_long_rate_deadlines_are_bounded() {
        assert!(pause(Duration::from_secs(20), &|| Ok(())).is_err());
        assert_eq!(
            pause(Duration::from_millis(100), &|| Err(error(
                ErrorCode::Cancelled,
                "cancelled"
            )))
            .unwrap_err()
            .code,
            ErrorCode::Cancelled
        );
    }
}
