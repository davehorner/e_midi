use std::error::Error;
use std::fs;
use std::io::{stdin, stdout, Write};
use std::path::Path;
use std::sync::{
    atomic::{AtomicBool, AtomicU32, Ordering},
    mpsc, Arc, Mutex,
};
use std::thread::{self, sleep, JoinHandle};
use std::time::{Duration, Instant};

use midir::MidiOutput;
use midly::{MidiMessage, Smf, TrackEventKind};

// Import the IPC module (now fixed)
pub mod ipc;
pub use ipc::{AppId, Event as IpcEvent, IpcServiceManager};

// Global shutdown flag for graceful Ctrl+C handling
static SHUTDOWN: AtomicBool = AtomicBool::new(false);

pub fn set_shutdown_flag() {
    SHUTDOWN.store(true, Ordering::Relaxed);
}

pub fn should_shutdown() -> bool {
    SHUTDOWN.load(Ordering::Relaxed)
}

// MIDI command messages for the background thread
#[derive(Debug, Clone)]
pub enum MidiCommand {
    NoteOn {
        channel: u8,
        pitch: u8,
        velocity: u8,
    },
    NoteOff {
        channel: u8,
        pitch: u8,
    },
    SendMessage(Vec<u8>),
    AllNotesOff,
    Shutdown,
}

// Include the generated MIDI data
include!(concat!(env!("OUT_DIR"), "/midi_data.rs"));

pub mod cli;
mod tui;

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
    static_songs: Vec<SongInfo>,
    dynamic_songs: Vec<SongInfo>,
    dynamic_midi_data: Vec<Vec<u8>>, // Store raw MIDI data for dynamic songs
    config: LoopConfig,
    ipc_manager: Option<IpcServiceManager>, // Optional IPC for pub/sub communication

    // Channel-based MIDI architecture
    midi_sender: mpsc::Sender<MidiCommand>,
    _midi_thread: JoinHandle<()>, // Background MIDI thread handle

    // Playback control
    playback_stop_flag: Arc<AtomicBool>,
    is_playing: Arc<AtomicBool>, // Shared playback state
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
        let port_name = midi_out
            .port_name(port)
            .unwrap_or_else(|_| "Unknown".to_string());
        let conn = midi_out.connect(port, "e_midi")?;
        println!("🔌 Connected to MIDI port: {}", port_name);

        // Create the channel for sending MIDI commands to the background thread
        let (sender, receiver) = mpsc::channel::<MidiCommand>();

        // Spawn the background MIDI thread
        let midi_thread = thread::spawn(move || {
            Self::midi_thread_loop(conn, receiver);
        });
        let static_songs = get_songs();
        Ok(MidiPlayer {
            static_songs,
            dynamic_songs: Vec::new(),
            dynamic_midi_data: Vec::new(),
            config: LoopConfig::default(),
            ipc_manager: None, // Initialize as None, can be enabled later
            midi_sender: sender,
            _midi_thread: midi_thread,
            playback_stop_flag: Arc::new(AtomicBool::new(false)),
            is_playing: Arc::new(AtomicBool::new(false)),
        })
    }

    // Background MIDI thread that handles all MIDI output
    fn midi_thread_loop(
        mut conn: midir::MidiOutputConnection,
        receiver: mpsc::Receiver<MidiCommand>,
    ) {
        println!("🎹 MIDI background thread started");
        while let Ok(command) = receiver.recv() {
            match command {
                MidiCommand::NoteOn {
                    channel,
                    pitch,
                    velocity,
                } => {
                    let msg = [0x90 | (channel & 0x0F), pitch, velocity];
                    if let Err(e) = conn.send(&msg) {
                        eprintln!("❌ MIDI send error (note on): {}", e);
                    }
                }
                MidiCommand::NoteOff { channel, pitch } => {
                    let msg = [0x80 | (channel & 0x0F), pitch, 0];
                    if let Err(e) = conn.send(&msg) {
                        eprintln!("❌ MIDI send error (note off): {}", e);
                    }
                }
                MidiCommand::SendMessage(msg) => {
                    if let Err(e) = conn.send(&msg) {
                        eprintln!("❌ MIDI send error: {}", e);
                    }
                }
                MidiCommand::AllNotesOff => {
                    // Send all notes off for all channels
                    for channel in 0..16 {
                        let msg = [0xB0 | channel, 123, 0];
                        if let Err(e) = conn.send(&msg) {
                            eprintln!("❌ MIDI send error (all notes off): {}", e);
                        }
                    }
                }
                MidiCommand::Shutdown => {
                    println!("🎹 MIDI background thread shutting down");
                    // Send all notes off before shutdown
                    for channel in 0..16 {
                        let msg = [0xB0 | channel, 123, 0];
                        let _ = conn.send(&msg); // Ignore errors during shutdown
                    }
                    break;
                }
            }
        }

        println!("🎹 MIDI background thread finished");
    } // Send a MIDI command to the background thread
    fn send_midi_command(&self, command: MidiCommand) -> Result<(), Box<dyn Error>> {
        self.midi_sender
            .send(command)
            .map_err(|e| format!("Failed to send MIDI command: {}", e).into())
    }
    /// Stop any currently playing background playback
    pub fn stop_playback(&mut self) {
        self.playback_stop_flag.store(true, Ordering::Relaxed);
        self.is_playing.store(false, Ordering::Relaxed);

        // Send all notes off command through the MIDI channel
        if let Err(_) = self.send_midi_command(MidiCommand::AllNotesOff) {
            eprintln!("Failed to send all notes off command");
        }
    }

    /// Reset the stop flag (called before starting new playback)
    fn reset_stop_flag(&mut self) {
        self.playback_stop_flag.store(false, Ordering::Relaxed);
    }

    /// Check if currently playing
    pub fn is_playing(&self) -> bool {
        self.is_playing.load(Ordering::Relaxed)
    }

    /// Get a clone of the playing state atomic bool for sharing with TUI
    pub fn get_playing_state(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.is_playing)
    }

    /// Get count of static songs
    pub fn get_static_song_count(&self) -> usize {
        self.static_songs.len()
    }

    /// Get count of dynamic songs
    pub fn get_dynamic_song_count(&self) -> usize {
        self.dynamic_songs.len()
    }

    /// Get total count of all songs (static + dynamic)
    pub fn get_total_song_count(&self) -> usize {
        self.static_songs.len() + self.dynamic_songs.len()
    }

    /// Get a song by index (static songs first, then dynamic)
    pub fn get_song(&self, index: usize) -> Option<&SongInfo> {
        let static_count = self.static_songs.len();
        if index < static_count {
            self.static_songs.get(index)
        } else {
            self.dynamic_songs.get(index - static_count)
        }
    }

    /// Get all songs as a single slice (creates a new vector)
    pub fn get_all_songs(&self) -> Vec<&SongInfo> {
        let mut all_songs = Vec::new();
        all_songs.extend(self.static_songs.iter());
        all_songs.extend(self.dynamic_songs.iter());
        all_songs
    }

    pub fn get_songs(&self) -> Vec<&SongInfo> {
        self.get_all_songs()
    }

    pub fn get_config(&self) -> &LoopConfig {
        &self.config
    }

    pub fn get_config_mut(&mut self) -> &mut LoopConfig {
        &mut self.config
    }
    pub fn list_songs(&self) {
        println!("🎵 Available Songs:");
        let all_songs = self.get_all_songs();
        for (i, song) in all_songs.iter().enumerate() {
            println!(
                "{}: {} ({} tracks, default tempo: {} BPM)",
                i,
                song.name,
                song.tracks.len(),
                song.default_tempo
            );
        }
    }
    pub fn play_song(
        &mut self,
        song_index: usize,
        tracks: Option<Vec<usize>>,
        tempo_bpm: Option<u32>,
    ) -> Result<bool, Box<dyn Error>> {
        if song_index >= self.get_total_song_count() {
            return Err("Invalid song index".into());
        }

        let selected_song = self.get_song(song_index).ok_or("Invalid song index")?;
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
        println!(
            "\n▶️  Playing {} - tracks: {:?} at {} BPM",
            selected_song.name, track_indices, tempo
        );
        println!("🎮 Controls: 't' = change tempo (or type BPM directly), 'n' = next song, 'q' = quit to menu\n");

        let events = self.get_events_for_song(song_index, &track_indices, tempo);
        if events.is_empty() {
            println!("⚠️  No events to play! Check track selection.");
            return Ok(false);
        }

        let continue_playing = self.play_events_with_tempo_control(&events, tempo)?;
        println!("✅ Done!");
        Ok(continue_playing)
    }
    pub fn play_all_songs(&mut self) -> Result<(), Box<dyn Error>> {
        let songs_count = self.get_total_song_count();
        println!("\n🎮 Controls: 't' = change tempo (or type BPM directly), 'n' = next song, 'q' = quit to menu\n");
        loop {
            for i in 0..songs_count {
                let song = self.get_song(i).ok_or("Invalid song index")?;
                println!(
                    "\n🔀 Playing song {} of {}: {}",
                    i + 1,
                    songs_count,
                    song.name
                );

                let events = self.get_events_for_song(
                    i,
                    &song.tracks.iter().map(|t| t.index).collect::<Vec<_>>(),
                    song.default_tempo,
                );
                if !events.is_empty() {
                    let continue_playing =
                        self.play_events_with_tempo_control(&events, song.default_tempo)?;
                    if !continue_playing {
                        return Ok(());
                    }
                }

                if self.config.delay_between_songs_ms > 0 {
                    println!(
                        "⏸️  Waiting {}ms before next song...",
                        self.config.delay_between_songs_ms
                    );
                    sleep(Duration::from_millis(
                        self.config.delay_between_songs_ms as u64,
                    ));
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
            if played_songs.len() >= self.get_total_song_count() {
                if !self.config.loop_playlist {
                    break;
                }
                played_songs.clear();
                println!("🔄 All songs played, restarting random playlist...");
            }

            let mut song_index;
            loop {
                song_index =
                    (std::ptr::addr_of!(self.static_songs) as usize) % self.get_total_song_count();
                if !played_songs.contains(&song_index) {
                    break;
                }
            }
            played_songs.insert(song_index);

            let song = self.get_song(song_index).ok_or("Invalid song index")?;
            println!("\n🎲 Random song {}: {}", song_index, song.name);

            let events = self.get_events_for_song(
                song_index,
                &song.tracks.iter().map(|t| t.index).collect::<Vec<_>>(),
                song.default_tempo,
            );
            if !events.is_empty() {
                let continue_playing =
                    self.play_events_with_tempo_control(&events, song.default_tempo)?;
                if !continue_playing {
                    break;
                }
            }

            if self.config.delay_between_songs_ms > 0 {
                sleep(Duration::from_millis(
                    self.config.delay_between_songs_ms as u64,
                ));
            }
        }
        Ok(())
    }
    pub fn scan_mode(&mut self, scan_duration: u32, scan_mode: u32) -> Result<(), Box<dyn Error>> {
        self.scan_mode_internal(scan_duration, scan_mode, true)
    }

    pub fn scan_mode_non_interactive(
        &mut self,
        scan_duration: u32,
        scan_mode: u32,
    ) -> Result<(), Box<dyn Error>> {
        self.scan_mode_internal(scan_duration, scan_mode, false)
    }
    fn scan_mode_internal(
        &mut self,
        scan_duration: u32,
        scan_mode: u32,
        interactive: bool,
    ) -> Result<(), Box<dyn Error>> {
        let songs_count = self.get_total_song_count();
        println!(
            "\n🎵 Scanning {} songs ({} seconds each)...",
            songs_count, scan_duration
        );
        if interactive {
            println!("🎮 Controls: 't' = change tempo (or type BPM directly), 'n' = next song, 'q' = quit to menu\n");
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
            if should_shutdown() {
                println!("🛑 Shutdown requested, exiting scan mode");
                break;
            }
            for song_index in 0..songs_count {
                if should_shutdown() {
                    println!("🛑 Shutdown requested during scan");
                    return Ok(());
                }

                let song = self.get_song(song_index).ok_or("Invalid song index")?;
                let song_duration = calculate_song_duration_ms(&self.get_events_for_song(
                    song_index,
                    &song.tracks.iter().map(|t| t.index).collect::<Vec<_>>(),
                    song.default_tempo,
                ));

                let start_position = match scan_mode {
                    1 => 0, // Sequential - always start from beginning
                    2 => {
                        // Random positions
                        if self.config.scan_random_start && song_duration > scan_duration * 1000 {
                            use std::collections::hash_map::DefaultHasher;
                            use std::hash::{Hash, Hasher};
                            let mut hasher = DefaultHasher::new();
                            song_index.hash(&mut hasher);
                            (hasher.finish() as u32) % (song_duration - scan_duration * 1000)
                        } else {
                            0
                        }
                    }
                    3 => {
                        // Progressive scan
                        let pos = positions[song_index];
                        if pos + scan_duration * 1000 >= song_duration {
                            positions[song_index] = 0; // Reset to start if we've reached the end
                            0
                        } else {
                            positions[song_index] += scan_duration * 1000; // Advance by full scan duration
                            pos
                        }
                    }
                    _ => 0,
                };
                println!(
                    "\n▶️  Scanning: {} ({}/{})",
                    song.name,
                    song_index + 1,
                    songs_count
                );
                let events = self.get_events_for_song(
                    song_index,
                    &song.tracks.iter().map(|t| t.index).collect::<Vec<_>>(),
                    song.default_tempo,
                );
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
                        2 => {
                            // Random scan
                            println!(
                                "🎲 Random start: {}% ({}) of {} total",
                                percentage,
                                format_duration(start_position),
                                full_duration_str
                            );
                        }
                        3 => {
                            // Progressive scan
                            let end_pos = std::cmp::min(
                                start_position + scan_duration * 1000,
                                full_duration_ms,
                            );
                            println!(
                                "🎯 Progressive scan: {}% ({} to {}) of {} total",
                                percentage,
                                format_duration(start_position),
                                format_duration(end_pos),
                                full_duration_str
                            );
                        }
                        _ => {
                            // Sequential scan
                            println!(
                                "📏 Sequential scan: 0% (0s to {}) of {} total",
                                format_duration(end_time_ms),
                                full_duration_str
                            );
                        }
                    }

                    // Filter events to start from the calculated position
                    let filtered_events: Vec<Note> = if start_position > 0 {
                        events
                            .iter()
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
                        self.play_events_with_tempo_control_and_scan_limit(
                            &filtered_events,
                            song.default_tempo,
                            scan_duration * 1000,
                        )?;
                    } else {
                        // For non-interactive scan mode, just play the events with simple timing
                        self.play_events_simple(
                            &filtered_events,
                            song.default_tempo,
                            scan_duration * 1000,
                        )?;
                    }
                }

                if self.config.delay_between_songs_ms > 0 {
                    sleep(Duration::from_millis(
                        self.config.delay_between_songs_ms as u64,
                    ));
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
            if should_shutdown() {
                println!("🛑 Shutdown requested, exiting interactive mode");
                break;
            }
            self.show_main_menu()?;
        }
        Ok(())
    }
    pub fn run_tui_mode(&mut self) -> Result<(), Box<dyn Error>> {
        crate::tui::run_tui_mode(self)
    }
    fn show_main_menu(&mut self) -> Result<(), Box<dyn Error>> {
        println!("\n🎵 e_midi - Interactive MIDI Player");
        println!("══════════════════════════════════");

        // Song management
        let static_count = self.get_static_song_count();
        let dynamic_count = self.get_dynamic_song_count();
        let total_count = self.get_total_song_count();

        println!(
            "\n📚 Song Management ({} total: {} static + {} dynamic):",
            total_count, static_count, dynamic_count
        );
        println!("1: List all songs");
        println!("2: List static songs only");
        println!("3: List dynamic songs only");
        println!("4: Load MIDI file(s) or directory");
        println!("5: Clear dynamic songs");

        // Settings display
        println!("\n⚙️  Settings:");
        println!(
            "6: Loop playlist: {}",
            if self.config.loop_playlist {
                "✅ ON"
            } else {
                "❌ OFF"
            }
        );
        println!(
            "7: Loop individual songs: {}",
            if self.config.loop_individual_songs {
                "✅ ON"
            } else {
                "❌ OFF"
            }
        );
        println!(
            "8: Delay between songs: {}s",
            self.config.delay_between_songs_ms / 1000
        );
        println!(
            "9: Scan segment duration: {}s",
            self.config.scan_segment_duration_ms / 1000
        );
        println!(
            "10: Random scan start: {}",
            if self.config.scan_random_start {
                "✅ ON"
            } else {
                "❌ OFF"
            }
        );

        // Playback options
        println!("\n🎵 Playback Options:");
        println!("11: Play a specific song");
        println!("12: Play all songs");
        println!("13: Play random song");
        println!("14: Scan mode (play portions of songs)");

        // Control options
        println!("\n🎮 Controls:");
        println!("q: Main menu (you are here)");
        println!("x: Exit program");

        if self.config.loop_playlist || self.config.loop_individual_songs {
            println!("\n💡 During playback: 'n' = next song, 'q' = quit to menu");
        }

        print!("\nSelect option (1-14, q, x): ");
        stdout().flush()?;
        let mut input = String::new();
        let bytes_read = stdin().read_line(&mut input)?;

        // If no bytes were read, stdin is closed (EOF), so exit gracefully
        if bytes_read == 0 {
            println!("👋 Goodbye!");
            std::process::exit(0);
        }

        let input = input.trim();

        // Skip empty input silently and continue to next iteration
        if input.is_empty() {
            return Ok(());
        }

        match input {
            "1" => {
                self.list_songs();
            }
            "2" => {
                self.list_static_songs();
            }
            "3" => {
                self.list_dynamic_songs();
            }
            "4" => {
                self.load_midi_interactive()?;
            }
            "5" => {
                self.clear_dynamic_songs();
            }
            "6" => {
                self.config.loop_playlist = !self.config.loop_playlist;
                println!(
                    "🔄 Playlist looping: {}",
                    if self.config.loop_playlist {
                        "ON"
                    } else {
                        "OFF"
                    }
                );
            }
            "7" => {
                self.config.loop_individual_songs = !self.config.loop_individual_songs;
                println!(
                    "🔄 Individual song looping: {}",
                    if self.config.loop_individual_songs {
                        "ON"
                    } else {
                        "OFF"
                    }
                );
            }
            "8" => {
                print!(
                    "⏱️  Enter delay between songs in seconds (current: {}): ",
                    self.config.delay_between_songs_ms / 1000
                );
                stdout().flush()?;
                let mut delay_input = String::new();
                stdin().read_line(&mut delay_input)?;
                if let Ok(delay_seconds) = delay_input.trim().parse::<u32>() {
                    self.config.delay_between_songs_ms = delay_seconds * 1000;
                    println!("⏱️  Delay set to {}s", delay_seconds);
                }
            }
            "9" => {
                print!(
                    "🔍 Enter scan segment duration in seconds (current: {}): ",
                    self.config.scan_segment_duration_ms / 1000
                );
                stdout().flush()?;
                let mut scan_input = String::new();
                stdin().read_line(&mut scan_input)?;
                if let Ok(scan_seconds) = scan_input.trim().parse::<u32>() {
                    self.config.scan_segment_duration_ms = scan_seconds * 1000;
                    println!("🔍 Scan duration set to {}s", scan_seconds);
                }
            }
            "10" => {
                self.config.scan_random_start = !self.config.scan_random_start;
                println!(
                    "🎲 Random scan start: {}",
                    if self.config.scan_random_start {
                        "ON"
                    } else {
                        "OFF"
                    }
                );
            }
            "11" => self.play_single_song_interactive()?,
            "12" => self.play_all_songs()?,
            "13" => self.play_random_song()?,
            "14" => self.scan_mode_interactive()?,
            "q" => {
                println!("📍 Already at main menu");
            }
            "x" => {
                println!("👋 Goodbye!");
                std::process::exit(0);
            }
            _ => {
                println!("❌ Invalid option. Please select 1-14, q, or x.");
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

        if song_index >= self.get_total_song_count() {
            println!("Invalid song selection.");
            return Ok(());
        }

        let selected_song = self.get_song(song_index).ok_or("Invalid song index")?;
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

        print!(
            "\nEnter track numbers to play (comma separated, 0 for all tracks, or ENTER for all): "
        );
        stdout().flush()?;
        let mut track_input = String::new();
        stdin().read_line(&mut track_input)?;

        // Check for quit command
        if track_input.trim() == "q" {
            return Ok(());
        }

        let mut tracks: Vec<usize> = if track_input.trim().is_empty() {
            selected_song.tracks.iter().map(|t| t.index).collect()
        } else {
            track_input
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
        print!(
            "\nEnter tempo in BPM (default {} or ENTER for default): ",
            selected_song.default_tempo
        );
        stdout().flush()?;
        let mut tempo_input = String::new();
        stdin().read_line(&mut tempo_input)?;

        // Check for quit command
        if tempo_input.trim() == "q" {
            return Ok(());
        }

        let tempo_bpm = if tempo_input.trim().is_empty() {
            selected_song.default_tempo
        } else {
            tempo_input
                .trim()
                .parse()
                .unwrap_or(selected_song.default_tempo)
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
        let mut mode_input = String::new();
        stdin().read_line(&mut mode_input)?;

        // Check for quit command
        if mode_input.trim() == "q" {
            return Ok(());
        }

        let scan_mode: u32 = mode_input.trim().parse().unwrap_or(1);

        self.scan_mode(scan_duration, scan_mode)
    }
    /// Add a single MIDI file to the dynamic song list
    pub fn add_song_from_file<P: AsRef<Path>>(&mut self, path: P) -> Result<(), Box<dyn Error>> {
        let path = path.as_ref();
        let midi_data = fs::read(path)?;
        let song_info = self.parse_midi_file_from_data(&midi_data, path)?;

        self.dynamic_songs.push(song_info);
        self.dynamic_midi_data.push(midi_data);

        println!(
            "✅ Added song: {} (index {})",
            self.dynamic_songs.last().unwrap().name,
            self.get_static_song_count() + self.dynamic_songs.len() - 1
        );
        Ok(())
    }

    /// Scan a directory and add all MIDI files to the dynamic song list
    pub fn scan_directory<P: AsRef<Path>>(&mut self, dir_path: P) -> Result<usize, Box<dyn Error>> {
        let dir_path = dir_path.as_ref();
        let mut added_count = 0;

        if !dir_path.is_dir() {
            return Err(format!("Path is not a directory: {}", dir_path.display()).into());
        }

        println!("🔍 Scanning directory: {}", dir_path.display());

        for entry in fs::read_dir(dir_path)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("mid") {
                match fs::read(&path) {
                    Ok(midi_data) => match self.parse_midi_file_from_data(&midi_data, &path) {
                        Ok(song_info) => {
                            self.dynamic_songs.push(song_info);
                            self.dynamic_midi_data.push(midi_data);
                            added_count += 1;
                            println!(
                                "  ✅ Added: {}",
                                path.file_name().unwrap().to_string_lossy()
                            );
                        }
                        Err(e) => {
                            println!("  ❌ Failed to parse {}: {}", path.display(), e);
                        }
                    },
                    Err(e) => {
                        println!("  ❌ Failed to read {}: {}", path.display(), e);
                    }
                }
            }
        }

        println!("🎵 Added {} songs from directory", added_count);
        Ok(added_count)
    }

    /// Clear all dynamic songs
    pub fn clear_dynamic_songs(&mut self) {
        let count = self.dynamic_songs.len();
        self.dynamic_songs.clear();
        self.dynamic_midi_data.clear();
        println!("🧹 Cleared {} dynamic songs", count);
    }

    /// List only dynamic songs
    pub fn list_dynamic_songs(&self) {
        let static_count = self.get_static_song_count();
        let dynamic_count = self.get_dynamic_song_count();

        if dynamic_count == 0 {
            println!("📭 No dynamic songs loaded");
            return;
        }

        println!("🎶 Dynamic Songs ({} total):", dynamic_count);
        for (i, song) in self.dynamic_songs.iter().enumerate() {
            let actual_index = static_count + i;
            println!(
                "  {}: {} ({} tracks, default tempo: {} BPM)",
                actual_index,
                song.name,
                song.tracks.len(),
                song.default_tempo
            );
        }
    }

    /// List only static songs
    pub fn list_static_songs(&self) {
        let static_count = self.get_static_song_count();

        if static_count == 0 {
            println!("📭 No static songs available");
            return;
        }

        println!("📀 Static Songs ({} total):", static_count);
        for (i, song) in self.static_songs.iter().enumerate() {
            println!(
                "  {}: {} ({} tracks, default tempo: {} BPM)",
                i,
                song.name,
                song.tracks.len(),
                song.default_tempo
            );
        }
    }

    /// Parse a MIDI file and create a SongInfo structure
    fn parse_midi_file_from_data<P: AsRef<Path>>(
        &self,
        data: &[u8],
        path: P,
    ) -> Result<SongInfo, Box<dyn Error>> {
        let path = path.as_ref();
        let smf = Smf::parse(data)?;

        let song_name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Unknown")
            .to_string();

        // Parse tracks and extract information
        let mut tracks = Vec::new();
        let mut default_tempo = 120u32; // Default tempo

        for (track_index, track) in smf.tracks.iter().enumerate() {
            let mut track_info = TrackInfo {
                index: track_index,
                program: None,
                guess: None,
                channels: Vec::new(),
                note_count: 0,
                pitch_range: (127, 0), // min, max
                sample_notes: Vec::new(),
            };

            for event in track.iter() {
                match &event.kind {
                    TrackEventKind::Midi { channel, message } => {
                        let ch = channel.as_int();
                        if !track_info.channels.contains(&ch) {
                            track_info.channels.push(ch);
                        }

                        match message {
                            MidiMessage::NoteOn { key, vel: _ } => {
                                track_info.note_count += 1;
                                let pitch = key.as_int();
                                track_info.pitch_range.0 = track_info.pitch_range.0.min(pitch);
                                track_info.pitch_range.1 = track_info.pitch_range.1.max(pitch);

                                if track_info.sample_notes.len() < 5 {
                                    track_info.sample_notes.push(pitch);
                                }
                            }
                            MidiMessage::ProgramChange { program } => {
                                track_info.program = Some(program.as_int());
                            }
                            _ => {}
                        }
                    }
                    TrackEventKind::Meta(meta) => {
                        if let midly::MetaMessage::Tempo(tempo) = meta {
                            // Convert microseconds per quarter note to BPM
                            default_tempo = 60_000_000 / tempo.as_int();
                        }
                    }
                    _ => {}
                }
            }

            // Only add tracks that have notes
            if track_info.note_count > 0 {
                // Make a simple guess about the instrument
                track_info.guess = match track_info.program {
                    Some(0..=7) => Some("Piano".to_string()),
                    Some(8..=15) => Some("Chromatic".to_string()),
                    Some(16..=23) => Some("Organ".to_string()),
                    Some(24..=31) => Some("Guitar".to_string()),
                    Some(32..=39) => Some("Bass".to_string()),
                    Some(40..=47) => Some("Strings".to_string()),
                    Some(48..=55) => Some("Ensemble".to_string()),
                    Some(56..=63) => Some("Brass".to_string()),
                    Some(64..=71) => Some("Reed".to_string()),
                    Some(72..=79) => Some("Pipe".to_string()),
                    Some(80..=87) => Some("Synth Lead".to_string()),
                    Some(88..=95) => Some("Synth Pad".to_string()),
                    Some(96..=103) => Some("Synth Effects".to_string()),
                    Some(104..=111) => Some("Ethnic".to_string()),
                    Some(112..=119) => Some("Percussive".to_string()),
                    Some(120..=127) => Some("Sound Effects".to_string()),
                    _ => Some("Unknown".to_string()),
                };

                tracks.push(track_info);
            }
        }

        Ok(SongInfo {
            filename: path.to_string_lossy().to_string(),
            name: song_name,
            tracks,
            default_tempo,
        })
    }

    /// Get events for any song (static or dynamic) by index
    pub fn get_events_for_song(
        &self,
        song_index: usize,
        track_indices: &[usize],
        tempo_bpm: u32,
    ) -> Vec<Note> {
        let static_count = self.get_static_song_count();

        if song_index < static_count {
            // Static song - use the generated function
            get_events_for_song_tracks(song_index, track_indices, tempo_bpm)
        } else {
            // Dynamic song
            let dynamic_index = song_index - static_count;
            self.get_events_for_dynamic_song(dynamic_index, track_indices, tempo_bpm)
        }
    }

    /// Get events for dynamic songs
    fn get_events_for_dynamic_song(
        &self,
        dynamic_song_index: usize,
        track_indices: &[usize],
        tempo_bpm: u32,
    ) -> Vec<Note> {
        if dynamic_song_index >= self.dynamic_midi_data.len() {
            return Vec::new();
        }

        let midi_data = &self.dynamic_midi_data[dynamic_song_index];
        let smf = match Smf::parse(midi_data) {
            Ok(smf) => smf,
            Err(_) => return Vec::new(),
        };

        let mut events = Vec::new();
        let ticks_per_q = match smf.header.timing {
            midly::Timing::Metrical(ticks) => ticks.as_int() as u32,
            midly::Timing::Timecode(fps, ticks) => (fps.as_int() as u32) * (ticks as u32),
        };

        let tempo_usec_per_q = 60_000_000 / tempo_bpm;

        for track_index in track_indices {
            if let Some(track) = smf.tracks.get(*track_index) {
                let mut current_time = 0u32;
                let mut note_ons = std::collections::HashMap::new();

                for event in track.iter() {
                    current_time += event.delta.as_int();

                    if let TrackEventKind::Midi { channel, message } = &event.kind {
                        let ch = channel.as_int();
                        match message {
                            MidiMessage::NoteOn { key, vel } => {
                                let pitch = key.as_int();
                                let velocity = vel.as_int();

                                if velocity > 0 {
                                    // Convert ticks to milliseconds
                                    let start_ms = (current_time as u64 * tempo_usec_per_q as u64
                                        / ticks_per_q as u64
                                        / 1000)
                                        as u32;
                                    note_ons.insert((ch, pitch), start_ms);
                                } else {
                                    // Note off (velocity 0)
                                    if let Some(start_ms) = note_ons.remove(&(ch, pitch)) {
                                        let end_ms = (current_time as u64 * tempo_usec_per_q as u64
                                            / ticks_per_q as u64
                                            / 1000)
                                            as u32;
                                        let duration = end_ms.saturating_sub(start_ms).max(50); // Minimum 50ms duration

                                        events.push(Note {
                                            start_ms,
                                            dur_ms: duration,
                                            chan: ch,
                                            pitch,
                                            vel: velocity,
                                        });
                                    }
                                }
                            }
                            MidiMessage::NoteOff { key, vel: _ } => {
                                let pitch = key.as_int();
                                if let Some(start_ms) = note_ons.remove(&(ch, pitch)) {
                                    let end_ms = (current_time as u64 * tempo_usec_per_q as u64
                                        / ticks_per_q as u64
                                        / 1000)
                                        as u32;
                                    let duration = end_ms.saturating_sub(start_ms).max(50); // Minimum 50ms duration

                                    events.push(Note {
                                        start_ms,
                                        dur_ms: duration,
                                        chan: ch,
                                        pitch,
                                        vel: 127, // Default velocity for note off
                                    });
                                }
                            }
                            _ => {}
                        }
                    }
                }

                // Handle any remaining note-ons (notes that never got a note-off)
                for ((ch, pitch), start_ms) in note_ons {
                    let duration = 500; // Default 500ms duration for hanging notes
                    events.push(Note {
                        start_ms,
                        dur_ms: duration,
                        chan: ch,
                        pitch,
                        vel: 127,
                    });
                }
            }
        }

        // Sort events by start time
        events.sort_by_key(|note| note.start_ms);
        events
    }

    fn play_events_with_tempo_control(
        &mut self,
        events: &[Note],
        initial_tempo_bpm: u32,
    ) -> Result<bool, Box<dyn Error>> {
        // Publish playback started event
        self.publish_midi_event(crate::ipc::Event::MidiPlaybackStarted {
            song_index: 0,
            song_name: "Unknown Song".to_string(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        });

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
        let playback_finished = Arc::new(Mutex::new(false));

        // Spawn input handling thread
        let tempo_clone = Arc::clone(&tempo_multiplier);
        let quit_clone = Arc::clone(&should_quit);
        let next_clone = Arc::clone(&should_next);
        let finished_clone = Arc::clone(&playback_finished);
        let input_thread = thread::spawn(move || {
            let stdin = stdin();
            loop {
                // Check if playback has finished before trying to read input
                if let Ok(finished) = finished_clone.lock() {
                    if *finished {
                        break;
                    }
                }

                let mut input = String::new();
                if stdin.read_line(&mut input).is_ok() {
                    // Check again after reading - playback might have finished while we were reading
                    if let Ok(finished) = finished_clone.lock() {
                        if *finished {
                            break;
                        }
                    }

                    let input = input.trim();
                    if input.is_empty() {
                        // Empty input (just Enter) - check if playback finished and exit if so
                        continue;
                    }

                    if input == "q" {
                        if let Ok(mut quit) = quit_clone.lock() {
                            *quit = true;
                        }
                        break;
                    } else if input == "n" {
                        if let Ok(mut next) = next_clone.lock() {
                            *next = true;
                        }
                        break;
                    } else if input.starts_with("t") {
                        // Handle both "t" alone and "t<number>" (e.g. "t120")
                        let tempo_str = if input == "t" {
                            // Prompt for tempo input
                            println!("Enter new tempo (BPM): ");
                            let mut tempo_input = String::new();
                            if stdin.read_line(&mut tempo_input).is_ok() {
                                tempo_input.trim().to_string()
                            } else {
                                continue;
                            }
                        } else {
                            // Extract tempo from "t<number>" format
                            input[1..].to_string()
                        };
                        if let Ok(new_tempo) = tempo_str.parse::<u32>() {
                            if new_tempo > 0 && new_tempo <= 500 {
                                // Reasonable tempo range
                                tempo_clone
                                    .store((new_tempo as f32 * 1000.0) as u32, Ordering::Relaxed);
                                println!("⏱️  Tempo changed to {} BPM", new_tempo);
                            } else {
                                println!("⚠️  Invalid tempo: {} (must be 1-500 BPM)", new_tempo);
                            }
                        } else {
                            println!("⚠️  Invalid tempo format. Use 't' then enter BPM, or 't<BPM>' (e.g. 't120')");
                        }
                    } else if let Ok(new_tempo) = input.parse::<u32>() {
                        if new_tempo > 0 && new_tempo <= 500 {
                            // Reasonable tempo range
                            tempo_clone
                                .store((new_tempo as f32 * 1000.0) as u32, Ordering::Relaxed);
                            println!("⏱️  Tempo changed to {} BPM", new_tempo);
                        } else {
                            println!("⚠️  Invalid tempo: {} (must be 1-500 BPM)", new_tempo);
                        }
                    }
                } else {
                    // If read_line fails (e.g., stdin closed), break the loop
                    break;
                }
            }
        });

        let start = Instant::now();
        let mut idx = 0;
        let mut last_tempo = initial_tempo_bpm as f32 * 1000.0;
        let mut time_offset = 0.0;
        let mut last_real_time = 0.0;
        let mut last_print_time = 0u32;

        // Calculate total song duration for progress display
        let total_duration_ms = if let Some(last_event) = timeline.last() {
            last_event.t
        } else {
            0
        };

        println!("🎵 Starting playback with {} events...", timeline.len());
        while idx < timeline.len() {
            // Check if we should quit or go to next song
            if should_shutdown() {
                println!("🛑 Shutdown requested, stopping playback");
                break;
            }
            if let Ok(should_quit_guard) = should_quit.lock() {
                if *should_quit_guard {
                    println!("🛑 Playback stopped by user");
                    break;
                }
            }
            if let Ok(should_next_guard) = should_next.lock() {
                if *should_next_guard {
                    println!("⏭️  Skipping to next song...");
                    // Send all notes off before moving to next
                    for channel in 0..16 {
                        self.send_midi_command(MidiCommand::AllNotesOff)?;
                    }
                    return Ok(true);
                }
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

            // Print time progress every 100ms (similar to scan mode)
            if adjusted_time / 100 != last_print_time / 100 {
                let progress_seconds = adjusted_time / 1000;
                let total_seconds = total_duration_ms / 1000;
                let progress_percentage = if total_duration_ms > 0 {
                    (adjusted_time as f32 / total_duration_ms as f32 * 100.0) as u32
                } else {
                    0
                };
                print!(
                    "\r🎵 Playing: {}s/{}s ({}%) @ {:.0} BPM",
                    progress_seconds, total_seconds, progress_percentage, current_tempo
                );
                stdout().flush().unwrap_or(());
                last_print_time = adjusted_time;
            }

            while idx < timeline.len() && timeline[idx].t <= adjusted_time {
                let e = &timeline[idx];
                let msg = match e.kind {
                    Kind::On => [0x90 | (e.chan & 0x0F), e.p, e.v],
                    Kind::Off => [0x80 | (e.chan & 0x0F), e.p, 0],
                };
                self.send_midi_command(MidiCommand::SendMessage(msg.to_vec()))?;
                idx += 1;
            }

            sleep(Duration::from_millis(1));
        }

        // Print final newline to end the progress line
        println!();

        println!("🎼 Playbook loop finished, sending all notes off");

        // Send all notes off
        for channel in 0..16 {
            self.send_midi_command(MidiCommand::SendMessage(vec![0xB0 | channel, 123, 0]))?;
        }

        // Signal input thread that playback has finished
        if let Ok(mut finished) = playback_finished.lock() {
            *finished = true;
        }

        println!("✅ Playback complete!");
        // Don't wait for input thread - it may be blocked on stdin reading
        // Just drop the handle and let it be cleaned up
        drop(input_thread);

        // Publish playback stopped event
        self.publish_midi_event(crate::ipc::Event::MidiPlaybackStopped {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        });

        // Return true if user didn't quit (wants to continue looping), false if they quit
        let user_quit = should_quit.lock().map(|guard| *guard).unwrap_or(false);
        Ok(!user_quit)
    }

    fn play_events_with_tempo_control_and_scan_limit(
        &mut self,
        events: &[Note],
        initial_tempo_bpm: u32,
        max_duration_ms: u32,
    ) -> Result<bool, Box<dyn Error>> {
        self.play_events_with_tempo_control_and_scan_limit_internal(
            events,
            initial_tempo_bpm,
            max_duration_ms,
            true,
        )
    }

    fn play_events_with_tempo_control_and_scan_limit_non_interactive(
        &mut self,
        events: &[Note],
        initial_tempo_bpm: u32,
        max_duration_ms: u32,
    ) -> Result<bool, Box<dyn Error>> {
        self.play_events_with_tempo_control_and_scan_limit_internal(
            events,
            initial_tempo_bpm,
            max_duration_ms,
            false,
        )
    }

    fn play_events_with_tempo_control_and_scan_limit_internal(
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
                t: if end_time <= max_duration_ms {
                    end_time
                } else {
                    max_duration_ms
                },
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
                                    tempo_clone.store(
                                        (new_tempo as f32 * 1000.0) as u32,
                                        Ordering::Relaxed,
                                    );
                                    println!("⏱️  Tempo changed to {} BPM", new_tempo);
                                }
                            }
                        } else if let Ok(new_tempo) = input.parse::<u32>() {
                            tempo_clone
                                .store((new_tempo as f32 * 1000.0) as u32, Ordering::Relaxed);
                            println!("⏱️  Tempo changed to {} BPM", new_tempo);
                        }
                    }
                }
            }))
        } else {
            None
        };
        let start = Instant::now();
        let mut idx = 0;
        let mut last_tempo = initial_tempo_bpm as f32 * 1000.0;
        let mut time_offset = 0.0;
        let mut last_real_time = 0.0;
        let mut last_print_time = 0u32;
        while idx < timeline.len() {
            let real_elapsed = start.elapsed().as_millis() as u32;

            if real_elapsed >= max_duration_ms {
                break;
            }

            if should_shutdown() {
                println!("🛑 Shutdown requested, stopping scan playback");
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

            // Print time progress every 100ms
            if real_elapsed / 100 != last_print_time / 100 {
                let progress_seconds = real_elapsed / 1000;
                let total_seconds = max_duration_ms / 1000;
                let progress_percentage =
                    (real_elapsed as f32 / max_duration_ms as f32 * 100.0) as u32;
                print!(
                    "\r🎵 Playing: {}s/{}s ({}%) @ {} BPM",
                    progress_seconds, total_seconds, progress_percentage, current_tempo
                );
                stdout().flush().unwrap_or(());
                last_print_time = real_elapsed;
            }
            while idx < timeline.len() && timeline[idx].t <= adjusted_time {
                let e = &timeline[idx];
                let msg = match e.kind {
                    Kind::On => [0x90 | (e.chan & 0x0F), e.p, e.v],
                    Kind::Off => [0x80 | (e.chan & 0x0F), e.p, 0],
                };
                self.send_midi_command(MidiCommand::SendMessage(msg.to_vec()))?;
                idx += 1;
            }
            sleep(Duration::from_millis(1));
        }

        // Print final newline to end the progress line
        println!();

        // Send all notes off
        for channel in 0..16 {
            self.send_midi_command(MidiCommand::SendMessage(vec![0xB0 | channel, 123, 0]))?;
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
    /// Initialize IPC publisher for event-driven communication
    pub fn init_ipc_publisher(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.ipc_manager.is_none() {
            match IpcServiceManager::new(AppId::EMidi) {
                Ok(manager) => {
                    self.ipc_manager = Some(manager);
                    // IPC initialized silently - no output to avoid TUI corruption
                    Ok(())
                }
                Err(_) => {
                    // Silently fail - IPC is optional for the MIDI player
                    Ok(())
                }
            }
        } else {
            Ok(()) // Already initialized
        }
    }

    /// Publish a MIDI event via IPC if publisher is available
    fn publish_midi_event(&self, event: crate::ipc::Event) {
        if let Some(ref ipc_manager) = self.ipc_manager {
            let _ = ipc_manager.publish_event(event); // Silently ignore errors
        }
    }
    /// Play a song with IPC event publishing for TUI integration
    pub fn play_song_with_ipc(&mut self, song_index: usize) -> Result<(), Box<dyn Error>> {
        if song_index >= self.get_total_song_count() {
            return Err("Invalid song index".into());
        }

        let selected_song = self.get_song(song_index).ok_or("Invalid song index")?;
        let track_indices: Vec<usize> = selected_song.tracks.iter().map(|t| t.index).collect();
        let tempo = selected_song.default_tempo;

        // Publish playback started event
        self.publish_midi_event(crate::ipc::Event::MidiPlaybackStarted {
            song_index,
            song_name: selected_song.name.clone(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        });

        let events = self.get_events_for_song(song_index, &track_indices, tempo);
        if events.is_empty() {
            // Publish stopped event immediately if no events
            self.publish_midi_event(crate::ipc::Event::MidiPlaybackStopped {
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64,
            });
            return Err("No events to play! Check track selection.".into());
        } // Use the non-blocking playback method to avoid blocking the TUI
        self.play_song_with_ipc_nonblocking(song_index)?;

        Ok(())
    }

    /// Non-blocking version of play_song_with_ipc that spawns playback in a background thread
    pub fn play_song_with_ipc_nonblocking(
        &mut self,
        song_index: usize,
    ) -> Result<(), Box<dyn Error>> {
        if song_index >= self.get_total_song_count() {
            return Err("Invalid song index".into());
        }

        let selected_song = self.get_song(song_index).ok_or("Invalid song index")?;
        let track_indices: Vec<usize> = selected_song.tracks.iter().map(|t| t.index).collect();
        let tempo = selected_song.default_tempo;

        // Publish playback started event
        self.publish_midi_event(crate::ipc::Event::MidiPlaybackStarted {
            song_index,
            song_name: selected_song.name.clone(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        });

        let events = self.get_events_for_song(song_index, &track_indices, tempo);
        if events.is_empty() {
            // Publish stopped event immediately if no events
            self.publish_midi_event(crate::ipc::Event::MidiPlaybackStopped {
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64,
            });
            return Err("No events to play! Check track selection.".into());
        } // Reset the stop flag before starting new playback
        self.reset_stop_flag();
        self.is_playing.store(true, Ordering::Relaxed);

        // Clone the MIDI sender and stop flag for the background thread
        let midi_sender = self.midi_sender.clone();
        let stop_flag = Arc::clone(&self.playback_stop_flag);
        let playing_state = Arc::clone(&self.is_playing);
        let events_clone = events.clone();

        // Spawn playback in a background thread
        thread::spawn(move || {
            if let Err(e) = Self::play_events_in_background(
                events_clone,
                tempo,
                midi_sender,
                stop_flag,
                playing_state,
            ) {
                eprintln!("Background playback error: {}", e);
            }
        });

        Ok(())
    }
    /// Static method to play events in a background thread (doesn't need &mut self)
    fn play_events_in_background(
        events: Vec<Note>,
        tempo_bpm: u32,
        midi_sender: std::sync::mpsc::Sender<MidiCommand>,
        stop_flag: Arc<AtomicBool>,
        playing_state: Arc<AtomicBool>,
    ) -> Result<(), Box<dyn Error>> {
        use std::thread;
        use std::time::Instant;

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
        for n in &events {
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

        let start = Instant::now();
        let mut idx = 0;
        let tempo_ms_per_beat = 60000.0 / tempo_bpm as f32;
        while idx < timeline.len() {
            // Check for global shutdown or local stop flag
            if should_shutdown() || stop_flag.load(Ordering::Relaxed) {
                break;
            }

            let event = &timeline[idx];
            let target_time_ms = (event.t as f32 * tempo_ms_per_beat / 500.0) as u64;
            let elapsed_ms = start.elapsed().as_millis() as u64;

            if elapsed_ms < target_time_ms {
                thread::sleep(Duration::from_millis(std::cmp::min(
                    target_time_ms - elapsed_ms,
                    50,
                )));
                continue;
            }

            // Send MIDI event through the channel
            let msg = match event.kind {
                Kind::On => vec![0x90 | event.chan, event.p, event.v],
                Kind::Off => vec![0x80 | event.chan, event.p, 0],
            };

            if let Err(_) = midi_sender.send(MidiCommand::SendMessage(msg)) {
                // MIDI thread is probably shutdown, exit gracefully
                break;
            }
            idx += 1;
        } // Mark playback as finished
        playing_state.store(false, Ordering::Relaxed);
        Ok(())
    }
}

/// Calculate the total duration of a song in milliseconds
pub fn calculate_song_duration_ms(events: &[Note]) -> u32 {
    events
        .iter()
        .map(|note| note.start_ms + note.dur_ms)
        .max()
        .unwrap_or(0)
}

/// Format duration in milliseconds to a readable string
pub fn format_duration(duration_ms: u32) -> String {
    let seconds = duration_ms / 1000;
    let minutes = seconds / 60;
    let remaining_seconds = seconds % 60;

    if minutes > 0 {
        format!("{}m{:02}s", minutes, remaining_seconds)
    } else {
        format!("{}s", remaining_seconds)
    }
}

impl MidiPlayer {
    /// Simple playback method for non-interactive scan mode - no input handling, just plays for the specified duration
    fn play_events_simple(
        &mut self,
        events: &[Note],
        tempo_bpm: u32,
        max_duration_ms: u32,
    ) -> Result<(), Box<dyn Error>> {
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
                t: if end_time <= max_duration_ms {
                    end_time
                } else {
                    max_duration_ms
                },
                kind: Kind::Off,
                chan: note.chan,
                p: note.pitch,
                v: note.vel,
            });
        }
        timeline.sort_by_key(|e| e.t);

        let start = Instant::now();
        let mut idx = 0;
        let mut last_print_time = 0u32;

        while idx < timeline.len() {
            let real_elapsed = start.elapsed().as_millis() as u32;

            // Stop if we've reached the maximum duration
            if real_elapsed >= max_duration_ms {
                break;
            }

            // Check for shutdown signal
            if should_shutdown() {
                println!("🛑 Shutdown requested, stopping scan playback");
                break;
            }

            // Print time progress every 100ms
            if real_elapsed / 100 != last_print_time / 100 {
                let progress_seconds = real_elapsed / 1000;
                let total_seconds = max_duration_ms / 1000;
                let progress_percentage =
                    (real_elapsed as f32 / max_duration_ms as f32 * 100.0) as u32;
                print!(
                    "\r🎵 Playing: {}s/{}s ({}%) @ {} BPM",
                    progress_seconds, total_seconds, progress_percentage, tempo_bpm
                );
                stdout().flush().unwrap_or(());
                last_print_time = real_elapsed;
            } // Play all events scheduled for this time
            while idx < timeline.len() && timeline[idx].t <= real_elapsed {
                let e = &timeline[idx];
                let msg = match e.kind {
                    Kind::On => [0x90 | (e.chan & 0x0F), e.p, e.v],
                    Kind::Off => [0x80 | (e.chan & 0x0F), e.p, 0],
                };
                self.send_midi_command(MidiCommand::SendMessage(msg.to_vec()))?;
                idx += 1;
            }

            sleep(Duration::from_millis(1));
        }
        // Print final newline to end the progress line
        println!();

        // Send all notes off
        for channel in 0..16 {
            self.send_midi_command(MidiCommand::SendMessage(vec![0xB0 | channel, 123, 0]))?;
        }

        Ok(())
    }

    /// Interactive method to load MIDI files or directories
    fn load_midi_interactive(&mut self) -> Result<(), Box<dyn Error>> {
        println!("\n📁 Load MIDI Files or Directories");
        println!("Enter path(s) separated by spaces (files or directories):");
        print!("Path(s): ");
        stdout().flush()?;

        let mut input = String::new();
        stdin().read_line(&mut input)?;
        let input = input.trim();

        if input.is_empty() {
            println!("❌ No path provided");
            return Ok(());
        }

        let paths: Vec<&str> = input.split_whitespace().collect();
        let mut total_added = 0;

        for path_str in paths {
            // Strip surrounding quotes if present
            let cleaned_path = if (path_str.starts_with('"') && path_str.ends_with('"'))
                || (path_str.starts_with('\'') && path_str.ends_with('\''))
            {
                &path_str[1..path_str.len() - 1]
            } else {
                path_str
            };

            let path = std::path::Path::new(cleaned_path);

            if !path.exists() {
                println!("❌ Path does not exist: {}", cleaned_path);
                continue;
            }

            if path.is_file() {
                if path.extension().and_then(|s| s.to_str()) == Some("mid") {
                    match self.add_song_from_file(path) {
                        Ok(()) => total_added += 1,
                        Err(e) => println!("❌ Failed to load {}: {}", cleaned_path, e),
                    }
                } else {
                    println!("❌ Not a MIDI file: {}", cleaned_path);
                }
            } else if path.is_dir() {
                match self.scan_directory(path) {
                    Ok(count) => total_added += count,
                    Err(e) => println!("❌ Failed to scan directory {}: {}", cleaned_path, e),
                }
            }
        }
        if total_added > 0 {
            println!("✅ Successfully loaded {} songs total", total_added);
        } else {
            println!("❌ No songs were loaded");
        }

        Ok(())
    }
    /// Run TUI mode with IPC relay
    pub fn run_tui_mode_with_ipc(&mut self) -> Result<(), Box<dyn Error>> {
        // Initialize IPC publisher for status events
        self.init_ipc_publisher()?;

        // Run TUI mode normally - it will handle its own IPC communication
        crate::tui::run_tui_mode(self)
    }

    /// Process IPC commands from TUI and execute them
    fn run_ipc_command_loop(
        &mut self,
        mut subscriber: crate::ipc::EventSubscriber,
    ) -> Result<(), Box<dyn Error>> {
        println!("🔗 IPC command loop started, listening for TUI commands...");

        loop {
            if should_shutdown() {
                break;
            }

            // Check for commands from TUI
            match subscriber.try_receive() {
                Ok(events) => {
                    for event in events {
                        self.handle_ipc_command(event)?;
                    }
                }
                Err(_) => {
                    // No events available - continue
                }
            }

            // Small delay to prevent busy waiting
            thread::sleep(Duration::from_millis(10));
        }

        println!("🔗 IPC command loop finished");
        Ok(())
    }

    /// Handle individual IPC commands
    fn handle_ipc_command(&mut self, event: crate::ipc::Event) -> Result<(), Box<dyn Error>> {
        match event {
            crate::ipc::Event::MidiCommandPlay { song_index, .. } => {
                println!("🎵 Received play command for song {}", song_index);
                if song_index < self.get_total_song_count() {
                    // Use the IPC-enabled playback method
                    self.play_song_with_ipc(song_index)?;
                } else {
                    println!("❌ Invalid song index: {}", song_index);
                }
            }
            crate::ipc::Event::MidiCommandStop { .. } => {
                println!("⏹️ Received stop command");
                // Send all notes off
                for channel in 0..16 {
                    self.send_midi_command(MidiCommand::SendMessage(vec![0xB0 | channel, 123, 0]))?;
                }
                self.publish_midi_event(crate::ipc::Event::MidiPlaybackStopped {
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as u64,
                });
            }
            crate::ipc::Event::MidiCommandNext { .. } => {
                println!("⏭️ Received next command");
                // TODO: Implement next song logic
            }
            crate::ipc::Event::MidiCommandPrevious { .. } => {
                println!("⏮️ Received previous command");
                // TODO: Implement previous song logic
            }
            _ => {
                // Ignore other events
            }
        }

        Ok(())
    }
}
