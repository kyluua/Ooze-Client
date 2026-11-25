use crate::{
    api::{EngineState, ErrorCode, Settings},
    auth::Session,
    install::Prepared,
    metadata,
    util::{self, Result, error, io_error, metadata_error},
};
use regex::Regex;
use std::{
    collections::BTreeMap,
    fs,
    io::{BufRead, BufReader, Read, Write},
    path::Path,
    process::{Command, Stdio},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use zeroize::Zeroizing;

fn command(exe: &Path) -> Command {
    let mut cmd = Command::new(exe);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }
    for key in [
        "JAVA_TOOL_OPTIONS",
        "JDK_JAVA_OPTIONS",
        "_JAVA_OPTIONS",
        "CLASSPATH",
    ] {
        cmd.env_remove(key);
    }
    cmd
}
pub(crate) fn validate_java(path: &Path) -> Result<()> {
    util::absolute(path)?;
    if !path.is_file() {
        return Err(error(
            ErrorCode::JavaRuntime,
            "Java executable is missing. Repair the managed runtime or choose Java 21 x64.",
        ));
    }
    let mut child = command(path)
        .args(["-XshowSettings:properties", "-version"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|_| {
            error(
                ErrorCode::JavaRuntime,
                "Couldn't execute Java. Check Windows application policy and select Java 21 x64.",
            )
        })?;
    let stdout = child.stdout.take().ok_or_else(io_error)?;
    let stderr = child.stderr.take().ok_or_else(io_error)?;
    let out = std::thread::spawn(move || bounded_output(stdout));
    let err = std::thread::spawn(move || bounded_output(stderr));
    let deadline = Instant::now() + Duration::from_secs(10);
    let status = loop {
        if let Some(status) = child.try_wait().map_err(|_| io_error())? {
            break status;
        }
        if Instant::now() > deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err(error(
                ErrorCode::JavaRuntime,
                "Java validation timed out. Select a working Java 21 x64 runtime.",
            ));
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    let text = format!(
        "{}\n{}",
        out.join().unwrap_or_default(),
        err.join().unwrap_or_default()
    );
    validate_properties(status.success(), &text)
}
fn bounded_output(input: impl Read) -> String {
    let mut bytes = Vec::new();
    let _ = input.take(1024 * 1024).read_to_end(&mut bytes);
    String::from_utf8_lossy(&bytes).into_owned()
}
fn validate_properties(success: bool, text: &str) -> Result<()> {
    let property = |name: &str| {
        text.lines().find_map(|line| {
            line.trim()
                .strip_prefix(name)
                .and_then(|v| v.strip_prefix(" = "))
        })
    };
    if !success
        || !property("java.version").is_some_and(|v| v == "21" || v.starts_with("21."))
        || !property("os.arch").is_some_and(|v| matches!(v, "amd64" | "x86_64"))
    {
        return Err(error(
            ErrorCode::JavaRuntime,
            "Ooze requires Java 21 x64. The selected runtime does not match.",
        ));
    }
    Ok(())
}
pub(crate) const MAX_GAME_LOG_BYTES: usize = 16 * 1024 * 1024;

pub(crate) struct Redactor {
    secret: Zeroizing<String>,
    patterns: Vec<Regex>,
}
impl Redactor {
    pub(crate) fn new(token: &str) -> Result<Self> {
        let patterns = [
            r#"(?i)(\b(?:authorization["']?\s*[:=]\s*|identitytoken(?:["']?\s*[:=]\s*|\s+)))([^\r\n]+)"#,
            r#"(?i)(\b(?:access[_-]?token|refresh[_-]?token|id[_-]?token)[\s"':=]+)([^\s",}]+)"#,
            r"\beyJ[A-Za-z0-9_-]{8,}\.[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+\b",
        ]
        .into_iter()
        .map(|pattern| Regex::new(pattern).map_err(|_| metadata_error()))
        .collect::<Result<Vec<_>>>()?;
        Ok(Self {
            secret: Zeroizing::new(token.to_owned()),
            patterns,
        })
    }
    pub(crate) fn line(&self, line: &str) -> String {
        let mut text = if self.secret.is_empty() {
            line.to_owned()
        } else {
            line.replace(self.secret.as_str(), "[REDACTED]")
        };
        for pattern in &self.patterns {
            text = pattern
                .replace_all(&text, |caps: &regex::Captures<'_>| {
                    if caps.len() > 2 {
                        format!("{}[REDACTED]", &caps[1])
                    } else {
                        "[REDACTED]".into()
                    }
                })
                .into_owned();
        }
        text
    }
}
fn quote_argfile(value: &str) -> String {
    format!(
        "\"{}\"",
        value
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
            .replace('\r', "\\r")
    )
}
pub(crate) fn launch(
    prepared: &Prepared,
    settings: &Settings,
    session: Session,
    client_id: &str,
    data: &Path,
    on_started: impl Fn(u32),
    on_line: impl Fn(String, Option<EngineState>) + Send + Sync + 'static,
) -> Result<Option<i32>> {
    let install = &prepared.installation;
    let root = &prepared.root;
    let java = match &settings.java_path {
        Some(path) => path.clone(),
        None => util::join(
            root,
            install.managed_java.as_deref().ok_or_else(metadata_error)?,
        )?,
    };
    validate_java(&java)?;
    let paths = install
        .classpath
        .iter()
        .map(|v| util::join(root, v))
        .collect::<Result<Vec<_>>>()?;
    let classpath = std::env::join_paths(paths)
        .map_err(|_| metadata_error())?
        .to_string_lossy()
        .into_owned();
    // Java's argument file keeps the long, non-secret classpath off Windows' command-line limit.
    let argfile = util::join(root, "classpath.args")?;
    fs::write(&argfile, format!("-cp\n{}\n", quote_argfile(&classpath))).map_err(|_| io_error())?;
    let values = BTreeMap::from([
        (
            "natives_directory",
            util::join(root, "natives")?.to_string_lossy().into_owned(),
        ),
        ("launcher_name", "Ooze".into()),
        ("launcher_version", env!("CARGO_PKG_VERSION").into()),
        ("classpath", classpath),
        ("classpath_separator", ";".into()),
        (
            "library_directory",
            util::join(root, "libraries")?
                .to_string_lossy()
                .into_owned(),
        ),
        ("auth_player_name", session.account.username.clone()),
        ("auth_uuid", session.account.uuid.clone()),
        ("auth_access_token", session.token.to_string()),
        ("clientid", client_id.into()),
        ("auth_xuid", session.xuid.clone()),
        ("user_type", "msa".into()),
        ("user_properties", "{}".into()),
        ("version_name", install.version_name.clone()),
        ("version_type", "release".into()),
        (
            "game_directory",
            settings.instance_dir.to_string_lossy().into_owned(),
        ),
        (
            "assets_root",
            util::join(root, "assets")?.to_string_lossy().into_owned(),
        ),
        ("assets_index_name", install.asset_index.clone()),
    ]);
    let mut jvm = metadata::expand(&install.jvm_args, &values)?;
    if let Some(index) = jvm.iter().position(|v| v == "-cp") {
        if index + 1 >= jvm.len() {
            return Err(metadata_error());
        }
        jvm.drain(index..=index + 1);
    } else {
        return Err(metadata_error());
    }
    let game_args = metadata::expand(&install.game_args, &values)?;
    let logs = util::join(data, "logs")?;
    util::mkdir(&logs)?;
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| io_error())?
        .as_millis();
    let log = Arc::new(Mutex::new((
        fs::File::create(util::join(&logs, &format!("game-{stamp}.log"))?)
            .map_err(|_| io_error())?,
        0usize,
    )));
    let redactor = Arc::new(Redactor::new(&session.token)?);
    let callback = Arc::new(on_line);
    let mut cmd = command(&java);
    cmd.arg(format!("@{}", argfile.display()))
        .args(jvm)
        .arg(format!("-Xmx{}M", settings.memory_mib));
    if install.main_class == "net.fabricmc.loader.impl.launch.knot.KnotClient" {
        cmd.arg(format!(
            "-Dfabric.addMods={}",
            util::join(root, "mods")?.display()
        ));
    }
    cmd.arg(&install.main_class)
        .args(game_args)
        .current_dir(&settings.instance_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    // Never Debug/print cmd: its game arguments contain the session token.
    let mut child=cmd.spawn().map_err(|_|error(ErrorCode::Process,"Java could not be started. Check the runtime, Windows application policy and installation."))?;
    let pid = child.id();
    if let Err(error) = crate::game_guard::record(&settings.instance_dir, pid) {
        let _ = child.kill();
        let _ = child.wait();
        return Err(error);
    }
    on_started(pid);
    let stdout = child.stdout.take().ok_or_else(io_error)?;
    let stderr = child.stderr.take().ok_or_else(io_error)?;
    let handles = [
        stream(
            stdout,
            "stdout",
            log.clone(),
            redactor.clone(),
            callback.clone(),
        ),
        stream(stderr, "stderr", log, redactor, callback),
    ];
    let status = child.wait().map_err(|_| {
        error(
            ErrorCode::Process,
            "Couldn't obtain the game's exit status.",
        )
    })?;
    for handle in handles {
        let _ = handle.join();
    }
    crate::game_guard::clear(&settings.instance_dir, pid)?;
    Ok(status.code())
}
fn stream<R: Read + Send + 'static, F: Fn(String, Option<EngineState>) + Send + Sync + 'static>(
    input: R,
    label: &'static str,
    log: Arc<Mutex<(fs::File, usize)>>,
    redactor: Arc<Redactor>,
    callback: Arc<F>,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let mut reader = BufReader::new(input);
        let mut line = Vec::new();
        let mut oversized = false;
        while let Ok(available) = reader.fill_buf() {
            if available.is_empty() {
                if !line.is_empty() || oversized {
                    emit_line(&line, oversized, label, &log, &redactor, &*callback);
                }
                break;
            }
            let take = available
                .iter()
                .position(|b| *b == b'\n')
                .map(|i| i + 1)
                .unwrap_or(available.len());
            let newline = available[take - 1] == b'\n';
            if !oversized {
                if line.len() + take <= 65536 {
                    line.extend_from_slice(&available[..take]);
                } else {
                    oversized = true;
                    line.clear();
                }
            }
            reader.consume(take);
            if newline {
                emit_line(&line, oversized, label, &log, &redactor, &*callback);
                line.clear();
                oversized = false;
            }
        }
    })
}
fn emit_line(
    line: &[u8],
    oversized: bool,
    label: &str,
    log: &Mutex<(fs::File, usize)>,
    redactor: &Redactor,
    callback: &impl Fn(String, Option<EngineState>),
) {
    let text = if oversized {
        "[oversize output omitted]".to_owned()
    } else {
        redactor.line(String::from_utf8_lossy(line).trim_end())
    };
    let text = format!("[{label}] {text}");
    if let Ok(mut log) = log.lock()
        && log.1 + text.len() < MAX_GAME_LOG_BYTES
    {
        let _ = writeln!(log.0, "{text}");
        log.1 += text.len() + 1;
    }
    let ready = if label == "stdout" && !oversized {
        std::str::from_utf8(line)
            .ok()
            .and_then(crate::readiness::parse)
    } else {
        None
    };
    callback(text, ready);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[cfg(windows)]
    fn real_java21_process_nonzero_exit_and_redacted_logs() {
        let java_home = std::env::var_os("JAVA_HOME_21_X64")
            .expect("Set JAVA_HOME_21_X64 to an actual Java 21 JDK for this test");
        let jdk = std::path::PathBuf::from(java_home);
        let dir = tempfile::Builder::new()
            .prefix("ooze paths with spaces ")
            .tempdir()
            .unwrap();
        let root = dir.path().join("generation");
        fs::create_dir(&root).unwrap();
        fs::create_dir(root.join("natives")).unwrap();
        fs::create_dir(root.join("mods")).unwrap();
        let source = root.join("LauncherFixture.java");
        fs::write(&source,r#"public class LauncherFixture { public static void main(String[] args) throws Exception { Thread.sleep(500); System.out.println("--accessToken " + args[0]); System.err.println("Authorization: Bearer " + args[0]); System.exit(7); } }"#).unwrap();
        assert!(
            command(&jdk.join("bin/javac.exe"))
                .arg(&source)
                .status()
                .unwrap()
                .success()
        );
        let instance = dir.path().join("game");
        fs::create_dir(&instance).unwrap();
        let settings = Settings {
            instance_dir: instance,
            java_path: Some(jdk.join("bin/java.exe")),
            memory_mib: 1024,
        };
        let prepared = Prepared {
            root: root.clone(),
            installation: metadata::Installation {
                schema: 1,
                release_sha256: "test-only".into(),
                directory: "test-only".into(),
                files: vec![],
                classpath: vec!["classes".into()],
                main_class: "LauncherFixture".into(),
                jvm_args: vec!["-cp".into(), "${classpath}".into()],
                game_args: vec!["${auth_access_token}".into()],
                asset_index: "29".into(),
                managed_java: None,
                version_name: "test-only".into(),
            },
        };
        // A private test fixture exercises process/redaction only, never product authentication.
        fs::create_dir(root.join("classes")).unwrap();
        fs::copy(
            root.join("LauncherFixture.class"),
            root.join("classes/LauncherFixture.class"),
        )
        .unwrap();
        for _ in 0..2 {
            let session = Session {
                token: Zeroizing::new("private-test-token".into()),
                account: crate::api::AccountSummary {
                    username: "Fixture".into(),
                    uuid: "0".repeat(32),
                },
                xuid: "".into(),
            };
            assert_eq!(
                launch(
                    &prepared,
                    &settings,
                    session,
                    "test-only",
                    dir.path(),
                    |pid| assert!(pid > 0),
                    |line, _| assert!(!line.contains("private-test-token"))
                )
                .unwrap(),
                Some(7)
            );
            assert!(
                !settings
                    .instance_dir
                    .join(".ooze-running-game.json")
                    .exists()
            );
        }
        for file in fs::read_dir(dir.path().join("logs")).unwrap() {
            assert!(
                !fs::read_to_string(file.unwrap().path())
                    .unwrap()
                    .contains("private-test-token")
            );
        }
        let args = fs::read_to_string(root.join("classpath.args")).unwrap();
        assert!(!args.contains("private-test-token"));
    }
    #[test]
    fn java_21_only_and_argfile_paths() {
        assert!(validate_properties(true, "java.version = 21.0.12\nos.arch = amd64").is_ok());
        assert!(validate_properties(true, "java.version = 25.0.4\nos.arch = amd64").is_err());
        assert!(validate_properties(true, "java.version = 21.0.12\nos.arch = x86").is_err());
        assert_eq!(
            quote_argfile("C:\\path with spaces\\lib.jar"),
            "\"C:\\\\path with spaces\\\\lib.jar\""
        );
    }
    #[test]
    fn credentials_redacted_even_in_crash_output() {
        let r = Redactor::new("real-secret-session").unwrap();
        for line in [
            "--accessToken real-secret-session",
            "Authorization: Bearer abc123",
            "refresh_token = hidden",
            "raw real-secret-session",
        ] {
            let clean = r.line(line);
            assert!(!clean.contains("real-secret-session"));
            assert!(!clean.contains("hidden"));
            assert!(!clean.contains("abc123"));
        }
    }
}
