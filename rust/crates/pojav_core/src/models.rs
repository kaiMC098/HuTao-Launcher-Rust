use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RendererBackend {
    Opengles2,
    Opengles3,
    VulkanZink,
    Virgl,
    Osmesa,
}

impl RendererBackend {
    pub fn as_env_str(&self) -> &'static str {
        match self {
            Self::Opengles2 => "opengles2",
            Self::Opengles3 => "opengles3",
            Self::VulkanZink => "vulkan_zink",
            Self::Virgl => "virgl",
            Self::Osmesa => "osmesa",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LauncherSettings {
    pub renderer: RendererBackend,
    pub ram_allocation_mb: u32,
    pub resolution_scale: f32,
    pub custom_jvm_args: String,
    pub selected_runtime: String,
    pub force_vsync: bool,
    pub dump_shaders: bool,
    pub zink_prefer_system_driver: bool,
    pub control_scale: f32,
    pub gyro_enabled: bool,
    pub gyro_sensitivity: f32,
}

impl Default for LauncherSettings {
    fn default() -> Self {
        Self {
            renderer: RendererBackend::Opengles2,
            ram_allocation_mb: 1024,
            resolution_scale: 1.0,
            custom_jvm_args: "-XX:+UseG1GC -XX:+ParallelRefProcEnabled -XX:MaxGCPauseMillis=200".into(),
            selected_runtime: "Internal".into(),
            force_vsync: false,
            dump_shaders: false,
            zink_prefer_system_driver: false,
            control_scale: 1.0,
            gyro_enabled: false,
            gyro_sensitivity: 1.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinecraftAccount {
    pub username: String,
    pub uuid: String,
    pub access_token: String,
    pub xuid: String,
    pub client_token: String,
    pub is_microsoft: bool,
    pub expires_at_unix: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MinecraftProfile {
    pub name: String,
    #[serde(rename = "type", default = "default_profile_type")]
    pub profile_type: String,
    pub last_version_id: String,
    #[serde(default)]
    pub game_dir: Option<String>,
    #[serde(default)]
    pub java_dir: Option<String>,
    #[serde(default)]
    pub java_args: Option<String>,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub control_file: Option<String>,
    #[serde(default)]
    pub renderer: Option<String>,
}

fn default_profile_type() -> String {
    "custom".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct LauncherProfilesFile {
    #[serde(default)]
    pub profiles: HashMap<String, MinecraftProfile>,
    #[serde(default)]
    pub selected_profile: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlButtonData {
    pub name: String,
    #[serde(default)]
    pub keycodes: [i32; 4],
    pub dynamic_x: String,
    pub dynamic_y: String,
    pub width: f32,
    pub height: f32,
    #[serde(default = "default_opacity")]
    pub opacity: f32,
    #[serde(default)]
    pub is_toggle: bool,
    #[serde(default)]
    pub pass_thru_enabled: bool,
}

fn default_opacity() -> f32 {
    1.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlLayout {
    pub version: u32,
    #[serde(default = "default_scaled_at")]
    pub scaled_at: f32,
    #[serde(default)]
    pub m_control_data_list: Vec<ControlButtonData>,
}

fn default_scaled_at() -> f32 {
    100.0
}
