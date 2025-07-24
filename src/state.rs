use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

const STATE_FILE: &str = ".flatplay/state.json";

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct State {
    pub active_manifest: Option<PathBuf>,
    pub dependencies_updated: bool,
    pub dependencies_built: bool,
    pub application_built: bool,
}

impl State {
    pub fn load() -> Result<Self> {
        if !Path::new(STATE_FILE).exists() {
            return Ok(State::default());
        }
        let content = fs::read_to_string(STATE_FILE)?;
        let state = serde_json::from_str(&content)?;
        Ok(state)
    }

    pub fn save(&self) -> Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        fs::create_dir_all(".flatplay")?;
        fs::write(STATE_FILE, content)?;
        Ok(())
    }

    pub fn reset(&mut self) {
        self.dependencies_updated = false;
        self.dependencies_built = false;
        self.application_built = false;
    }
}
