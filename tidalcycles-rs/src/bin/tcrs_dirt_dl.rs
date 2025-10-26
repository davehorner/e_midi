// tcrs_dirt_dl.rs — pattern-based SuperDirt installer (no manifests)
// Installs/removes:
//   • uxn-st "dirt" folders → Dirt-Samples root (11_st*, 22_st*)
//   • uxn-akwf single-cycle waveforms → downloaded-quarks/akwf (akwf_*)
// Default `install` installs 22_st* + all akwf_*
// `remove` removes 11_st*, 22_st*, and the entire akwf folder.
//
// Build (Windows):
//   rustc -O tcrs_dirt_dl.rs -o tcrs_dirt_dl.exe

use std::env;
use std::fs;
use std::io::{self};
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Clone)]
struct Repo {
    name: &'static str,
    url: &'static str,
    branch: &'static str,
    dir_prefixes: &'static [&'static str],
}

const REPO_ST: Repo = Repo {
    name: "uxn-st",
    url: "https://github.com/davehorner/uxn-st.git",
    branch: "dirt",
    dir_prefixes: &["11_st", "22_st"],
};

const REPO_AKWF: Repo = Repo {
    name: "uxn-akwf",
    url: "https://github.com/davehorner/uxn-akwf.git",
    branch: "dirt",
    dir_prefixes: &["akwf_"],
};

fn main() {
    let mut args = env::args().skip(1).collect::<Vec<_>>();
    if args.is_empty() || args.iter().any(|a| a == "-h" || a == "--help") {
        print_help();
        return;
    }

    let mut dest_samples: Option<PathBuf> = None;
    let mut dest_akwf: Option<PathBuf> = None;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--dest" | "--dest-samples" => {
                if i + 1 >= args.len() {
                    eprintln!("{} requires a path", args[i]);
                    std::process::exit(2);
                }
                dest_samples = Some(PathBuf::from(&args[i + 1]));
                args.drain(i..=i + 1);
            }
            "--dest-akwf" => {
                if i + 1 >= args.len() {
                    eprintln!("--dest-akwf requires a path");
                    std::process::exit(2);
                }
                dest_akwf = Some(PathBuf::from(&args[i + 1]));
                args.drain(i..=i + 1);
            }
            _ => i += 1,
        }
    }

    let cmd = args.first().map(String::as_str).unwrap_or("");

    let samples_root = dest_samples.unwrap_or_else(default_dirt_samples_dir);
    let quarks_root = default_quarks_root();
    let akwf_root = dest_akwf.unwrap_or(quarks_root.join("akwf"));

    match cmd {
        "install" => {
            must_git();
            install_repo_filtered(&REPO_ST, &samples_root, Some(&["22_st"]))
                .unwrap_or_else(exit_err);
            install_akwf(&REPO_AKWF, &akwf_root).unwrap_or_else(exit_err);
            println!(
                "✅ Installed 22_st* into {}\n✅ Installed akwf_* into {}",
                samples_root.display(),
                akwf_root.display()
            );
        }
        "install-st" | "install-st-22" => {
            must_git();
            install_repo_filtered(&REPO_ST, &samples_root, Some(&["22_st"]))
                .unwrap_or_else(exit_err);
        }
        "install-st-11" => {
            must_git();
            install_repo_filtered(&REPO_ST, &samples_root, Some(&["11_st"]))
                .unwrap_or_else(exit_err);
        }
        "install-st-all" => {
            must_git();
            install_repo_filtered(&REPO_ST, &samples_root, Some(&["11_st", "22_st"]))
                .unwrap_or_else(exit_err);
        }
        "install-akwf" => {
            must_git();
            install_akwf(&REPO_AKWF, &akwf_root).unwrap_or_else(exit_err);
        }
        "remove" => {
            remove_repo_filtered(&REPO_ST, &samples_root, Some(&["11_st", "22_st"]))
                .unwrap_or_else(exit_err);
            remove_akwf_root(&akwf_root).unwrap_or_else(exit_err);
        }
        "remove-st" => {
            remove_repo_filtered(&REPO_ST, &samples_root, Some(&["11_st", "22_st"]))
                .unwrap_or_else(exit_err);
        }
        "remove-st-22" => {
            remove_repo_filtered(&REPO_ST, &samples_root, Some(&["22_st"]))
                .unwrap_or_else(exit_err);
        }
        "remove-st-11" => {
            remove_repo_filtered(&REPO_ST, &samples_root, Some(&["11_st"]))
                .unwrap_or_else(exit_err);
        }
        "remove-akwf" => {
            remove_akwf_root(&akwf_root).unwrap_or_else(exit_err);
        }
        "status" => {
            status(&samples_root, &akwf_root);
        }
        _ => {
            eprintln!("Unknown command: {cmd}");
            print_help();
            std::process::exit(2);
        }
    }
}

fn exit_err(e: io::Error) {
    eprintln!("Error: {e}");
    std::process::exit(1);
}

// --- helper functions ---

fn print_help() {
    eprintln!(
        r#"tcrs_dirt_dl — install/remove uxn-st (Dirt-Samples) & uxn-akwf (downloaded-quarks/akwf)

USAGE:
  tcrs_dirt_dl <command> [--dest-samples <path>] [--dest-akwf <path>]

COMMANDS:
  install            Install 22_st (uxn-st) into Dirt-Samples AND akwf_* into downloaded-quarks/akwf
  install-st         Install uxn-st (22_st* only)
  install-st-11      Install uxn-st (11_st* only)
  install-st-all     Install uxn-st (both 11_st* and 22_st*)
  install-akwf       Install uxn-akwf (akwf_*) into downloaded-quarks/akwf
  remove             Remove 11_st*, 22_st*, and akwf folder
  remove-st          Remove both 11_st* and 22_st*
  remove-st-22       Remove 22_st* only
  remove-st-11       Remove 11_st* only
  remove-akwf        Remove akwf folder
  status             Show installed sets
"#
    );
}

fn must_git() {
    let ok = Command::new("git")
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if !ok {
        eprintln!("git not found. Please install Git and ensure it’s on PATH.");
        std::process::exit(1);
    }
}

fn default_dirt_samples_dir() -> PathBuf {
    if cfg!(windows) {
        if let Ok(local_app) = env::var("LOCALAPPDATA") {
            return PathBuf::from(local_app)
                .join("SuperCollider")
                .join("downloaded-quarks")
                .join("Dirt-Samples");
        }
    }
    dirs_home()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("Dirt-Samples")
}

fn default_quarks_root() -> PathBuf {
    if cfg!(windows) {
        if let Ok(local_app) = env::var("LOCALAPPDATA") {
            return PathBuf::from(local_app)
                .join("SuperCollider")
                .join("downloaded-quarks");
        }
    }
    dirs_home()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("SuperCollider")
        .join("downloaded-quarks")
}

fn dirs_home() -> Option<PathBuf> {
    env::var_os("HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("USERPROFILE").map(PathBuf::from))
}

// ---------------- core ops ----------------

fn install_repo_filtered(
    repo: &Repo,
    dest: &Path,
    only_prefixes: Option<&[&str]>,
) -> io::Result<()> {
    fs::create_dir_all(dest)?;
    let tmp = temp_dir()?.join(format!("{}_dl", repo.name));
    if tmp.exists() {
        fs::remove_dir_all(&tmp).ok();
    }
    fs::create_dir_all(&tmp)?;

    println!("→ Cloning {} (branch {}) …", repo.url, repo.branch);
    let status = Command::new("git")
        .args([
            "clone",
            "--branch",
            repo.branch,
            "--single-branch",
            "--depth",
            "1",
            repo.url,
            tmp.to_string_lossy().as_ref(),
        ])
        .status()?;
    if !status.success() {
        return Err(io::Error::other("git clone failed"));
    }

    let src_dirt = tmp.join("dirt");
    if !src_dirt.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!(
                "no 'dirt/' directory in cloned repo: {}",
                src_dirt.display()
            ),
        ));
    }

    let entries = fs::read_dir(&src_dirt)?
        .filter_map(|e| e.ok())
        .collect::<Vec<_>>();

    // determine which prefixes to include
    let wanted = only_prefixes.unwrap_or(repo.dir_prefixes);

    for e in entries {
        let name = e.file_name().to_string_lossy().into_owned();
        if !wanted.iter().any(|p| name.starts_with(p)) {
            continue;
        }
        let src = e.path();
        let dst = dest.join(&name);

        if e.file_type()?.is_dir() {
            if dst.exists() {
                println!("= exists, skipping  {}", dst.display());
            } else {
                println!("+ add dir          {}", dst.display());
                copy_tree(&src, &dst)?;
            }
        } else if e.file_type()?.is_file() {
            if dst.exists() {
                println!("= exists, skipping  {}", dst.display());
            } else {
                println!("+ add file         {}", dst.display());
                if let Some(parent) = dst.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::copy(&src, &dst)?;
            }
        }
    }

    let _ = fs::remove_dir_all(&tmp);
    Ok(())
}

fn install_akwf(repo: &Repo, akwf_root: &Path) -> io::Result<()> {
    // similar to install_repo_filtered, but copies ONLY akwf_* from repo dirt/ into akwf_root/
    fs::create_dir_all(akwf_root)?;
    let tmp = temp_dir()?.join(format!("{}_dl", repo.name));
    if tmp.exists() {
        fs::remove_dir_all(&tmp).ok();
    }
    fs::create_dir_all(&tmp)?;

    println!("→ Cloning {} (branch {}) …", repo.url, repo.branch);
    let status = Command::new("git")
        .args([
            "clone",
            "--branch",
            repo.branch,
            "--single-branch",
            "--depth",
            "1",
            repo.url,
            tmp.to_string_lossy().as_ref(),
        ])
        .status()?;
    if !status.success() {
        return Err(io::Error::other("git clone failed"));
    }

    let src_dirt = tmp.join("dirt");
    if !src_dirt.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!(
                "no 'dirt/' directory in cloned repo: {}",
                src_dirt.display()
            ),
        ));
    }

    for e in fs::read_dir(&src_dirt)? {
        let e = e?;
        let name = e.file_name().to_string_lossy().into_owned();
        if !name.starts_with("akwf_") {
            continue;
        }
        let src = e.path();
        let dst = akwf_root.join(&name);
        if e.file_type()?.is_dir() {
            if dst.exists() {
                println!("= exists, skipping  {}", dst.display());
            } else {
                println!("+ add dir          {}", dst.display());
                copy_tree(&src, &dst)?;
            }
        }
    }

    let _ = fs::remove_dir_all(&tmp);
    Ok(())
}

fn remove_repo_filtered(
    repo: &Repo,
    dest: &Path,
    only_prefixes: Option<&[&str]>,
) -> io::Result<()> {
    let wanted = only_prefixes.unwrap_or(repo.dir_prefixes);
    if !dest.is_dir() {
        println!("(dest not found: {})", dest.display());
        return Ok(());
    }
    let mut removed_any = false;
    for entry in fs::read_dir(dest)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if wanted.iter().any(|p| name.starts_with(p)) {
            let p = entry.path();
            if entry.file_type()?.is_dir() {
                println!("- remove dir       {}", p.display());
                fs::remove_dir_all(&p)?;
            } else {
                println!("- remove file      {}", p.display());
                fs::remove_file(&p)?;
            }
            removed_any = true;
        }
    }
    if !removed_any {
        println!(
            "(no matching '{}' items to remove in {})",
            repo.name,
            dest.display()
        );
    }
    Ok(())
}

fn remove_akwf_root(akwf_root: &Path) -> io::Result<()> {
    if akwf_root.exists() {
        println!("- remove akwf root {}", akwf_root.display());
        fs::remove_dir_all(akwf_root)?;
    } else {
        println!("(akwf root not found: {})", akwf_root.display());
    }
    Ok(())
}

fn status(samples_root: &Path, akwf_root: &Path) {
    let (st_11, st_11_list) = count_by_prefixes(samples_root, &["11_st"]);
    let (st_22, st_22_list) = count_by_prefixes(samples_root, &["22_st"]);
    let akwf_count = if akwf_root.is_dir() {
        fs::read_dir(akwf_root)
            .map(|rd| {
                rd.filter_map(|e| e.ok())
                    .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
                    .filter(|e| e.file_name().to_string_lossy().starts_with("akwf_"))
                    .count()
            })
            .unwrap_or(0)
    } else {
        0
    };

    println!("Dirt-Samples: {}", samples_root.display());
    println!("  uxn-st  11_st* : {} folders", st_11);
    if !st_11_list.is_empty() {
        for d in st_11_list.iter().take(8) {
            println!("    - {}", d);
        }
        if st_11_list.len() > 8 {
            println!("    … and {} more", st_11_list.len() - 8);
        }
    }
    println!("  uxn-st  22_st* : {} folders", st_22);
    if !st_22_list.is_empty() {
        for d in st_22_list.iter().take(8) {
            println!("    - {}", d);
        }
        if st_22_list.len() > 8 {
            println!("    … and {} more", st_22_list.len() - 8);
        }
    }

    println!("AKWF root: {}", akwf_root.display());
    println!("  akwf_* folders : {}", akwf_count);
}

fn count_by_prefixes(dest: &Path, prefixes: &[&str]) -> (usize, Vec<String>) {
    if !dest.is_dir() {
        return (0, Vec::new());
    }
    let mut names = Vec::new();
    if let Ok(rd) = fs::read_dir(dest) {
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            if e.file_type().map(|t| t.is_dir()).unwrap_or(false)
                && prefixes.iter().any(|p| name.starts_with(p))
            {
                names.push(name);
            }
        }
    }
    names.sort();
    (names.len(), names)
}

// --------------- helpers ----------------

fn copy_tree(src: &Path, dst: &Path) -> io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in walkdir(src)? {
        let rel = entry.strip_prefix(src).unwrap();
        let target = dst.join(rel);
        if entry.is_dir() {
            fs::create_dir_all(&target)?;
        } else {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(&entry, &target)?;
        }
    }
    Ok(())
}

fn walkdir(root: &Path) -> io::Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    fn rec(cur: &Path, acc: &mut Vec<PathBuf>) -> io::Result<()> {
        for e in fs::read_dir(cur)? {
            let e = e?;
            let p = e.path();
            acc.push(p.clone());
            if e.file_type()?.is_dir() {
                rec(&p, acc)?;
            }
        }
        Ok(())
    }
    rec(root, &mut out)?;
    Ok(out)
}

fn temp_dir() -> io::Result<PathBuf> {
    let base = env::temp_dir();
    let here = base.join("tcrs_dirt_dl");
    fs::create_dir_all(&here)?;
    Ok(here)
}
