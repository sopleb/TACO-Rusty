use std::path::Path;

use crate::resources;

pub const SOUND_LIST: &[&str] = &[
    "1up1",
    "Boo2",
    "Coin",
    "KamekLaugh",
    "Powerup",
    "RedCoin2",
    "RedCoin3",
    "StarCoin",
    "SuitFly",
    "SuitSpin",
    "Whistle",
    "CallInsideHouse",
    "Hostiles1jump",
    "Hostiles2jump",
    "Hostiles3jump",
    "Hostiles4jump",
    "HostilesHere",
];

pub struct SoundManager {
    muted: bool,
    #[cfg(not(target_os = "linux"))]
    _stream: Option<rodio::OutputStream>,
    #[cfg(not(target_os = "linux"))]
    stream_handle: Option<rodio::OutputStreamHandle>,
    #[cfg(target_os = "linux")]
    cache_dir: Option<std::path::PathBuf>,
}

impl SoundManager {
    pub fn new() -> Self {
        #[cfg(not(target_os = "linux"))]
        {
            let (stream, handle) = match rodio::OutputStream::try_default() {
                Ok((s, h)) => (Some(s), Some(h)),
                Err(e) => {
                    log::warn!("Failed to open audio output: {}", e);
                    (None, None)
                }
            };
            Self {
                muted: false,
                _stream: stream,
                stream_handle: handle,
            }
        }

        #[cfg(target_os = "linux")]
        {
            let cache_dir = dirs::cache_dir()
                .map(|d| d.join("taco").join("sounds"))
                .or_else(|| Some(std::path::PathBuf::from("/tmp/taco-sounds")));
            if let Some(ref dir) = cache_dir {
                let _ = std::fs::create_dir_all(dir);
            }
            Self {
                muted: false,
                cache_dir,
            }
        }
    }

    pub fn play_sound(&self, name: &str) -> bool {
        if self.muted {
            return false;
        }
        if let Some(data) = resources::get_sound_data(name) {
            return self.play_wav_data(name, data);
        }
        log::warn!("Sound not found: {}", name);
        false
    }

    pub fn play_sound_by_id(&self, sound_id: i32) -> bool {
        if self.muted || sound_id < 0 || sound_id as usize >= SOUND_LIST.len() {
            return false;
        }
        self.play_sound(SOUND_LIST[sound_id as usize])
    }

    pub fn play_custom_sound(&self, file_path: &str) {
        if self.muted {
            return;
        }
        if resources::get_sound_data(file_path).is_some() {
            self.play_sound(file_path);
            return;
        }
        let path = Path::new(file_path);
        if path.exists() {
            #[cfg(not(target_os = "linux"))]
            self.play_file_rodio(path);
            #[cfg(target_os = "linux")]
            self.play_file_native(path);
        }
    }

    fn play_wav_data(&self, name: &str, data: &[u8]) -> bool {
        #[cfg(not(target_os = "linux"))]
        {
            let Some(ref handle) = self.stream_handle else {
                return false;
            };
            let cursor = std::io::Cursor::new(data.to_vec());
            let Ok(source) = rodio::Decoder::new(cursor) else {
                log::warn!("Failed to decode audio: {}", name);
                return false;
            };
            let Ok(sink) = rodio::Sink::try_new(handle) else {
                return false;
            };
            sink.append(source);
            sink.detach();
            true
        }

        #[cfg(target_os = "linux")]
        {
            let Some(ref cache_dir) = self.cache_dir else {
                return false;
            };
            let wav_path = cache_dir.join(format!("{}.wav", name));
            if !wav_path.exists() {
                use std::io::Write;
                match std::fs::File::create(&wav_path) {
                    Ok(mut f) => {
                        if f.write_all(data).is_err() {
                            return false;
                        }
                    }
                    Err(_) => return false,
                }
            }
            self.play_file_native(&wav_path)
        }
    }

    #[cfg(not(target_os = "linux"))]
    fn play_file_rodio(&self, path: &Path) -> bool {
        let Some(ref handle) = self.stream_handle else {
            return false;
        };
        let Ok(file) = std::fs::File::open(path) else {
            return false;
        };
        let Ok(source) = rodio::Decoder::new(std::io::BufReader::new(file)) else {
            return false;
        };
        let Ok(sink) = rodio::Sink::try_new(handle) else {
            return false;
        };
        sink.append(source);
        sink.detach();
        true
    }

    #[cfg(target_os = "linux")]
    fn play_file_native(&self, path: &Path) -> bool {
        for cmd in &["paplay", "pw-play", "aplay"] {
            if let Ok(output) = std::process::Command::new("which").arg(cmd).output() {
                if output.status.success() {
                    let result = std::process::Command::new(cmd)
                        .arg(path)
                        .stdout(std::process::Stdio::null())
                        .stderr(std::process::Stdio::null())
                        .spawn();
                    if result.is_ok() {
                        return true;
                    }
                }
            }
        }
        log::warn!("No audio player found (tried paplay, pw-play, aplay)");
        false
    }

    pub fn set_muted(&mut self, muted: bool) {
        self.muted = muted;
    }
}
