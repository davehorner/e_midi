use std::path::PathBuf;
use std::process::Command;

/// Ensures the TidalLooper Quark is installed in the user's SuperCollider Quarks directory.
/// Returns Ok(true) if installed or cloned, Ok(false) if already present, Err if failed.
pub fn ensure_tidallooper_quark_installed() -> Result<bool, String> {
    let quarks_dir = dirs::home_dir()
        .map(|h| h.join(".local/share/SuperCollider/Quarks"))
        .ok_or("Could not determine home directory")?;
    let looper_dir = quarks_dir.join("TidalLooper");
    if looper_dir.exists() {
        return Ok(false); // Already present
    }
    println!("TidalLooper Quark not found, attempting to clone...");
    let status = Command::new("git")
        .args(&[
            "clone",
            "https://github.com/thgrund/tidal-looper.git",
            looper_dir.to_str().unwrap(),
        ])
        .status();
    match status {
        Ok(s) if s.success() => Ok(true),
        Ok(_) | Err(_) => Err(
            "Failed to clone TidalLooper Quark. Please install manually if you need live looping."
                .to_string(),
        ),
    }
}

/// Returns the SuperCollider user Extensions directory (for plugins), e.g. AppData/Local/SuperCollider/Extensions on Windows.
pub fn get_sc_user_plugins_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join("AppData/Local/SuperCollider/Extensions"))
}

/// Ensures the TidalLooper Quark is installed in the user's SuperCollider Extensions directory (AppData/Local/SuperCollider/Extensions on Windows).
/// Returns Ok(true) if installed or cloned, Ok(false) if already present, Err if failed.
pub fn ensure_tidallooper_in_user_extensions() -> Result<bool, String> {
    let sc_user_plugins_dir = dirs::home_dir()
        .ok_or("Could not find home directory")?
        .join("AppData/Local/SuperCollider/Extensions");
    let looper_dir = sc_user_plugins_dir.join("tidal-looper");
    if looper_dir.exists() {
        return Ok(false); // Already present
    }
    println!("TidalLooper not found in Extensions, attempting to clone...");
    std::fs::create_dir_all(&sc_user_plugins_dir)
        .map_err(|e| format!("Failed to create Extensions dir: {}", e))?;
    let status = Command::new("git")
        .args(&[
            "clone",
            "https://github.com/thgrund/tidal-looper.git",
            looper_dir.to_str().unwrap(),
        ])
        .status();
    match status {
        Ok(s) if s.success() => Ok(true),
        Ok(_) | Err(_) => Err("Failed to clone TidalLooper into Extensions. Please install manually if you need live looping.".to_string()),
    }
}
