use std::{
    collections::BTreeMap,
    fs, io,
    path::{Path, PathBuf},
};

pub struct State {
    path: PathBuf,
    entries: BTreeMap<PathBuf, String>,
    dirty: bool,
}

impl State {
    pub fn load(path: PathBuf) -> State {
        let content = fs::read_to_string(&path).unwrap_or_default();
        let entries = content
            .lines()
            .filter_map(|line| {
                let (hash, file) = line.split_once("  ")?;
                Some((PathBuf::from(file), hash.to_string()))
            })
            .collect();
        State {
            path,
            entries,
            dirty: false,
        }
    }

    pub fn get(&self, relative: &Path) -> Option<&str> {
        self.entries.get(relative).map(String::as_str)
    }

    pub fn set(&mut self, relative: &Path, hash: &str) {
        if self.get(relative) != Some(hash) {
            self.entries
                .insert(relative.to_path_buf(), hash.to_string());
            self.dirty = true;
        }
    }

    pub fn remove(&mut self, relative: &Path) {
        if self.entries.remove(relative).is_some() {
            self.dirty = true;
        }
    }

    pub fn save(&mut self) -> io::Result<()> {
        if !self.dirty {
            return Ok(());
        }
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content: String = self
            .entries
            .iter()
            .map(|(file, hash)| format!("{hash}  {}\n", file.display()))
            .collect();
        let tmp = self.path.with_extension("tmp");
        fs::write(&tmp, content)?;
        fs::rename(&tmp, &self.path)?;
        self.dirty = false;
        Ok(())
    }
}
