mod install;

use std::time::Duration;
use std::{fs};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::{process::Command as TokioCommand, time::sleep};
use crate::install::ensure_supercollider_installed;
use crate::install::ensure_ghcup_installed;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let ghcup_path = ensure_ghcup_installed();
    // Ensure ghcup is installed
    if ghcup_path.is_none() {
        eprintln!("ghcup is not installed and could not be installed automatically.");
        std::process::exit(1);
    } else {
        let ghcup= ghcup_path.unwrap();
        // Install GHC and Cabal using ghcup
        let status = TokioCommand::new(&ghcup)
            .args(&["install", "ghc"])
            .status()
            .await?;
        if !status.success() {
            eprintln!("Failed to install GHC with ghcup.");
            std::process::exit(1);
        }

        let status = TokioCommand::new(&ghcup)
            .args(&["install", "cabal"])
            .status()
            .await?;
        if !status.success() {
            eprintln!("Failed to install Cabal with ghcup.");
            std::process::exit(1);
        }

        // Set GHC and Cabal as default
        let status = TokioCommand::new(&ghcup)
            .args(&["set", "ghc"])
            .status()
            .await?;
        if !status.success() {
            eprintln!("Failed to set GHC as default with ghcup.");
            std::process::exit(1);
        }
        let status = TokioCommand::new(&ghcup)
            .args(&["set", "cabal"])
            .status()
            .await?;
        if !status.success() {
            eprintln!("Failed to set Cabal as default with ghcup.");
            std::process::exit(1);
        }

        let cabal_path = install::ensure_cabal_installed();
        if cabal_path.is_none() {
            eprintln!("cabal is not installed and could not be installed automatically.");
            std::process::exit(1);
        }
        let cabal = cabal_path.unwrap();
        // Update cabal package list
        let status = TokioCommand::new(&cabal)
            .arg("update")
            .status()
            .await?;
        if !status.success() {
            eprintln!("Failed to update cabal package list.");
            std::process::exit(1);
        }

        // Check if tidal is already installed
        let check_status = TokioCommand::new(&cabal)
            .args(&["list", "--installed", "tidal"])
            .output()
            .await?;
        let output = String::from_utf8_lossy(&check_status.stdout);
        println!("Cabal installed packages:\n{}", output);
        if !output.contains("tidal") {

            // // Delete contents of GHC and Cabal directories in AppData
            // let user_profile = std::env::var("USERPROFILE").unwrap_or_default();
            // let dirs = [
            //     format!(r"{}\AppData\Roaming\ghc", user_profile),
            //     format!(r"{}\AppData\Roaming\cabal", user_profile),
            //     format!(r"{}\AppData\Local\ghc", user_profile),
            //     format!(r"{}\AppData\Local\cabal", user_profile),
            // ];

            // for dir in dirs.iter() {
            //     let path = Path::new(dir);
            //     if path.exists() && path.is_dir() {
            //         match fs::read_dir(path) {
            //             Ok(entries) => {
            //                 for entry in entries {
            //                     if let Ok(entry) = entry {
            //                         let entry_path = entry.path();
            //                         if entry_path.is_dir() {
            //                             if let Err(e) = fs::remove_dir_all(&entry_path) {
            //                                 eprintln!("Failed to remove directory {:?}: {}", entry_path, e);
            //                             }
            //                         } else {
            //                             if let Err(e) = fs::remove_file(&entry_path) {
            //                                 eprintln!("Failed to remove file {:?}: {}", entry_path, e);
            //                             }
            //                         }
            //                     }
            //                 }
            //             }
            //             Err(e) => eprintln!("Failed to read directory {:?}: {}", path, e),
            //         }
            //     }
            // }


            // Install tidal as a library with verbose output and a timeout
            use tokio::time::timeout;
            use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};
use futures::stream::{StreamExt, select};
use futures::future;
            println!("Installing tidal with cabal (this may take a while)...");
            let mut cmd = TokioCommand::new(&cabal);
            cmd.args(["v1-install", "tidal", "--force-reinstalls", "--verbose"]);
            let child = cmd.stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()?;
            // Set a timeout (e.g., 10 minutes)
            let output = child.wait_with_output().await?;
            let out = String::from_utf8_lossy(&output.stdout);
            let err = String::from_utf8_lossy(&output.stderr);
            println!("cabal install tidal output:\n{}\n{}", out, err);
            if !output.status.success() {
                eprintln!("Failed to install tidal with cabal.");
                // std::process::exit(1);
            }
            
        } else {
            println!("tidal is already installed.");
        }
    }

    // Ensure SuperCollider is installed
    let sclang_path = ensure_supercollider_installed();
    if sclang_path.is_none() {
        eprintln!("SuperCollider (sclang) is not installed and could not be installed automatically.");
        std::process::exit(1);
    }

    let sclang = sclang_path.unwrap();
    // Ensure C:\ghcup\bin and MSYS2 paths are in PATH
    // let ghcup_bin = r"C:\ghcup\bin";
    // let msys2_usr_bin = r"C:\msys64\usr\bin";
    // let msys2_mingw64_bin = r"C:\msys64\mingw64\bin";

    // let mut paths = std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default()).collect::<Vec<_>>();
    // if !paths.iter().any(|p| p == ghcup_bin) {
    //     paths.push(ghcup_bin.into());
    // }
    // if !paths.iter().any(|p| p == msys2_usr_bin) {
    //     paths.push(msys2_usr_bin.into());
    // }
    // if !paths.iter().any(|p| p == msys2_mingw64_bin) {
    //     paths.push(msys2_mingw64_bin.into());
    // }
    // let new_path = std::env::join_paths(paths)?;
    // std::env::set_var("PATH", &new_path);

    tidalcycles_rs::find::find_tools_set_env_path();

    // 1. Write startup .scd for headless SC
    let scd = r#"
(
"[DEBUG] startup.scd begin".postln;
s.options.numBuffers = 4096;
s.options.memSize = 131072;
s.options.maxNodes = 1024;
s.options.maxSynthDefs = 1024;
s.options.numWireBufs = 128;
if(SuperDirt.notNil) {
    "[DEBUG] SuperDirt is present".postln;
    s.reboot { s.waitForBoot {
        "[DEBUG] Server booted, starting SuperDirt".postln;
        ~dirt = SuperDirt(2, s);
        ~dirt.loadSoundFiles;
        ~dirt.start(57120, 0 ! 12);
        ~d1 = ~dirt.orbits[0];
        "[DEBUG] SuperDirt started".postln;
       // 0.exit;
    };
    }
} {
    "[DEBUG] SuperDirt not found, installing...".postln;
    Quarks.install("SuperDirt");
    thisProcess.recompile;
    "SuperDirt installed. Please restart SuperCollider.".postln;
    0.exit;
}
)
"#;
    fs::write("startup.scd", scd)?;

    // 2. Launch SuperCollider headless and check for SuperDirt install message
    let mut sc = TokioCommand::new(&sclang)
        .arg("-D")
        .arg("startup.scd")
        // .stdout(std::process::Stdio::piped())
        // .stderr(std::process::Stdio::piped())
        .spawn()?;
    println!("Waiting for SuperCollider to boot and check for SuperDirt installation...");
    let output = sc.wait_with_output().await?;
    let sc_stdout = String::from_utf8_lossy(&output.stdout);
    let sc_stderr = String::from_utf8_lossy(&output.stderr);
    let sc_output = format!("{}\n{}", sc_stdout, sc_stderr);
    println!("SuperCollider output:\n{}", sc_output);
    if sc_output.contains("SuperDirt installed. Please restart SuperCollider.") {
        println!("SuperDirt was just installed. Restarting SuperCollider to load SuperDirt...");
        let mut sc2 = TokioCommand::new(&sclang)
            .arg("-D")
            .arg("startup.scd")
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()?;
        sleep(Duration::from_secs(5)).await;
        let _ = sc2.wait_with_output().await?;
    }
    // 3. Wait for server to boot
    sleep(Duration::from_secs(5)).await;
    // 4. Send pattern to SuperCollider via file argument
    let pattern_code = r#"
~d1 = ~dirt.orbits[0];
~d1.soundLibrary.addMIDI(\\tidal, (type: \\midi, midiout: MIDIOut(0)));
~d1.sendMsg("/dirt/play", \"d1 $ s \\\"bd sn\\\" # gain \\\"0.8\\\" # orbit \\\"0\\\"\");
0.exit;
"#;
    fs::write("pattern.scd", pattern_code)?;
    let mut sc_pattern = TokioCommand::new(&sclang)
        .arg("-D")
        .arg("pattern.scd")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()?;
    let output = sc_pattern.wait_with_output().await?;
    println!("SuperCollider pattern output:\n{}", String::from_utf8_lossy(&output.stdout));
    Ok(())
}
