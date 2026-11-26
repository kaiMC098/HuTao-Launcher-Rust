use pojav_core::models::{LauncherProfilesFile, LauncherSettings, MinecraftAccount};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LauncherScreen {
    Home,
    Profiles,
    Accounts,
    ModBrowser,
    ControlEditor,
    Settings,
    ConsoleLog,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HuTaoThemePalette {
    pub theme_name: String,
    pub primary_crimson: String,
    pub bright_vermilion: String,
    pub lantern_gold: String,
    pub obsidian_bg: String,
    pub velvet_surface: String,
}

impl Default for HuTaoThemePalette {
    fn default() -> Self {
        Self {
            theme_name: "HuTao Wangsheng Pyro Edition".into(),
            primary_crimson: "#D93829".into(),
            bright_vermilion: "#F2523D".into(),
            lantern_gold: "#F5B041".into(),
            obsidian_bg: "#120A0C".into(),
            velvet_surface: "#231116".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiSnapshot {
    pub launcher_title: String,
    pub active_screen: LauncherScreen,
    pub theme: HuTaoThemePalette,
    pub settings: LauncherSettings,
    pub profiles: LauncherProfilesFile,
    pub active_account: Option<MinecraftAccount>,
    pub status_banner: String,
    pub download_progress: f32,
    pub is_launching: bool,
}

impl Default for UiSnapshot {
    fn default() -> Self {
        Self {
            launcher_title: "HuTao Launcher".into(),
            active_screen: LauncherScreen::Home,
            theme: HuTaoThemePalette::default(),
            settings: LauncherSettings::default(),
            profiles: LauncherProfilesFile::default(),
            active_account: None,
            status_banner: "HuTao Launcher sẵn sàng khởi chạy Minecraft Java Edition (Rust Engine)".into(),
            download_progress: 0.0,
            is_launching: false,
        }
    }
}

pub struct LauncherUiState {
    pub snapshot: UiSnapshot,
}

impl LauncherUiState {
    pub fn new() -> Self {
        Self {
            snapshot: UiSnapshot::default(),
        }
    }

    pub fn navigate(&mut self, screen: LauncherScreen) {
        self.snapshot.active_screen = screen;
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(&self.snapshot).unwrap_or_else(|_| "{}".into())
    }
}

impl Default for LauncherUiState {
    fn default() -> Self {
        Self::new()
    }
}
