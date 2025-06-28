use std::process::Command;
use std::fs;
use std::cmp::Ordering;

pub fn ensure_supercollider_installed() -> Option<std::path::PathBuf> {

    // Try to find sclang in PATH
    if let Ok(path) = which::which("sclang") {
        return Some(path);
    }

    // Try to find the latest SuperCollider in Program Files
    let program_files = std::env::var("ProgramFiles").unwrap_or_else(|_| r"C:\Program Files".to_string());
    let sc_prefix = "SuperCollider-";
    let mut latest_version: Option<(semver::Version, std::path::PathBuf)> = None;

    if let Ok(entries) = fs::read_dir(&program_files) {
        for entry in entries.flatten() {
            let file_name = entry.file_name();
            let file_name = file_name.to_string_lossy();
            if file_name.starts_with(sc_prefix) {
                let version_str = &file_name[sc_prefix.len()..];
                if let Ok(version) = semver::Version::parse(version_str) {
                    let exe_path = entry.path().join("sclang.exe");
                    if exe_path.exists() {
                        match &latest_version {
                            Some((latest, _)) if version.cmp(latest) == Ordering::Greater => {
                                latest_version = Some((version, exe_path));
                            }
                            None => {
                                latest_version = Some((version, exe_path));
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }

    if let Some((_, path)) = latest_version {
        return Some(path);
    }

    // Try to install SuperCollider via winget with elevation
    let status = Command::new("powershell")
        .args([
            "-Command",
            "Start-Process winget -ArgumentList 'install --id=SuperCollider.SuperCollider -e --accept-source-agreements --accept-package-agreements' -Verb RunAs -Wait",
        ])
        .status();

    match status {
        Ok(s) if s.success() => {
            // Try again to find sclang in PATH
            if let Ok(path) = which::which("sclang") {
                Some(path)
            } else {
                // Try again to find the latest SuperCollider in Program Files
                let mut latest_version: Option<(semver::Version, std::path::PathBuf)> = None;
                if let Ok(entries) = fs::read_dir(&program_files) {
                    for entry in entries.flatten() {
                        let file_name = entry.file_name();
                        let file_name = file_name.to_string_lossy();
                        if file_name.starts_with(sc_prefix) {
                            let version_str = &file_name[sc_prefix.len()..];
                            if let Ok(version) = semver::Version::parse(version_str) {
                                let exe_path = entry.path().join("sclang.exe");
                                if exe_path.exists() {
                                    match &latest_version {
                                        Some((latest, _)) if version.cmp(latest) == Ordering::Greater => {
                                            latest_version = Some((version, exe_path));
                                        }
                                        None => {
                                            latest_version = Some((version, exe_path));
                                        }
                                        _ => {}
                                    }
                                }
                            }
                        }
                    }
                }
                latest_version.map(|(_, path)| path)
            }
        }
        _ => None,
    }
}


pub fn ensure_ghcup_installed() -> Option<std::path::PathBuf> {
    if let Ok(path) = which::which("ghcup") {
        Some(path)
    } else if std::path::Path::new(r"C:\ghcup\bin\ghcup.exe").exists() {
        Some(std::path::PathBuf::from(r"C:\ghcup\bin\ghcup.exe"))
    } else {
        let ps_script = r#"Set-ExecutionPolicy Bypass -Scope Process -Force; [System.Net.ServicePointManager]::SecurityProtocol = [System.Net.ServicePointManager]::SecurityProtocol -bor 3072; & ([ScriptBlock]::Create((Invoke-WebRequest https://www.haskell.org/ghcup/sh/bootstrap-haskell.ps1 -UseBasicParsing))) -Interactive -DisableCurl"#;
        let status = Command::new("powershell")
            .args([
                "-Command",
                &format!(
                    "Start-Process powershell -Verb runAs -ArgumentList '{}'",
                    ps_script.replace("'", "''")
                ),
            ])
            .status();
        match status {
            Ok(s) if s.success() => {
                if let Ok(path) = which::which("ghcup") {
                    Some(path)
                } else if std::path::Path::new(r"C:\ghcup\bin\ghcup.exe").exists() {
                    Some(std::path::PathBuf::from(r"C:\ghcup\bin\ghcup.exe"))
                } else {
                    None
                }
            },
            _ => None,
        }
    }
}

pub fn ensure_cabal_installed() -> Option<std::path::PathBuf> {
    if let Ok(path) = which::which("cabal") {
        Some(path)
    } else if std::path::Path::new(r"C:\ghcup\bin\cabal.exe").exists() {
        Some(std::path::PathBuf::from(r"C:\ghcup\bin\cabal.exe"))
    } else {
        // Try to install ghcup (which installs cabal) if not present
        let _ = ensure_ghcup_installed();
        if let Ok(path) = which::which("cabal") {
            Some(path)
        } else if std::path::Path::new(r"C:\ghcup\bin\cabal.exe").exists() {
            Some(std::path::PathBuf::from(r"C:\ghcup\bin\cabal.exe"))
        } else {
            None
        }
    }
}