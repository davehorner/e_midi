
// This example demonstrates how to use e_midi with e_grid to play MIDI songs when windows
// are focused or unfocused, and to play a song when a window is moved or resized.
// It also demonstrates how to assign a song to each window and clean up when the window is destroyed.
// This example is for Windows only.
// It requires the e_grid and e_midi crates to be added to your Cargo.toml file.

#[cfg(target_os = "windows")]
use e_grid::ipc_protocol::{WindowEvent, WindowFocusEvent};
#[cfg(target_os = "windows")]
use e_grid::ipc_server::start_server;
#[cfg(target_os = "windows")]
use e_grid::GridClient;
use e_midi::MidiPlayer;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
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
    let midi_player = Arc::new(Mutex::new(midi_player));

    let midi_player_for_start = Arc::clone(&midi_player);
    client
        .set_move_resize_start_callback(move |e| {
            println!("DUDE [debug] Move/resize start callback triggered {:?}", e);
            // You can handle move/resize start events here if needed
            let mut midi_player = midi_player_for_start.lock().unwrap();
            // if !midi_player.is_playing() {
            let _ = midi_player.play_song_with_ipc_nonblocking(0); //play_song(0, None, None);
            std::thread::sleep(Duration::from_millis(100)); // Give some time to stop playback
            println!("▶️ Play non-blocking song 0 (move/resize start)");
            // }
        })
        .unwrap();

    let midi_player_for_stop = Arc::clone(&midi_player);
    client
        .set_move_resize_stop_callback(move |e| {
            let mut midi_player = midi_player_for_stop.lock().unwrap();
            if midi_player.is_playing() {
                midi_player.stop_playback();
                std::thread::sleep(Duration::from_millis(100)); // Give some time to stop playback
                println!("DUDE [debug] Move/resize end callback triggered");
            }
            // You can handle move/resize end events here if needed
        })
        .unwrap();
    // Assign a song index to each window as it is created, and clean up on destroy
    client.set_window_event_callback({
        println!("Setting window event callback");
        let song_map = Arc::clone(&song_map);
        let next_song = Arc::clone(&next_song);
        let midi_player = Arc::clone(&midi_player);
        move |event: WindowEvent| {
            println!("[debug] Window event received: {:?}", event);
            let (class, title) = get_window_class_and_title(event.hwnd);
            match event.event_type {
                0 => { // CREATED
                    let hwnd = event.hwnd;
                    let mut idx = next_song.lock().unwrap();
                    let song_index = *idx % total_songs;
                    *idx += 1;
                    song_map.lock().unwrap().insert(hwnd, song_index);
                    println!("🎵 Assigned song {} to HWND {} [class='{}', title='{}']", song_index, hwnd, class, title);
                }

                // 2 => { // FOCUS
                //     if !is_hwnd_foreground_and_mouse_over(event.hwnd) {
                //         println!("[skip] FOCUS event for HWND {} [class='{}', title='{}'] (not foreground or mouse not over)", event.hwnd, class, title);
                //         return;
                //     }
                //     let hwnd = event.hwnd;
                //     let song_map = song_map.lock().unwrap();
                //     if let Some(&song_index) = song_map.get(&hwnd) {
                //         let mut midi_player = midi_player.lock().unwrap();
                //         let _ = midi_player.play_song(song_index, None, None);
                //         println!("▶️ Play song {} for HWND {} (focus) [class='{}', title='{}']", song_index, hwnd, class, title);
                //     }
                // }
                // 3 => { // DEFOCUS
                //     if !is_hwnd_foreground_and_mouse_over(event.hwnd) {
                //         println!("[skip] DEFOCUS event for HWND {} [class='{}', title='{}'] (not foreground or mouse not over)", event.hwnd, class, title);
                //         return;
                //     }
                //     let hwnd = event.hwnd;
                //     let song_map = song_map.lock().unwrap();
                //     if let Some(&song_index) = song_map.get(&hwnd) {
                //         let mut midi_player = midi_player.lock().unwrap();
                //         midi_player.stop_playback();
                //         println!("⏹️ Stop song {} for HWND {} (defocus) [class='{}', title='{}']", song_index, hwnd, class, title);
                //     }
                // }
                4 => {
                    println!("\n\n\n[debug] Window event type START! received for HWND {} [class='{}', title='{}']", event.hwnd, class, title);
                },
                5 => {
                    println!("\n\n\n[debug] Window event type STOP! received for HWND {} [class='{}', title='{}']", event.hwnd, class, title);
                },
                1 => { // DESTROYED
                    let hwnd = event.hwnd;
                    if let Some(song_index) = song_map.lock().unwrap().remove(&hwnd) {
                        let mut midi_player = midi_player.lock().unwrap();
                        midi_player.stop_playback();
                        println!("⏹️ Stop song {} for HWND {} (window destroyed) [class='{}', title='{}']", song_index, hwnd, class, title);
                    }
                }
                _ => {
                    println!("[debug] Window event type {} received for HWND {} [class='{}', title='{}']", event.event_type, event.hwnd, class, title);
                }
            }
        }
    }).unwrap();

    // Play/stop song on focus/unfocus
    client.set_focus_callback({
        let song_map = Arc::clone(&song_map);
        let midi_player = Arc::clone(&midi_player);
        move |focus_event: WindowFocusEvent| {
            let (class, title) = get_window_class_and_title(focus_event.hwnd);
            // if !is_hwnd_foreground_and_mouse_over(focus_event.hwnd) {
            //     println!("[skip] Focus event for HWND {} - Type: {} [class='{}', title='{}'] (not foreground or mouse not over)", focus_event.hwnd, focus_event.event_type, class, title);
            //     return;
            // }
            if !is_hwnd_foreground(focus_event.hwnd) {
                println!("[skip] Focus event for HWND {} - Type: {} [class='{}', title='{}'] (not foreground)", focus_event.hwnd, focus_event.event_type, class, title);
                return;
            }
            let hwnd = focus_event.hwnd;
            let focused = focus_event.event_type == 0;
            let mut song_map = song_map.lock().unwrap();

            let mut song_index = 0;
            if song_map.get(&hwnd).is_none() {
                println!("❗ No song assigned for HWND {} [class='{}', title='{}']", hwnd, class, title);
                let mut idx = next_song.lock().unwrap();
                song_index = *idx % total_songs;
                *idx += 1;
                song_map.insert(hwnd, song_index);
            } else {
                song_index = *song_map.get(&hwnd).unwrap();
                println!("🎵 Using assigned song {} for HWND {} [class='{}', title='{}']", song_index, hwnd, class, title);
            }
            let mut midi_player = midi_player.lock().unwrap();
            if focused {
                    midi_player.stop_playback();
                    std::thread::sleep(Duration::from_millis(100)); // Give some time to stop playback
                    let _ = midi_player.play_song_with_ipc_nonblocking(song_index); //play_song(song_index, None, None);
                    println!("▶️ Play song {} for HWND {} [class='{}', title='{}']", song_index, hwnd, class, title);
            } else {
                    midi_player.stop_playback();
                    std::thread::sleep(Duration::from_millis(100)); // Give some time to stop playback
                    println!("⏹️ Stop song {} for HWND {} [class='{}', title='{}']", song_index, hwnd, class, title);
            }
        }
    }).unwrap();

    client.start_background_monitoring().unwrap();
    loop {
        thread::sleep(Duration::from_secs(1));
    }
}

#[cfg(not(target_os = "windows"))]
fn main() {
    println!("demo_focus is only supported on Windows.");
}
