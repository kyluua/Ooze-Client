use super::{
    model::*,
    modrinth::{self, Modrinth, Resolution},
    store::Store,
};
use crate::{
    api::ErrorCode,
    util::{self, Result, error, io_error, metadata_error},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha512};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};

pub(super) struct Plan {
    pub id: Id,
    pub instance: Id,
    pub revision: u64,
    pub created: std::time::Instant,
    pub resolution: Resolution,
    pub world: Option<String>,
}
impl Plan {
    pub fn summary(&self) -> PlanSummary {
        PlanSummary {
            id: self.id.clone(),
            instance: self.instance.clone(),
            revision: self.revision,
            files: self
                .resolution
                .files
                .iter()
                .map(|f| PlanFile {
                    project: f.project.clone(),
                    version: f.version.clone(),
                    kind: f.kind,
                    filename: f.file.filename.clone(),
                    size: f.file.size,
                })
                .collect(),
            optional: self.resolution.optional.clone(),
            embedded: self.resolution.embedded.clone(),
            expires_seconds: 600,
        }
    }
}
pub(super) fn hash(path: &Path, check: &dyn Fn() -> Result<()>) -> Result<String> {
    util::reject_links(path)?;
    let mut file = fs::File::open(path).map_err(|_| io_error())?;
    let mut hash = Sha512::new();
    let mut buffer = [0u8; 65536];
    loop {
        check()?;
        let n = file.read(&mut buffer).map_err(|_| io_error())?;
        if n == 0 {
            break;
        }
        hash.update(&buffer[..n]);
    }
    Ok(hex::encode(hash.finalize()))
}
pub(super) fn list_worlds(game: &Path, check: &dyn Fn() -> Result<()>) -> Result<Vec<LocalWorld>> {
    let saves = util::join(game, "saves")?;
    if !saves.exists() {
        return Ok(Vec::new());
    }
    let mut worlds = Vec::new();
    let mut scanned = 0;
    for entry in fs::read_dir(saves).map_err(|_| io_error())? {
        check()?;
        scanned += 1;
        if scanned > 512 {
            return Err(invalid("World listing exceeds its 512-entry limit."));
        }
        let entry = entry.map_err(|_| io_error())?;
        util::reject_links(&entry.path())?;
        if !entry.file_type().map_err(|_| io_error())?.is_dir() {
            continue;
        }
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| invalid("World directory name is not valid Unicode."))?;
        util::relative(&name)?;
        if name.len() > 96 {
            continue;
        }
        let level = util::join(&entry.path(), "level.dat")?;
        if !level.is_file() {
            continue;
        }
        let key = format!(
            "world:{}",
            &hex::encode(sha2::Sha256::digest(name.as_bytes()))[..32]
        );
        worlds.push(LocalWorld { key, name });
    }
    worlds.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(worlds)
}
fn world_directory(game: &Path, world: &str) -> Result<PathBuf> {
    let world = if world.starts_with("world:") {
        list_worlds(game, &|| Ok(()))?
            .into_iter()
            .find(|w| w.key == world)
            .ok_or_else(|| invalid("The selected world no longer exists; list worlds again."))?
            .name
    } else {
        world.to_owned()
    };
    util::relative(&world)?;
    if world.contains('/') || world.len() > 96 {
        return Err(invalid(
            "Select a local world directly inside this instance's saves directory.",
        ));
    }
    let path = util::join(game, &format!("saves/{world}"))?;
    let level = util::join(&path, "level.dat")?;
    if !level.is_file() {
        return Err(invalid(
            "Select an existing local world containing level.dat.",
        ));
    }
    Ok(path)
}
pub(super) fn destination(
    game: &Path,
    kind: ContentKind,
    filename: &str,
    world: Option<&str>,
) -> Result<String> {
    util::relative(filename)?;
    if filename.contains('/') {
        return Err(metadata_error());
    }
    if kind == ContentKind::Datapack {
        let world =
            world.ok_or_else(|| invalid("Select a local world before installing a data pack."))?;
        let directory = world_directory(game, world)?;
        let relative = directory
            .strip_prefix(game)
            .map_err(|_| metadata_error())?
            .to_str()
            .ok_or_else(metadata_error)?
            .replace('\\', "/");
        Ok(format!("{relative}/datapacks/{filename}"))
    } else {
        Ok(format!("{}/{filename}", kind.directory()))
    }
}
fn download(
    http: &Modrinth,
    file: &modrinth::VersionFile,
    path: &Path,
    check: &dyn Fn() -> Result<()>,
    report: Reporter<'_>,
) -> Result<()> {
    modrinth::validate_file(file)?;
    check()?;
    let response = http
        .client
        .get(modrinth::cdn_url(&file.url)?)
        .send()
        .map_err(|_| error(ErrorCode::Network, "Modrinth content download failed."))?;
    if !response.status().is_success() || response.content_length().is_some_and(|n| n != file.size)
    {
        return Err(error(
            ErrorCode::Network,
            "Modrinth content download returned an invalid status or length.",
        ));
    }
    let mut input = response.take(file.size + 1);
    let mut output = fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)
        .map_err(|_| io_error())?;
    let mut hash = Sha512::new();
    let mut buffer = [0u8; 65536];
    let mut size = 0;
    let mut last_report = std::time::Instant::now();
    report(
        ProgressStage::Downloading,
        0,
        Some(file.size),
        Some(&file.filename),
    )?;
    loop {
        check()?;
        let n = input.read(&mut buffer).map_err(|_| {
            error(
                ErrorCode::Network,
                "Content download was interrupted; retry the plan.",
            )
        })?;
        if n == 0 {
            break;
        }
        size += n as u64;
        if size > file.size {
            return Err(error(
                ErrorCode::Integrity,
                "Content exceeds its declared size.",
            ));
        }
        hash.update(&buffer[..n]);
        output.write_all(&buffer[..n]).map_err(|_| io_error())?;
        if last_report.elapsed() >= std::time::Duration::from_millis(50) || size == file.size {
            report(
                ProgressStage::Downloading,
                size,
                Some(file.size),
                Some(&file.filename),
            )?;
            last_report = std::time::Instant::now();
        }
    }
    output.sync_all().map_err(|_| io_error())?;
    if size != file.size || !hex::encode(hash.finalize()).eq_ignore_ascii_case(&file.hashes.sha512)
    {
        return Err(error(
            ErrorCode::Integrity,
            "Downloaded content does not match its declared hash/size.",
        ));
    }
    drop(output);
    report(
        ProgressStage::Verifying,
        file.size,
        Some(file.size),
        Some(&file.filename),
    )?;
    validate_archive(path, check)
}
fn context(stage: &str, mut e: crate::api::LauncherError) -> crate::api::LauncherError {
    if e.code == ErrorCode::Io && !e.message.starts_with("Content operation") {
        e.message =
            format!("Content operation failed while {stage}. Check disk space and permissions.");
    }
    e
}
fn io_context(stage: &str, cause: &std::io::Error) -> crate::api::LauncherError {
    error(
        ErrorCode::Io,
        format!(
            "Content operation failed while {stage} (OS code {}).",
            cause.raw_os_error().unwrap_or(0)
        ),
    )
}
pub(super) fn validate_archive(path: &Path, check: &dyn Fn() -> Result<()>) -> Result<()> {
    let input = fs::File::open(path).map_err(|_| io_error())?;
    let mut zip = zip::ZipArchive::new(input).map_err(|_| metadata_error())?;
    if zip.len() > 20000 {
        return Err(metadata_error());
    }
    let mut seen = BTreeSet::new();
    let mut total = 0u64;
    let mut buffer = [0u8; 65536];
    for i in 0..zip.len() {
        check()?;
        let mut entry = zip.by_index(i).map_err(|_| metadata_error())?;
        let path = entry.name().trim_end_matches('/');
        util::relative(path)?;
        if !seen.insert(path.to_ascii_lowercase())
            || entry
                .unix_mode()
                .is_some_and(|m| m & 0o170000 != 0 && !matches!(m & 0o170000, 0o100000 | 0o040000))
            || entry.size() > 512 * 1024 * 1024
        {
            return Err(metadata_error());
        }
        total = total.checked_add(entry.size()).ok_or_else(metadata_error)?;
        if total > 2 * 1024 * 1024 * 1024 {
            return Err(metadata_error());
        }
        let mut read = 0u64;
        loop {
            check()?;
            let n = entry.read(&mut buffer).map_err(|_| metadata_error())?;
            if n == 0 {
                break;
            }
            read += n as u64;
            if read > 512 * 1024 * 1024 {
                return Err(metadata_error());
            }
        }
        if read != entry.size() {
            return Err(metadata_error());
        }
    }
    Ok(())
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Move {
    path: String,
    backup: bool,
    new_file: Option<String>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Journal {
    schema: u32,
    before_revision: u64,
    after: Instance,
    moves: Vec<Move>,
}
fn txn_path(store: &Store, id: &Id) -> Result<PathBuf> {
    util::join(&store.game(id)?, ".ooze-content-transaction")
}

pub(super) fn recover(store: &Store, id: &Id) -> Result<()> {
    let dir = txn_path(store, id)?;
    if !dir.exists() {
        return Ok(());
    }
    util::reject_links(&dir)?;
    let journal: Journal = util::read_json(&util::join(&dir, "journal.json")?)?;
    journal.after.validate()?;
    if journal.schema != SCHEMA
        || &journal.after.id != id
        || journal.before_revision.checked_add(1) != Some(journal.after.revision)
        || journal.moves.len() > 1024
    {
        return Err(metadata_error());
    }
    let current = store.instance(id)?;
    if current.revision == journal.after.revision
        && serde_json::to_vec(&current).map_err(|_| metadata_error())?
            != serde_json::to_vec(&journal.after).map_err(|_| metadata_error())?
    {
        return Err(error(
            ErrorCode::Integrity,
            "Committed transaction manifest differs; preserve it for recovery.",
        ));
    }
    for (index, m) in journal.moves.iter().enumerate() {
        util::relative(&m.path)?;
        if (current.revision != journal.after.revision
            && !current
                .content
                .iter()
                .chain(journal.after.content.iter())
                .any(|r| r.path == m.path))
            || m.new_file
                .as_ref()
                .is_some_and(|n| n != &format!("new-{index}"))
        {
            return Err(error(
                ErrorCode::Integrity,
                "Invalid transaction path; player files were preserved.",
            ));
        }
    }
    if current.revision == journal.after.revision {
        // The manifest's atomic replacement is the commit point. Preserve backups.
        let history = util::join(&store.container(id)?, "history")?;
        util::mkdir(&history)?;
        let destination = util::join(&history, &Id::new().0)?;
        fs::rename(&dir, destination)
            .map_err(|e| io_context("retaining committed recovery history", &e))?;
        return Ok(());
    }
    if current.revision != journal.before_revision {
        return Err(error(
            ErrorCode::Integrity,
            "An interrupted content transaction needs explicit recovery; files were preserved.",
        ));
    }
    for (index, m) in journal.moves.iter().enumerate().rev() {
        util::relative(&m.path)?;
        let target = util::join(&store.game(id)?, &m.path)?;
        let backup = util::join(&dir, &format!("old-{index}"))?;
        if backup.is_file() {
            if target.exists() {
                fs::remove_file(&target).map_err(|_| io_error())?;
            }
            util::mkdir(target.parent().ok_or_else(io_error)?)?;
            fs::rename(backup, target).map_err(|_| io_error())?;
        } else if !m.backup && m.new_file.is_some() && target.is_file() {
            fs::remove_file(target).map_err(|_| io_error())?;
        }
    }
    // Every path is confined to this exact transaction directory; no player tree is deleted.
    util::reject_links(&dir)?;
    fs::remove_dir_all(&dir).map_err(|_| io_error())?;
    Ok(())
}

#[cfg(test)]
fn commit(
    store: &Store,
    before: &Instance,
    after: Instance,
    changes: BTreeMap<String, Option<PathBuf>>,
    check: &dyn Fn() -> Result<()>,
) -> Result<()> {
    commit_with_repair(
        store,
        before,
        after,
        changes,
        check,
        false,
        &|_, _, _, _| Ok(()),
    )
}
fn commit_with_repair(
    store: &Store,
    before: &Instance,
    mut after: Instance,
    mut changes: BTreeMap<String, Option<PathBuf>>,
    check: &dyn Fn() -> Result<()>,
    repair: bool,
    report: Reporter<'_>,
) -> Result<()> {
    check()?;
    let game = store.game(&before.id)?;
    let dir = txn_path(store, &before.id)?;
    if dir.exists() {
        return Err(error(
            ErrorCode::Busy,
            "Recover the prior content operation first.",
        ));
    }
    let stage = tempfile::Builder::new()
        .prefix(".ooze-staging-")
        .tempdir_in(&game)
        .map_err(|_| io_error())?;
    let mut moves = Vec::new();
    for (index, (relative, source)) in changes.iter_mut().enumerate() {
        check()?;
        let dest = util::join(&game, relative)?;
        util::mkdir(dest.parent().ok_or_else(io_error)?)?;
        let old = before.content.iter().find(|r| r.path == *relative);
        if dest.exists() && old.is_none() {
            return Err(error(
                ErrorCode::Integrity,
                "An unrelated file already uses this content path; it was preserved.",
            ));
        }
        if let Some(receipt) = old
            && dest.exists()
            && hash(&dest, check)? != receipt.sha512
            && !repair
        {
            return Err(error(
                ErrorCode::Integrity,
                "Installed content was modified; preserve it and choose explicit repair.",
            ));
        }
        let new_file = if let Some(source) = source {
            let name = format!("new-{index}");
            fs::copy(source, util::join(stage.path(), &name)?).map_err(|_| io_error())?;
            fs::OpenOptions::new()
                .write(true)
                .open(util::join(stage.path(), &name)?)
                .map_err(|_| io_error())?
                .sync_all()
                .map_err(|_| io_error())?;
            Some(name)
        } else {
            None
        };
        moves.push(Move {
            path: relative.clone(),
            backup: dest.is_file(),
            new_file,
        });
    }
    after.revision = before.revision.checked_add(1).ok_or_else(metadata_error)?;
    after.validate()?;
    util::atomic_json(
        &util::join(stage.path(), "journal.json")?,
        &Journal {
            schema: SCHEMA,
            before_revision: before.revision,
            after: after.clone(),
            moves,
        },
    )?;
    check()?;
    report(ProgressStage::Committing, 0, None, None)?;
    let kept = stage.keep();
    fs::rename(&kept, &dir).map_err(|e| io_context("activating a staged transaction", &e))?;
    let work = (|| {
        let journal: Journal = util::read_json(&util::join(&dir, "journal.json")?)?;
        for (index, m) in journal.moves.iter().enumerate() {
            check()?;
            let target = util::join(&game, &m.path)?;
            if m.backup {
                fs::rename(&target, util::join(&dir, &format!("old-{index}"))?)
                    .map_err(|e| io_context("backing up registered content", &e))?;
            }
            if let Some(new) = &m.new_file {
                fs::rename(util::join(&dir, new)?, target)
                    .map_err(|e| io_context("activating verified content", &e))?;
            }
        }
        check()?;
        util::atomic_json(
            &util::join(&store.container(&before.id)?, "instance.json")?,
            &after,
        )
        .map_err(|e| context("saving the instance receipt", e))
    })();
    let recovery = recover(store, &before.id);
    work?;
    recovery
}

pub(super) fn apply(
    store: &Store,
    http: &Modrinth,
    plan: &Plan,
    check: &dyn Fn() -> Result<()>,
    report: Reporter<'_>,
) -> Result<()> {
    let (before, _lock) = store.claim(&plan.instance)?;
    if before.mode == Mode::Managed {
        return Err(invalid(
            "This release approves no extra content in Managed mode. Explicitly convert this instance to Custom first.",
        ));
    }
    if before.revision != plan.revision
        || plan.created.elapsed() > std::time::Duration::from_secs(600)
    {
        return Err(invalid(
            "This content plan expired or the instance changed. Resolve it again.",
        ));
    }
    let game = store.game(&before.id)?;
    let download_dir = tempfile::Builder::new()
        .prefix(".ooze-download-")
        .tempdir_in(&game)
        .map_err(|_| io_error())?;
    let mut after = before.clone();
    let mut changes = BTreeMap::new();
    let mut paths = BTreeSet::new();
    let selected: BTreeMap<_, _> = plan
        .resolution
        .files
        .iter()
        .map(|f| (f.project.clone(), f.version.clone()))
        .collect();
    for existing in &before.content {
        if !selected.contains_key(&existing.project)
            && existing
                .required_versions
                .iter()
                .any(|(p, v)| selected.get(p).is_some_and(|new| new != v))
        {
            return Err(invalid(
                "Updating this dependency would break another installed project's pinned requirement.",
            ));
        }
    }
    for (i, resolved) in plan.resolution.files.iter().enumerate() {
        check()?;
        let file = &resolved.file;
        let path = destination(&game, resolved.kind, &file.filename, plan.world.as_deref())?;
        if !paths.insert(path.to_ascii_lowercase()) {
            return Err(invalid("Multiple content files use the same destination."));
        }
        let staged = util::join(download_dir.path(), &i.to_string())?;
        download(http, file, &staged, check, report)
            .map_err(|e| context("downloading and validating a file", e))?;
        let world_prefix = if resolved.kind == ContentKind::Datapack {
            path.rsplit_once('/').map(|(p, _)| p.to_owned())
        } else {
            None
        };
        let old: Vec<_> = after
            .content
            .iter()
            .filter(|r| {
                r.project == resolved.project
                    && (world_prefix
                        .as_ref()
                        .is_none_or(|p| r.path.starts_with(&format!("{p}/"))))
            })
            .map(|r| r.path.clone())
            .collect();
        for old in old {
            changes.insert(old.clone(), None);
            after.content.retain(|r| r.path != old);
        }
        changes.insert(path.clone(), Some(staged));
        after.content.push(ContentReceipt {
            project: resolved.project.clone(),
            version: resolved.version.clone(),
            kind: resolved.kind,
            path,
            size: file.size,
            sha512: file.hashes.sha512.to_ascii_lowercase(),
            enabled: true,
            required_projects: resolved.required.clone(),
            required_versions: resolved
                .required
                .iter()
                .filter_map(|p| selected.get(p).map(|v| (p.clone(), v.clone())))
                .collect(),
        });
    }
    commit_with_repair(store, &before, after, changes, check, false, report)
        .map_err(|e| context("committing staged files or retaining recovery history", e))
}

#[cfg(test)]
fn change(
    store: &Store,
    id: &Id,
    project: &str,
    enabled: Option<bool>,
    check: &dyn Fn() -> Result<()>,
) -> Result<()> {
    change_reported(store, id, project, enabled, check, &|_, _, _, _| Ok(()))
}
pub(super) fn change_reported(
    store: &Store,
    id: &Id,
    project: &str,
    enabled: Option<bool>,
    check: &dyn Fn() -> Result<()>,
    report: Reporter<'_>,
) -> Result<()> {
    modrinth::identifier(project)?;
    let (before, _lock) = store.claim(id)?;
    if before.mode == Mode::Managed {
        return Err(invalid(
            "Convert this managed instance to Custom before editing extra content.",
        ));
    }
    if enabled != Some(true)
        && before.content.iter().any(|r| {
            r.enabled && r.project != project && r.required_projects.iter().any(|p| p == project)
        })
    {
        return Err(invalid(
            "Other enabled content requires this project. Disable or remove those dependents first.",
        ));
    }
    let mut after = before.clone();
    let mut changes = BTreeMap::new();
    let game = store.game(id)?;
    let selected: Vec<_> = before
        .content
        .iter()
        .filter(|r| r.project == project)
        .cloned()
        .collect();
    if selected.is_empty() {
        return Err(invalid("This project is not installed."));
    }
    for receipt in selected {
        if let Some(enable) = enabled {
            if receipt.enabled == enable {
                continue;
            }
            if enable
                && receipt.required_projects.iter().any(|p| {
                    !(before.target.ooze && p == "P7dR8mSH")
                        && !before.content.iter().any(|r| r.enabled && &r.project == p)
                })
            {
                return Err(invalid("Enable required dependencies first."));
            }
            let new_path = if enable {
                receipt
                    .path
                    .strip_suffix(".disabled")
                    .ok_or_else(metadata_error)?
                    .to_owned()
            } else {
                format!("{}.disabled", receipt.path)
            };
            changes.insert(receipt.path.clone(), None);
            changes.insert(new_path.clone(), Some(util::join(&game, &receipt.path)?));
            if let Some(r) = after.content.iter_mut().find(|r| r.path == receipt.path) {
                r.path = new_path;
                r.enabled = enable;
            }
        } else {
            changes.insert(receipt.path.clone(), None);
            after.content.retain(|r| r.path != receipt.path);
        }
    }
    commit_with_repair(store, &before, after, changes, check, false, report)
}

pub(super) fn repair(
    store: &Store,
    http: &Modrinth,
    id: &Id,
    project: &str,
    check: &dyn Fn() -> Result<()>,
    report: Reporter<'_>,
) -> Result<()> {
    use modrinth::Source;
    modrinth::identifier(project)?;
    let (before, _lock) = store.claim(id)?;
    if before.mode == Mode::Managed {
        return Err(invalid(
            "Managed baseline repair uses the launcher's Repair operation.",
        ));
    }
    let selected: Vec<_> = before
        .content
        .iter()
        .filter(|r| r.project == project)
        .collect();
    if selected.is_empty() {
        return Err(invalid("This project is not installed."));
    }
    let game = store.game(id)?;
    let stage = tempfile::Builder::new()
        .prefix(".ooze-repair-")
        .tempdir_in(&game)
        .map_err(|_| io_error())?;
    let mut changes = BTreeMap::new();
    for (i, receipt) in selected.iter().enumerate() {
        check()?;
        let version = http.version(&receipt.version, check)?;
        if version.summary.project_id != receipt.project {
            return Err(metadata_error());
        }
        let source=version.files.iter().find(|f|f.size==receipt.size && f.hashes.sha512.eq_ignore_ascii_case(&receipt.sha512)).ok_or_else(||error(ErrorCode::Integrity,"Provider metadata no longer matches the recorded content hash; existing files were preserved."))?;
        let downloaded = util::join(stage.path(), &i.to_string())?;
        download(http, source, &downloaded, check, report)?;
        changes.insert(receipt.path.clone(), Some(downloaded));
    }
    commit_with_repair(store, &before, before.clone(), changes, check, true, report)
}

pub(super) fn verify(
    instance: &Instance,
    game: &Path,
    check: &dyn Fn() -> Result<()>,
) -> Result<IntegrityReport> {
    let mut result = IntegrityReport {
        managed: instance.mode == Mode::Managed,
        baseline_verified: false,
        verified_files: 0,
        issues: Vec::new(),
    };
    for receipt in &instance.content {
        check()?;
        let path = util::join(game, &receipt.path)?;
        if !path.is_file()
            || fs::metadata(&path).map_err(|_| io_error())?.len() != receipt.size
            || hash(&path, check)? != receipt.sha512
        {
            result.issues.push(format!(
                "Registered content changed or is missing: {}",
                receipt.path
            ));
        } else {
            result.verified_files += 1;
        }
    }
    if instance.mode == Mode::Managed {
        if !instance.content.is_empty() {
            result.issues.push(
                "This release's managed policy approves only its pinned baseline content.".into(),
            );
        }
        for directory in ["mods", "resourcepacks", "shaderpacks"] {
            let path = util::join(game, directory)?;
            if path.exists() {
                for entry in fs::read_dir(path).map_err(|_| io_error())? {
                    check()?;
                    let entry = entry.map_err(|_| io_error())?;
                    util::reject_links(&entry.path())?;
                    result.issues.push(format!(
                        "Unapproved entry in {directory}; convert to Custom or move it yourself."
                    ));
                    if result.issues.len() > 512 {
                        return Err(invalid("Too many unapproved content entries."));
                    }
                }
            }
        }
        let saves = util::join(game, "saves")?;
        if saves.exists() {
            for world in fs::read_dir(saves).map_err(|_| io_error())? {
                check()?;
                let world = world.map_err(|_| io_error())?;
                util::reject_links(&world.path())?;
                let packs = util::join(&world.path(), "datapacks")?;
                if packs.is_dir()
                    && fs::read_dir(packs)
                        .map_err(|_| io_error())?
                        .next()
                        .is_some()
                {
                    result.issues.push("An installed local-world data pack is not approved by this managed policy.".into());
                }
            }
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn opaque_world_keys_resolve_only_existing_local_worlds() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("saves/world")).unwrap();
        fs::write(dir.path().join("saves/world/level.dat"), b"world").unwrap();
        let worlds = list_worlds(dir.path(), &|| Ok(())).unwrap();
        assert_eq!(worlds.len(), 1);
        assert!(worlds[0].key.starts_with("world:"));
        assert_eq!(
            destination(
                dir.path(),
                ContentKind::Datapack,
                "pack.zip",
                Some(&worlds[0].key)
            )
            .unwrap(),
            "saves/world/datapacks/pack.zip"
        );
        assert!(
            destination(
                dir.path(),
                ContentKind::Datapack,
                "pack.zip",
                Some("world:unknown")
            )
            .is_err()
        );
    }
    #[test]
    fn successful_commit_and_duplicate_sync_writable_handles_on_windows() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(dir.path()).unwrap();
        let id = store
            .create(
                "Install".into(),
                Target::fabric(),
                Preset::Minimal,
                Mode::Custom,
            )
            .unwrap();
        let before = store.instance(&id).unwrap();
        let source = dir.path().join("source.jar");
        fs::write(&source, b"verified").unwrap();
        let mut after = before.clone();
        after.content.push(ContentReceipt {
            project: "A".into(),
            version: "a".into(),
            kind: ContentKind::Mod,
            path: "mods/a.jar".into(),
            size: 8,
            sha512: hash(&source, &|| Ok(())).unwrap(),
            enabled: true,
            required_projects: Vec::new(),
            required_versions: BTreeMap::new(),
        });
        commit(
            &store,
            &before,
            after,
            BTreeMap::from([("mods/a.jar".into(), Some(source))]),
            &|| Ok(()),
        )
        .unwrap();
        assert_eq!(store.instance(&id).unwrap().content.len(), 1);
        let copy = store.duplicate(&id, "Copy".into(), &|| Ok(())).unwrap();
        assert_eq!(
            fs::read(store.game(&copy).unwrap().join("mods/a.jar")).unwrap(),
            b"verified"
        );
        change(&store, &id, "A", Some(false), &|| Ok(())).unwrap();
        assert!(
            store
                .game(&id)
                .unwrap()
                .join("mods/a.jar.disabled")
                .is_file()
        );
        change(&store, &id, "A", Some(true), &|| Ok(())).unwrap();
        assert!(store.game(&id).unwrap().join("mods/a.jar").is_file());
        change(&store, &id, "A", None, &|| Ok(())).unwrap();
        assert!(!store.game(&id).unwrap().join("mods/a.jar").exists());
        assert!(store.game(&copy).unwrap().join("mods/a.jar").exists());
    }
    #[test]
    fn interrupted_content_replacement_rolls_back_and_rejects_unrelated_paths() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(dir.path()).unwrap();
        let id = store
            .create(
                "Recovery".into(),
                Target::fabric(),
                Preset::Minimal,
                Mode::Custom,
            )
            .unwrap();
        let game = store.game(&id).unwrap();
        fs::create_dir(game.join("mods")).unwrap();
        let target = game.join("mods/a.jar");
        fs::write(&target, b"original").unwrap();
        let mut before = store.instance(&id).unwrap();
        before.content.push(ContentReceipt {
            project: "A".into(),
            version: "a".into(),
            kind: ContentKind::Mod,
            path: "mods/a.jar".into(),
            size: 8,
            sha512: hash(&target, &|| Ok(())).unwrap(),
            enabled: true,
            required_projects: Vec::new(),
            required_versions: BTreeMap::new(),
        });
        store.save(&mut before).unwrap();
        let txn = txn_path(&store, &id).unwrap();
        fs::create_dir(&txn).unwrap();
        fs::rename(&target, txn.join("old-0")).unwrap();
        fs::write(&target, b"new").unwrap();
        let mut after = before.clone();
        after.revision += 1;
        after.content[0].size = 3;
        after.content[0].sha512 = hash(&target, &|| Ok(())).unwrap();
        util::atomic_json(
            &txn.join("journal.json"),
            &Journal {
                schema: SCHEMA,
                before_revision: before.revision,
                after,
                moves: vec![Move {
                    path: "mods/a.jar".into(),
                    backup: true,
                    new_file: Some("new-0".into()),
                }],
            },
        )
        .unwrap();
        recover(&store, &id).unwrap();
        assert_eq!(fs::read(&target).unwrap(), b"original");
        assert_eq!(store.instance(&id).unwrap().revision, before.revision);
        fs::create_dir(&txn).unwrap();
        let options = game.join("options.txt");
        fs::write(&options, b"preserve").unwrap();
        let mut after = before.clone();
        after.revision += 1;
        util::atomic_json(
            &txn.join("journal.json"),
            &Journal {
                schema: SCHEMA,
                before_revision: before.revision,
                after,
                moves: vec![Move {
                    path: "options.txt".into(),
                    backup: false,
                    new_file: Some("new-0".into()),
                }],
            },
        )
        .unwrap();
        assert!(recover(&store, &id).is_err());
        assert_eq!(fs::read(options).unwrap(), b"preserve");
    }
    #[test]
    fn archives_reject_traversal_and_duplicate_case() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bad.zip");
        for names in [vec!["../escape"], vec!["a.txt", "A.txt"]] {
            let mut zip = zip::ZipWriter::new(fs::File::create(&path).unwrap());
            for name in names {
                zip.start_file(name, zip::write::SimpleFileOptions::default())
                    .unwrap();
                zip.write_all(b"x").unwrap();
            }
            zip.finish().unwrap();
            assert!(validate_archive(&path, &|| Ok(())).is_err());
        }
    }
    #[test]
    fn managed_mode_detects_extra_mod_without_deleting_it() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(dir.path()).unwrap();
        let id = store
            .create(
                "Managed".into(),
                Target::fabric(),
                Preset::Pvp,
                Mode::Managed,
            )
            .unwrap();
        let game = store.game(&id).unwrap();
        fs::create_dir(game.join("mods")).unwrap();
        let extra = game.join("mods/unknown.jar");
        fs::write(&extra, b"unexpected").unwrap();
        let report = verify(&store.instance(&id).unwrap(), &game, &|| Ok(())).unwrap();
        assert!(!report.issues.is_empty());
        assert!(extra.is_file());
    }
    #[test]
    fn untracked_files_survive_failed_install_transaction() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(dir.path()).unwrap();
        let id = store
            .create(
                "Custom".into(),
                Target::fabric(),
                Preset::Minimal,
                Mode::Custom,
            )
            .unwrap();
        let before = store.instance(&id).unwrap();
        let game = store.game(&id).unwrap();
        fs::create_dir(game.join("mods")).unwrap();
        fs::write(game.join("mods/user.jar"), b"user").unwrap();
        let source = dir.path().join("new");
        fs::write(&source, b"new").unwrap();
        let changes = BTreeMap::from([("mods/user.jar".into(), Some(source))]);
        assert!(commit(&store, &before, before.clone(), changes, &|| Ok(())).is_err());
        assert_eq!(fs::read(game.join("mods/user.jar")).unwrap(), b"user");
    }
    #[test]
    fn datapacks_require_an_existing_explicit_world() {
        let dir = tempfile::tempdir().unwrap();
        assert!(destination(dir.path(), ContentKind::Datapack, "pack.zip", None).is_err());
        assert!(
            destination(
                dir.path(),
                ContentKind::Datapack,
                "pack.zip",
                Some("../outside")
            )
            .is_err()
        );
        fs::create_dir_all(dir.path().join("saves/world")).unwrap();
        fs::write(dir.path().join("saves/world/level.dat"), b"world").unwrap();
        assert_eq!(
            destination(dir.path(), ContentKind::Datapack, "pack.zip", Some("world")).unwrap(),
            "saves/world/datapacks/pack.zip"
        );
    }
}
