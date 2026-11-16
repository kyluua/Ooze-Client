use crate::{
    api::ErrorCode,
    util::{self, Result, error, io_error, metadata_error},
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Read, Write},
    path::Path,
    time::Duration,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Artifact {
    pub url: Option<String>,
    pub file: Option<String>,
    pub size: u64,
    pub sha256: Option<String>,
    pub sha1: Option<String>,
}
impl Artifact {
    pub fn validate(&self) -> Result<()> {
        if self.url.is_some() == self.file.is_some()
            || self.size == 0
            || self.size > 2 * 1024 * 1024 * 1024
            || (self.sha1.is_none() && self.sha256.is_none())
        {
            return Err(metadata_error());
        }
        for (value, length) in [(&self.sha1, 40), (&self.sha256, 64)] {
            if let Some(value) = value
                && (value.len() != length || !value.bytes().all(|c| c.is_ascii_hexdigit()))
            {
                return Err(metadata_error());
            }
        }
        if let Some(url) = &self.url {
            trusted_url(url)?;
        }
        if let Some(file) = &self.file {
            util::relative(file)?;
            if self.sha256.is_none() {
                return Err(metadata_error());
            }
        }
        Ok(())
    }
    pub fn verify(&self, path: &Path) -> Result<bool> {
        util::reject_links(path)?;
        if !path.is_file() {
            return Ok(false);
        }
        if fs::metadata(path).map_err(|_| io_error())?.len() != self.size {
            return Ok(false);
        }
        let mut input = fs::File::open(path).map_err(|_| io_error())?;
        let mut sha256 = Sha256::new();
        let mut sha1 = sha1::Sha1::new();
        let mut bytes = [0u8; 65536];
        loop {
            let count = input.read(&mut bytes).map_err(|_| io_error())?;
            if count == 0 {
                break;
            }
            sha256.update(&bytes[..count]);
            sha1.update(&bytes[..count]);
        }
        Ok(self
            .sha256
            .as_ref()
            .is_none_or(|v| v.eq_ignore_ascii_case(&hex::encode(sha256.finalize())))
            && self
                .sha1
                .as_ref()
                .is_none_or(|v| v.eq_ignore_ascii_case(&hex::encode(sha1.finalize()))))
    }
    pub fn cache_key(&self) -> String {
        self.sha256
            .as_ref()
            .map(|s| format!("sha256-{}", s.to_ascii_lowercase()))
            .unwrap_or_else(|| {
                format!(
                    "sha1-{}",
                    self.sha1.as_deref().unwrap_or("").to_ascii_lowercase()
                )
            })
    }
}

pub(crate) fn trusted_url(raw: &str) -> Result<url::Url> {
    let url = url::Url::parse(raw).map_err(|_| metadata_error())?;
    let host = url.host_str().unwrap_or("");
    let allowed = matches!(
        host,
        "piston-meta.mojang.com"
            | "piston-data.mojang.com"
            | "launchermeta.mojang.com"
            | "launcher.mojang.com"
            | "libraries.minecraft.net"
            | "resources.download.minecraft.net"
            | "meta.fabricmc.net"
            | "maven.fabricmc.net"
            | "api.adoptium.net"
            | "release-assets.githubusercontent.com"
            | "objects.githubusercontent.com"
    ) || (host == "github.com"
        && url
            .path()
            .starts_with("/adoptium/temurin21-binaries/releases/download/"));
    if !allowed
        || url.scheme() != "https"
        || !url.username().is_empty()
        || url.password().is_some()
        || url.port().is_some_and(|p| p != 443)
    {
        return Err(error(
            ErrorCode::Metadata,
            "Download URL is outside the release's trusted upstreams.",
        ));
    }
    Ok(url)
}

pub(crate) struct Http {
    pub client: reqwest::blocking::Client,
}
impl Http {
    pub fn new() -> Result<Self> {
        let client = reqwest::blocking::Client::builder()
            .user_agent("OozeLauncher/0.1.0")
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(30))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|_| error(ErrorCode::Network, "Couldn't initialize HTTPS."))?;
        Ok(Self { client })
    }
    pub fn artifact(
        &self,
        source: &Artifact,
        package: &Path,
        cache: &Path,
        cancelled: &impl Fn() -> Result<()>,
    ) -> Result<std::path::PathBuf> {
        source.validate()?;
        cancelled()?;
        if let Some(file) = &source.file {
            let path = util::join(package, file)?;
            if source.verify(&path)? {
                return Ok(path);
            }
            return Err(error(
                ErrorCode::Integrity,
                "A bundled Ooze artifact failed verification. Re-extract the release package.",
            ));
        }
        util::mkdir(cache)?;
        let target = util::join(cache, &source.cache_key())?;
        if source.verify(&target)? {
            return Ok(target);
        }
        let mut last = network_error();
        for attempt in 0..3 {
            cancelled()?;
            match self.download(source, &target, cancelled) {
                Ok(()) => return Ok(target),
                Err(e) if e.code == ErrorCode::Cancelled => return Err(e),
                Err(e) => last = e,
            }
            if attempt < 2 {
                std::thread::sleep(Duration::from_millis(150 * (attempt + 1)));
            }
        }
        Err(last)
    }
    fn download(
        &self,
        source: &Artifact,
        target: &Path,
        cancelled: &impl Fn() -> Result<()>,
    ) -> Result<()> {
        let mut url = source.url.clone().ok_or_else(metadata_error)?;
        let mut response = None;
        for _ in 0..6 {
            trusted_url(&url)?;
            let result = self
                .client
                .get(&url)
                .timeout(Duration::from_secs(120))
                .send()
                .map_err(|_| network_error())?;
            if result.status().is_redirection() {
                let location = result
                    .headers()
                    .get(reqwest::header::LOCATION)
                    .and_then(|v| v.to_str().ok())
                    .ok_or_else(metadata_error)?;
                url = url::Url::parse(&url)
                    .map_err(|_| metadata_error())?
                    .join(location)
                    .map_err(|_| metadata_error())?
                    .to_string();
                continue;
            }
            response = Some(result);
            break;
        }
        let mut response = response.ok_or_else(network_error)?;
        if !response.status().is_success() {
            return Err(network_error());
        }
        if response.content_length().is_some_and(|n| n != source.size) {
            return Err(error(
                ErrorCode::Integrity,
                "Download size differs from the pinned manifest.",
            ));
        }
        let mut output = tempfile::NamedTempFile::new_in(target.parent().ok_or_else(io_error)?)
            .map_err(|_| io_error())?;
        let mut size = 0u64;
        let mut bytes = [0u8; 65536];
        loop {
            cancelled()?;
            let n = response.read(&mut bytes).map_err(|_| network_error())?;
            if n == 0 {
                break;
            }
            size += n as u64;
            if size > source.size {
                return Err(error(
                    ErrorCode::Integrity,
                    "Download exceeded its pinned size.",
                ));
            }
            output.write_all(&bytes[..n]).map_err(|_| io_error())?;
        }
        output.flush().map_err(|_| io_error())?;
        if !source.verify(output.path())? {
            return Err(error(
                ErrorCode::Integrity,
                "Download checksum mismatch. Retry on a trusted connection.",
            ));
        }
        output.as_file().sync_all().map_err(|_| io_error())?;
        util::reject_links(target)?;
        output.persist(target).map_err(|_| io_error())?;
        Ok(())
    }
}
pub(crate) fn network_error() -> crate::api::LauncherError {
    error(
        ErrorCode::Network,
        "The upstream request failed or timed out. Check your connection and retry.",
    )
}
pub(crate) fn decode<T: DeserializeOwned>(response: reqwest::blocking::Response) -> Result<T> {
    if !response.status().is_success() {
        return Err(network_error());
    }
    let mut bytes = Vec::new();
    response
        .take(8 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| network_error())?;
    if bytes.len() > 8 * 1024 * 1024 {
        return Err(metadata_error());
    }
    serde_json::from_slice(&bytes).map_err(|_| metadata_error())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_untrusted_redirects_and_hashes() {
        for url in [
            "http://libraries.minecraft.net/x",
            "https://libraries.minecraft.net.evil.test/x",
            "https://evil.test/",
            "https://github.com/attacker/x",
            "https://user@libraries.minecraft.net/x",
        ] {
            assert!(trusted_url(url).is_err());
        }
        let a = Artifact {
            url: Some("https://libraries.minecraft.net/x".into()),
            file: None,
            size: 3,
            sha256: Some("0".repeat(64)),
            sha1: None,
        };
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("file");
        fs::write(&file, b"bad").unwrap();
        assert!(!a.verify(&file).unwrap());
        let mut invalid = a;
        invalid.sha256 = Some("../bad".into());
        assert!(invalid.validate().is_err());
    }
}
