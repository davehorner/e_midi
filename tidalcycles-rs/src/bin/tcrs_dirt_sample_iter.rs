use std::io::{self, Write};
use std::net::UdpSocket;
use std::thread::sleep;
use std::time::Duration;
use tidalcycles_rs::supercollider_dirt::{default_dirt_samples_dir, DirtSampleMap};

fn main() {
    // Get the Dirt-Samples directory
    let dirt_dir = match default_dirt_samples_dir() {
        Some(path) => path,
        None => {
            eprintln!("Could not find Dirt-Samples directory");
            return;
        }
    };
    println!("Using Dirt-Samples dir: {}", dirt_dir.display());

    // Build the DirtSampleMap
    let dirt_map = DirtSampleMap::from_dir(&dirt_dir);
    if dirt_map.bank_to_files.is_empty() {
        println!("No sample banks found.");
        return;
    }

    println!("Would you like to play only the first sample in each bank, or all samples in all banks?");
    println!("Enter 1 for first sample only, 2 for all samples:");
    print!("> ");
    io::stdout().flush().unwrap();
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    let play_all = matches!(input.trim(), "2" | "all");

    // Set up OSC socket (to SuperDirt, default port 57120)
    let osc_addr = "127.0.0.1:57120";
    let sock = UdpSocket::bind("0.0.0.0:0").expect("bind");

    if play_all {
        // Play all samples in all banks
        for (bank, files) in &dirt_map.bank_to_files {
            for (idx, file) in files.iter().enumerate() {
                println!("Triggering SuperDirt: bank='{}', index={}, file={}", bank, idx, file);
                let osc_packet = rosc::encoder::encode(&rosc::OscPacket::Message(
                    rosc::OscMessage {
                        addr: "/dirt/play".to_string(),
                        args: vec![
                            rosc::OscType::String("s".to_string()),
                            rosc::OscType::String(bank.clone()),
                            rosc::OscType::String("n".to_string()),
                            rosc::OscType::Int(idx as i32),
                            rosc::OscType::String("orbit".to_string()),
                            rosc::OscType::Int(0),
                        ],
                    },
                ))
                .unwrap();
                sock.send_to(&osc_packet, osc_addr).unwrap();
                sleep(Duration::from_millis(800));
            }
        }
    } else {
        // Play only the first sample in each bank
        for (bank, files) in &dirt_map.bank_to_files {
            if !files.is_empty() {
                println!("Triggering SuperDirt: bank='{}', index=0, file={}", bank, files[0]);
                let osc_packet = rosc::encoder::encode(&rosc::OscPacket::Message(
                    rosc::OscMessage {
                        addr: "/dirt/play".to_string(),
                        args: vec![
                            rosc::OscType::String("s".to_string()),
                            rosc::OscType::String(bank.clone()),
                            rosc::OscType::String("n".to_string()),
                            rosc::OscType::Int(0),
                            rosc::OscType::String("orbit".to_string()),
                            rosc::OscType::Int(0),
                        ],
                    },
                ))
                .unwrap();
                sock.send_to(&osc_packet, osc_addr).unwrap();
                sleep(Duration::from_millis(800));
            }
        }
    }
    println!("Done iterating sample banks.");
}
