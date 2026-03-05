use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::resources;

use chrono::Utc;
use eframe::egui;
use regex::Regex;

use crate::audio::sound_manager::SoundManager;
use crate::config::taco_config::TacoConfig;
use crate::core::alert_trigger::{AlertTrigger, AlertType, RangeAlertOperator, RangeAlertType};
use crate::core::path_info::generate_unique_path_id;
use crate::core::solar_system_manager::{SolarSystemManager, SystemDataJson};
use crate::intel::log_entry::{LogEntry, LogEntryType, LogFileType};
use crate::intel::log_watcher::{LogWatcher, WatcherMessage};
use crate::intel::local_watcher::LocalWatcher;
use crate::ui::config_panel::{ConfigEvent, ConfigPanel};
use crate::ui::gl_map::{GlMap, MapLabelText};
use crate::ui::intel_panel::IntelPanel;

pub struct TacoApp {
    config: TacoConfig,
    manager: Arc<Mutex<SolarSystemManager>>,
    sound_manager: SoundManager,

    gl_map: Arc<Mutex<GlMap>>,
    intel_panel: IntelPanel,
    config_panel: ConfigPanel,
    search_text: String,
    status_message: String,
    process_logs: bool,
    mute_sound: bool,
    show_right_panel: bool,
    show_settings: bool,

    log_watchers: HashMap<String, LogWatcher>,
    local_watcher: Option<LocalWatcher>,

    char_locations: HashMap<String, i32>,
    char_locations_dirty: bool,
    followed_chars: HashSet<String>,

    alert_triggers: Vec<AlertTrigger>,
    ignore_strings: Vec<Regex>,
    ignore_systems: Vec<usize>,
    sticky_highlights: HashSet<usize>,
    right_click_system: Option<usize>,

    refocus_index: usize,
    is_fullscreen: bool,
    theme_applied: bool,

    last_tick: Instant,
    last_watcher_tick: Instant,

    gl_initialized: bool,
}

impl TacoApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let config = TacoConfig::load();
        let mut manager = SolarSystemManager::new();
        let sound_manager = SoundManager::new();

        if let Ok(data) = serde_json::from_str::<Vec<SystemDataJson>>(resources::SYSTEMDATA_JSON) {
            manager.load_system_data(data);
        }

        manager.load_region_names(resources::REGIONS_JSON);

        manager.init_vbo_data();

        if config.map_mode == "2d" {
            manager.set_map_mode(true);
        }

        let alert_triggers: Vec<AlertTrigger> = config
            .alert_triggers
            .iter()
            .filter_map(|v| serde_json::from_value(v.clone()).ok())
            .collect();

        let ignore_strings: Vec<Regex> = config
            .ignore_strings
            .iter()
            .filter_map(|s| {
                let pattern = format!(r"(?i)\b{}\b", regex::escape(s));
                Regex::new(&pattern).ok()
            })
            .collect();

        if config.preserve_home_system && config.home_system_id != -1 {
            manager.set_current_home_system(config.home_system_id);
        }

        for &sys_id in &config.monitored_systems {
            if sys_id != manager.home_system_id as usize {
                manager.add_green_crosshair(sys_id);
            }
        }

        manager.max_alert_age = config.max_alert_age;
        manager.max_alerts = config.max_alerts;

        let ignore_systems = config.ignore_systems.clone();

        let mut gl_map = GlMap::new();
        gl_map.camera_distance = config.camera_distance;
        gl_map.look_at[0] = config.look_at_x;
        gl_map.look_at[1] = config.look_at_y;
        gl_map.map_text_size = config.map_text_size;
        gl_map.scroll_sensitivity = config.scroll_sensitivity;
        gl_map.persistent_labels = config.persistent_system_labels;
        gl_map.show_alert_age = config.show_alert_age;
        gl_map.display_char_names = config.display_character_names;
        gl_map.show_char_locations = config.show_character_locations;
        gl_map.landmark_systems = config.landmark_systems.iter().copied().collect();
        gl_map.map_mode_2d = config.map_mode == "2d";

        let mut intel_panel = IntelPanel::new();
        intel_panel.max_messages = config.max_intel_messages;
        for ch in &config.custom_channels {
            intel_panel.add_channel_tab(&ch.name);
        }

        Self {
            config,
            manager: Arc::new(Mutex::new(manager)),
            sound_manager,
            gl_map: Arc::new(Mutex::new(gl_map)),
            intel_panel,
            config_panel: ConfigPanel::new(),
            search_text: String::new(),
            status_message: "Ready".to_string(),
            process_logs: false,
            mute_sound: false,
            show_right_panel: true,
            show_settings: false,
            log_watchers: HashMap::new(),
            local_watcher: None,
            char_locations: HashMap::new(),
            char_locations_dirty: true,
            followed_chars: HashSet::new(),
            alert_triggers,
            ignore_strings,
            ignore_systems,
            sticky_highlights: HashSet::new(),
            right_click_system: None,
            refocus_index: 0,
            is_fullscreen: false,
            theme_applied: false,
            last_tick: Instant::now(),
            last_watcher_tick: Instant::now(),
            gl_initialized: false,
        }
    }

    fn start_logs(&mut self) {
        let log_path = if self.config.override_log_path {
            Some(PathBuf::from(&self.config.log_path))
        } else {
            None
        };

        let channels: Vec<_> = self.config.custom_channels.clone();
        for ch in &channels {
            if !ch.monitor {
                continue;
            }
            let mut watcher = LogWatcher::new(&ch.prefix, LogFileType::Chat, log_path.clone());
            if watcher.start() {
                let messages = watcher.init();
                self.process_watcher_messages(messages, &ch.name);
                self.write_system_intel(&format!(
                    "Monitoring channel: {} ({})",
                    ch.name, ch.prefix
                ));
            } else {
                self.write_system_intel(&format!(
                    "WARNING: Could not find logs for: {} ({})",
                    ch.name, ch.prefix
                ));
            }
            self.log_watchers.insert(ch.name.clone(), watcher);
        }

        if self.config.monitor_game_log {
            let mut watcher = LogWatcher::new("", LogFileType::Game, log_path.clone());
            if watcher.start() {
                let messages = watcher.init();
                self.process_watcher_messages(messages, "Game");
            }
            self.log_watchers.insert("__game__".to_string(), watcher);
        }

        let mut local = LocalWatcher::new(log_path);
        if local.start() {
            let changes = local.init();
            for change in changes {
                self.on_system_change(&change.system_name, &change.char_name);
            }
        }
        self.local_watcher = Some(local);

        self.process_logs = true;
        self.status_message = "Monitoring...".to_string();
        self.write_system_intel("Log monitoring started");
    }

    fn stop_logs(&mut self) {
        for watcher in self.log_watchers.values_mut() {
            watcher.stop();
        }
        self.log_watchers.clear();
        if let Some(ref mut local) = self.local_watcher {
            local.stop();
        }
        self.local_watcher = None;
        self.process_logs = false;
        self.status_message = "Stopped".to_string();
        self.write_system_intel("Log monitoring stopped");
    }

    fn tick_watchers(&mut self) {
        if !self.process_logs {
            return;
        }

        let channel_names: Vec<String> = self.log_watchers.keys().cloned().collect();
        for name in channel_names {
            if let Some(watcher) = self.log_watchers.get_mut(&name) {
                let messages = watcher.tick();
                if !messages.is_empty() {
                    let display_name = if name == "__game__" {
                        "Game".to_string()
                    } else {
                        name.clone()
                    };
                    self.process_watcher_messages(messages, &display_name);
                }
            }
        }

        if let Some(ref mut local) = self.local_watcher {
            let changes = local.tick();
            for change in changes {
                self.on_system_change(&change.system_name, &change.char_name);
            }
        }
    }

    fn process_watcher_messages(&mut self, messages: Vec<WatcherMessage>, channel: &str) {
        for msg in messages {
            match msg {
                WatcherMessage::LogEntry(entry) => self.on_new_log_entry(entry, channel),
                WatcherMessage::CombatEvent {
                    char_name,
                    event_type,
                } => {
                    let action = match event_type {
                        crate::intel::log_entry::CombatEventType::Start => "started",
                        crate::intel::log_entry::CombatEventType::Stop => "stopped",
                    };
                    let location = self.char_locations.get(&char_name)
                        .and_then(|&loc| {
                            if loc >= 0 {
                                let mgr = self.manager.lock().unwrap();
                                if (loc as usize) < mgr.solar_systems.len() {
                                    Some(mgr.solar_systems[loc as usize].name.clone())
                                } else { None }
                            } else { None }
                        });
                    let msg = if let Some(sys_name) = location {
                        format!("Combat {}: {} in {}", action, char_name, sys_name)
                    } else {
                        format!("Combat {}: {}", action, char_name)
                    };
                    self.write_system_intel(&msg);
                }
            }
        }
    }

    fn on_new_log_entry(&mut self, entry: LogEntry, channel: &str) {
        match entry.entry_type {
            LogEntryType::NewChatLog | LogEntryType::OpenChatLog => {
                if !entry.character_name.is_empty()
                    && !self.char_locations.contains_key(&entry.character_name)
                {
                    self.char_locations.insert(entry.character_name.clone(), -1);
                    self.char_locations_dirty = true;
                }
                return;
            }
            LogEntryType::ChatEvent => {}
            _ => return,
        }

        if !entry.parse_success || entry.log_type == LogFileType::Game {
            return;
        }

        let content = &entry.line_content;

        for pattern in &self.ignore_strings {
            if pattern.is_match(content) {
                return;
            }
        }

        let manager = self.manager.lock().unwrap();
        let content_lower = content.to_lowercase();
        let mut matched_systems = Vec::new();
        for i in 0..manager.system_count() {
            if self.ignore_systems.contains(&i) {
                continue;
            }
            if manager.solar_systems[i].match_name_in_lower(&content_lower) {
                matched_systems.push(i);
            }
        }

        let time_str = if entry.log_time.is_empty() {
            chrono::Local::now().format("%H:%M:%S").to_string()
        } else {
            entry.log_time.clone()
        };
        let player = if entry.player_name.is_empty() {
            "?".to_string()
        } else {
            entry.player_name.clone()
        };
        let display = format!("[{}] {} > {}", time_str, player, content);
        let system_names: Vec<String> = matched_systems
            .iter()
            .filter_map(|&sid| {
                if sid < manager.solar_systems.len() {
                    Some(manager.solar_systems[sid].name.clone())
                } else {
                    None
                }
            })
            .collect();
        drop(manager);

        self.intel_panel
            .write_intel(channel, &display, system_names);

        if matched_systems.is_empty() {
            return;
        }

        {
            let mut manager = self.manager.lock().unwrap();
            for &sys_id in &matched_systems {
                manager.add_alert(sys_id, Some(content));
            }
        }

        let mut sounds_to_play: Vec<(i32, String)> = Vec::new();
        let mut best_ranged: Option<(i32, usize, usize, String)> = None;
        let content_lower = content.to_lowercase();

        for &sys_id in &matched_systems {
            for trigger in &mut self.alert_triggers {
                if !trigger.enabled || trigger.alert_type != AlertType::Custom {
                    continue;
                }
                if !trigger.text.is_empty()
                    && content_lower.contains(&trigger.text.to_lowercase())
                {
                    let now = Utc::now();
                    let should_fire = trigger.repeat_interval == 0
                        || trigger
                            .trigger_time
                            .is_none_or(|t| (now - t).num_seconds() > trigger.repeat_interval as i64);
                    if should_fire {
                        trigger.trigger_time = Some(now);
                        sounds_to_play.push((trigger.sound_id, trigger.sound_path.clone()));
                    }
                }
            }

            for i in 0..self.alert_triggers.len() {
                let trigger = &self.alert_triggers[i];
                if !trigger.enabled || trigger.alert_type != AlertType::Ranged {
                    continue;
                }
                if let Some((jumps, ref_name)) = self.find_closest_for_trigger(trigger, sys_id) {
                    if best_ranged.is_none() || jumps < best_ranged.as_ref().unwrap().0 {
                        best_ranged = Some((jumps, i, sys_id, ref_name));
                    }
                }
            }
        }

        for (sound_id, sound_path) in &sounds_to_play {
            Self::play_alert_sound(*sound_id, sound_path, &self.sound_manager, self.mute_sound);
        }

        if let Some((jumps, trigger_idx, sys_id, ref_name)) = best_ranged {
            let trigger = &mut self.alert_triggers[trigger_idx];
            let now = Utc::now();
            let interval = trigger.repeat_interval.max(5) as i64;
            let should_fire = trigger.repeat_interval == 0
                || trigger
                    .trigger_time
                    .is_none_or(|t| (now - t).num_seconds() > interval);
            if should_fire {
                trigger.trigger_time = Some(now);
                let sound_id = trigger.sound_id;
                let sound_path = trigger.sound_path.clone();
                Self::play_alert_sound(
                    sound_id,
                    &sound_path,
                    &self.sound_manager,
                    self.mute_sound,
                );
                let manager = self.manager.lock().unwrap();
                let sys_name = if sys_id < manager.solar_systems.len() {
                    manager.solar_systems[sys_id].name.clone()
                } else {
                    "?".to_string()
                };
                drop(manager);
                let jump_label = if jumps == 1 { "jump" } else { "jumps" };
                let jump_info = format!("{} {} from {}", jumps, jump_label, ref_name);
                self.intel_panel.write_intel_with_jump(
                    channel,
                    &format!(
                        "  ** ALERT: {} - {} **",
                        sys_name, jump_info
                    ),
                    vec![sys_name],
                    Some(jump_info),
                );
            }
        }
    }

    fn play_alert_sound(
        sound_id: i32,
        sound_path: &str,
        sound_manager: &SoundManager,
        muted: bool,
    ) {
        if muted {
            return;
        }
        if sound_id >= 0 {
            if !sound_manager.play_sound_by_id(sound_id) {
                sound_manager.play_custom_sound(sound_path);
            }
        } else if !sound_path.is_empty() {
            sound_manager.play_custom_sound(sound_path);
        } else {
            sound_manager.play_sound_by_id(0);
        }
    }

    fn find_closest_for_trigger(
        &self,
        trigger: &AlertTrigger,
        system_id: usize,
    ) -> Option<(i32, String)> {
        let mut manager = self.manager.lock().unwrap();
        let mut candidates = Vec::new();

        match trigger.range_to {
            RangeAlertType::Home => {
                if self.config.map_range_from == 1 {
                    // Range from character locations
                    for (name, &loc) in &self.char_locations {
                        if loc >= 0 {
                            candidates.push((loc as usize, name.clone()));
                        }
                    }
                }
                if candidates.is_empty() {
                    for &green_id in manager.green_crosshair_ids.iter() {
                        let name = if green_id < manager.solar_systems.len() {
                            manager.solar_systems[green_id].name.clone()
                        } else {
                            "home".to_string()
                        };
                        candidates.push((green_id, name));
                    }
                }
            }
            RangeAlertType::System => {
                if trigger.system_id >= 0 {
                    candidates.push((
                        trigger.system_id as usize,
                        trigger.system_name.clone(),
                    ));
                }
            }
            RangeAlertType::AnyCharacter => {
                for (name, &loc) in &self.char_locations {
                    if loc >= 0 {
                        candidates.push((loc as usize, name.clone()));
                    }
                }
            }
            RangeAlertType::AnyFollowedCharacter => {
                for name in &self.followed_chars {
                    if let Some(&loc) = self.char_locations.get(name) {
                        if loc >= 0 {
                            candidates.push((loc as usize, name.clone()));
                        }
                    }
                }
            }
            _ => {}
        }

        let mut best: Option<(i32, String)> = None;
        for (target_id, ref_name) in candidates {
            if let Some(jumps) = self.check_range_match_with_manager(&mut manager, trigger, system_id, target_id) {
                if best.is_none() || jumps < best.as_ref().unwrap().0 {
                    best = Some((jumps, ref_name));
                }
            }
        }
        best
    }

    fn check_range_match_with_manager(
        &self,
        manager: &mut SolarSystemManager,
        trigger: &AlertTrigger,
        system_id: usize,
        target: usize,
    ) -> Option<i32> {
        let path_id = generate_unique_path_id(target, system_id);
        let jumps = if let Some(path) = manager.pathfinding_cache.get(&path_id) {
            path.total_jumps
        } else if let Some(result) = manager.find_path(target, system_id) {
            result.total_jumps
        } else {
            return None;
        };

        if jumps < 0 {
            return None;
        }

        let upper_ok = match trigger.upper_limit_operator {
            RangeAlertOperator::Equal => jumps == trigger.upper_range,
            RangeAlertOperator::LessThanOrEqual => jumps <= trigger.upper_range,
            RangeAlertOperator::LessThan => jumps < trigger.upper_range,
            _ => false,
        };
        if !upper_ok {
            return None;
        }

        let lower_ok = if trigger.lower_range > 0 {
            match trigger.lower_limit_operator {
                RangeAlertOperator::GreaterThanOrEqual => jumps >= trigger.lower_range,
                RangeAlertOperator::GreaterThan => jumps > trigger.lower_range,
                _ => true,
            }
        } else {
            true
        };

        if upper_ok && lower_ok {
            Some(jumps)
        } else {
            None
        }
    }

    fn on_system_change(&mut self, system_name: &str, char_name: &str) {
        if char_name.is_empty() {
            return;
        }
        self.char_locations
            .entry(char_name.to_string())
            .or_insert(-1);

        let mut sys_id: i32 = -1;
        {
            let manager = self.manager.lock().unwrap();
            if let Some(&id) = manager.names.get(system_name) {
                sys_id = id as i32;
            } else if let Ok(native_id) = system_name.parse::<u32>() {
                for (i, sys) in manager.solar_systems.iter().enumerate() {
                    if sys.native_id == native_id {
                        sys_id = i as i32;
                        break;
                    }
                }
            }
        }

        if sys_id >= 0 {
            self.char_locations.insert(char_name.to_string(), sys_id);
            self.char_locations_dirty = true;

            {
                let mut manager = self.manager.lock().unwrap();
                manager.set_character_location(sys_id);

                if self.config.show_character_locations {
                    let resolved: Vec<usize> = self
                        .char_locations
                        .values()
                        .filter(|&&v| v >= 0)
                        .map(|&v| v as usize)
                        .collect();
                    manager.set_character_location_systems(resolved);
                }
            }

            let name = {
                let manager = self.manager.lock().unwrap();
                if (sys_id as usize) < manager.solar_systems.len() {
                    manager.solar_systems[sys_id as usize].name.clone()
                } else {
                    system_name.to_string()
                }
            };

            if self.followed_chars.contains(char_name) {
                let manager = self.manager.lock().unwrap();
                let mut gl_map = self.gl_map.lock().unwrap();
                gl_map.pan_to_system(sys_id as usize, &manager.solar_systems);
            }

            self.write_system_intel(&format!("{} moved to {}", char_name, name));
            self.status_message = format!("{}: {}", char_name, name);
        }
    }

    fn write_system_intel(&mut self, text: &str) {
        let time_str = chrono::Local::now().format("%H:%M:%S").to_string();
        self.intel_panel
            .write_intel("System", &format!("[{}] {}", time_str, text), vec![]);
    }

    fn on_search(&mut self) {
        let name = self.search_text.trim().to_string();
        if name.is_empty() {
            return;
        }
        let name_lower = name.to_lowercase();
        let manager = self.manager.lock().unwrap();
        let mut sys_id: Option<usize> = None;
        for (n, &sid) in &manager.names {
            if n.to_lowercase() == name_lower {
                sys_id = Some(sid);
                break;
            }
        }
        if let Some(sid) = sys_id {
            let mut gl_map = self.gl_map.lock().unwrap();
            gl_map.zoom_to_system(sid, &manager.solar_systems);
            drop(gl_map);
            drop(manager);
            self.manager.lock().unwrap().add_highlight(sid, true);
            self.search_text.clear();
        } else {
            self.status_message = format!("System \"{}\" not found", name);
        }
    }
}

impl eframe::App for TacoApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let repaint_ms = if ctx.input(|i| i.pointer.any_down() || i.smooth_scroll_delta.length() > 0.0) {
            33
        } else {
            66
        };
        ctx.request_repaint_after(Duration::from_millis(repaint_ms));

        if !self.theme_applied {
            if self.config.dark_mode {
                ctx.set_visuals(egui::Visuals::dark());
            } else {
                ctx.set_visuals(egui::Visuals::light());
            }
            self.theme_applied = true;
        }

        ctx.input(|i| {
            let cmd = if cfg!(target_os = "macos") { i.modifiers.mac_cmd } else { i.modifiers.ctrl };
            if cmd && i.key_pressed(egui::Key::Q) {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            let fullscreen_pressed = if cfg!(target_os = "macos") {
                i.modifiers.ctrl && i.key_pressed(egui::Key::F11)
            } else {
                i.key_pressed(egui::Key::F11)
            };
            if fullscreen_pressed {
                self.is_fullscreen = !self.is_fullscreen;
                ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(self.is_fullscreen));
            }
            if i.key_pressed(egui::Key::Escape) && self.is_fullscreen {
                self.is_fullscreen = false;
                ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(false));
            }
            if cmd && i.key_pressed(egui::Key::H) {
                self.show_right_panel = !self.show_right_panel;
            }
        });

        let now = Instant::now();
        if now.duration_since(self.last_tick) >= Duration::from_millis(33) {
            self.last_tick = now;
            let mut manager = self.manager.lock().unwrap();
            let has_anims = manager.incoming_tick();
            if has_anims {
                manager.build_uniforms();
                self.gl_map.lock().unwrap().invalidate_uniforms();
            }
            manager.process_pathfinding_queue();
            manager.remove_expired_alerts();
        }

        if now.duration_since(self.last_watcher_tick) >= Duration::from_millis(250) {
            self.last_watcher_tick = now;
            self.tick_watchers();
        }

        let events = self.config_panel.drain_events();
        for event in events {
            match event {
                ConfigEvent::ConfigChanged => {
                    let mut manager = self.manager.lock().unwrap();
                    manager.max_alert_age = self.config.max_alert_age;
                    manager.max_alerts = self.config.max_alerts;
                    let mut gl_map = self.gl_map.lock().unwrap();
                    gl_map.show_alert_age = self.config.show_alert_age;
                    gl_map.display_char_names = self.config.display_character_names;
                }
                ConfigEvent::DarkModeChanged(dark) => {
                    if dark {
                        ctx.set_visuals(egui::Visuals::dark());
                    } else {
                        ctx.set_visuals(egui::Visuals::light());
                    }
                }
                ConfigEvent::PersistentLabelsChanged(v) => {
                    self.gl_map.lock().unwrap().persistent_labels = v;
                }
                ConfigEvent::MapTextSizeChanged(v) => {
                    self.gl_map.lock().unwrap().map_text_size = v;
                }
                ConfigEvent::ScrollSensitivityChanged(v) => {
                    self.gl_map.lock().unwrap().scroll_sensitivity = v;
                }
                ConfigEvent::MaxIntelMessagesChanged(v) => {
                    self.intel_panel.max_messages = v;
                }
                ConfigEvent::MapModeChanged(mode) => {
                    let is_2d = mode == "2d";
                    self.manager.lock().unwrap().set_map_mode(is_2d);
                    self.gl_map.lock().unwrap().map_mode_2d = is_2d;
                }
                ConfigEvent::ChannelAdded(name, prefix) => {
                    self.intel_panel.add_channel_tab(&name);
                    if self.process_logs {
                        let log_path = if self.config.override_log_path {
                            Some(PathBuf::from(&self.config.log_path))
                        } else {
                            None
                        };
                        let mut watcher = LogWatcher::new(&prefix, LogFileType::Chat, log_path);
                        watcher.start();
                        let msgs = watcher.init();
                        self.process_watcher_messages(msgs, &name);
                        self.log_watchers.insert(name.clone(), watcher);
                    }
                }
                ConfigEvent::ChannelRemoved(name) => {
                    if let Some(mut w) = self.log_watchers.remove(&name) {
                        w.stop();
                    }
                    self.intel_panel.remove_channel_tab(&name);
                }
                ConfigEvent::IgnoreStringAdded(s) => {
                    self.config.ignore_strings.push(s.clone());
                    let pattern = format!(r"(?i)\b{}\b", regex::escape(&s));
                    if let Ok(re) = Regex::new(&pattern) {
                        self.ignore_strings.push(re);
                    }
                    self.config.save();
                }
                ConfigEvent::IgnoreStringRemoved(idx) => {
                    if idx < self.config.ignore_strings.len() {
                        self.config.ignore_strings.remove(idx);
                        if idx < self.ignore_strings.len() {
                            self.ignore_strings.remove(idx);
                        }
                        self.config.save();
                    }
                }
                ConfigEvent::IgnoreSystemRemoved(sys_id) => {
                    self.ignore_systems.retain(|&x| x != sys_id);
                    self.config.ignore_systems.retain(|&x| x != sys_id);
                    self.config.save();
                }
                ConfigEvent::LandmarkAdded(sys_id) => {
                    if !self.config.landmark_systems.contains(&sys_id) {
                        self.config.landmark_systems.push(sys_id);
                        self.gl_map.lock().unwrap().landmark_systems.insert(sys_id);
                        self.config.save();
                    }
                }
                ConfigEvent::LandmarkRemoved(sys_id) => {
                    self.config.landmark_systems.retain(|&x| x != sys_id);
                    self.gl_map.lock().unwrap().landmark_systems.remove(&sys_id);
                    self.config.save();
                }
                ConfigEvent::AlertTriggerUpdated => {
                    self.config.alert_triggers = self.alert_triggers.iter()
                        .filter_map(|t| serde_json::to_value(t).ok())
                        .collect();
                    self.config.save();
                }
                ConfigEvent::AlertTriggerRemoved(idx) => {
                    if idx < self.alert_triggers.len() {
                        self.alert_triggers.remove(idx);
                        self.config.alert_triggers = self.alert_triggers.iter()
                            .filter_map(|t| serde_json::to_value(t).ok())
                            .collect();
                        self.config.save();
                    }
                }
                ConfigEvent::AlertTriggerMoved(idx, up) => {
                    if up && idx > 0 {
                        self.alert_triggers.swap(idx, idx - 1);
                    } else if !up && idx + 1 < self.alert_triggers.len() {
                        self.alert_triggers.swap(idx, idx + 1);
                    }
                    self.config.alert_triggers = self.alert_triggers.iter()
                        .filter_map(|t| serde_json::to_value(t).ok())
                        .collect();
                    self.config.save();
                }
                ConfigEvent::PlayTestSound(sound_id, sound_path) => {
                    log::info!("PlayTestSound: id={}, path={}", sound_id, sound_path);
                    Self::play_alert_sound(sound_id, &sound_path, &self.sound_manager, false);
                }
                ConfigEvent::ExportProfile => {
                    self.config.export_to();
                    self.write_system_intel("Profile exported");
                }
                ConfigEvent::ImportProfile => {
                    if let Some(imported) = TacoConfig::import_from() {
                        self.config = imported;
                        self.alert_triggers = self.config.alert_triggers.iter()
                            .filter_map(|v| serde_json::from_value(v.clone()).ok())
                            .collect();
                        self.ignore_strings = self.config.ignore_strings.iter()
                            .filter_map(|s| {
                                let pattern = format!(r"(?i)\b{}\b", regex::escape(s));
                                Regex::new(&pattern).ok()
                            })
                            .collect();
                        self.ignore_systems = self.config.ignore_systems.clone();
                        self.gl_map.lock().unwrap().landmark_systems = self.config.landmark_systems.iter().copied().collect();
                        self.write_system_intel("Profile imported");
                    }
                }
            }
        }

        egui::TopBottomPanel::top("controls").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("T.A.C.O.");
                ui.separator();

                ui.label("Search:");
                let response = ui.text_edit_singleline(&mut self.search_text);
                if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    self.on_search();
                }

                ui.separator();

                let btn_text = if self.process_logs { "Stop" } else { "Start" };
                if ui.button(btn_text).clicked() {
                    if self.process_logs {
                        self.stop_logs();
                    } else {
                        self.start_logs();
                    }
                }

                if ui.selectable_label(self.mute_sound, "Mute").clicked() {
                    self.mute_sound = !self.mute_sound;
                    self.sound_manager.set_muted(self.mute_sound);
                }

                if ui
                    .selectable_label(!self.show_right_panel, "Hide Panel")
                    .clicked()
                {
                    self.show_right_panel = !self.show_right_panel;
                }
            });
        });

        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(&self.status_message);
                if self.process_logs {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.colored_label(egui::Color32::GREEN, "Monitoring...");
                    });
                }
            });
        });

        if self.show_right_panel {
            egui::SidePanel::right("right_panel")
                .min_width(350.0)
                .default_width(450.0)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        if ui
                            .selectable_label(!self.show_settings, "Intel")
                            .clicked()
                        {
                            self.show_settings = false;
                        }
                        if ui
                            .selectable_label(self.show_settings, "Settings")
                            .clicked()
                        {
                            self.show_settings = true;
                        }
                    });
                    ui.separator();

                    if self.show_settings {
                        let mgr = self.manager.lock().unwrap();
                        let names = &mgr.names;
                        self.config_panel
                            .show(ui, &mut self.config, &mut self.alert_triggers, names);
                        drop(mgr);
                    } else {
                        let clicked_system = self.intel_panel.show(ui);
                        if let Some(name) = clicked_system {
                            let name_lower = name.to_lowercase();
                            let manager = self.manager.lock().unwrap();
                            if let Some(sid) = manager.names.iter().find(|(k, _)| k.to_lowercase() == name_lower).map(|(_, &v)| v) {
                                self.gl_map.lock().unwrap().zoom_to_system(sid, &manager.solar_systems);
                                drop(manager);
                                self.manager.lock().unwrap().add_highlight(sid, true);
                            }
                        }
                    }
                });
        }

        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(egui::Color32::from_rgb(5, 5, 15)))
            .show(ctx, |ui| {
                let rect = ui.available_rect_before_wrap();
                let response = ui.allocate_rect(rect, egui::Sense::click_and_drag());

                if response.dragged_by(egui::PointerButton::Primary) {
                    let delta = response.drag_delta();
                    self.gl_map.lock().unwrap().handle_drag(delta.x, delta.y);
                }

                if response.hovered() {
                    let scroll = ui.input(|i| i.smooth_scroll_delta.y);
                    if scroll.abs() > 0.1 {
                        self.gl_map.lock().unwrap().handle_scroll(scroll);
                    }
                }

                if response.clicked() {
                    if let Some(pos) = response.interact_pointer_pos() {
                        let local_pos = pos - rect.min.to_vec2();
                        let manager = self.manager.lock().unwrap();
                        let picked = self.gl_map.lock().unwrap().pick_system(
                            local_pos.x,
                            local_pos.y,
                            rect.width(),
                            rect.height(),
                            &manager.solar_systems,
                        );
                        if let Some(sys_id) = picked {
                            drop(manager);
                            self.manager.lock().unwrap().add_highlight(sys_id, true);
                            if self.sticky_highlights.contains(&sys_id) {
                                self.sticky_highlights.remove(&sys_id);
                            } else {
                                self.sticky_highlights.insert(sys_id);
                            }
                            let manager = self.manager.lock().unwrap();
                            let name = manager.solar_systems[sys_id].name.clone();
                            let mut status = name;
                            if let Some(stats) = manager.system_stats.get(&sys_id) {
                                let elapsed = chrono::Utc::now() - stats.last_report;
                                let mins = elapsed.num_minutes();
                                status.push_str(&format!(
                                    " - Reports: {}, Last: {}m ago",
                                    stats.report_count, mins
                                ));
                            }
                            self.status_message = status;
                        }
                    }
                }

                if response.secondary_clicked() {
                    if let Some(pos) = response.interact_pointer_pos() {
                        let local_pos = pos - rect.min.to_vec2();
                        let manager = self.manager.lock().unwrap();
                        self.right_click_system = self.gl_map.lock().unwrap().pick_system(
                            local_pos.x,
                            local_pos.y,
                            rect.width(),
                            rect.height(),
                            &manager.solar_systems,
                        );
                    }
                }

                if let Some(pos) = response.hover_pos() {
                    let local_pos = pos - rect.min.to_vec2();
                    let manager = self.manager.lock().unwrap();
                    let hovered = self.gl_map.lock().unwrap().pick_system(
                        local_pos.x,
                        local_pos.y,
                        rect.width(),
                        rect.height(),
                        &manager.solar_systems,
                    );
                    let mut gl_map = self.gl_map.lock().unwrap();
                    gl_map.hovered_system = hovered;
                    if let Some(sid) = hovered {
                        if sid < manager.solar_systems.len() {
                            gl_map.hovered_connections = manager.solar_systems[sid]
                                .connected_to.iter()
                                .map(|c| c.to_system_id)
                                .collect();
                        } else {
                            gl_map.hovered_connections.clear();
                        }
                    } else {
                        gl_map.hovered_connections.clear();
                    }
                } else {
                    let mut gl_map = self.gl_map.lock().unwrap();
                    gl_map.hovered_system = None;
                    gl_map.hovered_connections.clear();
                }

                let rclick_sys = self.right_click_system;
                response.context_menu(|ui| {
                    if let Some(sys_id) = rclick_sys {
                        let sys_name = {
                            let m = self.manager.lock().unwrap();
                            if sys_id < m.solar_systems.len() {
                                m.solar_systems[sys_id].name.clone()
                            } else {
                                String::new()
                            }
                        };
                        if !sys_name.is_empty() {
                            ui.label(egui::RichText::new(&sys_name).strong());
                            ui.separator();

                            if ui.button("Set Home").clicked() {
                                self.manager.lock().unwrap().set_current_home_system(sys_id as i32);
                                self.config.home_system_id = sys_id as i32;
                                self.config.save();
                                ui.close();
                            }
                            if ui.button("Zoom To").clicked() {
                                let manager = self.manager.lock().unwrap();
                                self.gl_map.lock().unwrap().zoom_to_system(sys_id, &manager.solar_systems);
                                ui.close();
                            }
                            let is_monitored = self.manager.lock().unwrap().green_crosshair_ids.contains(&sys_id);
                            if is_monitored {
                                if ui.button("Unmonitor").clicked() {
                                    self.manager.lock().unwrap().green_crosshair_ids.retain(|&x| x != sys_id);
                                    ui.close();
                                }
                            } else if ui.button("Monitor").clicked() {
                                self.manager.lock().unwrap().add_green_crosshair(sys_id);
                                ui.close();
                            }
                            let is_ignored = self.ignore_systems.contains(&sys_id);
                            if is_ignored {
                                if ui.button("Unignore").clicked() {
                                    self.ignore_systems.retain(|&x| x != sys_id);
                                    self.config.ignore_systems.retain(|&x| x != sys_id);
                                    self.config.save();
                                    ui.close();
                                }
                            } else if ui.button("Ignore").clicked() {
                                self.ignore_systems.push(sys_id);
                                if !self.config.ignore_systems.contains(&sys_id) {
                                    self.config.ignore_systems.push(sys_id);
                                }
                                self.config.save();
                                ui.close();
                            }
                            ui.separator();
                        }
                    }

                    ui.menu_button("Follow Characters", |ui| {
                        let chars: Vec<String> = self.char_locations.keys().cloned().collect();
                        if chars.is_empty() {
                            ui.label("No characters detected");
                        } else {
                            for ch in &chars {
                                let mut followed = self.followed_chars.contains(ch);
                                if ui.checkbox(&mut followed, ch).changed() {
                                    if followed {
                                        self.followed_chars.insert(ch.clone());
                                    } else {
                                        self.followed_chars.remove(ch);
                                    }
                                }
                            }
                        }
                    });

                    if ui.button("Refocus").clicked() {
                        let mut targets: Vec<usize> = Vec::new();
                        for ch in &self.followed_chars {
                            if let Some(&loc) = self.char_locations.get(ch) {
                                if loc >= 0 {
                                    targets.push(loc as usize);
                                }
                            }
                        }
                        let manager = self.manager.lock().unwrap();
                        if manager.home_system_id >= 0 {
                            let home = manager.home_system_id as usize;
                            if !targets.contains(&home) {
                                targets.push(home);
                            }
                        }
                        if !targets.is_empty() {
                            self.refocus_index %= targets.len();
                            let target = targets[self.refocus_index];
                            self.gl_map.lock().unwrap().zoom_to_system(
                                target,
                                &manager.solar_systems,
                            );
                            self.refocus_index = (self.refocus_index + 1) % targets.len();
                        }
                        ui.close();
                    }
                    if ui.button("Clear Home").clicked() {
                        self.manager.lock().unwrap().clear_current_system();
                        self.config.home_system_id = -1;
                        self.config.save();
                        ui.close();
                    }
                    ui.separator();
                    let is_2d = self.gl_map.lock().unwrap().map_mode_2d;
                    let mode_label = if is_2d { "Switch to 3D" } else { "Switch to 2D" };
                    if ui.button(mode_label).clicked() {
                        let new_2d = !is_2d;
                        self.manager.lock().unwrap().set_map_mode(new_2d);
                        self.gl_map.lock().unwrap().map_mode_2d = new_2d;
                        self.config.map_mode = if new_2d { "2d" } else { "3d" }.to_string();
                        self.config.save();
                        ui.close();
                    }
                    if ui.button("Toggle Labels").clicked() {
                        let mut gl_map = self.gl_map.lock().unwrap();
                        gl_map.persistent_labels = !gl_map.persistent_labels;
                        self.config.persistent_system_labels = gl_map.persistent_labels;
                        self.config.save();
                        ui.close();
                    }
                });

                {
                    let mut gl_map = self.gl_map.lock().unwrap();
                    if gl_map.sticky_highlights != self.sticky_highlights {
                        gl_map.sticky_highlights = self.sticky_highlights.clone();
                    }
                    gl_map.show_alert_age = self.config.show_alert_age;
                    gl_map.display_char_names = self.config.display_character_names;
                    if self.char_locations_dirty {
                        gl_map.char_locations = self.char_locations.iter()
                            .filter(|(_, &id)| id >= 0)
                            .map(|(name, &id)| (name.clone(), id as usize))
                            .collect();
                        self.char_locations_dirty = false;
                    }
                }

                let gl_map = self.gl_map.clone();
                let manager = self.manager.clone();
                let gl_initialized = self.gl_initialized;

                let callback = egui::PaintCallback {
                    rect,
                    callback: Arc::new(eframe::egui_glow::CallbackFn::new(
                        move |_info, painter| {
                            let gl = painter.gl();
                            let mut gl_map = gl_map.lock().unwrap();
                            let mut manager = manager.lock().unwrap();
                            if !gl_initialized {
                                gl_map.init_gl(gl);
                                gl_map.init_vbos(gl, &manager);
                            }
                            gl_map.paint(gl, &mut manager, rect.width(), rect.height());
                        },
                    )),
                };
                ui.painter().add(callback);
                if !self.gl_initialized {
                    self.gl_initialized = true;
                }

                let gl_map = self.gl_map.lock().unwrap();
                let manager = self.manager.lock().unwrap();
                let font_size = gl_map.map_text_size as f32;
                let font_id = egui::FontId::monospace(font_size);
                let painter = ui.painter();
                let clip = rect;

                for l in &gl_map.pending_labels {
                    let pos = clip.min + egui::vec2(l.x, l.y);
                    if !clip.contains(pos) {
                        continue;
                    }
                    let text = match &l.text {
                        MapLabelText::SystemName(id) => {
                            manager.solar_systems.get(*id).map(|s| s.name.as_str())
                        }
                        MapLabelText::RegionName(i) => {
                            manager.region_labels.get(*i).map(|r| r.name.as_str())
                        }
                        MapLabelText::CharName(i) => {
                            gl_map.char_locations.get(*i).map(|(name, _)| name.as_str())
                        }
                        MapLabelText::Owned(s) => Some(s.as_str()),
                    };
                    if let Some(text) = text {
                        let c = egui::Color32::from_rgba_unmultiplied(
                            (l.color[0] * 255.0) as u8,
                            (l.color[1] * 255.0) as u8,
                            (l.color[2] * 255.0) as u8,
                            (l.color[3] * 255.0) as u8,
                        );
                        painter.text(pos, egui::Align2::LEFT_TOP, text, font_id.clone(), c);
                    }
                }
                drop(manager);
                drop(gl_map);
            });
    }

    fn on_exit(&mut self, _gl: Option<&glow::Context>) {
        self.stop_logs();
        let gl_map = self.gl_map.lock().unwrap();
        self.config.camera_distance = gl_map.camera_distance;
        self.config.look_at_x = gl_map.look_at[0];
        self.config.look_at_y = gl_map.look_at[1];
        drop(gl_map);
        let manager = self.manager.lock().unwrap();
        self.config.home_system_id = manager.home_system_id;
        self.config.monitored_systems = manager
            .green_crosshair_ids
            .iter()
            .filter(|&&id| id != manager.home_system_id as usize)
            .copied()
            .collect();
        drop(manager);
        self.config.save();
    }
}
