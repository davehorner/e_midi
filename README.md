# e_midi - Interactive MIDI Player

A feature-rich, interactive MIDI player written in Rust with advanced playback options, looping capabilities, and scan modes.

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

## Installation

### Prerequisites
- Rust (latest stable version)
- A MIDI output device or software synthesizer
- MIDI files to play

### Building from Source
```bash
git clone https://github.com/davidhorner/e_midi.git
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

## Development

### Project Structure
```
├── build.rs              # MIDI processing and code generation
├── src/
│   ├── main.rs           # Main application logic
│   └── midi_data.rs      # Generated MIDI data (build artifact)
├── Cargo.toml           # Dependencies and metadata
└── *.mid               # MIDI files to process
```

### Adding Features
The codebase is modular and easy to extend:
- **New playback modes**: Add functions in `main.rs`
- **Additional MIDI processing**: Modify `build.rs`
- **Enhanced UI**: Extend the menu system
- **Export capabilities**: Add file output options

### Contributing
1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests if applicable
5. Submit a pull request

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- Built with the excellent Rust MIDI ecosystem
- Inspired by classic MIDI sequencers and players
- Thanks to the Rust community for amazing crates and documentation

## Changelog

### v0.2.0 (Current)
- Added comprehensive scan modes with progress reporting
- Implemented playlist and individual song looping
- Enhanced user interface with settings configuration
- Added configurable delays and timing options
- Fixed timing calculation bugs for accurate playback
- Improved track selection and BPM override functionality

### v0.1.0
- Initial release with basic MIDI playback
- Single song and random playback modes
- Basic track selection capabilities
