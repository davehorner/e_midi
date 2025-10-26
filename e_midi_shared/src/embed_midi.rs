//! MIDI embedding logic for build.rs and other tools
// Extracts MIDI timelines and metadata for static embedding

use midly::{Smf, TrackEventKind};
use std::fs;
use std::path::Path;

pub struct MidiTrackInfo {
    pub index: usize,
    pub program: Option<u8>,
    pub guess: Option<String>,
    pub channels: Vec<u8>,
    pub note_count: usize,
    pub pitch_range: (u8, u8),
    pub sample_notes: Vec<u8>,
}

pub struct MidiSongInfo {
    pub filename: String,
    pub name: String,
    pub tracks: Vec<MidiTrackInfo>,
    pub default_tempo: u32,
    pub ticks_per_q: u32,
    #[allow(clippy::type_complexity)]
    pub track_notes: Vec<Vec<(u32, u32, u8, u8, u8, usize)>>, // Add track index to tuple
}

pub fn extract_midi_songs(midi_dir: &Path) -> Vec<MidiSongInfo> {
    let mut songs = Vec::new();
    if midi_dir.exists() {
        for entry in fs::read_dir(midi_dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("mid") {
                let midi_bytes = fs::read(&path).unwrap();
                let smf = Smf::parse(&midi_bytes).unwrap();
                let ticks_per_q = match smf.header.timing {
                    midly::Timing::Metrical(t) => t.as_int() as u32,
                    other => {
                        println!("[MIDI LOAD][WARN] {}: Non-metrical timing in MIDI header: {:?} (using 96 ticks_per_q fallback)", path.display(), other);
                        96
                    }
                };
                // Print all tempo events
                for (track_idx, track) in smf.tracks.iter().enumerate() {
                    let mut abs_time = 0u32;
                    for event in track.iter() {
                        abs_time = abs_time.wrapping_add(event.delta.as_int());
                        if let TrackEventKind::Meta(midly::MetaMessage::Tempo(tempo)) = event.kind {
                            let bpm = 60000000 / tempo.as_int();
                            println!("[MIDI LOAD][TEMPO] {}: track {} tick {}: tempo meta-event = {} us/qn ({} BPM)", path.display(), track_idx, abs_time, tempo.as_int(), bpm);
                        }
                    }
                }
                let mut all_track_notes = Vec::new();
                let mut track_infos = Vec::new();
                for (i, track) in smf.tracks.iter().enumerate() {
                    let mut track_notes = Vec::new();
                    let mut note_ons = std::collections::HashMap::new();
                    let mut abs_time = 0u32;
                    let mut channels = vec![];
                    let mut note_count = 0;
                    let mut note_pitches = vec![];
                    let mut program = None;
                    let mut min_pitch = u8::MAX;
                    let mut max_pitch = u8::MIN;
                    for ev in track.iter() {
                        abs_time = abs_time.wrapping_add(ev.delta.as_int());
                        if let TrackEventKind::Midi { channel, message } = ev.kind {
                            if !channels.contains(&channel.as_int()) {
                                channels.push(channel.as_int());
                            }
                            match message {
                                midly::MidiMessage::NoteOn { key, vel } if vel > 0 => {
                                    let time_ticks = abs_time;
                                    note_ons.insert((channel.as_int(), key.as_int()), time_ticks);
                                    note_count += 1;
                                    note_pitches.push(key.as_int());
                                    if key.as_int() < min_pitch {
                                        min_pitch = key.as_int();
                                    }
                                    if key.as_int() > max_pitch {
                                        max_pitch = key.as_int();
                                    }
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
                                            i, // track index
                                        ));
                                    }
                                }
                                midly::MidiMessage::ProgramChange { program: p } => {
                                    program = Some(p.as_int());
                                }
                                _ => {}
                            }
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
                        track_infos.push(MidiTrackInfo {
                            index: i,
                            program,
                            guess: role.map(|r| r.to_string()),
                            channels,
                            note_count,
                            pitch_range: (min_pitch, max_pitch),
                            sample_notes,
                        });
                    }
                    all_track_notes.push(track_notes);
                }
                // Extract tempo
                let mut default_tempo = 500000u32;
                for track in &smf.tracks {
                    for event in track.iter() {
                        if let TrackEventKind::Meta(midly::MetaMessage::Tempo(tempo)) = event.kind {
                            default_tempo = tempo.as_int();
                            break;
                        }
                    }
                    if default_tempo != 500000 {
                        break;
                    }
                }
                let filename = path.file_name().unwrap().to_str().unwrap().to_string();
                let song_name = filename.replace(".mid", "").replace("_", " ");
                let bpm = 60000000 / default_tempo;
                println!("[MIDI LOAD] {}: detected tempo meta-event = {} us/qn ({} BPM), ticks_per_q = {}", filename, default_tempo, bpm, ticks_per_q);
                songs.push(MidiSongInfo {
                    filename,
                    name: song_name,
                    tracks: track_infos,
                    default_tempo: bpm,
                    ticks_per_q,
                    track_notes: all_track_notes,
                });
            }
        }
    }
    songs
}
