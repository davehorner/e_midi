use std::error::Error;
use std::io::{stdout, stdin, Write};
use std::sync::{Arc, Mutex, atomic::{AtomicU32, Ordering}};
use std::thread::{self, sleep};
use std::time::{Duration, Instant};

use midir::{MidiOutput, MidiOutputConnection};

// Include the generated MIDI data
include!(concat!(env!("OUT_DIR"), "/midi_data.rs"));

pub mod cli;

#[derive(Clone, Debug)]
pub struct Note {
    pub start_ms: u32,
    pub dur_ms: u32,
    pub chan: u8,
    pub pitch: u8,
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
            delay_between_songs_ms: 0, // No delay between songs by default
        }
    }
}

pub struct MidiPlayer {
    songs: Vec<SongInfo>,
    conn: MidiOutputConnection,
    config: LoopConfig,
}

impl MidiPlayer {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        let midi_out = MidiOutput::new("e_midi")?;
        let ports = midi_out.ports();
        
        // Debug: List available MIDI ports
        println!("🎹 Available MIDI ports:");
        if ports.is_empty() {
            println!("❌ No MIDI output ports found!");
            println!("💡 To hear sound, you need:");
            println!("   - A software synthesizer (like Windows Media Player, VirtualMIDISynth)");
            println!("   - Or a hardware MIDI device");
            println!("   - Or enable Windows built-in MIDI synthesizer");
        } else {
            for (i, port) in ports.iter().enumerate() {
                match midi_out.port_name(port) {
                    Ok(name) => println!("  {}: {}", i, name),
                    Err(_) => println!("  {}: <Unknown>", i),
                }
            }
        }
        
        let port = ports.get(0).ok_or("missing MIDI output port")?;
        let port_name = midi_out.port_name(port).unwrap_or_else(|_| "Unknown".to_string());
        let conn = midi_out.connect(port, "e_midi")?;
        println!("🔌 Connected to MIDI port: {}", port_name);
        
        let songs = get_songs();
        
        Ok(MidiPlayer {
            songs,
            conn,
            config: LoopConfig::default(),
        })
    }

    pub fn get_songs(&self) -> &[SongInfo] {
        &self.songs
    }

    pub fn get_config(&self) -> &LoopConfig {
        &self.config
    }

    pub fn get_config_mut(&mut self) -> &mut LoopConfig {
        &mut self.config
    }

    pub fn list_songs(&self) {
        println!("🎵 Available Songs:");
        for (i, song) in self.songs.iter().enumerate() {
            println!("{}: {} ({} tracks, default tempo: {} BPM)", 
                i, song.name, song.tracks.len(), song.default_tempo);
        }
    }

    pub fn play_song(&mut self, song_index: usize, tracks: Option<Vec<usize>>, tempo_bpm: Option<u32>) -> Result<bool, Box<dyn Error>> {
        if song_index >= self.songs.len() {
            return Err("Invalid song index".into());
        }

        let selected_song = &self.songs[song_index];
        let tempo = tempo_bpm.unwrap_or(selected_song.default_tempo);
        
        let track_indices = if let Some(tracks) = tracks {
            if tracks.contains(&0) {
                // 0 means all tracks
                selected_song.tracks.iter().map(|t| t.index).collect()
            } else {
                tracks
            }
        } else {
            // Default to all tracks
            selected_song.tracks.iter().map(|t| t.index).collect()
        };

        println!("\n▶️  Playing {} - tracks: {:?} at {} BPM", selected_song.name, track_indices, tempo);
        println!("🎮 Controls: 't' = change tempo, 'n' = next song, 'q' = quit to menu\n");

        let events = get_events_for_song_tracks(song_index, &track_indices, tempo);
        if events.is_empty() {
            println!("⚠️  No events to play! Check track selection.");
            return Ok(false);
        }

        let continue_playing = self.play_events_with_tempo_control(&events, tempo)?;
        println!("✅ Done!");
        Ok(continue_playing)
    }    pub fn play_all_songs(&mut self) -> Result<(), Box<dyn Error>> {
        let songs_count = self.songs.len();
        loop {
            for i in 0..songs_count {
                let song = &self.songs[i];
                println!("\n🔀 Playing song {} of {}: {}", i + 1, songs_count, song.name);
                
                let events = get_events_for_song_tracks(i, &song.tracks.iter().map(|t| t.index).collect::<Vec<_>>(), song.default_tempo);
                if !events.is_empty() {
                    let continue_playing = self.play_events_with_tempo_control(&events, song.default_tempo)?;
                    if !continue_playing {
                        return Ok(());
                    }
                }
                
                if self.config.delay_between_songs_ms > 0 {
                    println!("⏸️  Waiting {}ms before next song...", self.config.delay_between_songs_ms);
                    sleep(Duration::from_millis(self.config.delay_between_songs_ms as u64));
                }
            }
            
            if !self.config.loop_playlist {
                break;
            }
            println!("🔄 Restarting playlist...");
        }
        Ok(())
    }

    pub fn play_random_song(&mut self) -> Result<(), Box<dyn Error>> {
        use std::collections::HashSet;
        let mut played_songs = HashSet::new();
        
        loop {
            if played_songs.len() >= self.songs.len() {
                if !self.config.loop_playlist {
                    break;
                }
                played_songs.clear();
                println!("🔄 All songs played, restarting random playlist...");
            }
            
            let mut song_index;
            loop {
                song_index = (std::ptr::addr_of!(self.songs) as usize) % self.songs.len();
                if !played_songs.contains(&song_index) {
                    break;
                }
            }
            played_songs.insert(song_index);
            
            let song = &self.songs[song_index];
            println!("\n🎲 Random song {}: {}", song_index, song.name);
            
            let events = get_events_for_song_tracks(song_index, &song.tracks.iter().map(|t| t.index).collect::<Vec<_>>(), song.default_tempo);
            if !events.is_empty() {
                let continue_playing = self.play_events_with_tempo_control(&events, song.default_tempo)?;
                if !continue_playing {
                    break;
                }
            }
            
            if self.config.delay_between_songs_ms > 0 {
                sleep(Duration::from_millis(self.config.delay_between_songs_ms as u64));
            }
        }
        Ok(())
    }    pub fn scan_mode(&mut self, scan_duration: u32, scan_mode: u32) -> Result<(), Box<dyn Error>> {
        self.scan_mode_internal(scan_duration, scan_mode, true)
    }

    pub fn scan_mode_non_interactive(&mut self, scan_duration: u32, scan_mode: u32) -> Result<(), Box<dyn Error>> {
        self.scan_mode_internal(scan_duration, scan_mode, false)
    }    fn scan_mode_internal(&mut self, scan_duration: u32, scan_mode: u32, interactive: bool) -> Result<(), Box<dyn Error>> {
        let songs_count = self.songs.len();        
        println!("\n🎵 Scanning {} songs ({} seconds each)...", songs_count, scan_duration);
        if interactive {
            println!("🎮 Controls: 't' = change tempo, 'n' = next song, 'q' = quit to menu\n");
        }
        
        // Progressive scan mode automatically enables playlist looping
        let original_loop_setting = self.config.loop_playlist;
        if scan_mode == 3 {
            self.config.loop_playlist = true;
        }
        
        let mut positions = if scan_mode == 3 {
            // Progressive scan - start with positions for each song
            vec![0u32; songs_count]
        } else {
            Vec::new()
        };

        loop {
            for song_index in 0..songs_count {
                let song = &self.songs[song_index];
                let song_duration = calculate_song_duration_ms(&get_events_for_song_tracks(song_index, &song.tracks.iter().map(|t| t.index).collect::<Vec<_>>(), song.default_tempo));
                
                let start_position = match scan_mode {
                    1 => 0, // Sequential - always start from beginning
                    2 => {  // Random positions
                        if self.config.scan_random_start && song_duration > scan_duration * 1000 {
                            use std::collections::hash_map::DefaultHasher;
                            use std::hash::{Hash, Hasher};
                            let mut hasher = DefaultHasher::new();
                            song_index.hash(&mut hasher);
                            (hasher.finish() as u32) % (song_duration - scan_duration * 1000)
                        } else {
                            0
                        }
                    },                    3 => {  // Progressive scan
                        let pos = positions[song_index];
                        if pos + scan_duration * 1000 >= song_duration {
                            positions[song_index] = 0; // Reset to start if we've reached the end
                            0
                        } else {
                            positions[song_index] += scan_duration * 1000; // Advance by full scan duration
                            pos
                        }
                    },
                    _ => 0,
                };                println!("\n▶️  Scanning: {} ({}/{})", song.name, song_index + 1, songs_count);
                
                let events = get_events_for_song_tracks(song_index, &song.tracks.iter().map(|t| t.index).collect::<Vec<_>>(), song.default_tempo);
                if !events.is_empty() {
                    // Calculate full song duration first
                    let full_duration_ms = calculate_song_duration_ms(&events);
                    let full_duration_str = format_duration(full_duration_ms);
                    
                    let end_time_ms = std::cmp::min(scan_duration * 1000, full_duration_ms);
                    let percentage = if full_duration_ms > 0 { 
                        (start_position as f32 / full_duration_ms as f32 * 100.0) as u32 
                    } else { 
                        0 
                    };
                    
                    match scan_mode {
                        2 => { // Random scan
                            println!("🎲 Random start: {}% ({}) of {} total", 
                                percentage, 
                                format_duration(start_position), 
                                full_duration_str
                            );
                        },
                        3 => { // Progressive scan
                            let end_pos = std::cmp::min(start_position + scan_duration * 1000, full_duration_ms);
                            println!("🎯 Progressive scan: {}% ({} to {}) of {} total", 
                                percentage,
                                format_duration(start_position), 
                                format_duration(end_pos),
                                full_duration_str
                            );
                        },
                        _ => { // Sequential scan
                            println!("📏 Sequential scan: 0% (0s to {}) of {} total", 
                                format_duration(end_time_ms), 
                                full_duration_str
                            );
                        }                    }
                    
                    // Filter events to start from the calculated position
                    let filtered_events: Vec<Note> = if start_position > 0 {
                        events.iter()
                            .filter(|note| note.start_ms >= start_position)
                            .map(|note| Note {
                                start_ms: note.start_ms - start_position, // Offset to start from 0
                                dur_ms: note.dur_ms,
                                chan: note.chan,
                                pitch: note.pitch,
                                vel: note.vel,
                            })
                            .collect()
                    } else {
                        events
                    };
                    
                    if interactive {
                        self.play_events_with_tempo_control_and_scan_limit(&filtered_events, song.default_tempo, scan_duration * 1000)?;
                    } else {
                        self.play_events_with_tempo_control_and_scan_limit_non_interactive(&filtered_events, song.default_tempo, scan_duration * 1000)?;
                    }
                }
                
                if self.config.delay_between_songs_ms > 0 {
                    sleep(Duration::from_millis(self.config.delay_between_songs_ms as u64));
                }
            }
              if !self.config.loop_playlist {
                break;
            }
            println!("🔄 Restarting scan...");
        }
        
        // Restore original loop setting
        self.config.loop_playlist = original_loop_setting;
        
        Ok(())
    }

    pub fn run_interactive(&mut self) -> Result<(), Box<dyn Error>> {
        loop {
            self.show_main_menu()?;
        }
    }

    fn show_main_menu(&mut self) -> Result<(), Box<dyn Error>> {
        println!("\n🎵 e_midi - Interactive MIDI Player");
        println!("══════════════════════════════════");
        
        // Settings display
        println!("\n⚙️  Current Settings:");
        println!("1: Loop playlist: {}", if self.config.loop_playlist { "✅ ON" } else { "❌ OFF" });
        println!("2: Loop individual songs: {}", if self.config.loop_individual_songs { "✅ ON" } else { "❌ OFF" });
        println!("3: Delay between songs: {}s", self.config.delay_between_songs_ms / 1000);
        println!("4: Scan segment duration: {}s", self.config.scan_segment_duration_ms / 1000);
        println!("5: Random scan start: {}", if self.config.scan_random_start { "✅ ON" } else { "❌ OFF" });
        
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
        
        if self.config.loop_playlist || self.config.loop_individual_songs {
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
                self.config.loop_playlist = !self.config.loop_playlist;
                println!("🔄 Playlist looping: {}", if self.config.loop_playlist { "ON" } else { "OFF" });
            },
            "2" => {
                self.config.loop_individual_songs = !self.config.loop_individual_songs;
                println!("🔄 Individual song looping: {}", if self.config.loop_individual_songs { "ON" } else { "OFF" });
            },
            "3" => {
                print!("⏱️  Enter delay between songs in seconds (current: {}): ", self.config.delay_between_songs_ms / 1000);
                stdout().flush()?;
                let mut delay_input = String::new();
                stdin().read_line(&mut delay_input)?;
                if let Ok(delay_seconds) = delay_input.trim().parse::<u32>() {
                    self.config.delay_between_songs_ms = delay_seconds * 1000;
                    println!("⏱️  Delay set to {}s", delay_seconds);
                }
            },
            "4" => {
                print!("🔍 Enter scan segment duration in seconds (current: {}): ", self.config.scan_segment_duration_ms / 1000);
                stdout().flush()?;
                let mut scan_input = String::new();
                stdin().read_line(&mut scan_input)?;
                if let Ok(scan_seconds) = scan_input.trim().parse::<u32>() {
                    self.config.scan_segment_duration_ms = scan_seconds * 1000;
                    println!("🔍 Scan duration set to {}s", scan_seconds);
                }
            },
            "5" => {
                self.config.scan_random_start = !self.config.scan_random_start;
                println!("🎲 Random scan start: {}", if self.config.scan_random_start { "ON" } else { "OFF" });
            },
            "6" => self.play_single_song_interactive()?,
            "7" => self.play_all_songs()?,
            "8" => self.play_random_song()?,
            "9" => self.scan_mode_interactive()?,
            "q" => {
                println!("📍 Already at main menu");
            },
            "x" => {
                println!("👋 Goodbye!");
                std::process::exit(0);
            },
            _ => {
                println!("❌ Invalid option. Please select 1-9, q, or x.");
            }
        }
        
        Ok(())
    }

    fn play_single_song_interactive(&mut self) -> Result<(), Box<dyn Error>> {
        self.list_songs();

        print!("\nSelect song number: ");
        stdout().flush()?;
        let mut input = String::new();
        stdin().read_line(&mut input)?;
        
        // Check for quit command
        if input.trim() == "q" {
            return Ok(());
        }
        
        let song_index: usize = input.trim().parse().unwrap_or(0);

        if song_index >= self.songs.len() {
            println!("Invalid song selection.");
            return Ok(());
        }

        let selected_song = &self.songs[song_index];
        println!("\n🎹 Selected: {}", selected_song.name);
        
        if self.config.loop_individual_songs {
            println!("🔄 Looping enabled for this song. Press 'q' + Enter to stop.");
        }

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
        }
        
        print!("\nEnter track numbers to play (comma separated, 0 for all tracks, or ENTER for all): ");
        stdout().flush()?;
        input.clear();
        stdin().read_line(&mut input)?;
        
        // Check for quit command
        if input.trim() == "q" {
            return Ok(());
        }
        
        let mut tracks: Vec<usize> = if input.trim().is_empty() {
            selected_song.tracks.iter().map(|t| t.index).collect()
        } else {
            input
                .trim()
                .split(',')
                .filter_map(|s| s.trim().parse::<usize>().ok())
                .collect()
        };

        if tracks.contains(&0) {
            tracks = selected_song.tracks.iter().map(|t| t.index).collect();
            println!("🎵 Playing all tracks!");
        } else if tracks.is_empty() {
            println!("🎵 No valid tracks specified, playing all tracks!");
            tracks = selected_song.tracks.iter().map(|t| t.index).collect();
        } else {
            let mut valid_tracks = Vec::new();
            for user_track in &tracks {
                if selected_song.tracks.iter().any(|t| t.index == *user_track) {
                    valid_tracks.push(*user_track);
                } else {
                    println!("⚠️  Track {} not found, skipping.", user_track);
                }
            }
            tracks = valid_tracks;
            
            if tracks.is_empty() {
                println!("🎵 No valid tracks found, playing all tracks!");
                tracks = selected_song.tracks.iter().map(|t| t.index).collect();
            }
        }

        // Tempo selection
        print!("\nEnter tempo in BPM (default {} or ENTER for default): ", selected_song.default_tempo);
        stdout().flush()?;
        input.clear();
        stdin().read_line(&mut input)?;
        
        // Check for quit command
        if input.trim() == "q" {
            return Ok(());
        }
        
        let tempo_bpm = if input.trim().is_empty() {
            selected_song.default_tempo
        } else {
            input.trim().parse().unwrap_or(selected_song.default_tempo)
        };
        
        self.play_song(song_index, Some(tracks), Some(tempo_bpm))?;
        Ok(())
    }

    fn scan_mode_interactive(&mut self) -> Result<(), Box<dyn Error>> {
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
        
        self.scan_mode(scan_duration, scan_mode)
    }    // Private playback methods...
    fn play_events_with_tempo_control(
        &mut self,
        events: &[Note],
        initial_tempo_bpm: u32,
    ) -> Result<bool, Box<dyn Error>> {
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
        timeline.sort_by_key(|e| e.t);

        let tempo_multiplier = Arc::new(AtomicU32::new((initial_tempo_bpm as f32 * 1000.0) as u32));
        let should_quit = Arc::new(Mutex::new(false));
        let should_next = Arc::new(Mutex::new(false));
        
        // Spawn input handling thread
        let tempo_clone = Arc::clone(&tempo_multiplier);
        let quit_clone = Arc::clone(&should_quit);
        let next_clone = Arc::clone(&should_next);
        
        let input_thread = thread::spawn(move || {
            let stdin = stdin();
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
        let mut note_count = 0;

        println!("🎵 Starting playback with {} events...", timeline.len());

        while idx < timeline.len() {
            // Check if we should quit or go to next song
            if *should_quit.lock().unwrap() {
                println!("🛑 Playback stopped by user");
                break;
            }
            if *should_next.lock().unwrap() {
                println!("⏭️  Skipping to next song...");
                // Send all notes off before moving to next
                for channel in 0..16 {
                    self.conn.send(&[0xB0 | channel, 123, 0])?;
                }
                return Ok(true);
            }

            let current_tempo = tempo_multiplier.load(Ordering::Relaxed) as f32 / 1000.0;
            let real_elapsed = start.elapsed().as_millis() as f32;
            
            // If tempo changed, adjust our time calculations
            if (current_tempo - last_tempo).abs() > 0.1 {
                let tempo_ratio = current_tempo / last_tempo;
                time_offset += (real_elapsed - last_real_time) * (1.0 - tempo_ratio);
                last_tempo = current_tempo;
            }
            
            let tempo_ratio = current_tempo / (initial_tempo_bpm as f32);
            let adjusted_time = ((real_elapsed - time_offset) * tempo_ratio) as u32;
            last_real_time = real_elapsed;

            while idx < timeline.len() && timeline[idx].t <= adjusted_time {
                let e = &timeline[idx];
                let msg = match e.kind {
                    Kind::On => {
                        note_count += 1;
                        if note_count <= 10 { // Only log first 10 notes to avoid spam
                            println!("🎵 Note ON: chan={}, pitch={}, vel={} at {}ms", e.chan, e.p, e.v, adjusted_time);
                        }
                        [0x90 | (e.chan & 0x0F), e.p, e.v]
                    },
                    Kind::Off => [0x80 | (e.chan & 0x0F), e.p, 0],
                };
                self.conn.send(&msg)?;
                idx += 1;
            }
            sleep(Duration::from_millis(1));
        }

        println!("🎵 Playback finished. Sent {} note events.", note_count);

        // Send all notes off
        for channel in 0..16 {
            self.conn.send(&[0xB0 | channel, 123, 0])?;
        }

        // Wait for input thread to finish
        let _ = input_thread.join();

        // Return true if user didn't quit (wants to continue looping), false if they quit
        let user_quit = *should_quit.lock().unwrap();
        Ok(!user_quit)
    }    fn play_events_with_tempo_control_and_scan_limit(
        &mut self,
        events: &[Note],
        initial_tempo_bpm: u32,
        max_duration_ms: u32,
    ) -> Result<bool, Box<dyn Error>> {
        self.play_events_with_tempo_control_and_scan_limit_internal(events, initial_tempo_bpm, max_duration_ms, true)
    }    fn play_events_with_tempo_control_and_scan_limit_non_interactive(
        &mut self,
        events: &[Note],
        initial_tempo_bpm: u32,
        max_duration_ms: u32,
    ) -> Result<bool, Box<dyn Error>> {
        self.play_events_with_tempo_control_and_scan_limit_internal(events, initial_tempo_bpm, max_duration_ms, false)
    }    fn play_events_with_tempo_control_and_scan_limit_internal(
        &mut self,
        events: &[Note],
        initial_tempo_bpm: u32,
        max_duration_ms: u32,
        interactive: bool,
    ) -> Result<bool, Box<dyn Error>> {
        #[derive(Copy, Clone)]
        enum Kind {
            On,
            Off,
        }

        struct Event {
            t: u32,
            kind: Kind,
            chan: u8,
            p: u8,
            v: u8,
        }

        let mut timeline = Vec::with_capacity(events.len() * 2);
        for note in events {
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
        
        // Only spawn input handling thread if in interactive mode
        let input_thread = if interactive {
            let tempo_clone = Arc::clone(&tempo_multiplier);
            let quit_clone = Arc::clone(&should_quit);
            let next_clone = Arc::clone(&should_next);
            
            Some(thread::spawn(move || {
                let stdin = stdin();
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
            }))
        } else {
            None
        };        let start = Instant::now();        let mut idx = 0;
        let mut last_tempo = initial_tempo_bpm as f32 * 1000.0;
        let mut time_offset = 0.0;
        let mut last_real_time = 0.0;

        while idx < timeline.len() {
            let real_elapsed = start.elapsed().as_millis() as u32;
            
            if real_elapsed >= max_duration_ms {
                break;
            }

            if *should_quit.lock().unwrap() || *should_next.lock().unwrap() {
                println!("🛑 User requested quit/next");
                break;
            }

            let current_tempo = tempo_multiplier.load(Ordering::Relaxed) as f32 / 1000.0;
            let real_elapsed_f = real_elapsed as f32;
            
            if (current_tempo - last_tempo).abs() > 0.1 {
                let tempo_ratio = current_tempo / last_tempo;
                time_offset += (real_elapsed_f - last_real_time) * (1.0 - tempo_ratio);
                last_tempo = current_tempo;
            }
            
            let tempo_ratio = current_tempo / (initial_tempo_bpm as f32);
            let adjusted_time = ((real_elapsed_f - time_offset) * tempo_ratio) as u32;
            last_real_time = real_elapsed_f;

            while idx < timeline.len() && timeline[idx].t <= adjusted_time {
                let e = &timeline[idx];
                let msg = match e.kind {
                    Kind::On => [0x90 | (e.chan & 0x0F), e.p, e.v],
                    Kind::Off => [0x80 | (e.chan & 0x0F), e.p, 0],
                };
                self.conn.send(&msg)?;
                idx += 1;
            }            sleep(Duration::from_millis(1));
        }
        
        println!("🎼 Playbook loop finished, sending all notes off");
        
        // Send all notes off
        for channel in 0..16 {
            self.conn.send(&[0xB0 | channel, 123, 0])?;
        }
        
        println!("🧵 Waiting for input thread to finish");
        
        // Wait for input thread to finish if it was spawned
        if let Some(thread) = input_thread {
            let _ = thread.join();
        }

        println!("🏁 Playback function completed");
        
        // Return false if user quit, true if song finished naturally or next was pressed
        let quit_flag = *should_quit.lock().unwrap();
        Ok(!quit_flag)
    }
}

fn calculate_song_duration_ms(events: &[Note]) -> u32 {
    if events.is_empty() {
        return 0;
    }
    
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
