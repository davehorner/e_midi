use clap::{Parser, Subcommand};
use std::error::Error;

use crate::MidiPlayer;

#[derive(Parser)]
#[command(name = "e_midi")]
#[command(about = "A feature-rich interactive MIDI player with advanced playback options")]
#[command(version = "0.2.0")]
#[command(author = "David Horner <david.horner@example.com>")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
    
    /// Loop the entire playlist continuously
    #[arg(long)]
    pub loop_playlist: bool,
    
    /// Loop individual songs
    #[arg(long)]
    pub loop_individual_songs: bool,
      /// Delay between songs in seconds
    #[arg(long, default_value = "0")]
    pub delay_between_songs: u32,
    
    /// Scan segment duration in seconds
    #[arg(long, default_value = "30")]
    pub scan_duration: u32,
    
    /// Start scan segments at random positions
    #[arg(long)]
    pub scan_random_start: bool,
    
    /// Use TUI mode with split panels (menu + playback info)
    #[arg(short = 't', long)]
    pub tui: bool,
    
    /// Add MIDI files to the dynamic playlist
    #[arg(long = "add-song")]
    pub add_songs: Vec<std::path::PathBuf>,
    
    /// Scan directories and add all MIDI files to the dynamic playlist
    #[arg(long = "scan-directory")]
    pub scan_directories: Vec<std::path::PathBuf>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// List all available songs
    List,
    
    /// Play a specific song
    Play {
        /// Song index to play
        song_index: usize,
        
        /// Track numbers to play (comma-separated, 0 for all tracks)
        #[arg(long, value_delimiter = ',')]
        tracks: Option<Vec<usize>>,
        
        /// Tempo in BPM
        #[arg(long)]
        tempo: Option<u32>,
    },
    
    /// Play all songs in sequence
    PlayAll,
    
    /// Play songs in random order
    PlayRandom,
      /// Scan mode - play portions of songs
    Scan {
        /// Scan mode: 1=sequential, 2=random positions, 3=progressive
        #[arg(long, default_value = "1")]
        mode: u32,
        
        /// Duration of each scan segment in seconds
        #[arg(long)]
        duration: Option<u32>,    },
      /// List only dynamically loaded songs
    ListDynamic,
    
    /// Clear all dynamically loaded songs
    ClearDynamic,
    
    /// Run in interactive mode (default)
    Interactive,
}

pub fn run_cli() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();
    
    let mut player = MidiPlayer::new()?;    // Apply CLI configuration
    {
        let config = player.get_config_mut();
        config.loop_playlist = cli.loop_playlist;
        config.loop_individual_songs = cli.loop_individual_songs;
        config.delay_between_songs_ms = cli.delay_between_songs * 1000;
        config.scan_segment_duration_ms = cli.scan_duration * 1000;
        config.scan_random_start = cli.scan_random_start;
    }
    
    // Process global options to add songs/directories to dynamic playlist
    for path in &cli.add_songs {
        if let Err(e) = player.add_song_from_file(path) {
            eprintln!("❌ Failed to add {}: {}", path.display(), e);
        }
    }
    
    for path in &cli.scan_directories {
        match player.scan_directory(path) {
            Ok(count) => println!("✅ Added {} songs from {}", count, path.display()),
            Err(e) => eprintln!("❌ Failed to scan {}: {}", path.display(), e),
        }
    }
    
    match cli.command {
        Some(Commands::List) => {
            player.list_songs();
        },
        
        Some(Commands::Play { song_index, tracks, tempo }) => {
            if song_index >= player.get_songs().len() {
                eprintln!("❌ Invalid song index {}. Use 'list' command to see available songs.", song_index);
                std::process::exit(1);
            }
            
            let loop_individual = player.get_config().loop_individual_songs;
            let result: Result<(), Box<dyn Error>> = if loop_individual {
                // For looping, we need to handle it differently
                loop {
                    let continue_playing = player.play_song(song_index, tracks.clone(), tempo)?;
                    if !continue_playing {
                        break;
                    }
                }
                Ok(())
            } else {
                player.play_song(song_index, tracks, tempo)?;
                Ok(())
            };
            
            result?;
        },
        
        Some(Commands::PlayAll) => {
            player.play_all_songs()?;
        },
        
        Some(Commands::PlayRandom) => {
            player.play_random_song()?;
        },        Some(Commands::Scan { mode, duration }) => {
            let scan_duration = duration.unwrap_or(cli.scan_duration);
            player.scan_mode_non_interactive(scan_duration, mode)?;
        },
          Some(Commands::ListDynamic) => {
            player.list_dynamic_songs();
        },
        
        Some(Commands::ClearDynamic) => {
            player.clear_dynamic_songs();
        },
          Some(Commands::Interactive) | None => {
            // Choose between TUI and CLI mode
            if cli.tui {
                player.run_tui_mode()?;
            } else {
                player.run_interactive()?;
            }
        },
    }
    
    Ok(())
}

pub fn print_help() {
    let _cli = Cli::parse_from(&["e_midi", "--help"]);
}

// Helper function to validate song index
pub fn validate_song_index(player: &MidiPlayer, index: usize) -> Result<(), String> {
    if index >= player.get_songs().len() {
        Err(format!("Invalid song index {}. Available songs: 0-{}", 
                   index, player.get_songs().len() - 1))
    } else {
        Ok(())
    }
}

// Helper function to format song list for CLI output
pub fn format_song_list(player: &MidiPlayer) -> String {
    let mut output = String::new();
    output.push_str("Available Songs:\n");
    for (i, song) in player.get_songs().iter().enumerate() {
        output.push_str(&format!("{}: {} ({} tracks, default tempo: {} BPM)\n", 
            i, song.name, song.tracks.len(), song.default_tempo));
    }
    output
}
