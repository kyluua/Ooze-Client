use super::model::*;
use crate::{
    api::ErrorCode,
    util::{self, Result, error, io_error},
};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Index {
    schema: u32,
    ids: Vec<Id>,
    selected: Option<Id>,
}
pub(super) struct Store {
    pub root: PathBuf,
}
impl Store {
    pub fn open(data: &Path) -> Result<Self> {
        util::absolute(data)?;
        let root = util::join(data, "library")?;
        util::mkdir(&root)?;
        util::mkdir(&util::join(&root, "instances")?)?;
        let store = Self { root };
        if !store.index_path()?.exists() {
            util::atomic_json(
                &store.index_path()?,
                &Index {
                    schema: SCHEMA,
                    ids: Vec::new(),
                    selected: None,
                },
            )?;
        }
        store.index()?;
        Ok(store)
    }
    fn index_path(&self) -> Result<PathBuf> {
        util::join(&self.root, "state.json")
    }
    fn index(&self) -> Result<Index> {
        let path = self.index_path()?;
        if fs::metadata(&path).map_err(|_| io_error())?.len() > 4 * 1024 * 1024 {
            return Err(invalid(
                "Library index exceeds its limit; preserve it for recovery.",
            ));
        }
        let index: Index = util::read_json(&path)?;
        if index.schema != SCHEMA || index.ids.len() > 128 {
            return Err(invalid(
                "Unsupported library schema or instance limit; preserve the original file.",
            ));
        }
        let mut ids = std::collections::HashSet::new();
        for id in &index.ids {
            id.validate()?;
            if !ids.insert(id) {
                return Err(invalid("Duplicate library identity."));
            }
        }
        if index.selected.as_ref().is_some_and(|id| !ids.contains(id)) {
            return Err(invalid("Invalid selected instance identity."));
        }
        Ok(index)
    }
    pub fn container(&self, id: &Id) -> Result<PathBuf> {
        id.validate()?;
        util::join(&self.root, &format!("instances/{}", id.0))
    }
    pub fn game(&self, id: &Id) -> Result<PathBuf> {
        util::join(&self.container(id)?, "game")
    }
    pub fn instance(&self, id: &Id) -> Result<Instance> {
        if !self.index()?.ids.contains(id) {
            return Err(invalid("Unknown instance identity."));
        }
        let value: Instance = util::read_json(&util::join(&self.container(id)?, "instance.json")?)?;
        value.validate()?;
        if &value.id != id {
            return Err(invalid("Instance identity does not match its directory."));
        }
        Ok(value)
    }
    pub fn snapshot(&self) -> Result<Snapshot> {
        let index = self.index()?;
        let instances = index
            .ids
            .iter()
            .map(|id| {
                Ok(InstanceSummary {
                    instance: self.instance(id)?,
                    game_dir: self.game(id)?,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Snapshot {
            api_version: LIBRARY_API_VERSION,
            instances,
            selected: index.selected,
            active_operation: None,
        })
    }
    pub fn save(&self, value: &mut Instance) -> Result<()> {
        value.revision = value
            .revision
            .checked_add(1)
            .ok_or_else(|| invalid("Instance revision exhausted."))?;
        value.validate()?;
        util::atomic_json(
            &util::join(&self.container(&value.id)?, "instance.json")?,
            value,
        )
    }
    pub fn claim(&self, id: &Id) -> Result<(Instance, fs::File)> {
        let instance = self.instance(id)?;
        if instance.archived {
            return Err(invalid(
                "Restore or choose an active instance before editing it.",
            ));
        }
        let lock = crate::install::claim_instance(&self.game(id)?)?;
        super::content::recover(self, id)?;
        Ok((self.instance(id)?, lock))
    }
    pub fn create(
        &self,
        name_value: String,
        target: Target,
        preset: Preset,
        mode: Mode,
    ) -> Result<Id> {
        self.create_filled(name_value, target, preset, mode, |_, _| Ok(()))
    }
    fn create_filled(
        &self,
        name_value: String,
        target: Target,
        preset: Preset,
        mode: Mode,
        fill: impl FnOnce(&Path, &mut Instance) -> Result<()>,
    ) -> Result<Id> {
        name(&name_value)?;
        target.validate()?;
        let mut index = self.index()?;
        if index.ids.len() >= 128 {
            return Err(invalid(
                "The library supports at most 128 instances, including archives.",
            ));
        }
        let id = Id::new();
        let directory = self.container(&id)?;
        if directory.exists() {
            return Err(error(
                ErrorCode::Busy,
                "Instance identity collision; retry.",
            ));
        }
        let stage = tempfile::Builder::new()
            .prefix(".new-instance-")
            .tempdir_in(util::join(&self.root, "instances")?)
            .map_err(|_| io_error())?;
        let game = util::join(stage.path(), "game")?;
        util::mkdir(&game)?;
        {
            let _lock = crate::install::claim_instance(&game)?;
        }
        let mut value = Instance {
            schema: SCHEMA,
            id: id.clone(),
            name: name_value,
            revision: 1,
            target,
            mode,
            profile: Profile::new(preset),
            theme: Theme::default(),
            archived: false,
            content: Vec::new(),
        };
        fill(&game, &mut value)?;
        value.validate()?;
        util::atomic_json(&util::join(stage.path(), "instance.json")?, &value)?;
        let kept = stage.keep();
        fs::rename(kept, &directory).map_err(|_| io_error())?;
        index.ids.push(id.clone());
        if index.selected.is_none() {
            index.selected = Some(id.clone());
        }
        util::atomic_json(&self.index_path()?, &index)?;
        Ok(id)
    }
    pub fn select(&self, id: &Id) -> Result<()> {
        let instance = self.instance(id)?;
        if instance.archived {
            return Err(invalid("Archived instances cannot be selected."));
        }
        let mut index = self.index()?;
        index.selected = Some(id.clone());
        util::atomic_json(&self.index_path()?, &index)
    }
    pub fn duplicate(
        &self,
        id: &Id,
        name_value: String,
        check: &dyn Fn() -> Result<()>,
    ) -> Result<Id> {
        let (original, _source) = self.claim(id)?;
        check()?;
        self.create_filled(
            name_value,
            original.target.clone(),
            original.profile.preset(),
            Mode::Custom,
            |game, copy| {
                copy.profile = original.profile.clone();
                copy.theme = original.theme.clone();
                for receipt in original
                    .content
                    .iter()
                    .filter(|r| r.kind != ContentKind::Datapack)
                {
                    check()?;
                    let source = util::join(&self.game(id)?, &receipt.path)?;
                    if super::content::hash(&source, check)? != receipt.sha512 {
                        return Err(error(
                            ErrorCode::Integrity,
                            "Source content changed; repair it before duplication.",
                        ));
                    }
                    let dest = util::join(game, &receipt.path)?;
                    util::mkdir(dest.parent().ok_or_else(io_error)?)?;
                    fs::copy(source, &dest).map_err(|_| io_error())?;
                    fs::OpenOptions::new()
                        .write(true)
                        .open(dest)
                        .map_err(|_| io_error())?
                        .sync_all()
                        .map_err(|_| io_error())?;
                    copy.content.push(receipt.clone());
                }
                check()
            },
        )
    }
    pub fn archive(&self, id: &Id) -> Result<()> {
        let (mut value, _lock) = self.claim(id)?;
        value.archived = true;
        self.save(&mut value)?;
        let mut index = self.index()?;
        if index.selected.as_ref() == Some(id) {
            index.selected = None;
            util::atomic_json(&self.index_path()?, &index)?;
        }
        Ok(())
    }
}

/// Recognize only the exact game directory produced by this library. Legacy API 1
/// instances have no parent instance.json and retain their existing target.
pub(crate) fn for_game(game: &Path) -> Result<Option<Instance>> {
    if game.file_name().and_then(|n| n.to_str()) != Some("game") {
        return Ok(None);
    }
    let container = game.parent().ok_or_else(io_error)?;
    if container
        .parent()
        .and_then(Path::file_name)
        .and_then(|n| n.to_str())
        != Some("instances")
        || container
            .parent()
            .and_then(Path::parent)
            .and_then(Path::file_name)
            .and_then(|n| n.to_str())
            != Some("library")
    {
        return Ok(None);
    }
    let manifest = util::join(container, "instance.json")?;
    if !manifest.is_file() {
        return Ok(None);
    }
    let value: Instance = util::read_json(&manifest)?;
    value.validate()?;
    if container.file_name().and_then(|n| n.to_str()) != Some(value.id.0.as_str()) || value.archived
    {
        return Err(invalid("Invalid or archived library instance."));
    }
    Ok(Some(value))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cancelled_duplicate_is_never_published() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(dir.path()).unwrap();
        let id = store
            .create(
                "Original".into(),
                Target::fabric(),
                Preset::Minimal,
                Mode::Custom,
            )
            .unwrap();
        let calls = std::cell::Cell::new(0);
        let result = store.duplicate(&id, "Cancelled".into(), &|| {
            calls.set(calls.get() + 1);
            if calls.get() >= 2 {
                Err(error(ErrorCode::Cancelled, "cancelled"))
            } else {
                Ok(())
            }
        });
        assert_eq!(result.unwrap_err().code, ErrorCode::Cancelled);
        assert_eq!(store.snapshot().unwrap().instances.len(), 1);
    }
    #[test]
    fn isolated_instances_preserve_player_files_and_archive_in_place() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(dir.path()).unwrap();
        let a = store
            .create("A".into(), Target::fabric(), Preset::Fps, Mode::Custom)
            .unwrap();
        let b = store
            .create("B".into(), Target::fabric(), Preset::Minimal, Mode::Custom)
            .unwrap();
        assert_ne!(store.game(&a).unwrap(), store.game(&b).unwrap());
        let options = store.game(&a).unwrap().join("options.txt");
        fs::write(&options, "preserve").unwrap();
        let mut instance = store.instance(&a).unwrap();
        instance.name = "renamed".into();
        store.save(&mut instance).unwrap();
        let c = store.duplicate(&a, "copy".into(), &|| Ok(())).unwrap();
        assert!(!store.game(&c).unwrap().join("options.txt").exists());
        store.archive(&a).unwrap();
        assert_eq!(fs::read_to_string(options).unwrap(), "preserve");
        assert!(store.instance(&a).unwrap().archived);
        assert_eq!(
            store.instance(&b).unwrap().profile.preset(),
            Preset::Minimal
        );
    }
    #[test]
    fn corrupt_and_future_indexes_are_preserved() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(dir.path()).unwrap();
        let raw = "{\"schema\":99,\"ids\":[],\"selected\":null}";
        fs::write(store.index_path().unwrap(), raw).unwrap();
        assert!(Store::open(dir.path()).is_err());
        assert_eq!(
            fs::read_to_string(store.index_path().unwrap()).unwrap(),
            raw
        );
    }
    #[test]
    fn active_instance_lock_prevents_content_mutation() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(dir.path()).unwrap();
        let id = store
            .create("A".into(), Target::fabric(), Preset::Fps, Mode::Custom)
            .unwrap();
        let (_, lock) = store.claim(&id).unwrap();
        assert_eq!(store.claim(&id).unwrap_err().code, ErrorCode::Busy);
        drop(lock);
        assert!(store.claim(&id).is_ok());
    }
}
