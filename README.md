# e_midi - Interactive MIDI Player

e_midi is more than a simple midi player.  It currently supports .mid and .xml files.
MusicXML support is very alpha and if you're interested, PRs are welcome!

now includes .ogg and .mp3 embedding/playing via rodio.  mp4/webm embed but do not play yet.

## Features

### 🎵 Playback Modes
- **Single Song**: Play a specific track with optional looping
- **All Songs**: Play through your entire MIDI collection
- **Random Song**: Randomly select and play a track
- **Scan Mode**: Preview segments of songs with multiple scan patterns

### 🔄 Looping Options
- **Playlist Looping**: Continuously loop through all songs
- **Individual Song Looping**: Repeat single tracks indefinitely
- **User Control**: Press 'q' + Enter during playback to quit loops

### 🎛️ Advanced Features
- **Track Selection**: Choose specific MIDI tracks to play
- **BPM Override**: Override default tempo with custom BPM
- **Configurable Delays**: Set custom delays between songs (including zero delay)
- **Progress Reporting**: Real-time progress display with timestamps and percentages
- **Multiple Scan Modes**: Sequential, random start, and progressive scanning

### 🎯 Scan Mode Options
- **Sequential Scan**: Play segments from each song in order
- **Random Start Scan**: Begin each song segment at a random position
- **Progressive Scan**: Gradually increase segment duration for deeper exploration
- **Configurable Duration**: Set custom scan segment lengths (default: 30 seconds)


> **Additional Binary:**
>
> - **e_midi_demo01**: Windows-only demo for window focus, resize, and move event integration with e_grid IPC. Useful for testing advanced window event handling and IPC features. Source: `examples/demo_focus_resize_move.rs`.
>
> **To run:**
> ```cmd
> cargo run --bin e_midi_demo01
> ```
> or after building:
> ```cmd
> target\release\e_midi_demo01.exe
> ```
>
> ---
>
> **Running after install:**
>
> After installing with:
> ```cmd
> cargo install e_midi
> ```
> you can run the main player binary directly as `e_midi` from your terminal or command prompt.  `e_midi_demo01` is also installed and should be available for your use.
>
> **Note:** The default `e_midi` binary/lib includes the curated MIDI sound effects from the `midi` folder within the repository, embedded at build time. These static songs are always available, even if you run the binary outside the repository directory.

A feature-rich, interactive MIDI player written in Rust with advanced playback options, looping capabilities, and scan modes.

> - **e_midi_demo02**: Persistent IPC event listener demo. Launches e_midi to play song 0 with IPC enabled, then displays all incoming MIDI note events in real time. Remains running and will display events from any e_midi instance with IPC enabled. Useful for debugging and monitoring event flow. Source: `examples/e_midi_demo02.rs`.

> - **e_midi_ipc_player**: Designed to be controlled entirely via inter-process communication. Tt listens for IPC midi events and plays the notes with a changing random voice.  run `e_midi_demo02`, then start as many `e_midi_ipc_player` as you desire; they will all play the same song.

### Prerequisites
- Rust (latest stable version)
- A MIDI output device or software synthesizer
- MIDI files to play

### Building from Source
```bash
git clone https://github.com/davehorner/e_midi.git
cd e_midi
cargo build --release
```

### Running
```bash
cargo run
```

## Usage

### Interactive Menu
The application starts with an interactive configuration menu:

1. **Loop Configuration**: Choose playlist and/or individual song looping
2. **Delay Settings**: Configure pause duration between songs
3. **Playback Mode Selection**: Choose from 4 different playback modes

### Song Selection
The player maintains a unified song index where:
- **Static songs** (compiled-in from `midi/`) appear first (indexes 0-N)
- **Dynamic songs** (runtime-loaded) appear after static songs (indexes N+1 onwards)
- Song selection by index works seamlessly across both types
- Static songs provide guaranteed availability and optimal performance

### Track and BPM Selection
When playing songs, you can:
- **Track Selection**: Enter track numbers (e.g., "1,3,5") or press Enter for all tracks
- **BPM Override**: Enter a custom BPM or press Enter to use the MIDI's default tempo

### Example Session
```
🎵 MIDI Player Settings
═══════════════════════
🔄 Loop the entire playlist? (y/N): y
🔄 Loop individual songs? (y/N): n
⏱️  Delay between songs in seconds (default 2, 0 for no delay): 5

🎵 Choose an option:
1: Play a specific song
2: Play all songs
3: Play random song
4: Scan mode (play portions of songs)

Select option (1-4): 4

🔍 Scan Mode Options:
1: Sequential scan (play segments in order)
2: Random start scan (random positions)
3: Progressive scan (increasing duration)

Select scan type (1-3): 3
```

## Command Line Interface

### Overview
e_midi provides both interactive and command-line modes. The CLI allows for scripting, automation, and integration with other tools.

**Important**: Global options must come before the subcommand (e.g., `e_midi --delay-between-songs 5 play-random`), while subcommand-specific options come after the subcommand (e.g., `e_midi scan --mode 2 --duration 45`).

### Full Help Output
```
An interactive/CLI/library MIDI player with advanced playback options, looping, and scan modes.

Usage: e_midi.exe [OPTIONS] [COMMAND]

Commands:
  list           List all available songs
  play           Play a specific song
  play-all       Play all songs in sequence
  play-random    Play songs in random order
  scan           Scan mode - play portions of songs
  list-dynamic   List only dynamically loaded songs
  clear-dynamic  Clear all dynamically loaded songs
  interactive    Run in interactive mode (default)
  help           Print this message or the help of the given subcommand(s)

Options:
      --loop-playlist
          Loop the entire playlist continuously
      --loop-individual-songs
          Loop individual songs
      --delay-between-songs <DELAY_BETWEEN_SONGS>
          Delay between songs in seconds [default: 0]
      --scan-duration <SCAN_DURATION>
          Scan segment duration in seconds [default: 30]
      --scan-random-start
          Start scan segments at random positions
  -t, --tui
          Use TUI mode with split panels (menu + playback info)
      --add-song <ADD_SONGS>
          Add MIDI files to the dynamic playlist
      --scan-directory <SCAN_DIRECTORIES>
          Scan directories and add all MIDI files to the dynamic playlist
  -h, --help
          Print help
  -V, --version
          Print version
```

### Command Examples

#### Basic Playback
```bash
# List available songs
e_midi list

# Play song at index 5
e_midi play 5

# Play song 3 with custom tempo
e_midi play 3 --tempo 140

# Play specific tracks (1, 3, 5) from song 2
e_midi play 2 --tracks 1,3,5

# Play all songs in sequence
e_midi play-all

# Play songs in random order
e_midi play-random
```

#### Looping and Timing
```bash
# Loop the entire playlist
e_midi --loop-playlist play-all

# Loop individual songs with 5-second delays
e_midi --loop-individual-songs --delay-between-songs 5 play-all

# Play song 0 on loop
e_midi --loop-individual-songs play 0
```

#### Scan Mode
```bash
# Sequential scan with default 30-second segments
e_midi scan

# Random position scan with 45-second segments
e_midi scan --mode 2 --duration 45

# Progressive scan (increasing duration)
e_midi scan --mode 3

# Scan with random start positions
e_midi --scan-random-start scan
```

#### Dynamic Playlist Management
```bash
# Add individual MIDI files
e_midi --add-song song1.mid --add-song song2.mid list

# Scan directory for MIDI files
e_midi --scan-directory /path/to/midi/files list

# List only dynamically loaded songs
e_midi list-dynamic

# Clear all dynamic songs
e_midi clear-dynamic
```

#### TUI Mode
```bash
# Launch with Terminal User Interface
e_midi --tui

# TUI with pre-loaded dynamic songs
e_midi --tui --scan-directory /path/to/midi/files
```

### Subcommand Details

#### `play` Command
```
Play a specific song

Usage: e_midi.exe play [OPTIONS] <SONG_INDEX>

Arguments:
  <SONG_INDEX>  Song index to play

Options:
      --tracks <TRACKS>  Track numbers to play (comma-separated)
      --tempo <TEMPO>    Tempo in BPM
  -h, --help             Print help
```

#### `scan` Command
```
Scan mode - play portions of songs

Usage: e_midi.exe scan [OPTIONS]

Options:
      --mode <MODE>          Scan mode: 1=sequential, 2=random positions, 3=progressive [default: 1]
      --duration <DURATION>  Duration of each scan segment in seconds
  -h, --help                 Print help
```

### Integration Examples

#### Batch Processing
```bash
# Play all songs with logging and 1-second delays
e_midi --delay-between-songs 1 play-all > playback.log 2>&1

# Scan all songs for 10 seconds each
e_midi --scan-duration 10 scan --mode 1
```

#### Scripting
```bash
#!/bin/bash
# Play random songs for background music with 2-second delays
while true; do
    e_midi --delay-between-songs 2 play-random
    sleep 5
done
```

## Song Management

### Static vs Dynamic Songs
The e_midi player uses a hybrid approach for managing MIDI content:

#### Static Songs (Compiled-In)
- **Build-time Processing**: MIDI and MusicXML files (`.mid`, `.xml`, `.musicxml`) in the `midi/` directory are processed at compile time by `build.rs`
- **Embedded Data**: Song data is compiled directly into the executable for fast access
- **Index Priority**: Static songs appear first in the song index (positions 0-N)
- **Performance**: Zero I/O overhead during playback - all data is in memory
- **Use Case**: Core repertoire, frequently played songs, or embedded deployments

#### Dynamic Songs (Runtime Loading)
- **Runtime Discovery**: Additional MIDI files can be loaded at runtime from specified directories
- **Flexible Content**: Add new songs without recompilation
- **Index Continuation**: Dynamic songs appear after static songs in the index (positions N+1 onwards)
- **File I/O**: Loaded on-demand with minimal caching
- **Use Case**: Experimental content, large libraries, or user-provided files

The player seamlessly handles both types, with static songs providing guaranteed availability and performance, while dynamic songs offer flexibility for expanding the music library.

## Inter-Process Communication (IPC)

### iceoryx2 Integration
e_midi includes built-in IPC capabilities using the iceoryx2 framework for lock-free, zero-copy communication:

#### Communication Features
- **Real-time Events**: Playback status, song changes, progress updates
- **Remote Control**: Play, stop, pause, next/previous, tempo control via IPC
- **State Synchronization**: Song lists, playback state, window management
- **Grid Integration**: Future support for e_grid pattern-based control

#### Ecosystem Integration
- **e_grid**: Pattern-based MIDI triggering and sequencing
- **State Server**: Centralized state management across e_* applications
- **Multi-instance**: Multiple e_midi instances can coordinate playback
- **External Control**: Third-party applications can control playback via IPC

#### Event Types
- **MIDI Commands**: Play, stop, pause, resume, tempo changes
- **Status Updates**: Playback started/stopped, song changes, progress
- **System Events**: Heartbeats, shutdown coordination, state requests

The IPC system enables e_midi to function as both a standalone player and a component in larger musical ecosystems.

## Configuration

### Build-time Configuration
The application processes MIDI files at build time using `build.rs`. Place your MIDI files in the project directory and they will be automatically processed and embedded.

### Runtime Configuration
- **Loop Settings**: Configure at startup
- **Scan Duration**: Default 30 seconds, configurable per session
- **Delay Between Songs**: 0-∞ seconds, configurable
- **Track Selection**: Per-song basis during playback

## Technical Details

### Architecture
- **Build Script**: Processes MIDI files and generates Rust code at compile time
- **Event Timeline**: Converts MIDI events to a timeline-based playback system
- **Non-blocking Input**: Allows user interaction during playback
- **Accurate Timing**: Precise millisecond-level timing for faithful MIDI reproduction
- **IPC Layer**: iceoryx2-based inter-process communication for ecosystem integration
- **Hybrid Storage**: Compile-time embedded songs + runtime dynamic loading

### MIDI Processing
- Supports standard MIDI files (SMF)
- Handles multiple tracks and channels
- Preserves original timing and velocity information
- Automatic tempo calculation and BPM override support

### Dependencies
- `midir`: MIDI I/O operations
- `midly`: MIDI file parsing
- `rimd`: Additional MIDI utilities
- `ansi_term`: Terminal color output
- `iceoryx2`: Lock-free inter-process communication
- `ratatui`: Terminal user interface framework
- `crossterm`: Cross-platform terminal manipulation

## Troubleshooting

### No MIDI Output
Ensure you have a MIDI output device available:
- **Windows**: Built-in software synthesizer or external MIDI device
- **macOS**: Built-in audio or external MIDI interface
- **Linux**: ALSA, JACK, or PulseAudio MIDI support

### Build Issues
If you encounter build errors:
1. Ensure all MIDI files are valid
2. Check that dependencies are up to date: `cargo update`
3. Clean and rebuild: `cargo clean && cargo build`

### Playback Issues
- **No sound**: Verify MIDI output device and volume settings
- **Timing issues**: Check system audio latency settings
- **Crash during playback**: Ensure MIDI files are not corrupted

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- Built with the excellent Rust MIDI ecosystem
- Inspired by classic MIDI sequencers and players
- Thanks to the Rust community for amazing crates and documentation

## Free/Public Domain MIDI Sound Effects

The midi folder contains a curated set of short, expressive MIDI sound effects (e.g., success, error, alert, coin, powerup) created by David Horner with the assistance of ChatGPT. All files listed in README.MIDI.md are released into the public domain under [CC0 1.0 Universal](https://creativecommons.org/publicdomain/zero/1.0/). You may use, modify, and distribute these files freely, even for commercial purposes, without attribution.

The `midi` folder supports both MIDI (`.mid`) and MusicXML (`.xml`, `.musicxml`) files for static, compiled-in songs.

For details and a full index of available sounds, see [README.MIDI.md](midi/README.MIDI.md).


## Changelog

### v0.1.8
- Added initial integration with [`tidalcycles-rs`](https://github.com/davehorner/e_midi/tree/develop/tidalcycles-rs) for pattern-based MIDI sequencing and experimental live coding support. This enables advanced rhythmic and melodic pattern playback alongside standard MIDI features.

### v0.1.7
- Added `e_midi_demo02` and `e_midi_ipc_player` binaries for IPC event monitoring and playback

### v0.1.0 (Current)
- Initial release with comprehensive MIDI playback capabilities
- **Complete CLI interface** with all interactive features
- **Interactive Menu Mode** with configuration options
- **Terminal User Interface (TUI)** mode with --tui flag
- **Multiple Playback Modes**: Single song, all songs, random, and scan modes
- **Advanced Scan Modes**: Sequential, random start, and progressive scanning
- **Looping Support**: Playlist and individual song looping with user control
- **Track Selection**: Choose specific MIDI tracks to play
- **BPM Override**: Custom tempo control with real-time adjustment
- **Dynamic Playlist Management**: --add-song and --scan-directory options
- **Static vs Dynamic Songs**: Compile-time embedded + runtime loading
- **Configurable Delays**: Custom timing between songs (including zero delay)
- **Progress Reporting**: Real-time progress with timestamps and percentages
- **Inter-process Communication (IPC)**: iceoryx2-based ecosystem integration
- **Cross-platform MIDI Support**: Windows, macOS, and Linux compatibility
