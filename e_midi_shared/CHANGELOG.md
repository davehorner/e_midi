# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0](https://github.com/davehorner/e_midi/releases/tag/e_midi_shared-v0.1.0) - 2025-06-26

### Other

- *(musicxml)* multi-track improvements, add MusicXML support and shared song embedding logic  - Introduced `e_midi_shared` crate to encapsulate shared MIDI and MusicXML logic - Extended `build.rs` to parse and embed MusicXML alongside MIDI - Added `embed_musicxml.rs` and `embed_midi.rs` to extract timelines and metadata - Refactored `SongInfo`, `Note`, and related types into shared `types.rs` - Updated CLI and player logic to support MusicXML input and playback - Included support for extracting part metadata (instrument names, programs) from MusicXML  25/06/19|c80f083|0.1.4  25/06/19|01418ae|0.1.4
