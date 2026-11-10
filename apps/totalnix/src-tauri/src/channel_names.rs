//! User channel names (double-click a strip's name, manual §25.3).
//!
//! Display only: nothing is sent to the unit. Saved to
//! `$XDG_CONFIG_HOME/totalnix/names.json` (or `~/.config/...`). Source
//! strips are keyed by their first channel, outputs by stereo pair, so a
//! stereo strip split into two mono strips keeps its name on the left.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use mixer_core::{CHANNELS, OUTPUT_PAIRS};
use serde::{Deserialize, Serialize};

/// Longest name kept, in characters; strips show about 10.
pub const MAX_NAME_CHARS: usize = 24;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NameRow {
    Input,
    Playback,
    Output,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ChannelNames {
    #[serde(default)]
    inputs: BTreeMap<usize, String>,
    #[serde(default)]
    playbacks: BTreeMap<usize, String>,
    #[serde(default)]
    outputs: BTreeMap<usize, String>,
}

impl ChannelNames {
    /// The custom name for a source strip's first channel or an output pair.
    pub fn get(
        &self,
        row: NameRow,
        index: usize,
    ) -> Option<&str> {
        self.map(row).get(&index).map(String::as_str)
    }

    /// Sets or (with `None` or blank text) clears a name.
    pub fn set(
        &mut self,
        row: NameRow,
        index: usize,
        name: Option<&str>,
    ) -> Result<(), String> {
        let limit = match row {
            NameRow::Input | NameRow::Playback => CHANNELS,
            NameRow::Output => OUTPUT_PAIRS,
        };
        if index >= limit {
            return Err(format!("no channel {index} to rename"));
        }
        let cleaned = name.map(clean).unwrap_or_default();
        let map = self.map_mut(row);
        if cleaned.is_empty() {
            map.remove(&index);
        } else {
            map.insert(index, cleaned);
        }
        Ok(())
    }

    /// Loads saved names; a missing or unreadable file gives no names.
    pub fn load() -> Self {
        file_path()
            .map(|path| Self::load_from(&path))
            .unwrap_or_default()
    }

    pub fn save(&self) -> io::Result<()> {
        let path =
            file_path().ok_or_else(|| io::Error::other("no config directory (HOME unset)"))?;
        self.save_to(&path)
    }

    fn load_from(path: &Path) -> Self {
        fs::read_to_string(path)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default()
    }

    /// Saves atomically: write a temporary file, then rename it over.
    fn save_to(
        &self,
        path: &Path,
    ) -> io::Result<()> {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        let temporary = path.with_extension("json.tmp");
        let text = serde_json::to_string_pretty(self).map_err(io::Error::other)?;
        fs::write(&temporary, text)?;
        fs::rename(temporary, path)
    }

    fn map(
        &self,
        row: NameRow,
    ) -> &BTreeMap<usize, String> {
        match row {
            NameRow::Input => &self.inputs,
            NameRow::Playback => &self.playbacks,
            NameRow::Output => &self.outputs,
        }
    }

    fn map_mut(
        &mut self,
        row: NameRow,
    ) -> &mut BTreeMap<usize, String> {
        match row {
            NameRow::Input => &mut self.inputs,
            NameRow::Playback => &mut self.playbacks,
            NameRow::Output => &mut self.outputs,
        }
    }
}

fn clean(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(MAX_NAME_CHARS)
        .collect()
}

fn file_path() -> Option<PathBuf> {
    let config = std::env::var_os("XDG_CONFIG_HOME")
        .filter(|dir| !dir.is_empty())
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))?;
    Some(config.join("totalnix").join("names.json"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_cleaned_and_capped() {
        let mut names = ChannelNames::default();
        names
            .set(NameRow::Input, 0, Some("  Kick\tIn\n  "))
            .unwrap();
        assert_eq!(names.get(NameRow::Input, 0), Some("Kick In"));
        names
            .set(NameRow::Output, 0, Some(&"x".repeat(100)))
            .unwrap();
        assert_eq!(
            names.get(NameRow::Output, 0).unwrap().chars().count(),
            MAX_NAME_CHARS
        );
    }

    #[test]
    fn blank_or_none_clears() {
        let mut names = ChannelNames::default();
        names.set(NameRow::Playback, 2, Some("DAW")).unwrap();
        names.set(NameRow::Playback, 2, Some("   ")).unwrap();
        assert_eq!(names.get(NameRow::Playback, 2), None);
        names.set(NameRow::Playback, 2, Some("DAW")).unwrap();
        names.set(NameRow::Playback, 2, None).unwrap();
        assert_eq!(names.get(NameRow::Playback, 2), None);
    }

    #[test]
    fn out_of_range_is_rejected() {
        let mut names = ChannelNames::default();
        assert!(names.set(NameRow::Input, CHANNELS, Some("x")).is_err());
        assert!(names.set(NameRow::Output, OUTPUT_PAIRS, Some("x")).is_err());
    }

    #[test]
    fn round_trips_through_json() {
        let mut names = ChannelNames::default();
        names.set(NameRow::Input, 4, Some("Vox")).unwrap();
        names.set(NameRow::Output, 0, Some("Monitors")).unwrap();
        let text = serde_json::to_string(&names).unwrap();
        assert_eq!(serde_json::from_str::<ChannelNames>(&text).unwrap(), names);
    }

    #[test]
    fn saves_and_loads_a_file() {
        let dir = std::env::temp_dir().join(format!("totalnix-names-{}", std::process::id()));
        let path = dir.join("totalnix").join("names.json");
        let mut names = ChannelNames::default();
        names.set(NameRow::Input, 0, Some("Kick")).unwrap();
        names.save_to(&path).unwrap();
        assert_eq!(ChannelNames::load_from(&path), names);
        assert!(!path.with_extension("json.tmp").exists());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn missing_or_corrupt_file_gives_no_names() {
        let dir = std::env::temp_dir().join(format!("totalnix-corrupt-{}", std::process::id()));
        let path = dir.join("names.json");
        assert_eq!(ChannelNames::load_from(&path), ChannelNames::default());
        fs::create_dir_all(&dir).unwrap();
        fs::write(&path, "{ not json").unwrap();
        assert_eq!(ChannelNames::load_from(&path), ChannelNames::default());
        fs::remove_dir_all(dir).unwrap();
    }
}
