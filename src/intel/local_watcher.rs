use std::collections::HashMap;
use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

use regex::Regex;

use crate::intel::log_path_detector::get_default_log_path;

pub struct LocalInfo {
    pub char_name: String,
    pub initial_system: i32,
    pub current_system: String,
}

impl Default for LocalInfo {
    fn default() -> Self {
        Self {
            char_name: String::new(),
            initial_system: -1,
            current_system: String::new(),
        }
    }
}

pub struct SystemChange {
    pub system_name: String,
    pub char_name: String,
}

struct InterestingFile {
    last_position: u64,
    last_update: Instant,
    char_name: String,
}

pub struct LocalWatcher {
    log_path: PathBuf,
    file_sizes: HashMap<PathBuf, u64>,
    interesting_files: HashMap<PathBuf, InterestingFile>,
    pub initial_local_info: Option<LocalInfo>,
    system_change_re: Regex,
    listener_re: Regex,
    initial_system_re: Regex,
    running: bool,
}

impl LocalWatcher {
    pub fn new(log_path: Option<PathBuf>) -> Self {
        let root = log_path.unwrap_or_else(get_default_log_path);
        Self {
            log_path: root.join("Chatlogs"),
            file_sizes: HashMap::new(),
            interesting_files: HashMap::new(),
            initial_local_info: None,
            system_change_re: Regex::new(
                r"EVE\sSystem\s>\sChannel\schanged\sto\sLocal\s:\s(?P<systemname>.*)",
            )
            .unwrap(),
            listener_re: Regex::new(r"Listener:\s*(?P<name>.*)").unwrap(),
            initial_system_re: Regex::new(
                r"Channel\sID:\s*\(\('solarsystemid2',\s(?P<initialsystem>[0-9]{8})",
            )
            .unwrap(),
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

    pub fn init(&mut self) -> Vec<SystemChange> {
        let mut changes = Vec::new();
        let files = self.init_log_file_info();
        let mut seen_chars = Vec::new();

        for path in files {
            let length = Self::get_file_length(&path);
            let info = self.init_local(&path);
            if info.char_name.is_empty() {
                continue;
            }
            if self.initial_local_info.is_none() {
                self.initial_local_info = Some(LocalInfo {
                    char_name: info.char_name.clone(),
                    initial_system: info.initial_system,
                    current_system: info.current_system.clone(),
                });
            }

            self.interesting_files.insert(
                path.clone(),
                InterestingFile {
                    last_position: length,
                    last_update: Instant::now(),
                    char_name: info.char_name.clone(),
                },
            );

            if !seen_chars.contains(&info.char_name) {
                seen_chars.push(info.char_name.clone());
                let system_name = if !info.current_system.is_empty() {
                    info.current_system
                } else {
                    info.initial_system.to_string()
                };
                changes.push(SystemChange {
                    system_name,
                    char_name: info.char_name,
                });
            }
        }

        changes.extend(self.do_tick());
        changes
    }

    pub fn tick(&mut self) -> Vec<SystemChange> {
        if !self.running {
            return Vec::new();
        }
        self.do_tick()
    }

    fn do_tick(&mut self) -> Vec<SystemChange> {
        let mut changes = Vec::new();

        let changed = self.get_changed_log_files();
        for path in changed {
            if self.interesting_files.contains_key(&path) {
                continue;
            }
            let length = Self::get_file_length(&path);
            let info = self.init_local(&path);
            self.interesting_files.insert(
                path,
                InterestingFile {
                    last_position: length,
                    last_update: Instant::now(),
                    char_name: info.char_name.clone(),
                },
            );
            let system_name = if !info.current_system.is_empty() {
                info.current_system
            } else {
                info.initial_system.to_string()
            };
            changes.push(SystemChange {
                system_name,
                char_name: info.char_name,
            });
        }

        let stale_cutoff = Duration::from_secs(7200);
        let stale: Vec<PathBuf> = self
            .interesting_files
            .iter()
            .filter(|(_, v)| v.last_update.elapsed() > stale_cutoff)
            .map(|(k, _)| k.clone())
            .collect();
        for k in stale {
            self.interesting_files.remove(&k);
            self.file_sizes.remove(&k);
        }

        let keys: Vec<PathBuf> = self.interesting_files.keys().cloned().collect();
        for fpath in keys {
            let (text, new_len) = {
                let ifile = &self.interesting_files[&fpath];
                Self::read_log_file(&fpath, ifile.last_position)
            };
            let ifile = self.interesting_files.get_mut(&fpath).unwrap();
            if new_len <= ifile.last_position || text.is_empty() {
                continue;
            }
            ifile.last_position = new_len;
            ifile.last_update = Instant::now();

            for line in text.lines() {
                let line = line.trim();
                if let Some(caps) = self.system_change_re.captures(line) {
                    if let Some(name) = caps.name("systemname") {
                        changes.push(SystemChange {
                            system_name: name.as_str().trim().to_string(),
                            char_name: ifile.char_name.clone(),
                        });
                    }
                }
            }
        }

        changes
    }

    fn init_log_file_info(&mut self) -> Vec<PathBuf> {
        let Ok(entries) = fs::read_dir(&self.log_path) else {
            return Vec::new();
        };
        let day_ago = SystemTime::now() - Duration::from_secs(86400);
        let mut files: Vec<(PathBuf, SystemTime)> = Vec::new();

        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if !name.to_lowercase().starts_with("local") || !name.ends_with(".txt") {
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

        files
            .into_iter()
            .map(|(path, _)| {
                let length = Self::get_file_length(&path);
                self.file_sizes.insert(path.clone(), length);
                path
            })
            .collect()
    }

    fn init_local(&self, path: &Path) -> LocalInfo {
        let content = Self::read_utf16le_file(path);
        let mut info = LocalInfo::default();

        for line in content.lines() {
            let line = Self::clean_line(line);

            if info.initial_system == -1 {
                if let Some(caps) = self.initial_system_re.captures(&line) {
                    if let Some(m) = caps.name("initialsystem") {
                        info.initial_system = m.as_str().parse().unwrap_or(-1);
                        continue;
                    }
                }
            }

            if info.char_name.is_empty() {
                if let Some(caps) = self.listener_re.captures(&line) {
                    if let Some(m) = caps.name("name") {
                        let name = m.as_str().trim();
                        if name.len() > 4 {
                            info.char_name = name.to_string();
                        }
                        continue;
                    }
                }
            }

            if let Some(caps) = self.system_change_re.captures(&line) {
                if let Some(m) = caps.name("systemname") {
                    info.current_system = m.as_str().trim().to_string();
                }
            }
        }
        info
    }

    fn get_changed_log_files(&mut self) -> Vec<PathBuf> {
        let Ok(entries) = fs::read_dir(&self.log_path) else {
            return Vec::new();
        };
        let day_ago = SystemTime::now() - Duration::from_secs(86400);
        let mut changed = Vec::new();

        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if !name.to_lowercase().starts_with("local") || !name.ends_with(".txt") {
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
                if let Some(&prev) = self.file_sizes.get(&path) {
                    if prev != length {
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

    fn read_log_file(path: &Path, start_pos: u64) -> (String, u64) {
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
        let text = Self::decode_utf16le(&raw);
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
