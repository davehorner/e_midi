use std::fs;
use std::fs::File;
use std::io;
use std::path::Path;
use std::process::Command;
use zip::ZipArchive;

fn download_sc3_plugins(tmp_dir: &str) -> io::Result<String> {
    // Ensure tmp_dir exists
    fs::create_dir_all(tmp_dir)?;

    if crate::install::ensure_gh_installed().is_none() {
        return Err(io::Error::other("gh CLI is not installed"));
    }

    // Download the latest release zip using gh CLI with --clobber
    let status = Command::new("gh")
        .args([
            "release",
            "download",
            "--repo",
            "supercollider/sc3-plugins",
            "--pattern",
            "*Windows-64bit.zip",
            "--dir",
            tmp_dir,
            "--clobber",
        ])
        .status()?;

    if !status.success() {
        return Err(io::Error::other("Failed to download sc3-plugins"));
    }

    // Find the downloaded zip file
    let zip_file = fs::read_dir(tmp_dir)?
        .filter_map(|entry| entry.ok())
        .find(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .ends_with("Windows-64bit.zip")
        })
        .map(|entry| entry.path())
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Zip file not found"))?;

    Ok(zip_file.to_string_lossy().to_string())
}

fn extract_zip(zip_path: &str, extract_to: &str) -> io::Result<()> {
    let file = File::open(zip_path)?;
    let mut archive = ZipArchive::new(file)?;
    fs::create_dir_all(extract_to)?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let outpath = Path::new(extract_to).join(file.name());

        if file.is_dir() {
            fs::create_dir_all(&outpath)?;
        } else {
            if let Some(parent) = outpath.parent() {
                fs::create_dir_all(parent)?;
            }
            let mut outfile = File::create(&outpath)?;
            io::copy(&mut file, &mut outfile)?;
        }
    }
    Ok(())
}

fn copy_sc3_plugins(extracted_dir: &str, sc_user_plugins_dir: &str) -> io::Result<()> {
    // Copy all plugin files to SuperCollider's user plugins directory
    fs::create_dir_all(sc_user_plugins_dir)?;
    for entry in fs::read_dir(extracted_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            let dest = Path::new(sc_user_plugins_dir).join(entry.file_name());
            if dest.exists() {
                fs::remove_dir_all(&dest)?;
            }
            fs::rename(&path, &dest)?;
        }
    }
    Ok(())
}

// Example usage
pub fn install_sc3_plugins() -> io::Result<()> {
    let tmp_dir = "tmp_gh";
    let sc_user_plugins_dir = dirs::home_dir()
        .expect("Could not find home directory")
        .join("AppData/Local/SuperCollider/Extensions/sc3-plugins");

    // Use a scope to ensure cleanup even if error occurs
    let result = (|| {
        let zip_path = download_sc3_plugins(tmp_dir)?;
        let extract_to = format!("{}/extracted", tmp_dir);
        extract_zip(&zip_path, &extract_to)?;
        copy_sc3_plugins(&extract_to, sc_user_plugins_dir.to_str().unwrap())?;
        Ok(())
    })();

    // Cleanup tmp_gh directory
    if let Err(e) = fs::remove_dir_all(tmp_dir) {
        eprintln!("Warning: failed to remove temp dir {}: {}", tmp_dir, e);
    }

    if result.is_ok() {
        println!("sc3-plugins installed successfully.");
    }
    result
}

pub fn is_sc3_plugins_installed() -> bool {
    let sc_user_plugins_dir = match dirs::home_dir() {
        Some(home) => home.join("AppData/Local/SuperCollider/Extensions/sc3-plugins"),
        None => return false,
    };
    sc_user_plugins_dir.exists()
        && sc_user_plugins_dir
            .read_dir()
            .map(|mut d| d.next().is_some())
            .unwrap_or(false)
}
