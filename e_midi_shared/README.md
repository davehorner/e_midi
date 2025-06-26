# e_midi_shared

Shared types and MusicXML/MIDI logic for the e_midi project.

## Purpose
This crate provides all the common types, MusicXML parsing, and extraction logic used by both the main `e_midi` crate and its build scripts. It is not intended to be used directly by end users, but as an internal dependency for the workspace.

## Features
- Shared Rust types for songs, tracks, and notes (MIDI and MusicXML).
- MusicXML extraction and part mapping logic (for static and dynamic song support).
- Utilities for instrument name to MIDI program mapping.

## Usage
Add as a dependency in your workspace:

```
[dependencies]
e_midi_shared = { path = "../e_midi_shared" }
```

Or as a build-dependency:

```
[build-dependencies]
e_midi_shared = { path = "../e_midi_shared" }
```

## License
See the main workspace LICENSE file.
