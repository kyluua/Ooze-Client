use crate::{
    api::{ErrorCode, Settings, Stage},
    metadata::{self, Installation, InstalledFile, Release},
    net::{Artifact, Http},
    util::{self, Result, error, io_error, metadata_error},
};
use serde::Deserialize;
use std::{
    collections::{BTreeMap, HashSet},
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};

pub(crate) struct Prepared {
    pub installation: Installation,
    pub root: PathBuf,
}
#[derive(Deserialize)]
struct AssetObjects {
    objects: BTreeMap<String, AssetObject>,
}
#[derive(Deserialize)]
struct AssetObject {
    hash: String,
    size: u64,
}

pub(crate) fn claim_instance(instance: &Path) -> Result<fs::File> {
    util::absolute(instance)?;
    let marker = instance.join(".ooze-launcher-instance-v1");
    if instance.exists()
        && !marker.is_file()
        && fs::read_dir(instance)
            .map_err(|_| io_error())?
            .next()
            .is_some()
    {
        return Err(error(
            ErrorCode::Configuration,
            "Choose an empty directory or an existing Ooze-managed instance. Existing Minecraft directories are not adopted.",
        ));
    }
    util::mkdir(instance)?;
    util::reject_links(&marker)?;
    let lock = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&marker)
        .map_err(|_| io_error())?;
    fs2::FileExt::try_lock_exclusive(&lock).map_err(|_| {
        error(
            ErrorCode::Busy,
            "Another launcher is using this game instance.",
        )
    })?;
    crate::game_guard::check(instance)?;
    Ok(lock)
}
pub(crate) fn prepare(
    settings: &Settings,
    manifest_path: &Path,
    data: &Path,
    http: &Http,
    check: &impl Fn() -> Result<()>,
    progress: &impl Fn(Stage, u64, Option<u64>),
) -> Result<Prepared> {
    check()?;
    progress(Stage::Resolving, 0, None);
    let release: Release = util::read_json(manifest_path)?;
    release.validate()?;
    let target = crate::library::store::for_game(&settings.instance_dir)?
        .map(|i| i.target)
        .unwrap_or_else(crate::library::Target::fabric);
    let is_fabric = target.loader == crate::library::Loader::Fabric;
    let expected_version = if is_fabric {
        format!("fabric-loader-{}-{}", release.loader, release.minecraft)
    } else {
        release.minecraft.clone()
    };
    let release_hash = util::sha256(manifest_path)?;
    let managed = util::join(&settings.instance_dir, ".ooze-launcher")?;
    util::mkdir(&managed)?;
    let pointer = util::join(&managed, "active-install.json")?;
    if pointer.is_file() {
        let current: Installation = util::read_json(&pointer)?;
        if current.schema == 1
            && !current.files.is_empty()
            && current.files.len() <= 100000
            && current.release_sha256 == release_hash
            && current.version_name == expected_version
            && current
                .files
                .iter()
                .any(|f| f.path == "mods/ooze-client.jar")
                == target.ooze
            && (settings.java_path.is_some() || current.managed_java.is_some())
        {
            let root = util::join(&managed, &current.directory)?;
            progress(Stage::Verifying, 0, Some(current.files.len() as u64));
            let mut valid = true;
            for (i, file) in current.files.iter().enumerate() {
                check()?;
                let path = util::join(&root, &file.path)?;
                if !path.is_file()
                    || fs::metadata(&path).map_err(|_| io_error())?.len() != file.size
                    || util::sha256(&path)? != file.sha256
                {
                    valid = false;
                    break;
                }
                progress(
                    Stage::Verifying,
                    i as u64 + 1,
                    Some(current.files.len() as u64),
                );
            }
            if valid {
                return Ok(Prepared {
                    installation: current,
                    root,
                });
            }
        }
    }
    let package = manifest_path.parent().ok_or_else(metadata_error)?;
    let cache = util::join(data, "cache")?;
    let game_meta = http.artifact(&release.minecraft_metadata, package, &cache, check)?;
    let fabric_meta = http.artifact(&release.fabric_profile, package, &cache, check)?;
    let minecraft: metadata::Minecraft = util::read_json(&game_meta)?;
    let fabric: metadata::Fabric = util::read_json(&fabric_meta)?;
    if minecraft.id != release.minecraft
        || minecraft.java_version.major_version != 21
        || fabric.inherits_from != minecraft.id
        || fabric.id != format!("fabric-loader-{}-{}", release.loader, minecraft.id)
        || fabric.main_class != "net.fabricmc.loader.impl.launch.knot.KnotClient"
        || minecraft.main_class != "net.minecraft.client.main.Main"
    {
        return Err(metadata_error());
    }
    let mut specs: Vec<(String, Artifact)> = Vec::new();
    let mut classpath = Vec::new();
    let mut library_ids = HashSet::new();
    if fabric.libraries.len() != release.fabric_libraries.len() {
        return Err(metadata_error());
    }
    for lib in fabric.libraries.iter().filter(|_| is_fabric) {
        let pin = release
            .fabric_libraries
            .iter()
            .find(|p| p.name == lib.name)
            .ok_or_else(metadata_error)?;
        let path = format!("libraries/{}", pin.path);
        classpath.push(path.clone());
        specs.push((path, pin.artifact.clone()));
        let parts = lib.name.split(':').collect::<Vec<_>>();
        library_ids.insert(format!("{}:{}", parts[0], parts[1]));
    }
    for lib in &minecraft.libraries {
        if !metadata::allowed(&lib.rules)? {
            continue;
        }
        // 1.21.11 carries modern native jars on the classpath; legacy classifier extraction isn't assumed.
        if !lib.natives.is_empty() {
            return Err(metadata_error());
        }
        let parts = lib.name.split(':').collect::<Vec<_>>();
        if parts.len() < 3 {
            return Err(metadata_error());
        }
        if parts.len() == 3 && library_ids.contains(&format!("{}:{}", parts[0], parts[1])) {
            continue;
        }
        if let Some(download) = &lib.downloads.artifact {
            let path = format!(
                "libraries/{}",
                download.path.as_ref().ok_or_else(metadata_error)?
            );
            classpath.push(path.clone());
            specs.push((path, download.artifact()));
        }
    }
    specs.push((
        "client.jar".into(),
        minecraft
            .downloads
            .get("client")
            .ok_or_else(metadata_error)?
            .artifact(),
    ));
    classpath.push("client.jar".into());
    let index_source = minecraft.asset_index.artifact();
    let index_cache = http.artifact(&index_source, package, &cache, check)?;
    let index: AssetObjects = util::read_json(&index_cache)?;
    if index.objects.len() > 100000 {
        return Err(metadata_error());
    }
    let index_path = format!("assets/indexes/{}.json", minecraft.asset_index.id);
    util::relative(&index_path)?;
    specs.push((index_path, index_source));
    let mut assets = HashSet::new();
    for object in index.objects.values() {
        if object.hash.len() != 40 || !object.hash.bytes().all(|c| c.is_ascii_hexdigit()) {
            return Err(metadata_error());
        }
        if assets.insert(&object.hash) {
            let hash = &object.hash;
            let partial = &hash[..2];
            specs.push((
                format!("assets/objects/{partial}/{hash}"),
                Artifact {
                    url: Some(format!(
                        "https://resources.download.minecraft.net/{partial}/{hash}"
                    )),
                    file: None,
                    size: object.size,
                    sha1: Some(hash.clone()),
                    sha256: None,
                },
            ));
        }
    }
    if target.ooze {
        specs.push(("mods/fabric-api.jar".into(), release.fabric_api.clone()));
        specs.push(("mods/ooze-client.jar".into(), release.ooze.clone()));
    }
    if specs.len() > 100000 {
        return Err(metadata_error());
    }
    let installs = util::join(&managed, "installs")?;
    util::mkdir(&installs)?;
    let stage = tempfile::Builder::new()
        .prefix("generation-")
        .tempdir_in(&installs)
        .map_err(|_| io_error())?;
    let root = stage.path();
    let mut files = Vec::new();
    let total = specs.len() as u64;
    let mut seen = HashSet::new();
    for (i, (path, source)) in specs.iter().enumerate() {
        check()?;
        util::relative(path)?;
        if !seen.insert(path.to_ascii_lowercase()) {
            return Err(metadata_error());
        }
        progress(Stage::Downloading, i as u64, Some(total));
        let cached = http.artifact(source, package, &cache, check)?;
        let target = util::join(root, path)?;
        util::mkdir(target.parent().ok_or_else(io_error)?)?;
        fs::copy(&cached, &target).map_err(|_| io_error())?;
        files.push(InstalledFile {
            path: path.clone(),
            size: source.size,
            sha256: util::sha256(&target)?,
        });
    }
    if target.ooze {
        check_native_jar(&util::join(root, "mods/ooze-client.jar")?)?;
    }
    let managed_java = if settings.java_path.is_none() {
        progress(Stage::Extracting, 0, None);
        let archive = http.artifact(&release.java.archive, package, &cache, check)?;
        let java_dir = util::join(root, "java")?;
        util::mkdir(&java_dir)?;
        extract_zip(&archive, &java_dir, check)?;
        record_tree(root, &java_dir, &mut files)?;
        Some(format!("java/{}", release.java.java_relative))
    } else {
        None
    };
    progress(Stage::ValidatingJava, 0, None);
    let java = match &settings.java_path {
        Some(path) => path.clone(),
        None => util::join(root, managed_java.as_deref().ok_or_else(metadata_error)?)?,
    };
    crate::process::validate_java(&java)?;
    util::mkdir(&util::join(root, "natives")?)?;
    let mut jvm_args = metadata::arguments(&minecraft.arguments.jvm)?;
    if is_fabric {
        jvm_args.extend(metadata::arguments(&fabric.arguments.jvm)?);
    }
    let mut game_args = metadata::arguments(&minecraft.arguments.game)?;
    if is_fabric {
        game_args.extend(metadata::arguments(&fabric.arguments.game)?);
    }
    let generation = root
        .file_name()
        .and_then(|v| v.to_str())
        .ok_or_else(io_error)?
        .to_owned();
    let installation = Installation {
        schema: 1,
        release_sha256: release_hash,
        directory: format!("installs/{generation}"),
        files,
        classpath,
        main_class: if is_fabric {
            fabric.main_class
        } else {
            minecraft.main_class
        },
        jvm_args,
        game_args,
        asset_index: minecraft.asset_index.id,
        managed_java,
        version_name: expected_version,
    };
    check()?;
    let root = stage.keep();
    util::atomic_json(&pointer, &installation)?;
    progress(Stage::Verifying, total, Some(total));
    Ok(Prepared { installation, root })
}

pub(crate) fn extract_zip(
    archive: &Path,
    destination: &Path,
    check: &impl Fn() -> Result<()>,
) -> Result<()> {
    let file = fs::File::open(archive).map_err(|_| io_error())?;
    let mut zip = zip::ZipArchive::new(file).map_err(|_| metadata_error())?;
    if zip.len() > 50000 {
        return Err(metadata_error());
    }
    let mut total = 0u64;
    let mut names = HashSet::new();
    for i in 0..zip.len() {
        check()?;
        let mut entry = zip.by_index(i).map_err(|_| metadata_error())?;
        let name = entry.name().trim_end_matches('/').to_owned();
        util::relative(&name)?;
        if !names.insert(name.to_ascii_lowercase())
            || entry.unix_mode().is_some_and(|m| {
                (m & 0o170000) != 0 && !matches!(m & 0o170000, 0o100000 | 0o040000)
            })
        {
            return Err(metadata_error());
        }
        total = total.checked_add(entry.size()).ok_or_else(metadata_error)?;
        if total > 1024 * 1024 * 1024 || entry.size() > 256 * 1024 * 1024 {
            return Err(metadata_error());
        }
        let path = util::join(destination, &name)?;
        if entry.is_dir() {
            util::mkdir(&path)?;
            continue;
        }
        util::mkdir(path.parent().ok_or_else(io_error)?)?;
        let mut output = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|_| io_error())?;
        let mut count = 0u64;
        let mut buffer = [0u8; 65536];
        loop {
            check()?;
            let n = entry.read(&mut buffer).map_err(|_| metadata_error())?;
            if n == 0 {
                break;
            }
            count += n as u64;
            if count > 256 * 1024 * 1024 {
                return Err(metadata_error());
            }
            output.write_all(&buffer[..n]).map_err(|_| io_error())?;
        }
        if count != entry.size() {
            return Err(metadata_error());
        }
    }
    Ok(())
}
fn record_tree(root: &Path, dir: &Path, files: &mut Vec<InstalledFile>) -> Result<()> {
    for entry in fs::read_dir(dir).map_err(|_| io_error())? {
        let path = entry.map_err(|_| io_error())?.path();
        util::reject_links(&path)?;
        if path.is_dir() {
            record_tree(root, &path, files)?;
        } else {
            files.push(InstalledFile {
                path: path
                    .strip_prefix(root)
                    .map_err(|_| io_error())?
                    .to_string_lossy()
                    .replace('\\', "/"),
                size: fs::metadata(&path).map_err(|_| io_error())?.len(),
                sha256: util::sha256(&path)?,
            });
        }
    }
    Ok(())
}
fn check_native_jar(path: &Path) -> Result<()> {
    let mut archive = zip::ZipArchive::new(fs::File::open(path).map_err(|_| io_error())?)
        .map_err(|_| metadata_error())?;
    let mut properties = String::new();
    archive
        .by_name("META-INF/ooze/native.properties")
        .map_err(|_| {
            error(
                ErrorCode::Integrity,
                "The Ooze mod lacks production native resources.",
            )
        })?
        .take(16384)
        .read_to_string(&mut properties)
        .map_err(|_| metadata_error())?;
    let values: BTreeMap<_, _> = properties
        .lines()
        .filter_map(|line| line.split_once('='))
        .collect();
    for (key, value) in [
        ("bridge.abi", "1"),
        ("variant", "release"),
        ("capabilities", "3"),
        ("platform", "windows-x86_64"),
    ] {
        if values.get(key).copied() != Some(value) {
            return Err(error(
                ErrorCode::Integrity,
                "The Ooze JAR must contain the production ABI 1 Windows x64 engine.",
            ));
        }
    }
    let resource = values.get("resource").ok_or_else(metadata_error)?;
    let hash = values.get("sha256").ok_or_else(metadata_error)?;
    use sha2::Digest;
    let mut hasher = sha2::Sha256::new();
    let mut native = archive.by_name(resource).map_err(|_| metadata_error())?;
    if native.size() > 32 * 1024 * 1024 {
        return Err(metadata_error());
    }
    let mut bytes = [0u8; 65536];
    loop {
        let count = native.read(&mut bytes).map_err(|_| metadata_error())?;
        if count == 0 {
            break;
        }
        hasher.update(&bytes[..count]);
    }
    if hex::encode(hasher.finalize()) != *hash {
        return Err(error(
            ErrorCode::Integrity,
            "The Ooze native library checksum differs from its manifest.",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn refuses_unrelated_instance_and_locks_managed_one() {
        let dir = tempfile::tempdir().unwrap();
        let instance = dir.path().join("instance");
        fs::create_dir(&instance).unwrap();
        fs::write(instance.join("world.txt"), "preserve").unwrap();
        assert!(claim_instance(&instance).is_err());
        assert_eq!(
            fs::read_to_string(instance.join("world.txt")).unwrap(),
            "preserve"
        );
        let owned = dir.path().join("owned");
        let lock = claim_instance(&owned).unwrap();
        assert!(claim_instance(&owned).is_err());
        drop(lock);
        assert!(claim_instance(&owned).is_ok());
    }
    #[test]
    fn hostile_archive_is_confined() {
        let dir = tempfile::tempdir().unwrap();
        let archive = dir.path().join("bad.zip");
        let mut writer = zip::ZipWriter::new(fs::File::create(&archive).unwrap());
        writer
            .start_file("../escaped", zip::write::SimpleFileOptions::default())
            .unwrap();
        writer.write_all(b"bad").unwrap();
        writer.finish().unwrap();
        let target = dir.path().join("target");
        fs::create_dir(&target).unwrap();
        assert!(extract_zip(&archive, &target, &|| Ok(())).is_err());
        assert!(!dir.path().join("escaped").exists());
    }
}
