// This example demonstrates how to use e_midi with e_grid to play MIDI songs when windows
// are focused or unfocused, and to play a song when a window is moved or resized.
// It also demonstrates how to assign a song to each window and clean up when the window is destroyed.
// This example is for Windows only.
// It requires the e_grid and e_midi crates to be added to your Cargo.toml file.

#[cfg(target_os = "windows")]
use e_grid::ipc_protocol::WindowFocusEvent;
#[cfg(target_os = "windows")]
use e_grid::ipc_server::start_server;
#[cfg(target_os = "windows")]
use e_grid::GridClient;
#[allow(unused_imports)]
use e_midi::MidiPlayer;
#[allow(unused_imports)]
use std::collections::HashMap;
#[allow(unused_imports)]
use std::sync::{Arc, Mutex};
#[allow(unused_imports)]
use std::thread;
#[cfg(target_os = "windows")]
use winapi::shared::windef::POINT;
#[cfg(target_os = "windows")]
use winapi::um::winuser::GetParent;
#[cfg(target_os = "windows")]
use winapi::um::winuser::{GetClassNameW, GetWindowTextW};
#[cfg(target_os = "windows")]
use winapi::um::winuser::{GetCursorPos, GetForegroundWindow, WindowFromPoint};

#[derive(Debug)]
#[allow(dead_code)]
enum MidiCommand {
    Start(u64, usize), // HWND, song_index
    Stop(u64),
    Resume(u64, usize), // HWND, song_index
}

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

    let song_map = Arc::new(Mutex::new(HashMap::<u64, usize>::new()));
    let next_song = Arc::new(Mutex::new(0usize));
    // Initialize the MIDI player as before
    let midi_player = Arc::new(Mutex::new(midi_player));
    let tick_tracker_map: Arc<Mutex<HashMap<u64, Arc<Mutex<Option<u32>>>>>> =
        Arc::new(Mutex::new(HashMap::<u64, Arc<Mutex<Option<u32>>>>::new()));

    // --- Remove all SegQueue/MidiCommand/thread code ---

    // Set up move/resize START callback
    let midi_player_start = Arc::clone(&midi_player);
    let song_map_for_start = Arc::clone(&song_map);
    client
        .set_move_resize_start_callback(move |e| {
            let song_index = {
                let map = song_map_for_start.lock().unwrap();
                *map.get(&e.hwnd).unwrap_or(&0)
            };
            // Instead of blocking in the callback, spawn a thread for playback
            let midi_player_clone = Arc::clone(&midi_player_start);
            thread::spawn(move || {
                let mut midi_player = midi_player_clone.lock().unwrap();
                midi_player.stop_playback();
                let _ = midi_player.play_song_resume_aware(Some(song_index), None, None, None);
                println!(
                    "▶️ [MOVE/RESIZE START] Play song {} for HWND {:?}",
                    song_index, e.hwnd
                );
            });
        })
        .unwrap();

    // Set up move/resize STOP callback
    let midi_player_stop = Arc::clone(&midi_player);
    client
        .set_move_resize_stop_callback(move |e| {
            let mut midi_player = midi_player_stop.lock().unwrap();
            midi_player.stop_playback();
            println!("⏹️ [MOVE/RESIZE STOP] Stop playback for HWND {:?}", e.hwnd);
        })
        .unwrap();

    // Set up focus callback (if you want to use it)
    let midi_player_focus = Arc::clone(&midi_player);
    let song_map_for_focus = Arc::clone(&song_map);
    let next_song_for_focus = Arc::clone(&next_song);
    client.set_focus_callback({
        let midi_player_focus = Arc::clone(&midi_player_focus);
        let song_map_for_focus = Arc::clone(&song_map_for_focus);
        let next_song_for_focus = Arc::clone(&next_song_for_focus);
        move |focus_event: WindowFocusEvent| {
            let (class, title) = get_window_class_and_title(focus_event.hwnd);
            if !is_hwnd_foreground(focus_event.hwnd) {
                println!("[skip] Focus event for HWND {} - Type: {} [class='{}', title='{}'] (not foreground)", focus_event.hwnd, focus_event.event_type, class, title);
                return;
            }
            let hwnd = focus_event.hwnd;
            let focused = focus_event.event_type == 0;
            let mut song_map = song_map_for_focus.lock().unwrap();
            let mut song_index = 0;
            if song_map.get(&hwnd).is_none() {
                println!("❗ No song assigned for HWND {} [class='{}', title='{}']", hwnd, class, title);
                let mut idx = next_song_for_focus.lock().unwrap();
                song_index = *idx % total_songs;
                *idx += 1;
                song_map.insert(hwnd, song_index);
            } else {
                song_index = *song_map.get(&hwnd).unwrap();
                println!("🎵 Using assigned song {} for HWND {} [class='{}', title='{}']", song_index, hwnd, class, title);
            }
            let midi_player_clone = Arc::clone(&midi_player_focus);
            thread::spawn(move || {
                let mut midi_player = midi_player_clone.lock().unwrap();
                if focused {
                    midi_player.stop_playback();
                    let _ = midi_player.play_song_resume_aware(Some(song_index), None, None, None);
                    println!("▶️ [FOCUS] Play song {} for HWND {:?}", song_index, hwnd);
                } else {
                    midi_player.stop_playback();
                    println!("⏹️ [FOCUS] Stop playback for HWND {:?}", hwnd);
                }
            });
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
