mod install;

use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::{fs};
use tokio::{process::Command as TokioCommand, time::sleep};
use crate::install::ensure_supercollider_installed;
use crate::install::ensure_ghcup_installed;
use tokio::process::Child;
use ctrlc;
use rosc::decoder::decode_udp;
use rosc::OscPacket;
use std::net::UdpSocket as StdUdpSocket;
use std::thread;
use std::process::{Stdio, Command as StdCommand, ChildStdin};
use std::io::{Write, BufRead};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Track all spawned child processes
    let children: Arc<Mutex<Vec<tokio::process::Child>>> = Arc::new(Mutex::new(Vec::new()));
    let children_for_ctrlc = children.clone();
    ctrlc::set_handler(move || {
        println!("\nCtrl+C received, killing all child processes...");
        let mut children = children_for_ctrlc.lock().unwrap();
        for child in children.iter_mut() {
            let _ = child.kill();
        }
        std::process::exit(1);
    }).expect("Error setting Ctrl+C handler");

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

            println!("Installing tidal with cabal (this may take a while)...");
            let mut cmd = TokioCommand::new(&cabal);
            cmd.args(["v1-install", "tidal", "--force-reinstalls", "--verbose"]);
            let child = cmd.stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()?;
            // Track this child
            children.lock().unwrap().push(child);
            // Take ownership of the child process to call wait_with_output
            let mut child = children.lock().unwrap().pop().unwrap();
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
if (SuperDirt.notNil) {
    "[DEBUG] SuperDirt is present".postln;
    s.reboot { s.waitForBoot {
        "[DEBUG] Server booted, starting SuperDirt".postln;
        ~dirt = SuperDirt(2, s);
        ~dirt.loadSoundFiles;
        ~dirt.start(57120, 0 ! 12);
        ~d1 = ~dirt.orbits[0];
        
        // OSC code evaluation handler (after SuperDirt is started)
        (
        ~oscEval = OSCFunc({ |msg, time, addr, recvPort|
            var code = msg[1];
            if (code.isString or: { code.isKindOf(Symbol) }) {
                code.asString.interpret;
            } {
                ("OSC /eval: code is not a String or Symbol: " ++ code.class).postln;
            }
        }, '/eval', nil);
        );
        "[DEBUG] SuperDirt started".postln;
        // 0.exit;
    };
    };
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
    // Ensure startup.scd is deleted on exit
    let _cleanup = scopeguard::guard((), |_| {
        let _ = fs::remove_file("startup.scd");
    });
    // 2. Launch SuperCollider headless and check for SuperDirt install message
    let sclang_clone = sclang.clone();
    thread::spawn(move || {
        let mut sc = StdCommand::new(&sclang_clone)
            .arg("startup.scd")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Failed to launch SuperCollider");
        // Print SuperCollider output in background threads
        if let Some(stdout) = sc.stdout.take() {
            let mut reader = std::io::BufReader::new(stdout);
            thread::spawn(move || {
                let mut line = String::new();
                while let Ok(n) = reader.read_line(&mut line) {
                    if n == 0 { break; }
                    print!("[SuperCollider stdout] {}", line);
                    line.clear();
                }
            });
        }
        if let Some(stderr) = sc.stderr.take() {
            let mut reader = std::io::BufReader::new(stderr);
            thread::spawn(move || {
                let mut line = String::new();
                while let Ok(n) = reader.read_line(&mut line) {
                    if n == 0 { break; }
                    print!("[SuperCollider stderr] {}", line);
                    line.clear();
                }
            });
        }
        // Do not wait; let process run in background
    });
    println!("SuperCollider launched in background..");
    // 4. Send pattern to SuperCollider via file argument (optional, can be removed)
//     let pattern_code = r#"
// ~d1 = ~dirt.orbits[0];
// ~d1.soundLibrary.addMIDI(\\tidal, (type: \\midi, midiout: MIDIOut(0)));
// ~d1.sendMsg("/dirt/play", \"d1 $ s \\\"bd sn\\\" # gain \\\"0.8\\\" # orbit \\\"0\\\"\");
// 0.exit;
// "#;
//     fs::write("pattern.scd", pattern_code)?;
//     let mut sc_pattern = TokioCommand::new(&sclang)
//         .arg("-D")
//         .arg("pattern.scd")
//         .stdout(std::process::Stdio::piped())
//         .stderr(std::process::Stdio::piped())
//         .spawn()?;
//     children.lock().unwrap().push(sc_pattern);
//     // Take ownership of the child process to call wait_with_output
//     let mut sc_pattern = children.lock().unwrap().pop().unwrap();
//     let output = sc_pattern.spawn().await?;
//     println!("SuperCollider pattern output:\n{}", String::from_utf8_lossy(&output.stdout));

    // === SPAWN TIDALCYCLES (GHCi) IN A THREAD WITH IO ===
    let (tidal_tx, tidal_rx) = std::sync::mpsc::channel::<String>();
    thread::spawn(move || {
        let gpath = tidalcycles_rs::find::find_ghci();
        if gpath.is_none() {
            eprintln!("ghci is not installed or not found in PATH.");
        } else {
            let ghci = gpath.unwrap();

            // Write modern BootTidal.hs for TidalCycles >=1.9
            let boot_code = r#"
            :set -fno-warn-orphans -Wno-type-defaults -XMultiParamTypeClasses -XOverloadedStrings
            :set prompt ""
            :set prompt-cont ""

            import Sound.Tidal.Boot

            default (Rational, Integer, Double, Pattern String)

            tidalInst <- mkTidal
            -- To customize, use mkTidalWith:
            -- tidalInst <- mkTidalWith [(superdirtTarget { oLatency = 0.1 }, [superdirtShape])] (defaultConfig {cFrameTimespan = 1/20})

            instance Tidally where tidal = tidalInst

            -- Uncomment to enable Ableton Link sync:
            -- enableLink

            -- Custom aliases can go here
            -- fastsquizzed pat = fast 2 $ pat # squiz 1.5

            :set prompt "tidal> "
            :set prompt-cont ""
            "#;
            std::fs::write("BootTidal.hs", boot_code).expect("Failed to write BootTidal.hs");
            let _cleanup = scopeguard::guard((), |_| {
                let _ = fs::remove_file("BootTidal.hs");
            });
            let mut tidal = StdCommand::new(&ghci)
                .arg("-ghci-script=BootTidal.hs") // adjust path as needed
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .expect("Failed to spawn GHCi/TidalCycles");
            let mut stdin = tidal.stdin.take().unwrap();
            let mut stdout = std::io::BufReader::new(tidal.stdout.take().unwrap());
            let mut stderr = std::io::BufReader::new(tidal.stderr.take().unwrap());
            // Thread for reading stdout
            let out_thread = thread::spawn(move || {
                let mut line = String::new();
                loop {
                    line.clear();
                    if let Ok(n) = stdout.read_line(&mut line) {
                        if n == 0 { break; }
                        print!("[Tidal stdout] {}", line);
                    } else { break; }
                }
            });
            // Thread for reading stderr
            let err_thread = thread::spawn(move || {
                let mut line = String::new();
                loop {
                    line.clear();
                    if let Ok(n) = stderr.read_line(&mut line) {
                        if n == 0 { break; }
                        print!("[Tidal stderr] {}", line);
                    } else { break; }
                }
            });
            // Main loop: receive code from channel and write to stdin
            for code in tidal_rx {
                writeln!(stdin, "{}", code).ok();
                stdin.flush().ok();
            }
            let _ = out_thread.join();
            let _ = err_thread.join();

        }
    });

    // === SPAWN OSC SERVER FOR TIDAL CONTROL ===
    thread::spawn(move || {
        let sock = StdUdpSocket::bind("0.0.0.0:57126").expect("could not bind UDP socket");
        let mut buf = [0u8; 2048];
        println!("OSC Tidal server listening on udp://0.0.0.0:57126 (send /tidal <string>)");
        loop {
            if let Ok((size, _addr)) = sock.recv_from(&mut buf) {
                if let Ok((_, packet)) = decode_udp(&buf[..size]) {
                    if let OscPacket::Message(msg) = packet {
                        if msg.addr == "/tidal" {
                            if let Some(rosc::OscType::String(code)) = msg.args.get(0) {
                                println!("[OSC] Received Tidal code: {}", code);
                                tidal_tx.send(code.clone()).ok();
                            }
                        }
                    }
                }
            }
        }
    });

    loop {
        // Keep the main thread alive
        thread::park();
    }
    Ok(())
}
