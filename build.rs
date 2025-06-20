//! Build script for processing MIDI files at compile time
//!
//! This script:
//! - Scans for MIDI files in the project directory
//! - Parses MIDI data using the midly crate
//! - Generates Rust code with song information and playback functions
//! - Handles tempo calculations and note timing
//! - Creates a timeline-based playback system

use midly::{Smf, TrackEventKind};
use std::env;
use std::fs::{read_dir, File};
use std::io::Write;
use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=midi/");

    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("midi_data.rs");
    let mut out = File::create(&dest_path).unwrap();

    // Scan midi directory for .mid files
    let midi_dir = Path::new("midi");
    let mut midi_file_paths = Vec::new();

    if midi_dir.exists() {
        for entry in read_dir(midi_dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("mid") {
                midi_file_paths.push(path);
            }
        }
    }

    if midi_file_paths.is_empty() {
        panic!("No MIDI files found in midi/ directory!");
    }

    // Generate structures
    writeln!(out, "#[derive(Debug, Clone)]").unwrap();
    writeln!(out, "pub struct TrackInfo {{").unwrap();
    writeln!(out, "    pub index: usize,").unwrap();
    writeln!(out, "    pub program: Option<u8>,").unwrap();
    writeln!(out, "    pub guess: Option<String>,").unwrap();
    writeln!(out, "    pub channels: Vec<u8>,").unwrap();
    writeln!(out, "    pub note_count: usize,").unwrap();
    writeln!(out, "    pub pitch_range: (u8, u8),").unwrap();
    writeln!(out, "    pub sample_notes: Vec<u8>,").unwrap();
    writeln!(out, "}}").unwrap();

    writeln!(out, "\n#[derive(Debug, Clone)]").unwrap();
    writeln!(out, "pub struct SongInfo {{").unwrap();
    writeln!(out, "    pub filename: String,").unwrap();
    writeln!(out, "    pub name: String,").unwrap();
    writeln!(out, "    pub tracks: Vec<TrackInfo>,").unwrap();
    writeln!(out, "    pub default_tempo: u32,").unwrap();
    writeln!(out, "}}").unwrap();

    writeln!(out, "\npub struct SongData {{").unwrap();
    writeln!(
        out,
        "    pub track_notes: &'static [&'static [(u32, u32, u8, u8, u8)]],"
    )
    .unwrap();
    writeln!(out, "    pub ticks_per_q: u32,").unwrap();
    writeln!(out, "}}").unwrap();

    // First pass: Generate all static data arrays
    for (song_idx, path) in midi_file_paths.iter().enumerate() {
        let midi_bytes = std::fs::read(path).unwrap();
        let smf = Smf::parse(&midi_bytes).unwrap();

        let ticks_per_q = match smf.header.timing {
            midly::Timing::Metrical(t) => t.as_int() as u32,
            _ => 480,
        };

        // Pre-parse all notes from all tracks for this song
        let mut all_track_notes = Vec::new();
        for track in smf.tracks.iter() {
            let mut track_notes = Vec::new();
            let mut note_ons: std::collections::HashMap<(u8, u8), u32> =
                std::collections::HashMap::new();
            let mut abs_time = 0u32;

            for ev in track.iter() {
                abs_time = abs_time.wrapping_add(ev.delta.as_int() as u32);
                if let TrackEventKind::Midi { channel, message } = ev.kind {
                    match message {
                        midly::MidiMessage::NoteOn { key, vel } if vel > 0 => {
                            let time_ticks = abs_time;
                            note_ons.insert((channel.as_int(), key.as_int()), time_ticks);
                        }
                        midly::MidiMessage::NoteOff { key, .. }
                        | midly::MidiMessage::NoteOn { key, .. } => {
                            let time_ticks = abs_time;
                            if let Some(start_ticks) =
                                note_ons.remove(&(channel.as_int(), key.as_int()))
                            {
                                let duration_ticks = time_ticks.saturating_sub(start_ticks);
                                track_notes.push((
                                    start_ticks,
                                    duration_ticks.max(ticks_per_q / 8),
                                    channel.as_int(),
                                    key.as_int(),
                                    64u8,
                                ));
                            }
                        }
                        _ => {}
                    }
                }
            }
            all_track_notes.push(track_notes);
        } // Generate static data for this song - only include tracks with notes
        writeln!(
            out,
            "\n// Song {}: {}",
            song_idx,
            path.file_name().unwrap().to_str().unwrap()
        )
        .unwrap();
        let mut non_empty_track_count = 0;
        for (_track_idx, track_notes) in all_track_notes.iter().enumerate() {
            if !track_notes.is_empty() {
                writeln!(
                    out,
                    "static SONG_{}_TRACK_{}_NOTES: &[(u32, u32, u8, u8, u8)] = &[",
                    song_idx, non_empty_track_count
                )
                .unwrap();
                for (start_ticks, dur_ticks, chan, pitch, vel) in track_notes {
                    writeln!(
                        out,
                        "    ({}, {}, {}, {}, {}),",
                        start_ticks, dur_ticks, chan, pitch, vel
                    )
                    .unwrap();
                }
                writeln!(out, "];").unwrap();
                non_empty_track_count += 1;
            }
        }

        writeln!(
            out,
            "static SONG_{}_TRACK_NOTES: &[&[(u32, u32, u8, u8, u8)]] = &[",
            song_idx
        )
        .unwrap();
        for track_idx in 0..non_empty_track_count {
            writeln!(out, "    SONG_{}_TRACK_{}_NOTES,", song_idx, track_idx).unwrap();
        }
        writeln!(out, "];").unwrap();
        writeln!(
            out,
            "static SONG_{}_TICKS_PER_Q: u32 = {};",
            song_idx, ticks_per_q
        )
        .unwrap();
    }

    // Generate song data lookup array
    writeln!(out, "\nstatic SONG_DATA: &[SongData] = &[").unwrap();
    for song_idx in 0..midi_file_paths.len() {
        writeln!(out, "    SongData {{").unwrap();
        writeln!(out, "        track_notes: SONG_{}_TRACK_NOTES,", song_idx).unwrap();
        writeln!(out, "        ticks_per_q: SONG_{}_TICKS_PER_Q,", song_idx).unwrap();
        writeln!(out, "    }},").unwrap();
    }
    writeln!(out, "];").unwrap();

    // Generate song info function
    writeln!(out, "\npub fn get_songs() -> Vec<SongInfo> {{").unwrap();
    writeln!(out, "    vec![").unwrap();
    for (_song_idx, path) in midi_file_paths.iter().enumerate() {
        let filename = path.file_name().unwrap().to_str().unwrap().to_string();
        let song_name = filename.replace(".mid", "").replace("_", " ");
        let midi_bytes = std::fs::read(path).unwrap();
        let smf = Smf::parse(&midi_bytes).unwrap();

        // Extract tempo from MIDI file (default 120 BPM if not found)
        let mut default_tempo = 500000u32; // microseconds per quarter note (120 BPM)

        // Look for tempo changes in all tracks (usually in track 0)
        for track in &smf.tracks {
            for event in track.iter() {
                if let TrackEventKind::Meta(midly::MetaMessage::Tempo(tempo)) = event.kind {
                    default_tempo = tempo.as_int();
                    break; // Use first tempo found
                }
            }
            if default_tempo != 500000 {
                break;
            } // Found tempo, stop searching
        }

        writeln!(out, "        SongInfo {{").unwrap();
        writeln!(out, "            filename: \"{}\".to_string(),", filename).unwrap();
        writeln!(out, "            name: \"{}\".to_string(),", song_name).unwrap();
        writeln!(
            out,
            "            default_tempo: {},",
            60000000 / default_tempo
        )
        .unwrap(); // BPM
        writeln!(out, "            tracks: vec![").unwrap();

        // Analyze tracks
        for (i, track) in smf.tracks.iter().enumerate() {
            let mut channels = vec![];
            let mut note_count = 0;
            let mut note_pitches = vec![];
            let mut program = None;
            let mut min_pitch = u8::MAX;
            let mut max_pitch = u8::MIN;

            for event in track.iter() {
                match event.kind {
                    TrackEventKind::Midi { channel, message } => {
                        if !channels.contains(&channel.as_int()) {
                            channels.push(channel.as_int());
                        }
                        match message {
                            midly::MidiMessage::NoteOn { key, vel } if vel > 0 => {
                                note_count += 1;
                                note_pitches.push(key.as_int());
                                if key.as_int() < min_pitch {
                                    min_pitch = key.as_int();
                                }
                                if key.as_int() > max_pitch {
                                    max_pitch = key.as_int();
                                }
                            }
                            midly::MidiMessage::ProgramChange { program: p } => {
                                program = Some(p.as_int());
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }

            if note_count > 0 {
                let role = match program {
                    Some(0..=8) => Some("Piano"),
                    Some(9..=16) => Some("Chromatic Percussion"),
                    Some(17..=24) => Some("Organ"),
                    Some(25..=32) => Some("Guitar"),
                    Some(33..=40) => Some("Bass"),
                    Some(41..=48) => Some("Strings"),
                    Some(49..=56) => Some("Ensemble"),
                    Some(57..=64) => Some("Brass"),
                    Some(65..=72) => Some("Reed"),
                    Some(73..=80) => Some("Pipe"),
                    Some(81..=88) => Some("Synth Lead"),
                    Some(89..=96) => Some("Synth Pad"),
                    Some(97..=104) => Some("Synth Effects"),
                    Some(105..=112) => Some("Ethnic"),
                    Some(113..=120) => Some("Percussive"),
                    Some(121..=128) => Some("Sound Effects"),
                    _ => None,
                };

                let sample_notes: Vec<u8> = note_pitches.iter().take(5).copied().collect();

                writeln!(out, "                TrackInfo {{").unwrap();
                writeln!(out, "                    index: {},", i).unwrap();
                writeln!(out, "                    program: {:?},", program).unwrap();
                writeln!(
                    out,
                    "                    guess: {},",
                    if let Some(r) = role {
                        format!("Some(\"{}\".to_string())", r)
                    } else {
                        "None".to_string()
                    }
                )
                .unwrap();
                writeln!(out, "                    channels: vec!{:?},", channels).unwrap();
                writeln!(out, "                    note_count: {},", note_count).unwrap();
                writeln!(
                    out,
                    "                    pitch_range: ({}, {}),",
                    min_pitch, max_pitch
                )
                .unwrap();
                writeln!(
                    out,
                    "                    sample_notes: vec!{:?},",
                    sample_notes
                )
                .unwrap();
                writeln!(out, "                }},").unwrap();
            }
        }

        writeln!(out, "            ],").unwrap();
        writeln!(out, "        }},").unwrap();
    }

    writeln!(out, "    ]").unwrap();
    writeln!(out, "}}").unwrap(); // Generate function to get events for specific song and tracks
    writeln!(out, "\npub fn get_events_for_song_tracks(song_index: usize, track_indices: &[usize], tempo_bpm: u32) -> Vec<crate::Note> {{").unwrap();
    writeln!(out, "    let mut notes = Vec::new();").unwrap();
    writeln!(out, "    if song_index < SONG_DATA.len() {{").unwrap();
    writeln!(out, "        let song_data = &SONG_DATA[song_index];").unwrap();
    writeln!(out, "        let ticks_per_q = song_data.ticks_per_q;").unwrap();
    writeln!(
        out,
        "        let ms_per_tick = 60000.0 / (tempo_bpm as f32 * ticks_per_q as f32);"
    )
    .unwrap();
    writeln!(out, "        let songs = get_songs();").unwrap();
    writeln!(out, "        let song_info = &songs[song_index];").unwrap();
    writeln!(out, "        for &midi_track_index in track_indices {{").unwrap();
    writeln!(
        out,
        "            // Find the position of this MIDI track in the tracks vector"
    )
    .unwrap();
    writeln!(out, "            if let Some(array_position) = song_info.tracks.iter().position(|t| t.index == midi_track_index) {{").unwrap();
    writeln!(
        out,
        "                if array_position < song_data.track_notes.len() {{"
    )
    .unwrap();
    writeln!(out, "                    for &(start_ticks, dur_ticks, chan, pitch, vel) in song_data.track_notes[array_position] {{").unwrap();
    writeln!(out, "                        notes.push(crate::Note {{").unwrap();
    writeln!(
        out,
        "                            start_ms: (start_ticks as f32 * ms_per_tick) as u32,"
    )
    .unwrap();
    writeln!(
        out,
        "                            dur_ms: ((dur_ticks as f32 * ms_per_tick) as u32).max(50),"
    )
    .unwrap();
    writeln!(out, "                            chan,").unwrap();
    writeln!(out, "                            pitch,").unwrap();
    writeln!(out, "                            vel,").unwrap();
    writeln!(out, "                        }});").unwrap();
    writeln!(out, "                    }}").unwrap();
    writeln!(out, "                }}").unwrap();
    writeln!(out, "            }}").unwrap();
    writeln!(out, "        }}").unwrap();
    writeln!(out, "    }}").unwrap();
    writeln!(out, "    notes.sort_by_key(|n| n.start_ms);").unwrap();
    writeln!(out, "    notes").unwrap();
    writeln!(out, "}}").unwrap();
}
