
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CombatEventType {
    Start = 0,
    Stop = 1,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub enum LogEntryType {
    OpenChatLog,
    NewChatLog,
    OpenGameLog,
    NewGameLog,
    UnknownChatLog,
    UnknownGameLog,
    ChatEvent,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogFileType {
    Game,
    Chat,
}

#[derive(Clone, Debug)]
pub struct LogEntry {
    pub log_time: String,
    pub player_name: String,
    pub character_name: String,
    pub line_content: String,
    pub log_type: LogFileType,
    pub entry_type: LogEntryType,
    pub parse_success: bool,
}

impl Default for LogEntry {
    fn default() -> Self {
        Self {
            log_time: String::new(),
            player_name: String::new(),
            character_name: String::new(),
            line_content: String::new(),
            log_type: LogFileType::Chat,
            entry_type: LogEntryType::ChatEvent,
            parse_success: false,
        }
    }
}
