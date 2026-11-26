use crate::{
    library::{Loader, Target, java_for_minecraft},
    metadata::{JavaRelease, PinnedLibrary, Release, maven_path},
    net::Artifact,
    util::{self, Result, metadata_error},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::HashSet, path::Path};

#[derive(Deserialize)]
#[serde(untagged)]
enum Package {
    Catalog(CatalogFile),
    Legacy(Box<Release>),
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CatalogFile {
    schema: u32,
    id: String,
    targets: Vec<ReleaseTarget>,
}

pub(crate) struct Catalog {
    entries: Vec<ReleaseTarget>,
    legacy_hash: Option<String>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ReleaseTarget {
    pub minecraft: String,
    pub java_major: u32,
    pub minecraft_metadata: Artifact,
    pub java: JavaRelease,
    pub fabric: Option<FabricRelease>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct FabricRelease {
    pub loader: String,
    pub profile: Artifact,
    pub libraries: Vec<PinnedLibrary>,
    pub bundle: Option<OozeBundle>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct OozeBundle {
    pub api_version: String,
    pub api: Artifact,
    pub ooze: Artifact,
}

pub(crate) const LEGACY_FABRIC_API: &str = "0.141.6+1.21.11";

impl Catalog {
    pub fn read(path: &Path) -> Result<Self> {
        util::absolute(path)?;
        match util::read_json(path)? {
            Package::Catalog(catalog) => Self::from_file(catalog),
            Package::Legacy(release) => Self::from_legacy(*release, util::sha256(path)?),
        }
    }

    fn from_file(catalog: CatalogFile) -> Result<Self> {
        if catalog.schema != 2
            || !valid_id(&catalog.id)
            || !(1..=12).contains(&catalog.targets.len())
        {
            return Err(metadata_error());
        }
        let mut keys = HashSet::new();
        for entry in &catalog.targets {
            entry.validate()?;
            if !keys.insert((entry.minecraft.as_str(), entry.fabric.is_some())) {
                return Err(metadata_error());
            }
        }
        Ok(Self {
            entries: catalog.targets,
            legacy_hash: None,
        })
    }

    fn from_legacy(release: Release, hash: String) -> Result<Self> {
        release.validate()?;
        let vanilla = ReleaseTarget {
            minecraft: release.minecraft.clone(),
            java_major: 21,
            minecraft_metadata: release.minecraft_metadata.clone(),
            java: JavaRelease {
                archive: release.java.archive.clone(),
                java_relative: release.java.java_relative.clone(),
            },
            fabric: None,
        };
        let fabric = ReleaseTarget {
            minecraft: release.minecraft,
            java_major: 21,
            minecraft_metadata: release.minecraft_metadata,
            java: release.java,
            fabric: Some(FabricRelease {
                loader: release.loader,
                profile: release.fabric_profile,
                libraries: release.fabric_libraries,
                bundle: Some(OozeBundle {
                    api_version: LEGACY_FABRIC_API.into(),
                    api: release.fabric_api,
                    ooze: release.ooze,
                }),
            }),
        };
        Ok(Self {
            entries: vec![fabric, vanilla],
            legacy_hash: Some(hash),
        })
    }

    pub fn targets(&self) -> Vec<Target> {
        let mut targets = Vec::new();
        for entry in &self.entries {
            if entry.bundle().is_some() {
                targets.push(entry.target(true));
            }
            targets.push(entry.target(false));
        }
        targets
    }

    pub fn select(&self, target: &Target) -> Result<&ReleaseTarget> {
        target.validate()?;
        self.entries
            .iter()
            .find(|entry| {
                (!target.ooze || entry.bundle().is_some()) && entry.target(target.ooze) == *target
            })
            .ok_or_else(|| {
                crate::library::invalid(
                    "This exact game, loader and Ooze combination is unavailable in the current package.",
                )
            })
    }

    pub fn receipt_hash(&self, entry: &ReleaseTarget) -> Result<String> {
        if let Some(hash) = &self.legacy_hash {
            return Ok(hash.clone());
        }
        let bytes = serde_json::to_vec(entry).map_err(|_| metadata_error())?;
        Ok(hex::encode(Sha256::digest(bytes)))
    }
}

impl ReleaseTarget {
    pub fn target(&self, ooze: bool) -> Target {
        Target {
            minecraft: self.minecraft.clone(),
            loader: if self.fabric.is_some() {
                Loader::Fabric
            } else {
                Loader::Vanilla
            },
            loader_version: self.fabric.as_ref().map(|fabric| fabric.loader.clone()),
            java_major: self.java_major,
            ooze,
        }
    }

    pub fn bundle(&self) -> Option<&OozeBundle> {
        self.fabric.as_ref()?.bundle.as_ref()
    }

    fn validate(&self) -> Result<()> {
        if java_for_minecraft(&self.minecraft) != Some(self.java_major)
            || self.minecraft_metadata.file.is_none()
            || self.java.archive.sha256.is_none()
        {
            return Err(metadata_error());
        }
        self.minecraft_metadata.validate()?;
        self.java.archive.validate()?;
        util::relative(&self.java.java_relative)?;
        if let Some(fabric) = &self.fabric {
            fabric.validate()?;
        }
        Ok(())
    }
}

impl FabricRelease {
    fn validate(&self) -> Result<()> {
        if !valid_id(&self.loader)
            || self.profile.file.is_none()
            || !(1..=128).contains(&self.libraries.len())
        {
            return Err(metadata_error());
        }
        self.profile.validate()?;
        let mut names = HashSet::new();
        for library in &self.libraries {
            library.artifact.validate()?;
            if library.artifact.sha256.is_none()
                || maven_path(&library.name)? != library.path
                || !names.insert(library.name.as_str())
            {
                return Err(metadata_error());
            }
        }
        if let Some(bundle) = &self.bundle {
            maven_path(&format!(
                "net.fabricmc.fabric-api:fabric-api:{}",
                bundle.api_version
            ))?;
            if bundle.api.sha256.is_none() || bundle.ooze.file.is_none() {
                return Err(metadata_error());
            }
            bundle.api.validate()?;
            bundle.ooze.validate()?;
        }
        Ok(())
    }
}

fn valid_id(value: &str) -> bool {
    (1..=128).contains(&value.len())
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_duplicate_rows_wrong_runtime_and_unpinned_fabric() {
        let file = serde_json::json!({
            "file": "metadata/minecraft.json",
            "size": 1,
            "sha256": "0".repeat(64)
        });
        let row = serde_json::json!({
            "minecraft": "26.3",
            "java_major": 25,
            "minecraft_metadata": file,
            "java": {"archive": file, "java_relative": "runtime/bin/java.exe"},
            "fabric": null
        });
        let parse = |targets| {
            let input = serde_json::json!({"schema": 2, "id": "catalog", "targets": targets});
            Catalog::from_file(serde_json::from_value(input).unwrap())
        };
        let catalog = parse(vec![row.clone()]).unwrap();
        let vanilla = catalog.targets().pop().unwrap();
        assert!(catalog.select(&vanilla).is_ok());
        let mut unsupported = vanilla.clone();
        unsupported.loader = Loader::Fabric;
        unsupported.loader_version = Some("0.20.1".into());
        assert!(catalog.select(&unsupported).is_err());
        unsupported = vanilla;
        unsupported.java_major = 21;
        assert!(catalog.select(&unsupported).is_err());
        assert!(parse(vec![row.clone(), row.clone()]).is_err());
        let mut wrong_runtime = row;
        wrong_runtime["java_major"] = 21.into();
        assert!(parse(vec![wrong_runtime]).is_err());
    }
}
