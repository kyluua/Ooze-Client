use crate::api::{ErrorCode, LauncherError};
use serde::{Serialize, de::DeserializeOwned};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Read, Write},
    path::{Component, Path, PathBuf},
};

pub(crate) type Result<T> = std::result::Result<T, LauncherError>;
pub(crate) fn error(code: ErrorCode, message: impl Into<String>) -> LauncherError {
    LauncherError {
        code,
        message: message.into(),
        retryable: matches!(code, ErrorCode::Network | ErrorCode::Busy),
    }
}
pub(crate) fn io_error() -> LauncherError {
    error(
        ErrorCode::Io,
        "Couldn't read or write Ooze's managed files. Check disk space and directory permissions.",
    )
}
pub(crate) fn metadata_error() -> LauncherError {
    error(
        ErrorCode::Metadata,
        "Unsupported or malformed installation metadata. Use a matching Ooze release package.",
    )
}

pub(crate) fn reject_links(path: &Path) -> Result<()> {
    for ancestor in path.ancestors() {
        match fs::symlink_metadata(ancestor) {
            Ok(meta) => {
                #[cfg(windows)]
                let reparse = {
                    use std::os::windows::fs::MetadataExt;
                    meta.file_attributes() & 0x400 != 0
                };
                #[cfg(not(windows))]
                let reparse = false;
                if meta.file_type().is_symlink() || reparse {
                    return Err(error(
                        ErrorCode::Configuration,
                        "Managed paths must not contain symbolic links or Windows reparse points.",
                    ));
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => return Err(io_error()),
        }
    }
    Ok(())
}
pub(crate) fn absolute(path: &Path) -> Result<()> {
    if !path.is_absolute() || path.components().any(|c| matches!(c, Component::ParentDir)) {
        return Err(error(
            ErrorCode::Configuration,
            "Choose an absolute local directory without parent traversal.",
        ));
    }
    #[cfg(windows)]
    if !matches!(path.components().next(), Some(Component::Prefix(p)) if matches!(p.kind(), std::path::Prefix::Disk(_) | std::path::Prefix::VerbatimDisk(_)))
    {
        return Err(error(
            ErrorCode::Configuration,
            "Choose a local Windows drive, not a network share.",
        ));
    }
    reject_links(path)
}
pub(crate) fn relative(path: &str) -> Result<PathBuf> {
    if path.is_empty() || path.len() > 1024 || path.contains(['\\', ':', '\0']) {
        return Err(metadata_error());
    }
    let candidate = Path::new(path);
    if candidate
        .components()
        .any(|c| !matches!(c, Component::Normal(_)))
        || candidate.is_absolute()
    {
        return Err(metadata_error());
    }
    for part in path.split('/') {
        let stem = part.split('.').next().unwrap_or("").to_ascii_uppercase();
        if part.is_empty()
            || matches!(part, "." | "..")
            || part.ends_with(['.', ' '])
            || part.contains(['<', '>', '"', '|', '?', '*'])
            || matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
            || (stem.len() == 4
                && (stem.starts_with("COM") || stem.starts_with("LPT"))
                && matches!(stem.as_bytes()[3], b'1'..=b'9'))
        {
            return Err(metadata_error());
        }
    }
    Ok(candidate.to_owned())
}
pub(crate) fn join(base: &Path, child: &str) -> Result<PathBuf> {
    let result = base.join(relative(child)?);
    reject_links(&result)?;
    Ok(result)
}
pub(crate) fn mkdir(path: &Path) -> Result<()> {
    reject_links(path)?;
    fs::create_dir_all(path).map_err(|_| io_error())?;
    reject_links(path)
}
pub(crate) fn atomic_json(path: &Path, value: &impl Serialize) -> Result<()> {
    reject_links(path)?;
    let mut temp = tempfile::NamedTempFile::new_in(path.parent().ok_or_else(io_error)?)
        .map_err(|_| io_error())?;
    serde_json::to_writer(&mut temp, value).map_err(|_| io_error())?;
    temp.flush().map_err(|_| io_error())?;
    temp.as_file().sync_all().map_err(|_| io_error())?;
    temp.persist(path).map_err(|_| io_error())?;
    Ok(())
}
pub(crate) fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T> {
    reject_links(path)?;
    let file = fs::File::open(path).map_err(|_| io_error())?;
    if file.metadata().map_err(|_| io_error())?.len() > 16 * 1024 * 1024 {
        return Err(metadata_error());
    }
    serde_json::from_reader(file).map_err(|_| metadata_error())
}
pub(crate) fn sha256(path: &Path) -> Result<String> {
    reject_links(path)?;
    let mut input = fs::File::open(path).map_err(|_| io_error())?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 65536];
    loop {
        let count = input.read(&mut buffer).map_err(|_| io_error())?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    Ok(hex::encode(hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_hostile_paths() {
        for path in [
            "../x",
            "/x",
            "C:/x",
            "x\\y",
            "a/../../b",
            "a:N",
            "CON.txt",
            "a/PRN",
            "LPT9",
            "a./b",
            "x/",
            "a//b",
            "x\0",
        ] {
            assert!(relative(path).is_err(), "{path:?}");
        }
        assert!(relative("libraries/org/foo.jar").is_ok());
    }
    #[test]
    fn atomic_settings_replace_and_malformed_json() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("state.json");
        atomic_json(&path, &vec![1]).unwrap();
        atomic_json(&path, &vec![2, 3]).unwrap();
        assert_eq!(read_json::<Vec<u32>>(&path).unwrap(), vec![2, 3]);
        fs::write(&path, b"{").unwrap();
        assert!(read_json::<Vec<u32>>(&path).is_err());
    }
}
