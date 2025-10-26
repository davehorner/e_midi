// This example demonstrates how to use e_midi with e_grid to play MIDI songs when windows
// are focused or unfocused, and to play a song when a window is moved or resized.
// It also demonstrates how to assign a song to each window and clean up when the window is destroyed.
// This example is for Windows only.
// It requires the e_grid and e_midi crates to be added to your Cargo.toml file.

#[allow(unused_imports)]
use dashmap::DashMap;
#[cfg(target_os = "windows")]
#[allow(dead_code)]
use e_grid::ipc_protocol::WindowFocusEvent;
#[cfg(target_os = "windows")]
#[allow(dead_code)]
use e_grid::ipc_server::start_server;
#[cfg(target_os = "windows")]
use e_grid::GridClient;
#[allow(unused_imports)]
use e_midi::MidiPlayer;
#[allow(unused_imports)]
use std::sync::atomic::{AtomicUsize, Ordering};
#[allow(unused_imports)]
use std::sync::Arc;
#[cfg(target_os = "windows")]
use winapi::shared::windef::POINT;
#[cfg(target_os = "windows")]
use winapi::um::winuser::GetParent;
#[cfg(target_os = "windows")]
use winapi::um::winuser::{GetClassNameW, GetWindowTextW};
#[cfg(target_os = "windows")]
use winapi::um::winuser::{GetCursorPos, GetForegroundWindow, WindowFromPoint};

#[cfg(target_os = "windows")]
fn get_window_class_and_title(hwnd: u64) -> (String, String) {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;
    let hwnd = hwnd as isize as winapi::shared::windef::HWND;
    let mut class_buf = [0u16; 256];
    let mut title_buf = [0u16; 256];
    let class_len = unsafe { GetClassNameW(hwnd, class_buf.as_mut_ptr(), class_buf.len() as i32) };
    let title_len = unsafe { GetWindowTextW(hwnd, title_buf.as_mut_ptr(), title_buf.len() as i32) };
    let class = if class_len > 0 {
        OsString::from_wide(&class_buf[..class_len as usize])
            .to_string_lossy()
            .into_owned()
    } else {
        String::from("")
    };
    let title = if title_len > 0 {
        OsString::from_wide(&title_buf[..title_len as usize])
            .to_string_lossy()
            .into_owned()
    } else {
        String::from("")
    };
    (class, title)
}

#[cfg(target_os = "windows")]
#[allow(dead_code)]
fn is_hwnd_or_ancestor(
    target: winapi::shared::windef::HWND,
    mut hwnd: winapi::shared::windef::HWND,
) -> bool {
    while !hwnd.is_null() {
        if hwnd == target {
            return true;
        }
        hwnd = unsafe { GetParent(hwnd) };
    }
    false
}

#[cfg(target_os = "windows")]
#[allow(dead_code)]
fn is_hwnd_foreground_and_mouse_over(hwnd: u64) -> bool {
    use winapi::shared::windef::HWND;
    let hwnd = hwnd as isize as HWND;
    unsafe {
        let fg = GetForegroundWindow();
        if fg != hwnd {
            println!(
                "[debug] HWND {} is not foreground (fg={:?})",
                hwnd as usize, fg
            );
            return false;
        }
        let mut pt = POINT { x: 0, y: 0 };
        if GetCursorPos(&mut pt) == 0 {
            println!("[debug] GetCursorPos failed");
            return false;
        }
        let mouse_hwnd = WindowFromPoint(pt);
        if !is_hwnd_or_ancestor(hwnd, mouse_hwnd) {
            println!(
                "[debug] Mouse is not over HWND {} or its children (mouse_hwnd={:?}, pt=({}, {}))",
                hwnd as usize, mouse_hwnd, pt.x, pt.y
            );
            return false;
        }
        println!(
            "[debug] HWND {} is foreground and mouse is over (pt=({}, {}))",
            hwnd as usize, pt.x, pt.y
        );
        true
    }
}

#[cfg(target_os = "windows")]
fn is_hwnd_foreground(hwnd: u64) -> bool {
    use winapi::shared::windef::HWND;
    let hwnd = hwnd as isize as HWND;
    unsafe {
        let fg = GetForegroundWindow();
        if fg != hwnd {
            return false;
        }
        true
    }
}
#[cfg(target_os = "windows")]
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

    let midi_player = MidiPlayer::new().unwrap();
    let total_songs = midi_player.get_total_song_count();
    println!("🎵 e_midi: {} songs available", total_songs);

    let song_map = Arc::new(DashMap::<u64, usize>::new());
    let next_song = Arc::new(AtomicUsize::new(0));
    // Get the MIDI command sender once, outside the callback, and wrap in Arc
    let midi_player = Arc::new(midi_player);
    // Get the MIDI command sender from the player (mpsc::Sender)
    let midi_sender = Arc::new(midi_player.get_command_sender());
    // Set up move/resize START callback
    let _song_map_for_start = Arc::clone(&song_map);
    let midi_sender_start = Arc::clone(&midi_sender);
    client
        .set_move_resize_start_callback(move |e| {
            let song_index = 0;
            let _ = midi_sender_start.send(e_midi::MidiCommand::Stop);
            // Add a short delay to allow the MIDI thread/device to process Stop
            std::thread::sleep(std::time::Duration::from_millis(50));
            let _ = midi_sender_start.send(e_midi::MidiCommand::PlaySongResumeAware {
                song_index: Some(song_index),
                position_ms: None,
                tracks: None,
                tempo_bpm: None,
            });
            println!(
                "▶️ [MOVE/RESIZE START] Queued play song {} for HWND {:?}",
                song_index, e.hwnd
            );
        })
        .unwrap();

    // Set up move/resize STOP callback
    let midi_sender_stop = Arc::clone(&midi_sender);
    client
        .set_move_resize_stop_callback(move |e| {
            let _ = midi_sender_stop.send(e_midi::MidiCommand::Stop);
            println!(
                "⏹️ [MOVE/RESIZE STOP] Queued stop playback for HWND {:?}",
                e.hwnd
            );
        })
        .unwrap();

    // Set up focus callback (lock-free)
    let song_map_for_focus = Arc::clone(&song_map);
    let next_song_for_focus = Arc::clone(&next_song);
    let midi_sender_focus = Arc::clone(&midi_sender);
    client.set_focus_callback(move |focus_event: WindowFocusEvent| {
        let (class, title) = get_window_class_and_title(focus_event.hwnd);
        if !is_hwnd_foreground(focus_event.hwnd) {
            println!("[skip] Focus event for HWND {} - Type: {} [class='{}', title='{}'] (not foreground)", focus_event.hwnd, focus_event.event_type, class, title);
            return;
        }
        let hwnd = focus_event.hwnd;
        let focused = focus_event.event_type == 0;
        let song_index = if let Some(idx) = song_map_for_focus.get(&hwnd) {
            println!("🎵 Using assigned song {} for HWND {} [class='{}', title='{}']", *idx, hwnd, class, title);
            *idx
        } else {
            println!("❗ No song assigned for HWND {} [class='{}', title='{}']", hwnd, class, title);
            let song_index = next_song_for_focus.fetch_add(1, Ordering::SeqCst) % total_songs;
            song_map_for_focus.insert(hwnd, song_index);
            song_index
        };
        if focused {
            let _ = midi_sender_focus.send(e_midi::MidiCommand::Stop);
            let _ = midi_sender_focus.send(e_midi::MidiCommand::PlaySongResumeAware {
                song_index: Some(song_index),
                position_ms: None,
                tracks: None,
                tempo_bpm: None,
            });
            println!("▶️ [FOCUS] Queued play song {} for HWND {:?}", song_index, hwnd);
        } else {
            let _ = midi_sender_focus.send(e_midi::MidiCommand::Stop);
            println!("⏹️ [FOCUS] Queued stop playback for HWND {:?}", hwnd);
        }
    }).unwrap();

    client.start_background_monitoring().unwrap();
    loop {
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}

#[cfg(not(target_os = "windows"))]
fn main() {
    println!("demo_focus is only supported on Windows.");
}
