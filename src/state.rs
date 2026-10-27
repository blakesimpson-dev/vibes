//! Where packs live, and the state file the CLI writes and the daemon follows.

use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::PathBuf;

const DEFAULT_PACK: &str = "nk-cream";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct State {
    pub enabled: bool,
    /// A folder name in the packs folder.
    pub pack: String,
    /// 0-100.
    pub volume: u8,
}

impl Default for State {
    fn default() -> Self {
        let packs = packs();
        let pack = if packs.iter().any(|p| p == DEFAULT_PACK) {
            DEFAULT_PACK.to_string()
        } else {
            packs.into_iter().next().unwrap_or_default()
        };
        State { enabled: true, pack, volume: 80 }
    }
}

fn home() -> PathBuf {
    PathBuf::from(std::env::var_os("HOME").expect("HOME is not set"))
}

fn xdg(var: &str, fallback: &str) -> PathBuf {
    std::env::var_os(var).map(PathBuf::from).unwrap_or_else(|| home().join(fallback))
}

/// $VIBES_PACKS, else $XDG_DATA_HOME/vibes/packs (~/.local/share/vibes/packs).
pub fn packs_dir() -> PathBuf {
    std::env::var_os("VIBES_PACKS")
        .map(PathBuf::from)
        .unwrap_or_else(|| xdg("XDG_DATA_HOME", ".local/share").join("vibes/packs"))
}

/// $XDG_STATE_HOME/vibes/state.json (~/.local/state/vibes/state.json).
pub fn state_path() -> PathBuf {
    xdg("XDG_STATE_HOME", ".local/state").join("vibes/state.json")
}

/// The packs installed: folders in the packs folder with a config.json, sorted.
pub fn packs() -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(packs_dir())
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.path().join("config.json").is_file())
        .filter_map(|e| e.file_name().into_string().ok())
        .collect();
    names.sort();
    names
}

impl State {
    /// The saved state, or the defaults if there is none (or it can't be read).
    pub fn load() -> State {
        fs::read_to_string(state_path())
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    /// Write it whole, then rename it into place, so the daemon never reads half a file.
    pub fn save(&self) -> io::Result<()> {
        let path = state_path();
        fs::create_dir_all(path.parent().unwrap())?;
        let tmp = path.with_extension("json.tmp");
        fs::write(&tmp, serde_json::to_string_pretty(self)? + "\n")?;
        fs::rename(tmp, path)
    }
}
