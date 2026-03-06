
use crate::audio::sound_manager::SOUND_LIST;
use crate::config::taco_config::{ChannelConfig, TacoConfig};
use crate::core::alert_trigger::{AlertTrigger, AlertType, RangeAlertOperator, RangeAlertType};

pub enum ConfigEvent {
    ConfigChanged,
    ChannelAdded(String, String),
    ChannelRemoved(String),
    DarkModeChanged(bool),
    PersistentLabelsChanged(bool),
    MapTextSizeChanged(u32),
    MapModeChanged(String),
    IgnoreStringAdded(String),
    IgnoreStringRemoved(usize),
    IgnoreSystemRemoved(usize),
    LandmarkAdded(usize),
    LandmarkRemoved(usize),
    AlertTriggerUpdated,
    AlertTriggerRemoved(usize),
    AlertTriggerMoved(usize, bool), // index, up?
    PlayTestSound(i32, String),
    ScrollSensitivityChanged(f32),
    MaxIntelMessagesChanged(usize),
    ExportProfile,
    ImportProfile,
}

pub struct ConfigPanel {
    pub events: Vec<ConfigEvent>,
    new_channel_name: String,
    new_channel_prefix: String,
    new_ignore_string: String,
    new_landmark_name: String,
    editing_trigger: Option<AlertTrigger>,
    editing_index: Option<usize>,
}

impl ConfigPanel {
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
            new_channel_name: String::new(),
            new_channel_prefix: String::new(),
            new_ignore_string: String::new(),
            new_landmark_name: String::new(),
            editing_trigger: None,
            editing_index: None,
        }
    }

    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        config: &mut TacoConfig,
        alert_triggers: &mut Vec<AlertTrigger>,
        system_names: &rustc_hash::FxHashMap<String, usize>,
    ) {
        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                ui.collapsing(format!("Channels ({})", config.custom_channels.len()), |ui| {
                    for ch in &config.custom_channels {
                        ui.horizontal(|ui| {
                            ui.label(&ch.name);
                            ui.label(format!("({})", ch.prefix));
                            if ui.small_button("Remove").clicked() {
                                self.events
                                    .push(ConfigEvent::ChannelRemoved(ch.name.clone()));
                            }
                        });
                    }
                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.label("Name:");
                        ui.text_edit_singleline(&mut self.new_channel_name);
                    });
                    ui.horizontal(|ui| {
                        ui.label("Prefix:");
                        ui.text_edit_singleline(&mut self.new_channel_prefix);
                    });
                    if ui.button("Add Channel").clicked()
                        && !self.new_channel_name.is_empty()
                        && !self.new_channel_prefix.is_empty()
                    {
                        let name = self.new_channel_name.clone();
                        let prefix = self.new_channel_prefix.clone();
                        config.custom_channels.push(ChannelConfig {
                            name: name.clone(),
                            prefix: prefix.clone(),
                            monitor: true,
                            alert: true,
                            short_name: String::new(),
                        });
                        self.events.push(ConfigEvent::ChannelAdded(name, prefix));
                        self.new_channel_name.clear();
                        self.new_channel_prefix.clear();
                        config.save();
                    }
                });

                ui.collapsing(format!("Alerts ({})", alert_triggers.len()), |ui| {
                    let mut action: Option<(&str, usize)> = None;
                    let trigger_count = alert_triggers.len();
                    let trigger_info: Vec<(bool, String)> = alert_triggers.iter()
                        .map(|t| (t.enabled, format!("{}", t)))
                        .collect();
                    for (i, (mut enabled, label)) in trigger_info.into_iter().enumerate() {
                        ui.horizontal(|ui| {
                            if ui.checkbox(&mut enabled, "").changed() {
                                action = Some(("update_enabled", i));
                            }
                            ui.label(&label);
                            if ui.small_button("Edit").clicked() {
                                action = Some(("edit", i));
                            }
                            if ui.small_button("Remove").clicked() {
                                action = Some(("remove", i));
                            }
                            if i > 0 && ui.small_button("\u{2191}").clicked() {
                                action = Some(("up", i));
                            }
                            if i + 1 < trigger_count && ui.small_button("\u{2193}").clicked() {
                                action = Some(("down", i));
                            }
                            if ui.small_button("\u{25b6}").clicked() {
                                action = Some(("play", i));
                            }
                        });
                        if action == Some(("update_enabled", i)) {
                            alert_triggers[i].enabled = enabled;
                        }
                    }
                    match action {
                        Some(("update_enabled", _)) => { self.events.push(ConfigEvent::AlertTriggerUpdated); }
                        Some(("edit", i)) => {
                            self.editing_trigger = Some(alert_triggers[i].clone());
                            self.editing_index = Some(i);
                        }
                        Some(("remove", i)) => { self.events.push(ConfigEvent::AlertTriggerRemoved(i)); }
                        Some(("up", i)) => { self.events.push(ConfigEvent::AlertTriggerMoved(i, true)); }
                        Some(("down", i)) => { self.events.push(ConfigEvent::AlertTriggerMoved(i, false)); }
                        Some(("play", i)) => {
                            let t = &alert_triggers[i];
                            self.events.push(ConfigEvent::PlayTestSound(t.sound_id, t.sound_path.clone()));
                        }
                        _ => {}
                    }

                    ui.separator();

                    if self.editing_trigger.is_none() {
                        ui.horizontal(|ui| {
                            if ui.button("Add Range Alert").clicked() {
                                self.editing_trigger = Some(AlertTrigger::default());
                                self.editing_index = None;
                            }
                            if ui.button("Add Custom Alert").clicked() {
                                self.editing_trigger = Some(AlertTrigger {
                                    alert_type: AlertType::Custom,
                                    ..AlertTrigger::default()
                                });
                                self.editing_index = None;
                            }
                        });

                        let mut preset_action = None;
                        egui::ComboBox::from_id_salt("preset_alerts")
                            .selected_text("Quick Add Preset...")
                            .show_ui(ui, |ui| {
                                if ui.selectable_label(false, "Alert in system (0 jumps)").clicked() {
                                    preset_action = Some(AlertTrigger {
                                        upper_limit_operator: RangeAlertOperator::Equal,
                                        upper_range: 0,
                                        sound_id: 16, // HostilesHere
                                        sound_path: "HostilesHere".to_string(),
                                        ..AlertTrigger::default()
                                    });
                                }
                                if ui.selectable_label(false, "Alert within 3 jumps").clicked() {
                                    preset_action = Some(AlertTrigger {
                                        upper_limit_operator: RangeAlertOperator::LessThanOrEqual,
                                        upper_range: 3,
                                        sound_id: 16, // HostilesHere
                                        sound_path: "HostilesHere".to_string(),
                                        ..AlertTrigger::default()
                                    });
                                }
                                if ui.selectable_label(false, "Alert within 5 jumps").clicked() {
                                    preset_action = Some(AlertTrigger {
                                        upper_limit_operator: RangeAlertOperator::LessThanOrEqual,
                                        upper_range: 5,
                                        sound_id: 1, // Boo2
                                        sound_path: "Boo2".to_string(),
                                        ..AlertTrigger::default()
                                    });
                                }
                            });
                        if let Some(preset) = preset_action {
                            alert_triggers.push(preset);
                            self.events.push(ConfigEvent::AlertTriggerUpdated);
                        }
                    }

                    if let Some(ref mut trigger) = self.editing_trigger.clone() {
                        let mut close_editor = false;
                        ui.group(|ui| {
                            let is_new = self.editing_index.is_none();
                            ui.label(if is_new { "New Alert" } else { "Edit Alert" });

                            match trigger.alert_type {
                                AlertType::Ranged => {
                                    self.show_ranged_edit(ui, trigger, system_names);
                                }
                                AlertType::Custom => {
                                    self.show_custom_edit(ui, trigger);
                                }
                            }

                            // Sound selector
                            self.show_sound_selector(ui, trigger);

                            ui.horizontal(|ui| {
                                if ui.button("Save").clicked() {
                                    if let Some(idx) = self.editing_index {
                                        alert_triggers[idx] = trigger.clone();
                                    } else {
                                        alert_triggers.push(trigger.clone());
                                    }
                                    self.events.push(ConfigEvent::AlertTriggerUpdated);
                                    close_editor = true;
                                }
                                if ui.button("Cancel").clicked() {
                                    close_editor = true;
                                }
                            });
                        });

                        if close_editor {
                            self.editing_trigger = None;
                            self.editing_index = None;
                        } else {
                            self.editing_trigger = Some(trigger.clone());
                        }
                    }
                });

                ui.collapsing(format!("Ignore Strings ({})", config.ignore_strings.len()), |ui| {
                    ui.label("Messages matching these patterns will be hidden").on_hover_text("Case-insensitive word-boundary matching");
                    let mut remove_idx = None;
                    for (i, s) in config.ignore_strings.iter().enumerate() {
                        ui.horizontal(|ui| {
                            ui.label(s);
                            if ui.small_button("Remove").clicked() {
                                remove_idx = Some(i);
                            }
                        });
                    }
                    if let Some(idx) = remove_idx {
                        self.events.push(ConfigEvent::IgnoreStringRemoved(idx));
                    }
                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.text_edit_singleline(&mut self.new_ignore_string);
                        if ui.button("Add").clicked() && !self.new_ignore_string.is_empty() {
                            self.events.push(ConfigEvent::IgnoreStringAdded(self.new_ignore_string.clone()));
                            self.new_ignore_string.clear();
                        }
                    });
                });

                ui.collapsing(format!("Ignore Systems ({})", config.ignore_systems.len()), |ui| {
                    let mut remove_idx = None;
                    for &sys_id in config.ignore_systems.iter() {
                        ui.horizontal(|ui| {
                            let name = system_names.iter()
                                .find(|(_, &v)| v == sys_id)
                                .map(|(k, _)| k.clone())
                                .unwrap_or_else(|| format!("System {}", sys_id));
                            ui.label(&name);
                            if ui.small_button("Remove").clicked() {
                                remove_idx = Some(sys_id);
                            }
                        });
                    }
                    if let Some(sys_id) = remove_idx {
                        self.events.push(ConfigEvent::IgnoreSystemRemoved(sys_id));
                    }
                });

                ui.collapsing(format!("Landmarks ({})", config.landmark_systems.len()), |ui| {
                    let mut remove_id = None;
                    for &sys_id in &config.landmark_systems {
                        ui.horizontal(|ui| {
                            let name = system_names.iter()
                                .find(|(_, &v)| v == sys_id)
                                .map(|(k, _)| k.clone())
                                .unwrap_or_else(|| format!("System {}", sys_id));
                            ui.label(&name);
                            if ui.small_button("Remove").clicked() {
                                remove_id = Some(sys_id);
                            }
                        });
                    }
                    if let Some(sys_id) = remove_id {
                        self.events.push(ConfigEvent::LandmarkRemoved(sys_id));
                    }
                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.text_edit_singleline(&mut self.new_landmark_name);
                        if ui.button("Add").clicked() && !self.new_landmark_name.is_empty() {
                            let name_lower = self.new_landmark_name.trim().to_lowercase();
                            if let Some((&_, &sid)) = system_names.iter().find(|(k, _)| k.to_lowercase() == name_lower) {
                                self.events.push(ConfigEvent::LandmarkAdded(sid));
                                self.new_landmark_name.clear();
                            }
                        }
                    });
                });

                ui.collapsing("Display", |ui| {
                    let mut dark = config.dark_mode;
                    if ui.checkbox(&mut dark, "Dark Mode").changed() {
                        config.dark_mode = dark;
                        self.events.push(ConfigEvent::DarkModeChanged(dark));
                        config.save();
                    }

                    let mut labels = config.persistent_system_labels;
                    if ui.checkbox(&mut labels, "Persistent Labels").changed() {
                        config.persistent_system_labels = labels;
                        self.events
                            .push(ConfigEvent::PersistentLabelsChanged(labels));
                        config.save();
                    }

                    let mut text_size = config.map_text_size as f32;
                    if ui
                        .add(egui::Slider::new(&mut text_size, 4.0..=24.0).text("Text Size"))
                        .changed()
                    {
                        config.map_text_size = text_size as u32;
                        self.events
                            .push(ConfigEvent::MapTextSizeChanged(config.map_text_size));
                        config.save();
                    }

                    let mut scroll_sens = config.scroll_sensitivity;
                    if ui
                        .add(egui::Slider::new(&mut scroll_sens, 1.0..=5.0).text("Scroll Sensitivity"))
                        .on_hover_text("How fast the map zooms when scrolling")
                        .changed()
                    {
                        config.scroll_sensitivity = scroll_sens;
                        self.events
                            .push(ConfigEvent::ScrollSensitivityChanged(scroll_sens));
                        config.save();
                    }

                    let mut max_msgs = config.max_intel_messages;
                    if ui
                        .add(egui::Slider::new(&mut max_msgs, 10..=2000).text("Max Intel Messages"))
                        .on_hover_text("Maximum number of messages to keep per channel")
                        .changed()
                    {
                        config.max_intel_messages = max_msgs;
                        self.events.push(ConfigEvent::MaxIntelMessagesChanged(max_msgs));
                        config.save();
                    }

                    let mut show_age = config.show_alert_age;
                    if ui.checkbox(&mut show_age, "Show Alert Age").changed() {
                        config.show_alert_age = show_age;
                        self.events.push(ConfigEvent::ConfigChanged);
                        config.save();
                    }

                    let mut show_chars = config.display_character_names;
                    if ui
                        .checkbox(&mut show_chars, "Display Character Names")
                        .changed()
                    {
                        config.display_character_names = show_chars;
                        self.events.push(ConfigEvent::ConfigChanged);
                        config.save();
                    }

                    let mut max_age = config.max_alert_age as f32;
                    if ui
                        .add(
                            egui::Slider::new(&mut max_age, 0.0..=60.0).text("Max Alert Age (min)"),
                        )
                        .on_hover_text("Alerts older than this are automatically removed (0 = never expire)")
                        .changed()
                    {
                        config.max_alert_age = max_age as u32;
                        self.events.push(ConfigEvent::ConfigChanged);
                        config.save();
                    }

                    let mut max_alerts = config.max_alerts as f32;
                    if ui
                        .add(egui::Slider::new(&mut max_alerts, 1.0..=50.0).text("Max Alerts"))
                        .on_hover_text("Maximum number of alert markers shown on the map")
                        .changed()
                    {
                        config.max_alerts = max_alerts as usize;
                        self.events.push(ConfigEvent::ConfigChanged);
                        config.save();
                    }
                });

                ui.collapsing("Map", |ui| {
                    ui.horizontal(|ui| {
                        if ui
                            .selectable_label(config.map_mode == "3d", "3D")
                            .clicked()
                        {
                            config.map_mode = "3d".to_string();
                            self.events
                                .push(ConfigEvent::MapModeChanged("3d".to_string()));
                            config.save();
                        }
                        if ui
                            .selectable_label(config.map_mode == "2d", "2D")
                            .clicked()
                        {
                            config.map_mode = "2d".to_string();
                            self.events
                                .push(ConfigEvent::MapModeChanged("2d".to_string()));
                            config.save();
                        }
                    });
                    ui.separator();
                    ui.label("Range From:").on_hover_text("Calculate jump distances from home system or character location");
                    let mut range_from = config.map_range_from;
                    if ui.radio_value(&mut range_from, 0, "Home").clicked()
                        || ui.radio_value(&mut range_from, 1, "Character").clicked()
                    {
                        config.map_range_from = range_from;
                        self.events.push(ConfigEvent::ConfigChanged);
                        config.save();
                    }
                });

                ui.collapsing("Log Path", |ui| {
                    let mut override_path = config.override_log_path;
                    if ui.checkbox(&mut override_path, "Override Log Path").changed() {
                        config.override_log_path = override_path;
                        config.save();
                    }
                    if config.override_log_path {
                        ui.horizontal(|ui| {
                            ui.label("Path:");
                            if ui.text_edit_singleline(&mut config.log_path).lost_focus() {
                                config.save();
                            }
                        });
                    }

                    let mut monitor_game = config.monitor_game_log;
                    if ui
                        .checkbox(&mut monitor_game, "Monitor Game Log")
                        .changed()
                    {
                        config.monitor_game_log = monitor_game;
                        config.save();
                    }
                });

                ui.collapsing("Profile", |ui| {
                    ui.horizontal(|ui| {
                        if ui.button("Export").clicked() {
                            self.events.push(ConfigEvent::ExportProfile);
                        }
                        if ui.button("Import").clicked() {
                            self.events.push(ConfigEvent::ImportProfile);
                        }
                    });
                });

                ui.collapsing("About", |ui| {
                    ui.label("T.A.C.O. - Tactical Awareness Control Overlay");
                    ui.label(format!("Version: {}", env!("TACO_VERSION")));
                    ui.separator();
                    ui.label("Controls:");
                    ui.label("  Left drag: Pan map");
                    ui.label("  Scroll: Zoom");
                    ui.label("  Click: Select system");
                    ui.label("  Right-click: Context menu");
                    ui.label("  Ctrl+F: Focus search");
                    ui.label("  Ctrl+Q: Quit");
                    ui.label("  F11: Toggle fullscreen");
                    ui.label("  Ctrl+H: Toggle panel");
                    ui.label("  Home: Refocus map");
                });
            });
    }

    fn show_ranged_edit(&self, ui: &mut egui::Ui, trigger: &mut AlertTrigger, system_names: &rustc_hash::FxHashMap<String, usize>) {
        ui.horizontal(|ui| {
            ui.label("Upper:").on_hover_text("Upper range limit — alert fires when jump count satisfies this condition");
            let mut op = trigger.upper_limit_operator as u8;
            egui::ComboBox::from_id_salt("upper_op")
                .selected_text(op_label(trigger.upper_limit_operator))
                .show_ui(ui, |ui| {
                    for v in 0..=4u8 {
                        let rop = RangeAlertOperator::try_from(v).unwrap();
                        ui.selectable_value(&mut op, v, op_label(rop));
                    }
                });
            trigger.upper_limit_operator = RangeAlertOperator::try_from(op).unwrap();
            let mut range = trigger.upper_range as f32;
            ui.add(egui::Slider::new(&mut range, 0.0..=50.0));
            trigger.upper_range = range as i32;
        });

        ui.horizontal(|ui| {
            ui.label("Lower:").on_hover_text("Lower range limit — optional minimum jump distance");
            let mut op = trigger.lower_limit_operator as u8;
            egui::ComboBox::from_id_salt("lower_op")
                .selected_text(op_label(trigger.lower_limit_operator))
                .show_ui(ui, |ui| {
                    for v in 0..=4u8 {
                        let rop = RangeAlertOperator::try_from(v).unwrap();
                        ui.selectable_value(&mut op, v, op_label(rop));
                    }
                });
            trigger.lower_limit_operator = RangeAlertOperator::try_from(op).unwrap();
            let mut range = trigger.lower_range as f32;
            ui.add(egui::Slider::new(&mut range, 0.0..=50.0));
            trigger.lower_range = range as i32;
        });

        ui.horizontal(|ui| {
            ui.label("Range To:").on_hover_text("Calculate jump distances from this reference point");
            let mut rt = trigger.range_to as u8;
            egui::ComboBox::from_id_salt("range_to")
                .selected_text(range_to_label(trigger.range_to))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut rt, 0, "Home").on_hover_text("Distance from home system");
                    ui.selectable_value(&mut rt, 1, "System").on_hover_text("Distance from a specific system");
                    ui.selectable_value(&mut rt, 3, "Any Character").on_hover_text("Distance from any detected character");
                    ui.selectable_value(&mut rt, 5, "Any Followed Character").on_hover_text("Distance from followed characters only");
                });
            trigger.range_to = RangeAlertType::try_from(rt).unwrap();
        });

        if trigger.range_to == RangeAlertType::System {
            ui.horizontal(|ui| {
                ui.label("System:");
                ui.text_edit_singleline(&mut trigger.system_name);
                if let Some((&_, &sid)) = system_names.iter().find(|(k, _)| k.to_lowercase() == trigger.system_name.trim().to_lowercase()) {
                    trigger.system_id = sid as i32;
                }
            });
        }
    }

    fn show_custom_edit(&self, ui: &mut egui::Ui, trigger: &mut AlertTrigger) {
        ui.horizontal(|ui| {
            ui.label("Text:");
            ui.text_edit_singleline(&mut trigger.text);
        });
        ui.horizontal(|ui| {
            ui.label("Repeat interval (sec):");
            let mut interval = trigger.repeat_interval as f32;
            ui.add(egui::Slider::new(&mut interval, 0.0..=300.0));
            trigger.repeat_interval = interval as u32;
        });
    }

    fn show_sound_selector(&self, ui: &mut egui::Ui, trigger: &mut AlertTrigger) {
        ui.horizontal(|ui| {
            ui.label("Sound:");
            let current_name = if trigger.sound_id >= 0 && (trigger.sound_id as usize) < SOUND_LIST.len() {
                SOUND_LIST[trigger.sound_id as usize].to_string()
            } else if !trigger.sound_path.is_empty() {
                format!("Custom: {}", trigger.sound_path)
            } else {
                "Default".to_string()
            };
            egui::ComboBox::from_id_salt("sound_sel")
                .selected_text(&current_name)
                .show_ui(ui, |ui| {
                    for (i, &name) in SOUND_LIST.iter().enumerate() {
                        if ui.selectable_label(trigger.sound_id == i as i32, name).clicked() {
                            trigger.sound_id = i as i32;
                            trigger.sound_path = name.to_string();
                        }
                    }
                    if ui.selectable_label(trigger.sound_id == -1, "Custom").clicked() {
                        trigger.sound_id = -1;
                    }
                });
        });
        if trigger.sound_id == -1 {
            ui.horizontal(|ui| {
                ui.label("Custom path:");
                ui.text_edit_singleline(&mut trigger.sound_path);
            });
        }
    }

    pub fn drain_events(&mut self) -> Vec<ConfigEvent> {
        std::mem::take(&mut self.events)
    }
}

fn op_label(op: RangeAlertOperator) -> &'static str {
    match op {
        RangeAlertOperator::Equal => "=",
        RangeAlertOperator::LessThan => "<",
        RangeAlertOperator::GreaterThan => ">",
        RangeAlertOperator::LessThanOrEqual => "<=",
        RangeAlertOperator::GreaterThanOrEqual => ">=",
    }
}

fn range_to_label(rt: RangeAlertType) -> &'static str {
    match rt {
        RangeAlertType::Home => "Home",
        RangeAlertType::System => "System",
        RangeAlertType::Character => "Character",
        RangeAlertType::AnyCharacter => "Any Character",
        RangeAlertType::None => "None",
        RangeAlertType::AnyFollowedCharacter => "Any Followed Character",
    }
}
