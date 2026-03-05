#![allow(dead_code)]
use std::path::{Path, PathBuf};

pub fn get_possible_log_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    #[cfg(target_os = "windows")]
    get_windows_paths(&mut paths);

    #[cfg(target_os = "linux")]
    get_linux_paths(&mut paths);

    #[cfg(target_os = "macos")]
    get_macos_paths(&mut paths);

    let mut seen = Vec::new();
    paths.retain(|p| {
        if seen.contains(p) {
            false
        } else {
            seen.push(p.clone());
            true
        }
    });
    paths
}

#[cfg(target_os = "windows")]
fn get_windows_paths(paths: &mut Vec<PathBuf>) {
    if let Some(home) = dirs::home_dir() {
        let documents = home.join("Documents");
        let standard = documents.join("EVE").join("logs");
        if standard.exists() {
            paths.push(standard);
        }
        let alt = documents.join("EVE Online").join("logs");
        if alt.exists() {
            paths.push(alt);
        }
        if let Ok(entries) = std::fs::read_dir(&documents) {
            for entry in entries.flatten() {
                if let Ok(name) = entry.file_name().into_string() {
                    if name.starts_with("EVE") && entry.path().is_dir() {
                        let logs = entry.path().join("logs");
                        if logs.exists() {
                            paths.push(logs);
                        }
                    }
                }
            }
        }
        for steam_base in &[
            r"C:\Program Files (x86)\Steam",
            r"C:\Program Files\Steam",
            r"D:\Steam",
            r"D:\SteamLibrary",
        ] {
            let eve_logs = PathBuf::from(steam_base)
                .join("steamapps/compatdata/8500/pfx/drive_c/users/steamuser/My Documents/EVE/logs");
            if eve_logs.exists() {
                paths.push(eve_logs);
            }
        }
    }
}

#[cfg(target_os = "linux")]
fn get_linux_paths(paths: &mut Vec<PathBuf>) {
    if let Some(home) = dirs::home_dir() {
        let user = home.file_name().unwrap_or_default().to_string_lossy().to_string();

        let wine = home.join(".wine/drive_c/users").join(&user).join("My Documents/EVE/logs");
        if wine.exists() {
            paths.push(wine);
        }
        let wine2 = home.join(".wine/drive_c/users").join(&user).join("Documents/EVE/logs");
        if wine2.exists() {
            paths.push(wine2);
        }

        let proton = home.join(".local/share/Steam/steamapps/compatdata/8500/pfx/drive_c/users/steamuser/My Documents/EVE/logs");
        if proton.exists() {
            paths.push(proton);
        }

        let flatpak = home.join(".var/app/com.valvesoftware.Steam/.local/share/Steam/steamapps/compatdata/8500/pfx/drive_c/users/steamuser/My Documents/EVE/logs");
        if flatpak.exists() {
            paths.push(flatpak);
        }

        let lutris = home.join("Games/eve-online/drive_c/users").join(&user).join("My Documents/EVE/logs");
        if lutris.exists() {
            paths.push(lutris);
        }

    }
}

#[cfg(target_os = "macos")]
fn get_macos_paths(paths: &mut Vec<PathBuf>) {
    if let Some(home) = dirs::home_dir() {
        let standard = home.join("Documents/EVE/logs");
        if standard.exists() {
            paths.push(standard);
        }
    }
}

pub fn get_default_log_path() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        dirs::home_dir()
            .unwrap_or_default()
            .join("Documents/EVE/logs")
    }
    #[cfg(target_os = "linux")]
    {
        let home = dirs::home_dir().unwrap_or_default();
        let proton = home.join(".local/share/Steam/steamapps/compatdata/8500/pfx/drive_c/users/steamuser/My Documents/EVE/logs");
        if proton.exists() {
            return proton;
        }
        let user = home.file_name().unwrap_or_default().to_string_lossy().to_string();
        let wine = home.join(".wine/drive_c/users").join(&user).join("My Documents/EVE/logs");
        if wine.exists() {
            return wine;
        }
        proton
    }
    #[cfg(target_os = "macos")]
    {
        dirs::home_dir()
            .unwrap_or_default()
            .join("Documents/EVE/logs")
    }
}

pub fn is_valid_eve_log_path(path: &Path) -> bool {
    if !path.is_dir() {
        return false;
    }
    path.join("Chatlogs").is_dir() || path.join("Gamelogs").is_dir()
}
