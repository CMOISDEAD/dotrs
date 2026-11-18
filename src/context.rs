use std::{
    env,
    path::{Path, PathBuf},
};

use crate::{
    error::{Result, msg},
    paths::normalize,
    state::State,
};

#[derive(Debug, Clone)]
pub struct Context {
    pub home: PathBuf,
    pub dots: PathBuf,
    pub state: PathBuf,
    pub cwd: PathBuf,
}

impl Context {
    pub fn from_env() -> Result<Context> {
        let home = env::home_dir()
            .filter(|p| p.is_absolute())
            .ok_or_else(|| msg("cannot determine HOME"))?;

        let dots = match env::var_os("DOTRS_DIR").filter(|v| !v.is_empty()) {
            Some(dir) => normalize(&home.join(dir)),
            None => home.join("dots"),
        };

        let state_base = env::var_os("XDG_STATE_HOME")
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
            .unwrap_or_else(|| home.join(".local/state"));

        let cwd = env::current_dir().unwrap_or_else(|_| home.clone());

        Ok(Context {
            state: state_base.join("dotrs").join("state"),
            dots,
            cwd,
            home,
        })
    }

    pub fn with_home(home: &Path) -> Context {
        Context {
            home: home.to_path_buf(),
            dots: home.join("dots"),
            state: home.join(".local/state/dotrs/state"),
            cwd: home.to_path_buf(),
        }
    }

    pub fn load_state(&self) -> State {
        State::load(self.state.clone())
    }
}
