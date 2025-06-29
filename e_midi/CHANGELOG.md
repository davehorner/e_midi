# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.10](https://github.com/davehorner/e_midi/compare/e_midi-v0.1.9...e_midi-v0.1.10) - 2025-06-29

### Other

- update Cargo.lock dependencies

## [0.1.9](https://github.com/davehorner/e_midi/compare/e_midi-v0.1.8...e_midi-v0.1.9) - 2025-06-29

### Added

- *(sc3-plugins)* support for automated sc3-plugin installation and

## [0.1.8](https://github.com/davehorner/e_midi/compare/e_midi-v0.1.7...e_midi-v0.1.8) - 2025-06-28

### Added

- Added initial integration with [`tidalcycles-rs`](https://github.com/davehorner/e_midi/tree/develop/tidalcycles-rs) for pattern-based MIDI sequencing and experimental live coding support. This enables advanced rhythmic and melodic pattern playback alongside standard MIDI features.

## [0.1.7](https://github.com/davehorner/e_midi/compare/e_midi-v0.1.6...e_midi-v0.1.7) - 2025-06-27

### Added

- *(ipc)* add full zero-copy MIDI event IPC system using iceoryx2

## [0.1.6](https://github.com/davehorner/e_midi/compare/e_midi-v0.1.5...e_midi-v0.1.6) - 2025-06-26

### Added

- Add duration calculation and audio file support

## [0.1.5](https://github.com/davehorner/e_midi/compare/e_midi-v0.1.4...e_midi-v0.1.5) - 2025-06-26

### Added

- *(e_midi)* add embedded audio/video Ogg,Mp3,Mp4,Webm  support to song player

## [0.1.4](https://github.com/davehorner/e_midi/compare/v0.1.3...v0.1.4) - 2025-06-26

### Added

- *(musicxml)* multi-track improvements, add MusicXML support and shared song embedding logic

### Other

- update dependencies in Cargo.toml and Cargo.lock to latest versions

## [0.1.3](https://github.com/davehorner/e_midi/compare/v0.1.2...v0.1.3) - 2025-06-24

### Added

- *(midi)* add resume-aware background playback and lock-free queue integration

### Other

- play_resune and new player.get_command_sender() midi_sender.send(e_midi::MidiCommand::PlaySongResumeAware
- song_resume

## [0.1.2](https://github.com/davehorner/e_midi/compare/v0.1.1...v0.1.2) - 2025-06-22

### Added

- *(demo)* add Windows-only focus/resize/move demo and new MIDI track

## [0.1.1](https://github.com/davehorner/e_midi/compare/v0.1.0...v0.1.1) - 2025-06-22

### Added

- *(integration)* add e_grid dependency and window focus MIDI playback

### Planned
- **Export Functionality**: Save processed MIDI data to files
- **Plugin System**: Support for audio effects and filters
- **MIDI Recording**: Record and save MIDI input
- **GUI Interface**: Optional graphical user interface
- **Network Streaming**: Stream MIDI over network protocols
- **Multi-file Support**: Process multiple MIDI files simultaneously

## [0.1.0] - 2025-06-19

### Added
- **Comprehensive Scan Modes**: Sequential, random start, and progressive scanning
- **Advanced Looping**: Both playlist and individual song looping with user control
- **Interactive Settings Menu**: Configure loop and delay settings at startup
- **Progress Reporting**: Real-time progress display with timestamps and percentages
- **Configurable Delays**: Set custom delays between songs (including zero delay)
- **Enhanced Track Selection**: Improved default handling for track and BPM selection
- **Duration Calculation**: Accurate song duration calculation and formatting
- **User Control**: Press 'q' + Enter to quit loops during playback

### Fixed
- **Timing Calculation Bug**: Fixed ms_per_tick truncation for high ticks_per_q values
- **Track Mapping**: Corrected track selection and mapping logic
- **Default Handling**: Proper defaults for empty track and BPM input

### Changed
- **Menu System**: Restructured main menu with clearer options
- **Code Organization**: Refactored playback functions to use LoopConfig
- **User Interface**: Improved prompts and progress indicators
- **Error Handling**: Enhanced error messages and input validation

### Removed
- **Unused Code**: Cleaned up legacy code and compiler warnings
- **Debug Output**: Removed excessive debug printing

### Added
- **Basic MIDI Playback**: Core functionality for playing MIDI files
- **Track Selection**: Choose specific MIDI tracks to play
- **Random Playback**: Random song selection mode
- **BPM Override**: Override default tempo with custom BPM
- **Build-time Processing**: Automatic MIDI file processing and code generation
- **Cross-platform Support**: Works on Windows, macOS, and Linux

### Technical Details
- Uses `midir` for MIDI I/O operations
- Uses `midly` and `rimd` for MIDI file parsing
- Implements timeline-based playback system
- Supports standard MIDI files (SMF)

