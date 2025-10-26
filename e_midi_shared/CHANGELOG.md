# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.6](https://github.com/davehorner/e_midi/compare/e_midi_shared-v0.1.5...e_midi_shared-v0.1.6) - 2025-10-26

### Added

- *(tidalcycles-rs)* add SuperDirt installer + harden Tidal boot

## [0.1.5](https://github.com/davehorner/e_midi/compare/e_midi_shared-v0.1.4...e_midi_shared-v0.1.5) - 2025-10-25

### Other

- update deps
- *(tidalcycles)* add TidalLooper auto-install and startup.scd integration  - Introduce `supercollider_looper.rs` for managing TidalLooper installation - Add logic to clone TidalLooper into SuperCollider user Extensions dir - Dynamically write `startup.scd` to load and initialize TidalLooper with SuperDirt - Update main workflow to print and embed the looper path into SuperCollider boot - Lockfile updated with `e_midi_shared 0.1.4` and registry metadata for `e_grid`

## [0.1.4](https://github.com/davehorner/e_midi/compare/e_midi_shared-v0.1.3...e_midi_shared-v0.1.4) - 2025-06-29

### Added

- *(sc3-plugins)* support for automated sc3-plugin installation and

## [0.1.3](https://github.com/davehorner/e_midi/compare/e_midi_shared-v0.1.2...e_midi_shared-v0.1.3) - 2025-06-27

### Added

- *(ipc)* add full zero-copy MIDI event IPC system using iceoryx2

## [0.1.2](https://github.com/davehorner/e_midi/compare/e_midi_shared-v0.1.1...e_midi_shared-v0.1.2) - 2025-06-26

### Added

- Add duration calculation and audio file support

## [0.1.1](https://github.com/davehorner/e_midi/compare/e_midi_shared-v0.1.0...e_midi_shared-v0.1.1) - 2025-06-26

### Added

- *(e_midi)* add embedded audio/video Ogg,Mp3,Mp4,Webm  support to song player

## [0.1.0](https://github.com/davehorner/e_midi/releases/tag/e_midi_shared-v0.1.0) - 2025-06-26

### Other

- *(musicxml)* multi-track improvements, add MusicXML support and shared song embedding logic  - Introduced `e_midi_shared` crate to encapsulate shared MIDI and MusicXML logic - Extended `build.rs` to parse and embed MusicXML alongside MIDI - Added `embed_musicxml.rs` and `embed_midi.rs` to extract timelines and metadata - Refactored `SongInfo`, `Note`, and related types into shared `types.rs` - Updated CLI and player logic to support MusicXML input and playback - Included support for extracting part metadata (instrument names, programs) from MusicXML  25/06/19|c80f083|0.1.4
