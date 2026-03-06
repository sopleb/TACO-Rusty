use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChannelConfig {
    pub name: String,
    pub prefix: String,
    #[serde(default = "default_true")]
    pub monitor: bool,
    #[serde(default = "default_true")]
    pub alert: bool,
    #[serde(default)]
    pub short_name: String,
}

fn default_true() -> bool {
    true
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TacoConfig {
    #[serde(default = "default_true")]
    pub preserve_window_position: bool,
    #[serde(default = "default_true")]
    pub preserve_window_size: bool,
    #[serde(default = "default_50")]
    pub window_position_x: i32,
    #[serde(default = "default_50")]
    pub window_position_y: i32,
    #[serde(default = "default_1253")]
    pub window_size_x: u32,
    #[serde(default = "default_815")]
    pub window_size_y: u32,
    #[serde(default = "default_true")]
    pub preserve_full_screen_status: bool,
    #[serde(default)]
    pub is_full_screen: bool,

    #[serde(default = "default_true")]
    pub preserve_home_system: bool,
    #[serde(default = "default_771")]
    pub home_system_id: i32,

    #[serde(default = "default_true")]
    pub monitor_game_log: bool,

    #[serde(default = "default_true")]
    pub preserve_camera_distance: bool,
    #[serde(default = "default_true")]
    pub preserve_look_at: bool,
    #[serde(default = "default_700")]
    pub camera_distance: f32,
    #[serde(default = "default_neg1416")]
    pub look_at_x: f32,
    #[serde(default = "default_3702")]
    pub look_at_y: f32,

    #[serde(default)]
    pub override_log_path: bool,
    #[serde(default)]
    pub log_path: String,

    #[serde(default = "default_true")]
    pub preserve_selected_systems: bool,
    #[serde(default)]
    pub selected_systems: Vec<usize>,

    #[serde(default)]
    pub landmark_systems: Vec<usize>,

    #[serde(default = "default_true")]
    pub display_character_names: bool,
    #[serde(default = "default_true")]
    pub show_character_locations: bool,

    #[serde(default)]
    pub map_range_from: u32,
    #[serde(default = "default_3d")]
    pub map_mode: String,

    #[serde(default = "default_true")]
    pub show_alert_age: bool,
    #[serde(default = "default_10")]
    pub max_alert_age: u32,
    #[serde(default = "default_20")]
    pub max_alerts: usize,

    #[serde(default = "default_8")]
    pub map_text_size: u32,
    #[serde(default = "default_1f")]
    pub scroll_sensitivity: f32,
    #[serde(default = "default_max_intel_messages")]
    pub max_intel_messages: usize,

    #[serde(default)]
    pub dark_mode: bool,
    #[serde(default)]
    pub persistent_system_labels: bool,
    #[serde(default = "default_popup_x")]
    pub alert_popup_x: f32,
    #[serde(default = "default_popup_y")]
    pub alert_popup_y: f32,

    #[serde(default)]
    pub custom_channels: Vec<ChannelConfig>,
    #[serde(default)]
    pub alert_triggers: Vec<serde_json::Value>,
    #[serde(default)]
    pub ignore_strings: Vec<String>,
    #[serde(default)]
    pub ignore_systems: Vec<usize>,
    #[serde(default)]
    pub monitored_systems: Vec<usize>,
}

fn default_50() -> i32 {
    50
}
fn default_1253() -> u32 {
    1253
}
fn default_815() -> u32 {
    815
}
fn default_771() -> i32 {
    771
}
fn default_700() -> f32 {
    700.0
}
fn default_neg1416() -> f32 {
    -1416.0
}
fn default_3702() -> f32 {
    3702.0
}
fn default_3d() -> String {
    "3d".to_string()
}
fn default_10() -> u32 {
    10
}
fn default_20() -> usize {
    20
}
fn default_8() -> u32 {
    8
}
fn default_1f() -> f32 {
    1.0
}

fn default_max_intel_messages() -> usize {
    100
}
fn default_popup_x() -> f32 {
    100.0
}
fn default_popup_y() -> f32 {
    100.0
}

impl Default for TacoConfig {
    fn default() -> Self {
        Self {
            preserve_window_position: true,
            preserve_window_size: true,
            window_position_x: 50,
            window_position_y: 50,
            window_size_x: 1253,
            window_size_y: 815,
            preserve_full_screen_status: true,
            is_full_screen: false,
            preserve_home_system: true,
            home_system_id: 771,
            monitor_game_log: true,
            preserve_camera_distance: true,
            preserve_look_at: true,
            camera_distance: 700.0,
            look_at_x: -1416.0,
            look_at_y: 3702.0,
            override_log_path: false,
            log_path: String::new(),
            preserve_selected_systems: true,
            selected_systems: Vec::new(),
            landmark_systems: Vec::new(),
            display_character_names: true,
            show_character_locations: true,
            map_range_from: 0,
            map_mode: "3d".to_string(),
            show_alert_age: true,
            max_alert_age: 10,
            max_alerts: 20,
            map_text_size: 8,
            scroll_sensitivity: 1.0,
            max_intel_messages: 100,
            dark_mode: false,
            persistent_system_labels: false,
            alert_popup_x: 100.0,
            alert_popup_y: 100.0,
            custom_channels: Vec::new(),
            alert_triggers: Vec::new(),
            ignore_strings: Vec::new(),
            ignore_systems: Vec::new(),
            monitored_systems: Vec::new(),
        }
    }
}

impl TacoConfig {
    pub fn get_config_dir() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| dirs::home_dir().unwrap_or_default().join(".config"))
            .join("taco")
    }

    pub fn get_config_path() -> PathBuf {
        Self::get_config_dir().join("taco.json")
    }

    pub fn save(&self) {
        let path = Self::get_config_path();
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        match serde_json::to_string_pretty(self) {
            Ok(json) => {
                if let Err(e) = fs::write(&path, json) {
                    log::error!("Failed to save config: {}", e);
                }
            }
            Err(e) => log::error!("Failed to serialize config: {}", e),
        }
    }

    pub fn export_to(&self) {
        let Some(path) = rfd::FileDialog::new()
            .set_title("Export TACO Profile")
            .set_file_name("taco_profile.json")
            .add_filter("JSON", &["json"])
            .set_directory(Self::get_config_dir())
            .save_file()
        else {
            return;
        };
        match serde_json::to_string_pretty(self) {
            Ok(json) => {
                if let Err(e) = fs::write(&path, json) {
                    log::error!("Failed to export config: {}", e);
                }
            }
            Err(e) => log::error!("Failed to serialize config for export: {}", e),
        }
    }

    pub fn import_from() -> Option<Self> {
        let path = rfd::FileDialog::new()
            .set_title("Import TACO Profile")
            .add_filter("JSON", &["json"])
            .set_directory(Self::get_config_dir())
            .pick_file()?;
        match fs::read_to_string(&path) {
            Ok(content) => match serde_json::from_str(&content) {
                Ok(config) => {
                    let cfg: Self = config;
                    cfg.save();
                    Some(cfg)
                }
                Err(e) => {
                    log::warn!("Failed to parse export file: {}", e);
                    None
                }
            },
            Err(e) => {
                log::warn!("Failed to read export file: {}", e);
                None
            }
        }
    }

    pub fn load() -> Self {
        let path = Self::get_config_path();
        if path.exists() {
            match fs::read_to_string(&path) {
                Ok(content) => match serde_json::from_str(&content) {
                    Ok(config) => return config,
                    Err(e) => {
                        log::warn!("Failed to parse config ({}); using defaults", e);
                    }
                },
                Err(e) => {
                    log::warn!("Failed to read config ({}); using defaults", e);
                }
            }
        }
        let config = Self::default();
        config.save();
        config
    }
}
