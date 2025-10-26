//! Example 02: Persistent Event Listener Demo
// Listens for IPC events from e_midi and prints them in real time.
// By default, launches e_midi to play song 0, then continues running and displays events from any e_midi instance with IPC enabled.
// This is a persistent event listener: it will show events from the e_midi it launches, and from any other e_midi process that publishes events.
//
// Usage: Run this demo to see real-time MIDI note events from e_midi. It is useful for debugging and monitoring event flow.
//
// NOTE: This demo is Windows-only.

use e_midi_shared::ipc;
use e_midi_shared::ipc_protocol::MidiNoteEvent;
use iceoryx2::prelude::*;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // --- Print demo information ---
    println!("\n=== Example 02: Persistent Event Listener Demo ===");
    println!("This demo launches e_midi to play song 0 with IPC enabled, then listens for and displays all incoming MIDI note events.\n");
    println!("- It will show events from the e_midi it launches, and from any other e_midi process that publishes events with IPC enabled.");
    println!("- Useful for debugging and monitoring event flow.\n");
    println!("(Press Ctrl+C to exit)\n");

    // --- Start the e_midi server process (plays song 0 with IPC enabled) ---
    // println!("[demo02_event_listener] Spawning e_midi server: cargo run --message-format=short --bin e_midi -- --ipc play 0");
    // let mut child = Command::new("cargo")
    //     .arg("run")
    //     .arg("--message-format=short")
    //     .arg("--bin")
    //     .arg("e_midi")
    //     .arg("--")
    //     .arg("--ipc")
    //     .arg("play")
    //     .arg("1")
    //     .stdout(Stdio::piped())
    //     .stderr(Stdio::piped())
    //     .spawn()?;

    println!("[demo02_event_listener] Spawning e_midi server: e_midi --ipc play 0");
    let mut child = Command::new("e_midi")
        .arg("--ipc")
        .arg("play")
        .arg("0")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();
    let stdout_reader = BufReader::new(stdout);
    let stderr_reader = BufReader::new(stderr);
    // Spawn a thread to print server stdout
    std::thread::spawn(move || {
        for line in stdout_reader.lines().map_while(Result::ok) {
            println!("[e_midi stdout] {}", line);
        }
    });
    // Spawn a thread to print server stderr
    std::thread::spawn(move || {
        for line in stderr_reader.lines().map_while(Result::ok) {
            eprintln!("[e_midi stderr] {}", line);
        }
    });
    // --- Wait a moment for the server to start ---
    std::thread::sleep(Duration::from_secs(2));

    // --- Use shared IPC library to create event subscriber ---
    let node = NodeBuilder::new().create::<iceoryx2::service::ipc::Service>()?;
    let event_service = node
        .service_builder(&ServiceName::new(ipc::EMIDI_EVENTS_SERVICE)?)
        .publish_subscribe::<MidiNoteEvent>()
        .open()?;
    let event_subscriber = event_service.subscriber_builder().create()?;
    println!("[demo02_event_listener] Connected to IPC event stream using shared library (zero-copy MidiNoteEvent)");

    loop {
        match event_subscriber.receive() {
            Ok(samples) => {
                if let Some(sample) = samples {
                    let midi_event: &MidiNoteEvent = sample.payload();
                    println!(
                        "[demo02_event_listener] MidiNoteEvent: kind={}, channel={}, pitch={}, velocity={}, timestamp={}",
                        midi_event.kind, midi_event.channel, midi_event.pitch, midi_event.velocity, midi_event.timestamp
                    );
                }
            }
            Err(e) => {
                eprintln!("[demo02_event_listener] Error receiving events: {}. Will attempt to reconnect...", e);
                std::thread::sleep(Duration::from_millis(200));
            }
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}
