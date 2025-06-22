use e_grid::GridClient;
use e_grid::ipc_protocol::{WindowFocusEvent, WindowEvent};
use e_midi::MidiPlayer;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use e_grid::ipc_server::start_server;

fn main() -> Result<(), Box<dyn std::error::Error>> {
//    let mut client = GridClient::new().unwrap();

    let mut client: Option<GridClient> = None;
    match GridClient::new() {
        Ok(c) => {
            client = Some(c);
        }
        Err(_) => {
            println!("Grid server not running, starting server in-process...");
            // Start the server in a background thread
            std::thread::spawn(|| {
                // Call your server main or run function here
                start_server().unwrap();
            });
            // Retry loop: try to connect up to 10 times, waiting 300ms each time
            let mut last_err = None;
            for _ in 0..10 {
                match GridClient::new() {
                    Ok(c) => {
                        println!("Connected to in-process server!");
                        client = Some(c);
                        break;
                    }
                    Err(e) => {
                        last_err = Some(e);
                        std::thread::sleep(std::time::Duration::from_millis(300));
                    }
                }
            }
            if client.is_none() {
                panic!("Failed to connect to in-process server: {:?}", last_err);
            }
        }
    }
    let mut client = client.unwrap();

    let mut midi_player = MidiPlayer::new().unwrap();
    let total_songs = midi_player.get_total_song_count();
    println!("🎵 e_midi: {} songs available", total_songs);

    let song_map = Arc::new(Mutex::new(HashMap::<u64, usize>::new()));
    let next_song = Arc::new(Mutex::new(0usize));
    let midi_player = Arc::new(Mutex::new(midi_player));

    // Assign a song index to each window as it is created, and clean up on destroy
    client.set_window_event_callback({
        let song_map = Arc::clone(&song_map);
        let next_song = Arc::clone(&next_song);
        let midi_player = Arc::clone(&midi_player);
        move |event: WindowEvent| {
            match event.event_type {
                0 => { // CREATED
                    let hwnd = event.hwnd;
                    let mut idx = next_song.lock().unwrap();
                    let song_index = *idx % total_songs;
                    *idx += 1;
                    song_map.lock().unwrap().insert(hwnd, song_index);
                    println!("🎵 Assigned song {} to HWND {}", song_index, hwnd);
                }
                2 => { // FOCUS
                    let hwnd = event.hwnd;
                    let song_map = song_map.lock().unwrap();
                    if let Some(&song_index) = song_map.get(&hwnd) {
                        let mut midi_player = midi_player.lock().unwrap();
                        let _ = midi_player.play_song(song_index, None, None);
                        println!("▶️ Play song {} for HWND {} (focus)", song_index, hwnd);
                    }
                }
                3 => { // DEFOCUS
                    let hwnd = event.hwnd;
                    let song_map = song_map.lock().unwrap();
                    if let Some(&song_index) = song_map.get(&hwnd) {
                        let mut midi_player = midi_player.lock().unwrap();
                        midi_player.stop_playback();
                        println!("⏹️ Stop song {} for HWND {} (defocus)", song_index, hwnd);
                    }
                }
                1 => { // DESTROYED
                    let hwnd = event.hwnd;
                    if let Some(song_index) = song_map.lock().unwrap().remove(&hwnd) {
                        let mut midi_player = midi_player.lock().unwrap();
                        midi_player.stop_playback();
                        println!("⏹️ Stop song {} for HWND {} (window destroyed)", song_index, hwnd);
                    }
                }
                _ => {}
            }
        }
    }).unwrap();

    // Play/stop song on focus/unfocus
    client.set_focus_callback({
        let song_map = Arc::clone(&song_map);
        let midi_player = Arc::clone(&midi_player);
        move |focus_event: WindowFocusEvent| {
            println!("🎯 Focus event: HWND {} - Type: {}", focus_event.hwnd, focus_event.event_type);
            let hwnd = focus_event.hwnd;
            let focused = focus_event.event_type == 0;
            let song_map = song_map.lock().unwrap();
            let song_index = 0;
            // if let Some(&song_index) = song_map.get(&hwnd) {
                let mut midi_player = midi_player.lock().unwrap();
                if focused {
                    let _ = midi_player.play_song(song_index, None, None);
                    println!("▶️ Play song {} for HWND {}", song_index, hwnd);
                } else {
                    midi_player.stop_playback();
                    println!("⏹️ Stop song {} for HWND {}", song_index, hwnd);
                }
            // } else {
            //     println!("❗ No song assigned for HWND {}", hwnd);
            // }
        }
    }).unwrap();

    client.start_background_monitoring().unwrap();
    println!("\n🎬 Starting focus → music demo...\n");
    loop {
        thread::sleep(Duration::from_secs(1));
    }
}