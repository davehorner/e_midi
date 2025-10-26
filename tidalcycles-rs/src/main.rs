// ...existing code...
mod install;

use crate::install::ensure_ghcup_installed;
use crate::install::ensure_supercollider_installed;
use rosc::decoder::decode_udp;
use rosc::OscPacket;
use std::fs;
use std::io::{BufRead, Write};
use std::net::UdpSocket as StdUdpSocket;
#[allow(unused_imports)]
use std::process::Command;
use std::process::{Command as StdCommand, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
#[allow(unused_imports)]
use std::time::Duration;
use tokio::process::Command as TokioCommand;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Parse CLI arguments for -f/--force and --spawn
    let mut force_kill = false;
    let mut spawn_mode = false;
    let mut filtered_args = Vec::new();
    for arg in std::env::args().skip(1) {
        if arg == "-f" || arg == "--force" {
            force_kill = true;
        } else if arg == "--spawn" {
            spawn_mode = true;
        } else {
            filtered_args.push(arg);
        }
    }
    // Check if a GHCi process is already running (indicating TidalCycles backend is likely running)
    #[cfg(windows)]
    {
        use std::ffi::OsStr;
        use sysinfo::{Signal, System};
        let mut system = System::new_all();
        system.refresh_all();
        let mut ghci_found = false;
        for process in system.processes_by_name(OsStr::new("ghci.exe")) {
            // Try to be more selective: check command line for port 57120 or BootTidal.hs
            let cmdline: String = process
                .cmd()
                .iter()
                .filter_map(|s| s.to_str())
                .collect::<Vec<_>>()
                .join(" ")
                .to_ascii_lowercase();
            if cmdline.contains("57120") || cmdline.contains("boottidal.hs") {
                ghci_found = true;
                if force_kill {
                    eprintln!(
                        "Killing likely TidalCycles GHCi process with PID {}...",
                        process.pid()
                    );
                    let _ = process.kill_with(Signal::Kill);
                    // Fallback: use taskkill in case sysinfo fails
                    let pid_str = process.pid().to_string();
                    let _ = std::process::Command::new("taskkill")
                        .args(["/PID", &pid_str, "/F"])
                        .output();
                }
            }
        }
        if ghci_found && !force_kill {
            eprintln!("A GHCi process (likely TidalCycles) is already running. Use -f or --force to kill it.");
            std::process::exit(100);
        }
        if ghci_found && force_kill {
            // Give the OS a moment to release the process
            std::thread::sleep(std::time::Duration::from_millis(500));
        }
    }
    #[cfg(unix)]
    {
        use std::ffi::OsStr;
        use sysinfo::{Signal, System};
        let mut system = System::new_all();
        system.refresh_all();
        let mut ghci_found = false;
        for process in system.processes_by_name(OsStr::new("ghci")) {
            ghci_found = true;
            if force_kill {
                eprintln!("Killing GHCi process with PID {}...", process.pid());
                let _ = process.kill_with(Signal::Kill);
            }
        }
        if ghci_found && !force_kill {
            eprintln!("A GHCi process is already running. TidalCycles backend may already be active. Use -f or --force to kill it.");
            std::process::exit(100);
        }
        if ghci_found && force_kill {
            std::thread::sleep(std::time::Duration::from_millis(500));
        }
    }
    // Check if the main OSC UDP port is already in use (indicating another instance is running)
    const MAIN_OSC_PORT: u16 = 57126;
    let osc_port_in_use = std::net::UdpSocket::bind(("127.0.0.1", MAIN_OSC_PORT)).is_err();
    if osc_port_in_use {
        eprintln!(
            "Another instance of tidalcycles-rs is already running (port {} in use).",
            MAIN_OSC_PORT
        );
        std::process::exit(100);
    }

    // ...singleton/process checks and force kill logic above...
    // Only spawn in background if we reach this point (no early exit)
    if spawn_mode {
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            use std::process::Command;
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            let exe_path = std::env::current_exe().expect("Failed to get current exe");
            let mut cmd = Command::new(&exe_path);
            cmd.args(&filtered_args);
            cmd.creation_flags(CREATE_NO_WINDOW);
            let _ = cmd.spawn();
        }
        #[cfg(unix)]
        {
            use std::process::Command;
            let exe = std::env::current_exe().expect("Failed to get current exe");
            let mut cmd = Command::new("nohup");
            let mut args = vec![exe.to_string_lossy().to_string()];
            args.extend(filtered_args.iter().cloned());
            let _ = cmd.args(&args).arg("&").spawn();
        }
        println!("Spawned in background. Exiting foreground process.");
        std::process::exit(0);
    }

    // Track all spawned child processes
    let children: Arc<Mutex<Vec<tokio::process::Child>>> = Arc::new(Mutex::new(Vec::new()));
    let children_for_ctrlc = children.clone();
    ctrlc::set_handler(move || {
        println!("\nCtrl+C received, killing all child processes...");
        let mut children = children_for_ctrlc.lock().unwrap();
        for child in children.iter_mut() {
            // let _ = child.kill();
            // Just drop the child, or call kill and drop
            let _ = child.start_kill();
        }
        std::process::exit(1);
    })
    .expect("Error setting Ctrl+C handler");

    let ghcup_path = ensure_ghcup_installed();
    // Ensure ghcup is installed
    if let Some(ghcup) = ghcup_path {
        // Install GHC and Cabal using ghcup
        let status = TokioCommand::new(&ghcup)
            .args(["install", "ghc"])
            .status()
            .await?;
        if !status.success() {
            eprintln!("Failed to install GHC with ghcup.");
            std::process::exit(1);
        }

        let status = TokioCommand::new(&ghcup)
            .args(["install", "cabal"])
            .status()
            .await?;
        if !status.success() {
            eprintln!("Failed to install Cabal with ghcup.");
            std::process::exit(1);
        }

        // Set GHC and Cabal as default
        let status = TokioCommand::new(&ghcup)
            .args(["set", "ghc"])
            .status()
            .await?;
        if !status.success() {
            eprintln!("Failed to set GHC as default with ghcup.");
            std::process::exit(1);
        }
        let status = TokioCommand::new(&ghcup)
            .args(["set", "cabal"])
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
        let status = TokioCommand::new(&cabal).arg("update").status().await?;
        if !status.success() {
            eprintln!("Failed to update cabal package list.");
            std::process::exit(1);
        }

        // Check if tidal is already installed
        let check_status = TokioCommand::new(&cabal)
            .args(["list", "--installed", "tidal"])
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
            let child = cmd
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()?;
            // Track this child
            children.lock().unwrap().push(child);
            // Take ownership of the child process to call wait_with_output
            let child = children.lock().unwrap().pop().unwrap();
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
    } else {
        eprintln!("ghcup is not installed and could not be installed automatically.");
        std::process::exit(1);
    }

    // Ensure SuperCollider is installed
    let sclang_path = ensure_supercollider_installed();
    if sclang_path.is_none() {
        eprintln!(
            "SuperCollider (sclang) is not installed and could not be installed automatically."
        );
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

    if !tidalcycles_rs::supercollider_sc3_plugins::is_sc3_plugins_installed() {
        // Install sc3-plugins if not already installed
        println!("Installing sc3-plugins...");
        tidalcycles_rs::supercollider_sc3_plugins::install_sc3_plugins()?;
    } else {
        println!("sc3-plugins are already installed.");
    }

    // === AUTOMATE TIDALLOOPER QUARK INSTALL ===
    // Use helper to install in Quarks dir
    // match tidalcycles_rs::supercollider_looper::ensure_tidallooper_quark_installed() {
    //     Ok(true) => println!("TidalLooper Quark cloned successfully to Quarks dir."),
    //     Ok(false) => println!("TidalLooper Quark already present in Quarks dir."),
    //     Err(e) => eprintln!("[WARN] {}", e),
    // }
    // Use helper to install in user Extensions dir (Windows plugin path)
    match tidalcycles_rs::supercollider_looper::ensure_tidallooper_in_user_extensions() {
        Ok(true) => println!("TidalLooper Quark cloned successfully to user Extensions dir."),
        Ok(false) => println!("TidalLooper Quark already present in user Extensions dir."),
        Err(e) => eprintln!("[WARN] {}", e),
    }

    // 1. Write startup .scd for headless SC
    let username = std::env::var("USERNAME").unwrap_or_else(|_| "User".to_string());
    let looper_path = format!(
        "C:/Users/{}/AppData/Local/SuperCollider/Extensions/tidal-looper",
        username
    )
    .replace("\\", "/");
    println!("TidalLooper path: {}", looper_path);

    // On Windows, check if port 57120 is open and kill any process using it, retrying a few times
    #[cfg(windows)]
    {
        let mut success = false;
        let mut last_pid: Option<u32> = None;
        for attempt in 0..7 {
            // Run: netstat -ano | findstr :57120
            let output = Command::new("cmd")
                .args(["/C", "netstat -ano | findstr :57120"])
                .output()
                .expect("Failed to run netstat");
            let stdout = String::from_utf8_lossy(&output.stdout);
            let mut killed = false;
            for line in stdout.lines() {
                // Example:  UDP    0.0.0.0:57120         *:*                                    1234
                let parts: Vec<&str> = line.split_whitespace().collect();
                if let Some(pid_str) = parts.last() {
                    if let Ok(pid) = pid_str.parse::<u32>() {
                        last_pid = Some(pid);
                        println!(
                            "Killing process with PID {} using port 57120... (attempt {})",
                            pid,
                            attempt + 1
                        );
                        // Use /T to kill child processes too, and /F for force
                        let _ = Command::new("taskkill")
                            .args(["/PID", &pid.to_string(), "/T", "/F"])
                            .output();
                        killed = true;
                    }
                }
            }
            // Also kill any running tidalcycles-rs processes (except self)
            let self_pid = std::process::id();
            let tasklist = Command::new("tasklist").output().unwrap();
            let tasklist_str = String::from_utf8_lossy(&tasklist.stdout);
            for line in tasklist_str.lines() {
                if line.to_ascii_lowercase().contains("tidalcycles-rs.exe") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() > 1 {
                        if let Ok(pid) = parts[1].parse::<u32>() {
                            if pid != self_pid {
                                println!(
                                    "Killing other tidalcycles-rs.exe process with PID {}...",
                                    pid
                                );
                                let _ = Command::new("taskkill")
                                    .args(["/PID", &pid.to_string(), "/T", "/F"])
                                    .output();
                            }
                        }
                    }
                }
            }
            // Try to bind to the port to check if it's free
            match StdUdpSocket::bind("0.0.0.0:57120") {
                Ok(_) => {
                    success = true;
                    break;
                }
                Err(_) => {
                    if killed {
                        println!("Waiting for port 57120 to be released...");
                        std::thread::sleep(Duration::from_millis(700 + attempt * 300));
                    } else {
                        println!("Port 57120 is still in use, but no process found to kill.");
                        std::thread::sleep(Duration::from_millis(700 + attempt * 300));
                    }
                }
            }
        }
        // Final attempt: if still not free, try to kill the last seen PID again
        if !success {
            if let Some(pid) = last_pid {
                println!("Final force kill attempt for PID {}...", pid);
                let _ = Command::new("taskkill")
                    .args(["/PID", &pid.to_string(), "/T", "/F"])
                    .output();
                std::thread::sleep(Duration::from_secs(2));
                // Try one last time
                if StdUdpSocket::bind("0.0.0.0:57120").is_ok() {
                    success = true;
                }
            }
        }
        if !success {
            eprintln!("Could not free up port 57120 after several forceful attempts. You may need to reboot or kill the process manually.");
            std::process::exit(1);
        }
    }

    let scd = format!(
        r#"
(
"[DEBUG] startup.scd begin".postln;
s.options.numBuffers = 16384;
s.options.memSize = 131072;
s.options.maxNodes = 1024;
s.options.maxSynthDefs = 1024;
s.options.numWireBufs = 128;
// Try to install TidalLooper Quark from user Extensions dir if not present
if((Quarks.installed.select(_.name == "TidalLooper")).isEmpty) {{
    Quarks.install("{}");
}};
if (SuperDirt.notNil) {{
    "[DEBUG] SuperDirt is present".postln;

    


    s.reboot {{ s.waitForBoot {{
        "[DEBUG] Server booted, starting SuperDirt".postln;
        ~dirt = SuperDirt(2, s);
        ~looper = TidalLooper(~dirt);
        ~dirt.loadSoundFiles;
        ~dirt.start(57120, 0 ! 12);
        ~d1 = ~dirt.orbits[0];
        ~superdirtPath = Quarks.folder +/+ "SuperDirt";
        this.executeFile(~superdirtPath +/+ "library" +/+ "default-synths-extra.scd");
        this.executeFile(~superdirtPath +/+ "library" +/+ "default-effects-extra.scd");

        // OSC code evaluation handler (after SuperDirt is started)
        (
        ~oscEval = OSCFunc({{ |msg, time, addr, recvPort|
            var code = msg[1];
            if (code.isString or: {{ code.isKindOf(Symbol) }}) {{
                code.asString.interpret;
            }} {{
                ("OSC /eval: code is not a String or Symbol: " ++ code.class).postln;
        }}
        }}, '/eval', nil);
        );
        "[DEBUG] SuperDirt started".postln;
        // 0.exit;
    }};
    }};
}} {{
    "[DEBUG] SuperDirt not found, installing...".postln;
    Quarks.install("SuperDirt");
    thisProcess.recompile;
    "SuperDirt installed. Please restart SuperCollider.".postln;
    0.exit;
}}
)
"#,
        looper_path
    );
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
                    if n == 0 {
                        break;
                    }
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
                    if n == 0 {
                        break;
                    }
                    print!("[SuperCollider stderr] {}", line);
                    line.clear();
                }
            });
        }
        // Wait for process to avoid zombie
        let _ = sc.wait();
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
        if let Some(ghci) = gpath {
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
                        if n == 0 {
                            break;
                        }
                        print!("[Tidal stdout] {}", line);
                    } else {
                        break;
                    }
                }
            });
            // Thread for reading stderr
            let err_thread = thread::spawn(move || {
                let mut line = String::new();
                loop {
                    line.clear();
                    if let Ok(n) = stderr.read_line(&mut line) {
                        if n == 0 {
                            break;
                        }
                        print!("[Tidal stderr] {}", line);
                    } else {
                        break;
                    }
                }
            });
            // Main loop: receive code from channel and write to stdin
            for code in tidal_rx {
                writeln!(stdin, "{}", code).ok();
                stdin.flush().ok();
            }
            let _ = out_thread.join();
            let _ = err_thread.join();
            // Wait for process to avoid zombie
            let _ = tidal.wait();
        } else {
            eprintln!("ghci is not installed or not found in PATH.");
        }
    });

    // === SPAWN OSC SERVER FOR TIDAL CONTROL ===
    thread::spawn(move || {
        // Try to bind UDP socket, and if it fails, attempt to kill any process using the port (Windows only)
        let sock = match StdUdpSocket::bind("0.0.0.0:57126") {
            Ok(s) => s,
            #[allow(unused_variables)]
            Err(e) => {
                #[cfg(windows)]
                {
                    println!("Port 57126 is in use, attempting to kill process using it (Windows only)...");
                    // Run: netstat -ano | findstr :57126
                    let output = Command::new("cmd")
                        .args(["/C", "netstat -ano | findstr :57126"])
                        .output()
                        .expect("Failed to run netstat");
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    for line in stdout.lines() {
                        // Example line:  UDP    0.0.0.0:57126         *:*                                    1234
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        if let Some(pid_str) = parts.last() {
                            if let Ok(pid) = pid_str.parse::<u32>() {
                                println!("Killing process with PID {} using port 57126...", pid);
                                let _ = Command::new("taskkill")
                                    .args(["/PID", &pid.to_string(), "/F"])
                                    .output();
                            }
                        }
                    }
                    // Try binding again
                    StdUdpSocket::bind("0.0.0.0:57126")
                        .expect("could not bind UDP socket after killing process")
                }
                #[cfg(not(windows))]
                {
                    panic!("could not bind UDP socket: {}", e);
                }
            }
        };
        let mut buf = [0u8; 2048];
        println!("OSC Tidal server listening on udp://0.0.0.0:57126 (send /tidal <string>)");
        loop {
            if let Ok((size, _addr)) = sock.recv_from(&mut buf) {
                if let Ok((_, OscPacket::Message(msg))) = decode_udp(&buf[..size]) {
                    if msg.addr == "/tidal" {
                        if let Some(rosc::OscType::String(code)) = msg.args.first() {
                            println!("[OSC] Received Tidal code: {}", code);
                            tidal_tx.send(code.clone()).ok();
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
}
