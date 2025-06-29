use std::cmp::Ordering;
use std::env;
use std::ffi::OsString;
use std::fs;

pub fn find_gh() -> Option<std::path::PathBuf> {
    if let Ok(path) = which::which("gh") {
        Some(path)
    } else if std::path::Path::new(r"C:\Program Files\GitHub CLI\gh.exe").exists() {
        Some(std::path::PathBuf::from(
            r"C:\Program Files\GitHub CLI\gh.exe",
        ))
    } else {
        None
    }
}

pub fn find_ghci() -> Option<std::path::PathBuf> {
    if let Ok(path) = which::which("ghci") {
        Some(path)
    } else if std::path::Path::new(r"C:\ghcup\bin\ghci.exe").exists() {
        Some(std::path::PathBuf::from(r"C:\ghcup\bin\ghci.exe"))
    } else {
        None
    }
}

pub fn find_supercollider() -> Option<std::path::PathBuf> {
    // Try to find sclang in PATH
    if let Ok(path) = which::which("sclang") {
        return Some(path);
    }

    // Try to find the latest SuperCollider in Program Files
    let program_files =
        std::env::var("ProgramFiles").unwrap_or_else(|_| r"C:\Program Files".to_string());
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

    latest_version.map(|(_, path)| path)
}

pub fn find_supercollider_scsynth() -> Option<std::path::PathBuf> {
    // Try to find scsynth in PATH
    if let Ok(path) = which::which("scsynth") {
        return Some(path);
    }

    // Try to find the latest SuperCollider scsynth.exe in Program Files
    let program_files =
        std::env::var("ProgramFiles").unwrap_or_else(|_| r"C:\Program Files".to_string());
    let sc_prefix = "SuperCollider-";
    let mut latest_version: Option<(semver::Version, std::path::PathBuf)> = None;

    if let Ok(entries) = fs::read_dir(&program_files) {
        for entry in entries.flatten() {
            let file_name = entry.file_name();
            let file_name = file_name.to_string_lossy();
            if file_name.starts_with(sc_prefix) {
                let version_str = &file_name[sc_prefix.len()..];
                if let Ok(version) = semver::Version::parse(version_str) {
                    let exe_path = entry.path().join("scsynth.exe");
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

pub fn find_ghcup() -> Option<std::path::PathBuf> {
    if let Ok(path) = which::which("ghcup") {
        Some(path)
    } else if std::path::Path::new(r"C:\ghcup\bin\ghcup.exe").exists() {
        Some(std::path::PathBuf::from(r"C:\ghcup\bin\ghcup.exe"))
    } else {
        None
    }
}

pub fn find_cabal() -> Option<std::path::PathBuf> {
    if let Ok(path) = which::which("cabal") {
        Some(path)
    } else if std::path::Path::new(r"C:\ghcup\bin\cabal.exe").exists() {
        Some(std::path::PathBuf::from(r"C:\ghcup\bin\cabal.exe"))
    } else {
        None
    }
}

pub fn find_tools_set_env_path() {
    if let Some(sc_path) = find_supercollider() {
        if let Some(sc_dir) = sc_path.parent() {
            let path_var = env::var_os("PATH").unwrap_or_else(|| OsString::new());
            let paths_vec: Vec<std::path::PathBuf> = env::split_paths(&path_var).collect();
            let sc_dir_str = sc_dir.to_string_lossy().to_lowercase();

            let already_in_path = paths_vec
                .iter()
                .any(|p| p.to_string_lossy().to_lowercase() == sc_dir_str);

            if !already_in_path {
                let mut new_paths: Vec<std::path::PathBuf> = vec![sc_dir.to_path_buf()];
                new_paths.extend(paths_vec);
                let new_path = env::join_paths(new_paths).expect("Failed to join PATH");
                unsafe {
                    env::set_var("PATH", &new_path);
                }
            }
        }
    }
}
