use tokio::process::Command;
use tokio::io::AsyncWriteExt;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let ghci_path = tidalcycles_rs::find::find_ghci();
    if ghci_path.is_none() {
        eprintln!("ghci is not installed or not found in PATH.");
        std::process::exit(1);
    }
    let ghci = ghci_path.unwrap();
    let mut child = Command::new(&ghci)
        .args(&["-package", "tidal","-XOverloadedStrings"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .spawn()?;

    let lines = [
        "import Sound.Tidal.Context\n",
        "tidal <- startTidal (superdirtTarget {oLatency = 0.1, oAddress = \"127.0.0.1\", oPort = 57120}) (defaultConfig {cFrameTimespan = 1/20})\n",
        "let d1 = streamReplace tidal 1\n",
        "let d2 = streamReplace tidal 2\n",
        "let d3 = streamReplace tidal 3\n",
        "let d4 = streamReplace tidal 4\n",
        "let d5 = streamReplace tidal 5\n",
        "let d6 = streamReplace tidal 6\n",
        "let d7 = streamReplace tidal 7\n",
        "let d8 = streamReplace tidal 8\n",
        ":set prompt \"\"\n",
        "cps 0.9\n",
        "-- ready\n",
    ];

    if let Some(mut stdin) = child.stdin.take() {
        for line in lines {
            stdin.write_all(line.as_bytes()).await?;
            stdin.flush().await?;
            tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        }
        // Wait for Tidal to finish booting
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        // Now send your pattern
        stdin.write_all(b"d1 $ sound \"bd sn cp*2 [~ bd/2]\"\n").await?;
        stdin.flush().await?;
            // Wait longer for Tidal to finish booting
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    // Now send your pattern
    stdin.write_all(b"d1 $ sound \"bd sn cp*2 [~ bd/3]\"\n").await?;
    stdin.flush().await?;

    // Example: send a new pattern after 8 seconds
    tokio::time::sleep(std::time::Duration::from_secs(8)).await;
    stdin.write_all(b"d1 $ sound \"cp future*4\"\n").await?;
    stdin.flush().await?;
    tokio::time::sleep(std::time::Duration::from_secs(8)).await;

    stdin.write_all(b"d1 $ sound \"bd*2 [[~ lt] sn:3] lt:1 [ht mt*2]\"\n").await?;
    stdin.flush().await?;
    tokio::time::sleep(std::time::Duration::from_secs(8)).await;
    
    }

    // Keep the process running until Ctrl+C or until ghci exits
    println!("TidalCycles is running. Press Ctrl+C to exit.");
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            println!("Ctrl+C received, exiting.");
        }
        status = child.wait() => {
            println!("ghci exited with status: {:?}", status);
        }
    }
    Ok(())
}