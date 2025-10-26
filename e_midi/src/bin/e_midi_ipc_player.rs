// This example demonstrates subscribing to the zero-copy IPC event stream and overriding the voice (instrument/program) for each note in real time.
// It listens for MidiNoteEvent via IPC, and when a note is triggered, it plays it with a random voice override for all notes.
// This is a pure IPC-based override: the client does not use any special player-side override, but rewrites the channel/program in the note data before playback.

use e_midi::MidiPlayer;
use e_midi_shared::ipc_protocol::MidiNoteEvent;
use e_midi_shared::midi::gm_instrument_name;
use iceoryx2::prelude::*;
use rand::Rng;
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Subscribe to zero-copy MidiNoteEvent stream from e_midi
    let node = NodeBuilder::new().create::<iceoryx2::service::ipc::Service>()?;
    let event_service = node
        .service_builder(&ServiceName::new(e_midi_shared::ipc::EMIDI_EVENTS_SERVICE)?)
        .publish_subscribe::<MidiNoteEvent>()
        .open()?;
    let event_sub = event_service.subscriber_builder().create()?;
    let midi_player = MidiPlayer::new()?;
    println!("[demo] Subscribed to zero-copy MidiNoteEvent stream");

    // This is the voice override for this client (random for demo)
    let mut rng = rand::rng();
    let mut my_voice = rng.random_range(0..16) as u8;
    let mut last_voice_change = std::time::Instant::now();
    println!(
        "[demo] This client will override all notes to voice {} ({})",
        my_voice,
        gm_instrument_name(my_voice)
    );

    loop {
        // Change voice every 4 seconds
        if last_voice_change.elapsed().as_secs() >= 4 {
            my_voice = rng.random_range(0..16) as u8;
            println!(
                "[demo] Changing override voice to {} ({})",
                my_voice,
                gm_instrument_name(my_voice)
            );
            last_voice_change = std::time::Instant::now();
        }
        match event_sub.receive() {
            Ok(samples) => {
                // Handle both Option<Sample> and Vec<Sample>
                // If samples is Some(sample):
                if let Some(sample) = samples {
                    let midi_event: &MidiNoteEvent = sample.payload();
                    if midi_event.kind == 0 && midi_event.velocity > 0 {
                        let _ = midi_player
                            .get_command_sender()
                            .send(e_midi::MidiCommand::SendMessage(vec![0xC0, my_voice]));
                        let _ = midi_player.get_command_sender().send(
                            e_midi::MidiCommand::SendMessage(vec![
                                0x90,
                                midi_event.pitch,
                                midi_event.velocity,
                            ]),
                        );
                        println!(
                            "[ipc] NoteOn: pitch={} velocity={} (overridden to channel 0, voice {} - {})",
                            midi_event.pitch, midi_event.velocity, my_voice, gm_instrument_name(my_voice)
                        );
                    } else if midi_event.kind == 1 {
                        let _ = midi_player.get_command_sender().send(
                            e_midi::MidiCommand::SendMessage(vec![0x80, midi_event.pitch, 0]),
                        );
                        println!(
                            "[ipc] NoteOff: pitch={} (overridden to channel 0)",
                            midi_event.pitch
                        );
                    }
                }
                // If samples is Vec<Sample>:
                // for sample in samples { ... } // (uncomment if needed)
            }
            Err(e) => {
                eprintln!(
                    "[demo] Error receiving events: {}. Will attempt to reconnect...",
                    e
                );
                std::thread::sleep(Duration::from_millis(200));
            }
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}
