use crate::{
    api::{ErrorCode, LAUNCHER_API_VERSION, LauncherConfig},
    metadata::Release,
    process::{MAX_GAME_LOG_BYTES, Redactor},
    util::{self, Result, error, io_error},
};
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};
use zip::{ZipWriter, write::SimpleFileOptions};

pub(crate) fn export(
    config: &LauncherConfig,
    destination: &Path,
    check: &impl Fn() -> Result<()>,
) -> Result<()> {
    check()?;
    util::absolute(destination)?;
    if destination.extension().and_then(|v| v.to_str()) != Some("zip") {
        return Err(error(
            ErrorCode::Configuration,
            "Choose a .zip diagnostics file.",
        ));
    }
    let release: Release = util::read_json(&config.release_manifest)?;
    release.validate()?;
    let metadata = serde_json::json!({
        "schema": 1,
        "launcherVersion": env!("CARGO_PKG_VERSION"),
        "launcherApi": LAUNCHER_API_VERSION,
        "releaseId": release.id,
        "packageMinecraft": release.minecraft,
        "packageFabricLoader": release.loader,
        "platform": std::env::consts::OS,
        "architecture": std::env::consts::ARCH,
    });
    write_archive(&config.data_dir, destination, &metadata, check)
}

fn write_archive(
    data_dir: &Path,
    destination: &Path,
    metadata: &serde_json::Value,
    check: &impl Fn() -> Result<()>,
) -> Result<()> {
    let mut output = tempfile::NamedTempFile::new_in(destination.parent().ok_or_else(io_error)?)
        .map_err(|_| io_error())?;
    let mut archive = ZipWriter::new(output.as_file_mut());
    let options = SimpleFileOptions::default();
    archive
        .start_file("diagnostics.json", options)
        .map_err(|_| io_error())?;
    serde_json::to_writer_pretty(&mut archive, metadata).map_err(|_| io_error())?;

    let logs = util::join(data_dir, "logs")?;
    if let Some(path) = latest_game_log(&logs)? {
        check()?;
        let mut log = String::new();
        fs::File::open(path)
            .map_err(|_| io_error())?
            .take(MAX_GAME_LOG_BYTES as u64 + 1)
            .read_to_string(&mut log)
            .map_err(|_| io_error())?;
        if log.len() > MAX_GAME_LOG_BYTES {
            return Err(error(
                ErrorCode::Io,
                "The launcher game log exceeds the diagnostics limit.",
            ));
        }
        // The persisted log already had the live session token removed. Apply
        // the field and JWT rules again, including for logs from older builds.
        let redactor = Redactor::new("")?;
        archive
            .start_file("game.log", options)
            .map_err(|_| io_error())?;
        for line in log.lines() {
            check()?;
            writeln!(archive, "{}", redactor.line(line)).map_err(|_| io_error())?;
        }
    }
    archive.finish().map_err(|_| io_error())?;
    output.as_file().sync_all().map_err(|_| io_error())?;
    check()?;
    util::reject_links(destination)?;
    output.persist_noclobber(destination).map_err(|e| {
        if e.error.kind() == std::io::ErrorKind::AlreadyExists {
            error(
                ErrorCode::Configuration,
                "That diagnostics file already exists. Choose a new file.",
            )
        } else {
            io_error()
        }
    })?;
    Ok(())
}

fn latest_game_log(logs: &Path) -> Result<Option<PathBuf>> {
    let entries = match fs::read_dir(logs) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(io_error()),
    };
    let mut latest = None::<(u64, PathBuf)>;
    for entry in entries {
        let entry = entry.map_err(|_| io_error())?;
        let Some(stamp) = entry.file_name().to_str().and_then(|name| {
            let stamp = name.strip_prefix("game-")?.strip_suffix(".log")?;
            if stamp.is_empty() || !stamp.bytes().all(|b| b.is_ascii_digit()) {
                return None;
            }
            stamp.parse::<u64>().ok()
        }) else {
            continue;
        };
        if latest.as_ref().is_none_or(|(current, _)| stamp > *current) {
            latest = Some((stamp, entry.path()));
        }
    }
    let Some((_, path)) = latest else {
        return Ok(None);
    };
    util::reject_links(&path)?;
    if !path.is_file() {
        return Err(io_error());
    }
    Ok(Some(path))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn archive_contains_only_latest_redacted_launcher_log_and_metadata() {
        let data = tempfile::tempdir().unwrap();
        let logs = data.path().join("logs");
        fs::create_dir(&logs).unwrap();
        fs::write(data.path().join("settings.json"), "private fixture").unwrap();
        fs::write(logs.join("game-9.log"), "obsolete fixture").unwrap();
        fs::write(logs.join("latest.log"), "unmanaged fixture").unwrap();
        fs::write(
            logs.join("game-10.log"),
            "[stdout] OOZE_READY v1 screen=title engine=LOADED\n\
             Authorization: Bearer synthetic-bearer\n\
             {\"identityToken\":\"XBL3.0 x=user;synthetic-xsts\"}\n\
             identityToken XBL3.0 x=user;synthetic-legacy-xsts\n\
             --accessToken synthetic-access\n\
             refresh_token = synthetic-refresh\n",
        )
        .unwrap();
        let archive = data.path().join("evidence.zip");
        let metadata = serde_json::json!({"schema": 1});
        write_archive(data.path(), &archive, &metadata, &|| Ok(())).unwrap();
        let mut contents = zip::ZipArchive::new(fs::File::open(&archive).unwrap()).unwrap();
        assert_eq!(contents.len(), 2);
        let mut log = String::new();
        contents
            .by_name("game.log")
            .unwrap()
            .read_to_string(&mut log)
            .unwrap();
        assert!(log.contains("OOZE_READY v1 screen=title engine=LOADED"));
        assert_eq!(log.matches("[REDACTED]").count(), 5);
        assert!(!log.contains("synthetic-"));
        drop(contents);
        let before = fs::read(&archive).unwrap();
        assert!(write_archive(data.path(), &archive, &metadata, &|| Ok(())).is_err());
        assert_eq!(fs::read(archive).unwrap(), before);
    }
}
