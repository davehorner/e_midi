//! # Interactive MIDI Player
//! 
//! A comprehensive MIDI player with advanced features including:
//! - Multiple playback modes (single, all, random, scan)
//! - Looping capabilities (playlist and individual songs)
//! - Interactive configuration and control
//! - Real-time progress reporting
//! - Configurable timing and delays
//! 
//! ## Usage
//! 
//! Run the program and follow the interactive prompts to configure
//! playback settings and select songs.

use midir::MidiOutput;
use std::{
    error::Error,
    io::{Write, stdin, stdout},
    thread::{self, sleep},
    time::Instant,
    sync::{Arc, Mutex},
    sync::atomic::{AtomicU32, Ordering},
};

// Load generated track info and playback functions
include!(concat!(env!("OUT_DIR"), "/midi_data.rs"));

#[derive(Clone, Debug)]
/// Represents a MIDI note event with timing and properties
pub struct Note {
    /// Start time in milliseconds from the beginning of the song
    pub start_ms: u32,
    /// Duration of the note in milliseconds
    pub dur_ms: u32,
    /// MIDI channel (0-15)
    pub chan: u8,
    /// MIDI note pitch (0-127)
    pub pitch: u8,
    /// Note velocity/volume (0-127)
    pub vel: u8,
}

#[derive(Clone, Debug)]
/// Configuration for looping and playback behavior
pub struct LoopConfig {
    /// Whether to loop the entire playlist continuously
    pub loop_playlist: bool,
    /// Whether to loop individual songs
    pub loop_individual_songs: bool,
    /// Duration of each scan segment in milliseconds
    pub scan_segment_duration_ms: u32,
    /// Whether to start scan segments at random positions
    pub scan_random_start: bool,
    /// Delay between songs in milliseconds
    pub delay_between_songs_ms: u32,
}

impl Default for LoopConfig {
    fn default() -> Self {
        LoopConfig {
            loop_playlist: false,
            loop_individual_songs: false,
            scan_segment_duration_ms: 30000, // 30 seconds
            scan_random_start: false,
            delay_between_songs_ms: 2000, // 2 seconds
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let midi_out = MidiOutput::new("MOCS")?;
    let ports = midi_out.ports();
    let port = ports.get(0).ok_or("missing MIDI output port")?;
    let mut conn = midi_out.connect(port, "mocs")?;
    let songs = get_songs();
    
    let mut config = LoopConfig::default();
    
    loop {
        show_main_menu(&mut config, &songs, &mut conn)?;
    }
}

fn show_main_menu(
    config: &mut LoopConfig,
    songs: &[SongInfo],
    conn: &mut midir::MidiOutputConnection,
) -> Result<(), Box<dyn Error>> {
    println!("\n🎵 e_midi - Interactive MIDI Player");
    println!("══════════════════════════════════");
    
    // Settings display
    println!("\n⚙️  Current Settings:");
    println!("1: Loop playlist: {}", if config.loop_playlist { "✅ ON" } else { "❌ OFF" });
    println!("2: Loop individual songs: {}", if config.loop_individual_songs { "✅ ON" } else { "❌ OFF" });
    println!("3: Delay between songs: {}s", config.delay_between_songs_ms / 1000);
    println!("4: Scan segment duration: {}s", config.scan_segment_duration_ms / 1000);
    println!("5: Random scan start: {}", if config.scan_random_start { "✅ ON" } else { "❌ OFF" });
    
    // Playback options
    println!("\n🎵 Playback Options:");
    println!("6: Play a specific song");
    println!("7: Play all songs");
    println!("8: Play random song");
    println!("9: Scan mode (play portions of songs)");
    
    // Control options
    println!("\n🎮 Controls:");
    println!("q: Main menu (you are here)");
    println!("x: Exit program");
    
    if config.loop_playlist || config.loop_individual_songs {
        println!("\n💡 During playback: 'n' = next song, 'q' = quit to menu");
    }
    
    print!("\nSelect option (1-9, q, x): ");
    stdout().flush()?;
      let mut input = String::new();
    stdin().read_line(&mut input)?;
    let input = input.trim();
    
    // Skip empty input silently and continue to next iteration
    if input.is_empty() {
        return Ok(());
    }
    
    match input {
        "1" => {
            config.loop_playlist = !config.loop_playlist;
            println!("🔄 Playlist looping: {}", if config.loop_playlist { "ON" } else { "OFF" });
        },
        "2" => {
            config.loop_individual_songs = !config.loop_individual_songs;
            println!("🔄 Individual song looping: {}", if config.loop_individual_songs { "ON" } else { "OFF" });
        },        "3" => {
            print!("⏱️  Enter delay between songs in seconds (current: {}): ", config.delay_between_songs_ms / 1000);
            stdout().flush()?;
            let mut delay_input = String::new();
            stdin().read_line(&mut delay_input)?;
            if let Ok(delay_seconds) = delay_input.trim().parse::<u32>() {
                config.delay_between_songs_ms = delay_seconds * 1000;
                println!("⏱️  Delay set to {}s", delay_seconds);
            }
        },
        "4" => {
            print!("� Enter scan segment duration in seconds (current: {}): ", config.scan_segment_duration_ms / 1000);
            stdout().flush()?;
            let mut scan_input = String::new();
            stdin().read_line(&mut scan_input)?;
            if let Ok(scan_seconds) = scan_input.trim().parse::<u32>() {
                config.scan_segment_duration_ms = scan_seconds * 1000;
                println!("🔍 Scan duration set to {}s", scan_seconds);
            }
        },
        "5" => {
            config.scan_random_start = !config.scan_random_start;
            println!("🎲 Random scan start: {}", if config.scan_random_start { "ON" } else { "OFF" });
        },
        "6" => play_single_song(songs, conn, config)?,
        "7" => play_all_songs(songs, conn, config)?,
        "8" => play_random_song(songs, conn, config)?,
        "9" => scan_mode(songs, conn, config)?,
        "q" => {
            println!("📍 Already at main menu");
        },
        "x" => {
            println!("👋 Goodbye!");
            std::process::exit(0);
        },
        _ => {
            println!("❌ Invalid option. Please select 1-9, q, or x.");
        }    }
    
    Ok(())
}

fn play_single_song(songs: &[SongInfo], conn: &mut midir::MidiOutputConnection, loop_config: &LoopConfig) -> Result<(), Box<dyn Error>> {
    // Song selection
    println!("\n🎵 Available Songs:");
    for (i, song) in songs.iter().enumerate() {
        println!("{}: {} ({} tracks, default tempo: {} BPM)", 
            i, song.name, song.tracks.len(), song.default_tempo);
    }    print!("\nSelect song number: ");
    stdout().flush()?;
    let mut input = String::new();
    stdin().read_line(&mut input)?;
    
    // Check for quit command
    if input.trim() == "q" {
        return Ok(());
    }
    
    let song_index: usize = input.trim().parse().unwrap_or(0);

    if song_index >= songs.len() {
        println!("Invalid song selection.");
        return Ok(());
    }

    let selected_song = &songs[song_index];
    println!("\n🎹 Selected: {}", selected_song.name);
    
    if loop_config.loop_individual_songs {
        println!("🔄 Looping enabled for this song. Press 'q' + Enter to stop.");
        loop {
            let continue_playing = play_song_with_options(selected_song, song_index, conn)?;            if !continue_playing {
                break;
            }}
        Ok(())
    } else {
        play_song_with_options(selected_song, song_index, conn).map(|_| ())
    }
}

fn play_all_songs(songs: &[SongInfo], conn: &mut midir::MidiOutputConnection, loop_config: &LoopConfig) -> Result<(), Box<dyn Error>> {
    println!("\n🎵 Playing all {} songs...", songs.len());
    
    if loop_config.loop_playlist {
        println!("🔄 Playlist looping enabled. Press 'q' + Enter to stop.");
    }
    
    loop {
        for (i, song) in songs.iter().enumerate() {
            println!("\n▶️  Now playing: {} ({}/{})", song.name, i + 1, songs.len());
            
            // Play all tracks at default tempo
            let all_tracks: Vec<usize> = song.tracks.iter().map(|t| t.index).collect();
            let events = get_events_for_song_tracks(i, &all_tracks, song.default_tempo);
            
            if events.len() > 0 {
                let continue_playing = play_events_with_tempo_control(&events, conn, song.default_tempo)?;
                if !continue_playing {
                    println!("🛑 Playback stopped by user.");
                    return Ok(());
                }
            }
              if i < songs.len() - 1 && loop_config.delay_between_songs_ms > 0 {
                println!("🎵 Next song in {} seconds... (Press 'q' + Enter to stop)", loop_config.delay_between_songs_ms / 1000);
                sleep(std::time::Duration::from_millis(loop_config.delay_between_songs_ms as u64));
            }
        }
        
        if !loop_config.loop_playlist {
            break;
        }
        
        println!("🔄 Restarting playlist...");
        sleep(std::time::Duration::from_secs(1));
    }
    
    println!("✅ All songs completed!");
    Ok(())
}

fn play_random_song(songs: &[SongInfo], conn: &mut midir::MidiOutputConnection, loop_config: &LoopConfig) -> Result<(), Box<dyn Error>> {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    use std::time::{SystemTime, UNIX_EPOCH};
    
    if loop_config.loop_individual_songs {
        println!("🔄 Random song looping enabled. Press 'q' + Enter to stop.");
    }
    
    loop {
        // Simple random number generation
        let mut hasher = DefaultHasher::new();
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos().hash(&mut hasher);
        let random_index = (hasher.finish() as usize) % songs.len();
        
        let selected_song = &songs[random_index];
        println!("\n🎲 Randomly selected: {}", selected_song.name);
        
        let continue_playing = play_song_with_options(selected_song, random_index, conn)?;
        if !continue_playing || !loop_config.loop_individual_songs {
            break;
        }
        
        println!("🔄 Selecting another random song...");
        sleep(std::time::Duration::from_secs(1));
    }
    
    Ok(())
}

fn scan_mode(songs: &[SongInfo], conn: &mut midir::MidiOutputConnection, loop_config: &LoopConfig) -> Result<(), Box<dyn Error>> {
    print!("\n⏱️  Enter scan duration in seconds (default 30): ");
    stdout().flush()?;
    let mut input = String::new();
    stdin().read_line(&mut input)?;
    
    // Check for quit command
    if input.trim() == "q" {
        return Ok(());
    }
    
    let scan_duration: u32 = input.trim().parse().unwrap_or(30);
    
    println!("🔀 Scan mode options:");
    println!("1: Sequential (play from start of each song)");
    println!("2: Random positions in each song");
    println!("3: Progressive scan (advance through each song on each loop)");
    
    print!("Select scan mode (1-3): ");
    stdout().flush()?;
    input.clear();
    stdin().read_line(&mut input)?;
    
    // Check for quit command
    if input.trim() == "q" {
        return Ok(());
    }
    let scan_mode: u32 = input.trim().parse().unwrap_or(1);
    
    if loop_config.loop_playlist && scan_mode == 3 {
        println!("🔄 Progressive scan with looping enabled. Each loop will scan further into each song.");
    } else if loop_config.loop_playlist {
        println!("🔄 Scan looping enabled. Press 'q' + Enter to stop.");
    }
    
    println!("\n🎵 Scanning {} songs ({} seconds each)...", songs.len(), scan_duration);
    println!("🎮 Controls: 't' = change tempo, 'n' = next song, 'q' = quit to menu\n");
    
    let mut scan_iteration = 0;
    
    loop {
        for (i, song) in songs.iter().enumerate() {
            println!("\n▶️  Scanning: {} ({}/{})", song.name, i + 1, songs.len());
            
            // Play all tracks at default tempo
            let all_tracks: Vec<usize> = song.tracks.iter().map(|t| t.index).collect();
            let mut events = get_events_for_song_tracks(i, &all_tracks, song.default_tempo);            if events.len() > 0 {
                // Calculate full song duration first
                let full_duration_ms = calculate_song_duration_ms(&events);
                let full_duration_str = format_duration(full_duration_ms);
                
                let skip_ratio = match scan_mode {
                    2 => {
                        // Random position: skip some events at the beginning
                        use std::collections::hash_map::DefaultHasher;
                        use std::hash::{Hash, Hasher};
                        use std::time::{SystemTime, UNIX_EPOCH};
                        
                        let mut hasher = DefaultHasher::new();
                        (SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos() + i as u128).hash(&mut hasher);
                        let ratio = (hasher.finish() % 70) as f32 / 100.0; // Skip 0-70% of the song
                        let start_time_ms = (full_duration_ms as f32 * ratio) as u32;
                        println!("🎲 Random start: {:.0}% ({}) of {} total", ratio * 100.0, format_duration(start_time_ms), full_duration_str);
                        ratio
                    }
                    3 => {
                        // Progressive scan: advance further into the song with each iteration
                        let ratio = (scan_iteration as f32 * 0.15) % 0.8; // Advance by 15% each time, wrap at 80%
                        let start_time_ms = (full_duration_ms as f32 * ratio) as u32;
                        let end_time_ms = start_time_ms + (scan_duration * 1000);
                        let actual_end = std::cmp::min(end_time_ms, full_duration_ms);
                        println!("🎯 Progressive scan: {:.0}% ({} to {}) of {} total", 
                            ratio * 100.0, 
                            format_duration(start_time_ms), 
                            format_duration(actual_end),
                            full_duration_str
                        );
                        ratio
                    }
                    _ => {
                        // Sequential: always start from beginning
                        let end_time_ms = std::cmp::min(scan_duration * 1000, full_duration_ms);
                        println!("📏 Sequential scan: 0% (0s to {}) of {} total", 
                            format_duration(end_time_ms), 
                            full_duration_str
                        );
                        0.0
                    }
                };
                
                if skip_ratio > 0.0 && skip_ratio < 1.0 {
                    let skip_events = (events.len() as f32 * skip_ratio) as usize;
                    if skip_events < events.len() {
                        let skip_time = events[skip_events].start_ms;
                        // Adjust all event times to start from 0
                        for event in &mut events[skip_events..] {
                            event.start_ms = event.start_ms.saturating_sub(skip_time);
                        }
                        events = events[skip_events..].to_vec();
                    }
                }
                
                play_events_with_tempo_control_and_scan_limit(&events, conn, song.default_tempo, scan_duration * 1000)?;
            }
              if i < songs.len() - 1 && loop_config.delay_between_songs_ms > 0 {
                println!("🎵 Next song in {} seconds...", loop_config.delay_between_songs_ms / 1000);
                sleep(std::time::Duration::from_millis(loop_config.delay_between_songs_ms as u64));
            }
        }
        
        if !loop_config.loop_playlist {
            break;
        }
        
        scan_iteration += 1;
        println!("🔄 Restarting scan (iteration {})...", scan_iteration + 1);
        sleep(std::time::Duration::from_secs(2));
    }
    
    println!("✅ Scan completed!");
    Ok(())
}

fn play_song_with_options(selected_song: &SongInfo, song_index: usize, conn: &mut midir::MidiOutputConnection) -> Result<bool, Box<dyn Error>> {
    let mut input = String::new();

    // Track selection
    println!("\n🎹 Available Tracks:");
    for track in &selected_song.tracks {
        println!(
            "{}: {} - notes: {} - channels: {:?} - pitch: {}–{} - sample: {:?}",
            track.index,
            track.guess.as_ref().unwrap_or(&"-".to_string()),
            track.note_count,
            track.channels,
            track.pitch_range.0,
            track.pitch_range.1,
            track.sample_notes
        );
    }    print!("\nEnter track numbers to play (comma separated, 0 for all tracks, or ENTER for all): ");
    stdout().flush()?;
    input.clear();
    stdin().read_line(&mut input)?;
    
    // Check for quit command
    if input.trim() == "q" {
        return Ok(false);
    }
    
    let mut tracks: Vec<usize> = if input.trim().is_empty() {
        // Default to all tracks if user just hits Enter
        selected_song.tracks.iter().map(|t| t.index).collect()
    } else {
        input
            .trim()
            .split(',')
            .filter_map(|s| s.trim().parse::<usize>().ok())
            .collect()
    };

    // If track 0 is specified, play all tracks
    if tracks.contains(&0) {
        tracks = selected_song.tracks.iter().map(|t| t.index).collect();
        println!("🎵 Playing all tracks!");
    } else if tracks.is_empty() {
        // Fallback to all tracks if parsing failed
        tracks = selected_song.tracks.iter().map(|t| t.index).collect();
        println!("🎵 No valid tracks specified, playing all tracks!");
    } else {
        // Validate that user-entered track numbers exist in the available tracks
        let mut valid_tracks = Vec::new();
        for &user_track in &tracks {
            if selected_song.tracks.iter().any(|t| t.index == user_track) {
                valid_tracks.push(user_track);
            } else {
                println!("⚠️  Track {} not found, skipping.", user_track);
            }
        }
        tracks = valid_tracks;
        
        if tracks.is_empty() {
            println!("🎵 No valid tracks found, playing all tracks!");
            tracks = selected_song.tracks.iter().map(|t| t.index).collect();
        }
    }    // Tempo selection - default to file's default tempo if user just hits Enter
    print!("\nEnter tempo in BPM (default {} or ENTER for default): ", selected_song.default_tempo);
    stdout().flush()?;
    input.clear();
    stdin().read_line(&mut input)?;
    
    // Check for quit command
    if input.trim() == "q" {
        return Ok(false);
    }
    
    let tempo_bpm = if input.trim().is_empty() {
        selected_song.default_tempo
    } else {
        input.trim().parse().unwrap_or(selected_song.default_tempo)
    };
    
    println!("\n▶️  Playing {} - tracks: {:?} at {} BPM", selected_song.name, tracks, tempo_bpm);
    println!("🎮 Controls: 't' = change tempo, 'n' = next song, 'q' = quit to menu\n");
    
    let events = get_events_for_song_tracks(song_index, &tracks, tempo_bpm);
      if events.len() == 0 {
        println!("⚠️  No events to play! Check track selection.");
        return Ok(false);    }
    let continue_playing = play_events_with_tempo_control(&events, conn, tempo_bpm)?;
    println!("✅ Done!");
    Ok(continue_playing)
}

fn play_events_with_tempo_control(
    events: &[Note],
    conn: &mut midir::MidiOutputConnection,
    initial_tempo_bpm: u32,
) -> Result<bool, Box<dyn Error>> {
    use std::thread;
    
    #[derive(Copy, Clone)]
    enum Kind {
        On,
        Off,
    }

    struct Scheduled {
        t: u32,
        kind: Kind,
        chan: u8,
        p: u8,
        v: u8,
    }

    let mut timeline = Vec::with_capacity(events.len() * 2);
    for n in events {
        timeline.push(Scheduled {
            t: n.start_ms,
            kind: Kind::On,
            chan: n.chan,
            p: n.pitch,
            v: n.vel,
        });
        timeline.push(Scheduled {
            t: n.start_ms + n.dur_ms,
            kind: Kind::Off,
            chan: n.chan,
            p: n.pitch,
            v: 0,
        });
    }
    timeline.sort_by_key(|e| e.t);    let tempo_multiplier = Arc::new(AtomicU32::new((initial_tempo_bpm as f32 * 1000.0) as u32)); // Store as BPM * 1000 for precision
    let should_quit = Arc::new(Mutex::new(false));
    let should_next = Arc::new(Mutex::new(false)); // New: flag for next song
    
    // Spawn input handling thread
    let tempo_clone = Arc::clone(&tempo_multiplier);
    let quit_clone = Arc::clone(&should_quit);
    let next_clone = Arc::clone(&should_next);
    
    let input_thread = thread::spawn(move || {
        let stdin = std::io::stdin();
        loop {
            let mut input = String::new();
            if stdin.read_line(&mut input).is_ok() {
                let input = input.trim();
                if input == "q" {
                    *quit_clone.lock().unwrap() = true;
                    break;
                } else if input == "n" {
                    *next_clone.lock().unwrap() = true;
                    break;
                } else if input == "t" {
                    print!("Enter new tempo (BPM): ");
                    stdout().flush().unwrap();
                    let mut tempo_input = String::new();
                    if stdin.read_line(&mut tempo_input).is_ok() {
                        if let Ok(new_tempo) = tempo_input.trim().parse::<u32>() {
                            tempo_clone.store((new_tempo as f32 * 1000.0) as u32, Ordering::Relaxed);
                            println!("⏱️  Tempo changed to {} BPM", new_tempo);
                        }
                    }
                } else if let Ok(new_tempo) = input.parse::<u32>() {
                    tempo_clone.store((new_tempo as f32 * 1000.0) as u32, Ordering::Relaxed);
                    println!("⏱️  Tempo changed to {} BPM", new_tempo);
                }
                // Continue the loop - don't break unless quit or next
            }
        }
    });

    let start = Instant::now();
    let mut idx = 0;
    let mut last_tempo = initial_tempo_bpm as f32 * 1000.0;
    let mut time_offset = 0.0;
    let mut last_real_time = 0.0;    while idx < timeline.len() {
        // Check if we should quit or go to next song
        if *should_quit.lock().unwrap() {
            break;
        }
        if *should_next.lock().unwrap() {
            println!("⏭️  Skipping to next song...");
            // Send all notes off before moving to next
            for channel in 0..16 {
                conn.send(&[0xB0 | channel, 123, 0])?; // All notes off
            }
            return Ok(true); // Continue to next song
        }

        let current_tempo = tempo_multiplier.load(Ordering::Relaxed) as f32 / 1000.0;let real_elapsed = start.elapsed().as_millis() as f32;
        
        // If tempo changed, adjust our time calculations
        if (current_tempo - last_tempo).abs() > 0.1 {
            let tempo_ratio = current_tempo / last_tempo; // Fixed: higher BPM = faster tempo
            time_offset += (real_elapsed - last_real_time) * (1.0 - tempo_ratio);
            last_tempo = current_tempo;
        }
        
        let tempo_ratio = current_tempo / (initial_tempo_bpm as f32); // Fixed: current/initial, not initial/current
        let adjusted_time = ((real_elapsed - time_offset) * tempo_ratio) as u32;
        last_real_time = real_elapsed;

        while idx < timeline.len() && timeline[idx].t <= adjusted_time {
            let e = &timeline[idx];
            let msg = match e.kind {
                Kind::On => [0x90 | (e.chan & 0x0F), e.p, e.v],
                Kind::Off => [0x80 | (e.chan & 0x0F), e.p, 0],
            };
            conn.send(&msg)?;
            idx += 1;
        }
        sleep(std::time::Duration::from_millis(1));
    }    // Send all notes off
    for channel in 0..16 {
        conn.send(&[0xB0 | channel, 123, 0])?; // All notes off
    }

    // Wait for input thread to finish
    let _ = input_thread.join();

    // Return true if user didn't quit (wants to continue looping), false if they quit
    let user_quit = *should_quit.lock().unwrap();
    Ok(!user_quit)
}

fn play_events_with_tempo_control_and_scan_limit(
    events: &[Note],
    conn: &mut midir::MidiOutputConnection,
    initial_tempo_bpm: u32,
    max_duration_ms: u32,
) -> Result<bool, Box<dyn Error>> {
    if events.is_empty() {
        return Ok(true);
    }

    #[derive(Clone, Copy)]
    enum Kind { On, Off }
    
    #[derive(Clone)]
    struct Event { t: u32, kind: Kind, chan: u8, p: u8, v: u8 }

    let mut timeline = Vec::new();
    for note in events {
        // Skip events that start after our max duration
        if note.start_ms > max_duration_ms {
            continue;
        }
        
        timeline.push(Event {
            t: note.start_ms,
            kind: Kind::On,
            chan: note.chan,
            p: note.pitch,
            v: note.vel,
        });
        
        let end_time = note.start_ms + note.dur_ms;
        timeline.push(Event {
            t: if end_time <= max_duration_ms { end_time } else { max_duration_ms },
            kind: Kind::Off,
            chan: note.chan,
            p: note.pitch,
            v: note.vel,
        });
    }
    timeline.sort_by_key(|e| e.t);

    let tempo_multiplier = Arc::new(AtomicU32::new((initial_tempo_bpm as f32 * 1000.0) as u32));
    let should_quit = Arc::new(Mutex::new(false));
    let should_next = Arc::new(Mutex::new(false));
      // Spawn input handling thread
    let tempo_clone = Arc::clone(&tempo_multiplier);
    let quit_clone = Arc::clone(&should_quit);
    let next_clone = Arc::clone(&should_next);
    
    let input_thread = thread::spawn(move || {
        let stdin = std::io::stdin();
        loop {
            let mut input = String::new();
            if stdin.read_line(&mut input).is_ok() {
                let input = input.trim();
                if input == "q" {
                    *quit_clone.lock().unwrap() = true;
                    break;
                } else if input == "n" {
                    *next_clone.lock().unwrap() = true;
                    break;
                } else if input == "t" {
                    print!("Enter new tempo (BPM): ");
                    stdout().flush().unwrap();
                    let mut tempo_input = String::new();
                    if stdin.read_line(&mut tempo_input).is_ok() {
                        if let Ok(new_tempo) = tempo_input.trim().parse::<u32>() {
                            tempo_clone.store((new_tempo as f32 * 1000.0) as u32, Ordering::Relaxed);
                            println!("⏱️  Tempo changed to {} BPM", new_tempo);
                        }
                    }
                } else if let Ok(new_tempo) = input.parse::<u32>() {
                    tempo_clone.store((new_tempo as f32 * 1000.0) as u32, Ordering::Relaxed);
                    println!("⏱️  Tempo changed to {} BPM", new_tempo);
                }
            }
        }
    });

    let start = Instant::now();
    let mut idx = 0;
    let mut last_tempo = initial_tempo_bpm as f32 * 1000.0;
    let mut time_offset = 0.0;
    let mut last_real_time = 0.0;

    while idx < timeline.len() {
        // Check for quit or next
        if *should_quit.lock().unwrap() {
            break;
        }
        if *should_next.lock().unwrap() {
            println!("⏭️  Skipping to next song...");
            // Send all notes off before moving to next
            for channel in 0..16 {
                conn.send(&[0xB0 | channel, 123, 0])?;
            }
            return Ok(true);
        }
        
        let current_tempo = tempo_multiplier.load(Ordering::Relaxed) as f32 / 1000.0;
        let real_elapsed = start.elapsed().as_millis() as f32;
        
        // Stop if we've exceeded our max duration (accounting for tempo changes)
        let tempo_ratio = current_tempo / (initial_tempo_bpm as f32);
        let adjusted_real_time = real_elapsed * tempo_ratio;
        if adjusted_real_time >= max_duration_ms as f32 {
            break;
        }
        
        // If tempo changed, adjust our time calculations
        if (current_tempo - last_tempo).abs() > 0.1 {
            let tempo_ratio = current_tempo / last_tempo;
            time_offset += (real_elapsed - last_real_time) * (1.0 - tempo_ratio);
            last_tempo = current_tempo;
        }
        
        let adjusted_time = ((real_elapsed - time_offset) * tempo_ratio) as u32;
        last_real_time = real_elapsed;

        while idx < timeline.len() && timeline[idx].t <= adjusted_time {
            let e = &timeline[idx];
            let msg = match e.kind {
                Kind::On => [0x90 | (e.chan & 0x0F), e.p, e.v],
                Kind::Off => [0x80 | (e.chan & 0x0F), e.p, 0],
            };
            conn.send(&msg)?;
            idx += 1;
        }
        sleep(std::time::Duration::from_millis(1));
    }    // Send all notes off
    for channel in 0..16 {
        conn.send(&[0xB0 | channel, 123, 0])?;
    }

    // Wait for input thread to finish
    let _ = input_thread.join();

    // Return false if user quit, true if song finished naturally or next was pressed
    let quit_flag = *should_quit.lock().unwrap();
    Ok(!quit_flag)
}

fn calculate_song_duration_ms(events: &[Note]) -> u32 {
    if events.is_empty() {
        return 0;
    }
    
    // Find the latest end time of any note
    events.iter()
        .map(|note| note.start_ms + note.dur_ms)
        .max()
        .unwrap_or(0)
}

fn format_duration(ms: u32) -> String {
    let seconds = ms / 1000;
    let minutes = seconds / 60;
    let remaining_seconds = seconds % 60;
    
    if minutes > 0 {
        format!("{}m{:02}s", minutes, remaining_seconds)
    } else {
        format!("{}s", remaining_seconds)
    }
}
