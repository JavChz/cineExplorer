use std::path::{Path, PathBuf};
use std::process::Command;

pub fn detect_davinci_resolve() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        let std_path = PathBuf::from(r"C:\Program Files\Blackmagic Design\DaVinci Resolve\Resolve.exe");
        if std_path.exists() {
            return Some(std_path);
        }
    }

    #[cfg(target_os = "macos")]
    {
        let mac_path = PathBuf::from("/Applications/DaVinci Resolve/DaVinci Resolve.app");
        if mac_path.exists() {
            return Some(mac_path);
        }
    }

    #[cfg(target_os = "linux")]
    {
        let linux_path = PathBuf::from("/opt/resolve/bin/resolve");
        if linux_path.exists() {
            return Some(linux_path);
        }
    }

    None
}

pub fn detect_premiere_pro() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        let adobe_dir = Path::new(r"C:\Program Files\Adobe");
        if adobe_dir.exists() {
            if let Ok(entries) = std::fs::read_dir(adobe_dir) {
                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if name.contains("Premiere Pro") {
                        let exe = entry.path().join("Adobe Premiere Pro.exe");
                        if exe.exists() {
                            return Some(exe);
                        }
                    }
                }
            }
        }
    }

    None
}

pub fn open_file_in_nle(target_nle: &str, file_path: &Path) -> Result<(), String> {
    if target_nle.to_lowercase().contains("resolve") {
        if let Some(resolve_exe) = detect_davinci_resolve() {
            let status = Command::new(resolve_exe)
                .arg(file_path)
                .spawn();
            if status.is_ok() {
                return Ok(());
            }
        }
    } else if target_nle.to_lowercase().contains("premiere") {
        if let Some(prem_exe) = detect_premiere_pro() {
            let status = Command::new(prem_exe)
                .arg(file_path)
                .spawn();
            if status.is_ok() {
                return Ok(());
            }
        }
    }

    // Default OS open fallback
    open::that(file_path).map_err(|e| format!("Failed to open file: {}", e))
}
