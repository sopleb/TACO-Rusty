use std::collections::{HashMap, VecDeque};

pub struct IntelMessage {
    pub text: String,
    pub system_names: Vec<String>,
    pub is_alert: bool,
    pub jump_info: Option<String>,
}

pub struct ChannelTab {
    pub name: String,
    pub messages: VecDeque<IntelMessage>,
    pub scroll_to_bottom: bool,
}

impl ChannelTab {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            messages: VecDeque::new(),
            scroll_to_bottom: false,
        }
    }
}

pub struct IntelPanel {
    pub channels: Vec<ChannelTab>,
    pub system_tab: ChannelTab,
    pub all_messages: VecDeque<IntelMessage>,
    pub selected_tab: usize,
    pub max_messages: usize,
    channel_indices: HashMap<String, usize>,
}

impl IntelPanel {
    pub fn new() -> Self {
        Self {
            channels: Vec::new(),
            system_tab: ChannelTab::new("System"),
            all_messages: VecDeque::new(),
            selected_tab: 0,
            max_messages: 100,
            channel_indices: HashMap::new(),
        }
    }

    pub fn add_channel_tab(&mut self, name: &str) {
        if !self.channel_indices.contains_key(name) {
            let idx = self.channels.len();
            self.channels.push(ChannelTab::new(name));
            self.channel_indices.insert(name.to_string(), idx);
        }
    }

    pub fn remove_channel_tab(&mut self, name: &str) {
        if let Some(idx) = self.channel_indices.remove(name) {
            self.channels.remove(idx);
            self.channel_indices.clear();
            for (i, ch) in self.channels.iter().enumerate() {
                self.channel_indices.insert(ch.name.clone(), i);
            }
        }
    }

    pub fn write_intel(&mut self, channel: &str, text: &str, system_names: Vec<String>) {
        self.write_intel_with_jump(channel, text, system_names, None);
    }

    pub fn write_intel_with_jump(&mut self, channel: &str, text: &str, system_names: Vec<String>, jump_info: Option<String>) {
        let is_alert = text.contains("ALERT");
        let msg = IntelMessage {
            text: text.to_string(),
            system_names: system_names.clone(),
            is_alert,
            jump_info: jump_info.clone(),
        };

        self.all_messages.push_back(IntelMessage {
            text: text.to_string(),
            system_names,
            is_alert,
            jump_info,
        });
        if self.all_messages.len() > self.max_messages {
            self.all_messages.pop_front();
        }

        if channel == "System" {
            self.system_tab.messages.push_back(msg);
            if self.system_tab.messages.len() > self.max_messages {
                self.system_tab.messages.pop_front();
            }
            self.system_tab.scroll_to_bottom = true;
        } else if let Some(&idx) = self.channel_indices.get(channel) {
            self.channels[idx].messages.push_back(msg);
            if self.channels[idx].messages.len() > self.max_messages {
                self.channels[idx].messages.pop_front();
            }
            self.channels[idx].scroll_to_bottom = true;
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui) -> Option<String> {
        let mut clicked_system: Option<String> = None;

        let tab_count = 1 + self.channels.len() + 1;
        ui.horizontal(|ui| {
            if ui
                .selectable_label(self.selected_tab == 0, "All")
                .clicked()
            {
                self.selected_tab = 0;
            }
            for (i, ch) in self.channels.iter().enumerate() {
                if ui
                    .selectable_label(self.selected_tab == i + 1, &ch.name)
                    .clicked()
                {
                    self.selected_tab = i + 1;
                }
            }
            if ui
                .selectable_label(self.selected_tab == tab_count - 1, "System")
                .clicked()
            {
                self.selected_tab = tab_count - 1;
            }
        });

        ui.separator();

        let messages = if self.selected_tab == 0 {
            &self.all_messages
        } else if self.selected_tab == tab_count - 1 {
            &self.system_tab.messages
        } else {
            let idx = self.selected_tab - 1;
            if idx < self.channels.len() {
                &self.channels[idx].messages
            } else {
                &self.all_messages
            }
        };

        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .stick_to_bottom(true)
            .show(ui, |ui| {
                for msg in messages {
                    if msg.is_alert {
                        ui.colored_label(egui::Color32::RED, &msg.text);
                    } else {
                        ui.label(&msg.text);
                    }
                    if !msg.system_names.is_empty() {
                        ui.horizontal(|ui| {
                            for name in &msg.system_names {
                                if ui.link(name).clicked() {
                                    clicked_system = Some(name.clone());
                                }
                            }
                            if let Some(ref info) = msg.jump_info {
                                ui.label(info);
                            }
                        });
                    }
                }
            });

        clicked_system
    }
}
