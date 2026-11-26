use crate::models::{LauncherProfilesFile, MinecraftProfile};
use std::fs;
use std::path::Path;

pub fn load_profiles(path: &Path) -> LauncherProfilesFile {
    fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save_profiles(path: &Path, data: &LauncherProfilesFile) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(data)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    fs::write(path, json)
}

pub fn upsert_profile(data: &mut LauncherProfilesFile, id: String, profile: MinecraftProfile) {
    data.selected_profile = Some(id.clone());
    data.profiles.insert(id, profile);
}
