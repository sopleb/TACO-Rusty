use std::collections::{HashMap, VecDeque};
use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use std::time::{Duration, Instant, SystemTime};

use regex::Regex;

use crate::intel::log_entry::{CombatEventType, LogEntry, LogEntryType, LogFileType};
use crate::intel::log_path_detector::get_default_log_path;

struct InterestingFile {
    last_position: u64,
    last_update: Instant,
    last_combat: Instant,
    char_name: String,
    timeout_triggered: bool,
    in_combat: bool,
}

pub enum WatcherMessage {
    LogEntry(LogEntry),
    CombatEvent {
        char_name: String,
        event_type: CombatEventType,
    },
}

pub struct LogWatcher {
    channel_prefix: String,
    log_file_type: LogFileType,
    log_path: PathBuf,
    encoding_utf16: bool,
    file_sizes: HashMap<PathBuf, u64>,
    interesting_files: HashMap<PathBuf, InterestingFile>,
    previous_entries: VecDeque<(String, Instant)>,
    chat_re: Regex,
    game_re: Regex,
    combat_re: Regex,
    listener_re: Regex,
    running: bool,
}

impl LogWatcher {
    pub fn new(
        channel_prefix: &str,
        log_file_type: LogFileType,
        log_path: Option<PathBuf>,
    ) -> Self {
        let root = log_path.unwrap_or_else(get_default_log_path);
        let (path, utf16) = match log_file_type {
            LogFileType::Game => (root.join("Gamelogs"), false),
            LogFileType::Chat => (root.join("Chatlogs"), true),
        };

        Self {
            channel_prefix: channel_prefix.to_string(),
            log_file_type,
            log_path: path,
            encoding_utf16: utf16,
            file_sizes: HashMap::new(),
            interesting_files: HashMap::new(),
            previous_entries: VecDeque::with_capacity(64),
            chat_re: Regex::new(
                r"\[\s\d{4}\.\d{2}\.\d{2}\s(?P<time>\d{2}:\d{2}:\d{2})\s\]\s(?P<name>\w.*)\s>\s(?P<content>.*)",
            ).unwrap(),
            game_re: Regex::new(
                r"\[\s\d{4}\.\d{2}\.\d{2}\s(?P<time>\d{2}:\d{2}:\d{2})\s\]\s\(\w.*\)\s(?P<content>.*)",
            ).unwrap(),
            combat_re: Regex::new(
                r"\[\s\d{4}\.\d{2}\.\d{2}\s\d{2}:\d{2}:\d{2}\s\]\s\(combat\)",
            ).unwrap(),
            listener_re: Regex::new(r"Listener:\s*(?P<name>.*)").unwrap(),
            running: false,
        }
    }

    pub fn start(&mut self) -> bool {
        if !self.log_path.is_dir() {
            return false;
        }
        self.interesting_files.clear();
        self.running = true;
        true
    }

    pub fn stop(&mut self) {
        self.running = false;
    }

    pub fn init(&mut self) -> Vec<WatcherMessage> {
        let mut messages = Vec::new();
        let files = self.init_log_file_info();

        for path in files {
            let length = Self::get_file_length(&path);
            let char_name = self.get_log_listener(&path);

            self.interesting_files.insert(
                path.clone(),
                InterestingFile {
                    last_position: length,
                    last_update: Instant::now(),
                    last_combat: Instant::now() - Duration::from_secs(3600),
                    char_name: char_name.clone(),
                    timeout_triggered: true,
                    in_combat: false,
                },
            );

            let entry_type = match self.log_file_type {
                LogFileType::Game => LogEntryType::OpenGameLog,
                LogFileType::Chat => LogEntryType::OpenChatLog,
            };

            messages.push(WatcherMessage::LogEntry(LogEntry {
                entry_type,
                line_content: length.to_string(),
                character_name: char_name,
                ..Default::default()
            }));
        }

        messages.extend(self.do_tick());
        messages
    }

    pub fn tick(&mut self) -> Vec<WatcherMessage> {
        if !self.running {
            return Vec::new();
        }
        self.do_tick()
    }

    fn do_tick(&mut self) -> Vec<WatcherMessage> {
        let mut messages = Vec::new();

        let cutoff = Instant::now() - Duration::from_secs(5);
        self.previous_entries.retain(|(_, t)| *t >= cutoff);

        let changed = self.get_changed_log_files();
        for path in changed {
            if self.interesting_files.contains_key(&path) {
                continue;
            }
            let char_name = self.get_log_listener(&path);
            self.interesting_files.insert(
                path.clone(),
                InterestingFile {
                    last_position: 0,
                    last_update: Instant::now(),
                    last_combat: Instant::now() - Duration::from_secs(3600),
                    char_name: char_name.clone(),
                    timeout_triggered: true,
                    in_combat: false,
                },
            );

            let entry_type = match self.log_file_type {
                LogFileType::Game => LogEntryType::NewGameLog,
                LogFileType::Chat => LogEntryType::NewChatLog,
            };

            messages.push(WatcherMessage::LogEntry(LogEntry {
                entry_type,
                character_name: char_name,
                ..Default::default()
            }));
        }

        if self.log_file_type == LogFileType::Game {
            let timeout = Duration::from_secs(30);
            for ifile in self.interesting_files.values_mut() {
                if ifile.in_combat
                    && !ifile.timeout_triggered
                    && ifile.last_combat.elapsed() > timeout
                {
                    ifile.timeout_triggered = true;
                    ifile.in_combat = false;
                    messages.push(WatcherMessage::CombatEvent {
                        char_name: ifile.char_name.clone(),
                        event_type: CombatEventType::Stop,
                    });
                }
            }
        }

        let stale_cutoff = Duration::from_secs(7200);
        let stale_keys: Vec<PathBuf> = self
            .interesting_files
            .iter()
            .filter(|(_, v)| v.last_update.elapsed() > stale_cutoff)
            .map(|(k, _)| k.clone())
            .collect();
        for k in stale_keys {
            self.interesting_files.remove(&k);
            self.file_sizes.remove(&k);
        }

        let file_keys: Vec<PathBuf> = self.interesting_files.keys().cloned().collect();
        for fpath in file_keys {
            let (text, new_len) = {
                let ifile = &self.interesting_files[&fpath];
                self.read_log_file(&fpath, ifile.last_position)
            };

            let ifile = self.interesting_files.get_mut(&fpath).unwrap();
            if new_len <= ifile.last_position || text.is_empty() {
                continue;
            }
            ifile.last_position = new_len;
            ifile.last_update = Instant::now();

            let char_name = ifile.char_name.clone();
            for line in text.lines() {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }

                let mut entry = LogEntry {
                    log_type: self.log_file_type,
                    character_name: char_name.clone(),
                    ..Default::default()
                };

                if self.log_file_type == LogFileType::Game && self.combat_re.is_match(line) {
                    let ifile = self.interesting_files.get_mut(&fpath).unwrap();
                    if !ifile.in_combat {
                        ifile.in_combat = true;
                        messages.push(WatcherMessage::CombatEvent {
                            char_name: char_name.clone(),
                            event_type: CombatEventType::Start,
                        });
                    }
                    ifile.timeout_triggered = false;
                    ifile.last_combat = Instant::now();
                }

                let regex = match self.log_file_type {
                    LogFileType::Chat => &self.chat_re,
                    LogFileType::Game => &self.game_re,
                };

                if let Some(caps) = regex.captures(line) {
                    entry.log_time = caps.name("time").map_or("", |m| m.as_str()).to_string();
                    entry.line_content =
                        caps.name("content").map_or("", |m| m.as_str()).to_string();
                    entry.entry_type = LogEntryType::ChatEvent;
                    entry.parse_success = true;
                    if self.log_file_type == LogFileType::Chat {
                        entry.player_name =
                            caps.name("name").map_or("", |m| m.as_str()).to_string();
                    }
                } else {
                    entry.line_content = line.to_string();
                    entry.parse_success = false;
                    entry.entry_type = match self.log_file_type {
                        LogFileType::Chat => LogEntryType::UnknownChatLog,
                        LogFileType::Game => LogEntryType::UnknownGameLog,
                    };
                }

                let dedup_key = format!("{}\x00{}", entry.player_name, entry.line_content);
                if !self.previous_entries.iter().any(|(k, _)| k == &dedup_key) {
                    self.previous_entries.push_back((dedup_key, Instant::now()));
                    messages.push(WatcherMessage::LogEntry(entry));
                }
            }
        }

        messages
    }

    fn init_log_file_info(&mut self) -> Vec<PathBuf> {
        let pattern = if self.log_file_type == LogFileType::Game {
            ""
        } else {
            &self.channel_prefix
        };

        let Ok(entries) = fs::read_dir(&self.log_path) else {
            return Vec::new();
        };

        let day_ago = SystemTime::now() - Duration::from_secs(86400);
        let mut files: Vec<(PathBuf, SystemTime)> = Vec::new();

        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if !name.ends_with(".txt") {
                continue;
            }
            if !pattern.is_empty() && !name.to_lowercase().starts_with(&pattern.to_lowercase()) {
                continue;
            }
            let path = entry.path();
            if let Ok(meta) = fs::metadata(&path) {
                if let Ok(created) = meta.created().or_else(|_| meta.modified()) {
                    if created > day_ago {
                        let mtime = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
                        files.push((path, mtime));
                    }
                }
            }
        }

        files.sort_by(|a, b| b.1.cmp(&a.1));

        let mut result = Vec::new();
        for (path, _) in files {
            let length = Self::get_file_length(&path);
            self.file_sizes.insert(path.clone(), length);
            result.push(path);
        }
        result
    }

    fn get_changed_log_files(&mut self) -> Vec<PathBuf> {
        let pattern = if self.log_file_type == LogFileType::Game {
            ""
        } else {
            &self.channel_prefix
        };

        let Ok(entries) = fs::read_dir(&self.log_path) else {
            return Vec::new();
        };

        let day_ago = SystemTime::now() - Duration::from_secs(86400);
        let mut changed = Vec::new();

        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if !name.ends_with(".txt") {
                continue;
            }
            if !pattern.is_empty() && !name.to_lowercase().starts_with(&pattern.to_lowercase()) {
                continue;
            }
            let path = entry.path();
            if let Ok(meta) = fs::metadata(&path) {
                if let Ok(created) = meta.created().or_else(|_| meta.modified()) {
                    if created < day_ago {
                        continue;
                    }
                }
                let length = Self::get_file_length(&path);
                if let Some(&prev_len) = self.file_sizes.get(&path) {
                    if prev_len != length {
                        self.file_sizes.insert(path.clone(), length);
                        changed.push(path);
                    }
                } else {
                    self.file_sizes.insert(path.clone(), length);
                    changed.push(path);
                }
            }
        }
        changed
    }

    fn get_file_length(path: &Path) -> u64 {
        fs::metadata(path).map(|m| m.len()).unwrap_or(0)
    }

    fn get_log_listener(&self, path: &Path) -> String {
        let content = if self.encoding_utf16 {
            Self::read_utf16le_file(path)
        } else {
            fs::read_to_string(path).unwrap_or_default()
        };

        for line in content.lines() {
            let line = Self::clean_line(line);
            if let Some(caps) = self.listener_re.captures(&line) {
                if let Some(name) = caps.name("name") {
                    return name.as_str().trim().to_string();
                }
            }
        }
        String::new()
    }

    fn read_log_file(&self, path: &Path, start_pos: u64) -> (String, u64) {
        let Ok(mut file) = fs::File::open(path) else {
            return (String::new(), start_pos);
        };
        if file.seek(SeekFrom::Start(start_pos)).is_err() {
            return (String::new(), start_pos);
        }
        let mut raw = Vec::new();
        if file.read_to_end(&mut raw).is_err() {
            return (String::new(), start_pos);
        }
        let new_pos = start_pos + raw.len() as u64;
        let text = if self.encoding_utf16 {
            Self::decode_utf16le(&raw)
        } else {
            String::from_utf8_lossy(&raw).to_string()
        };
        let text = Self::clean_line(&text);
        (text, new_pos)
    }

    fn read_utf16le_file(path: &Path) -> String {
        let Ok(raw) = fs::read(path) else {
            return String::new();
        };
        Self::decode_utf16le(&raw)
    }

    fn decode_utf16le(raw: &[u8]) -> String {
        let pairs: Vec<u16> = raw
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        String::from_utf16_lossy(&pairs)
    }

    fn clean_line(line: &str) -> String {
        line.replace(['\u{feff}', '\u{fffe}', '\r'], "")
            .trim()
            .to_string()
    }
}
