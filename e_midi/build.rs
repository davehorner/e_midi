//! Build script for processing MIDI and MusicXML files at compile time
//!
//! This script:
//! - Scans for MIDI and MusicXML files in the project directory
//! - Parses MIDI data using the midly crate
//! - Generates Rust code with song information and playback functions
//! - Handles tempo calculations and note timing
//! - Creates a timeline-based playback system

use std::fs::File;
use std::path::Path;
use e_midi_shared::embed_musicxml;
use e_midi_shared::embed_midi;

fn main() {
    println!("cargo:rerun-if-changed=midi/");

    // Extract MIDI and MusicXML songs
    let midi_songs = embed_midi::extract_midi_songs(Path::new("midi"));
    let xml_songs = embed_musicxml::extract_musicxml_songs(Path::new("midi"));

    // Write static arrays for all songs (MIDI and XML)
    let mut song_data_entries = Vec::new();
    let mut song_info_entries = Vec::new();
    let mut song_idx = 0;
    // --- DO NOT CHECK IN src/embedded_midi.rs! It is generated. ---
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("embedded_midi.rs");
    let mut out = File::create(&dest_path).unwrap();
    
    // Only import SongData to avoid duplicate type imports in generated code
    writeln!(out, "use e_midi_shared::types::SongData;\n").unwrap();
    for song in &midi_songs {
        // Write track notes arrays as &[Note]
        let mut non_empty_track_count = 0;
        for track_notes in song.track_notes.iter() {
            if !track_notes.is_empty() {
                writeln!(out, "static SONG_{}_TRACK_{}_NOTES: &[crate::Note] = &[", song_idx, non_empty_track_count).unwrap();
                for (start_ticks, dur_ticks, chan, pitch, vel, track_idx) in track_notes {
                    writeln!(out, "    crate::Note {{ start_ms: {}, dur_ms: {}, chan: {}, pitch: {}, vel: {}, track: {} }},", start_ticks, dur_ticks, chan, pitch, vel, track_idx).unwrap();
                }
                writeln!(out, "];").unwrap();
                non_empty_track_count += 1;
            }
        }
        writeln!(out, "static SONG_{}_TRACK_NOTES: &[&[crate::Note]] = &[", song_idx).unwrap();
        for track_idx in 0..non_empty_track_count {
            writeln!(out, "    SONG_{}_TRACK_{}_NOTES,", song_idx, track_idx).unwrap();
        }
        writeln!(out, "]\n").unwrap();
        writeln!(out, ";").unwrap();
        song_data_entries.push(format!(
            "SongData {{ track_notes: SONG_{}_TRACK_NOTES, ticks_per_q: {}, default_tempo: {}, filename: \"{}\", name: \"{}\" }}",
            song_idx, song.ticks_per_q, song.default_tempo, song.filename, song.name
        ));
        // TrackInfo for MIDI
        let tracks = song.tracks.iter().map(|t| {
            let guess_str = match &t.guess {
                Some(s) if !s.trim().is_empty() && s.trim() != "-" => format!("Some({:?}.to_string())", s),
                None => {
                    // Fallback: guess by channel if no program/guess
                    if let Some(ch) = t.channels.first() {
                        let guess = match ch {
                            0 => "Piano",
                            1 => "Strings",
                            2 => "Guitar",
                            9 => "Percussion",
                            _ => "Unknown",
                        };
                        if guess == "Unknown" {
                            "None".to_string()
                        } else {
                            format!("Some(\"{}\".to_string())", guess)
                        }
                    } else {
                        "None".to_string()
                    }
                }
                _ => "None".to_string(),
            };
            let channels_vec = format!("vec!{:?}", t.channels);
            format!(
                "TrackInfo {{ index: {}, program: {:?}, guess: {}, channels: {}, note_count: {}, pitch_range: ({}, {}), sample_notes: vec!{:?} }}",
                t.index, t.program, guess_str, channels_vec, t.note_count, t.pitch_range.0, t.pitch_range.1, t.sample_notes)
        }).collect::<Vec<_>>().join(",");
        // Build track_index_map for this song
        let mut track_index_map = String::from("{ let mut m = std::collections::HashMap::new(); ");
        for (dense_idx, t) in song.tracks.iter().enumerate() {
            track_index_map.push_str(&format!("m.insert({}, {}); ", t.index, dense_idx));
        }
        track_index_map.push_str("m }");
        song_info_entries.push(format!(
            "SongInfo {{ filename: \"{}\".to_string(), name: \"{}\".to_string(), tracks: vec![{}], default_tempo: {}, ticks_per_q: Some({}), song_type: SongType::Midi, source: SongSource::None, track_index_map: {} }}",
            song.filename, song.name, tracks, song.default_tempo, song.ticks_per_q, track_index_map));
        song_idx += 1;
    }
    for song in &xml_songs {
        // Debug output for MusicXML part mapping and assignment
        eprintln!("[BUILD DEBUG] MusicXML file: {}", song.filename);
        for track in &song.tracks {
            eprintln!("  part_idx: {}  name: '{}'  midi_program: {}  note_count: {}", track.index, track.name, track.program, track.note_count);
        }

        // Write track notes arrays as &[Note]
        let mut non_empty_track_count = 0;
        for track_notes in song.track_notes.iter() {
            if !track_notes.is_empty() {
                writeln!(out, "static SONG_{}_TRACK_{}_NOTES: &[crate::Note] = &[", song_idx, non_empty_track_count).unwrap();
                for (start_ticks, dur_ticks, voice, pitch, vel) in track_notes {
                    writeln!(out, "    crate::Note {{ start_ms: {}, dur_ms: {}, chan: {}, pitch: {}, vel: {}, track: {} }},", start_ticks, dur_ticks, voice, pitch, vel, non_empty_track_count).unwrap();
                }
                writeln!(out, "];").unwrap();
                non_empty_track_count += 1;
            }
        }
        writeln!(out, "static SONG_{}_TRACK_NOTES: &[&[crate::Note]] = &[", song_idx).unwrap();
        for track_idx in 0..non_empty_track_count {
            writeln!(out, "    SONG_{}_TRACK_{}_NOTES,", song_idx, track_idx).unwrap();
        }
        writeln!(out, "]\n").unwrap();
        writeln!(out, ";").unwrap();
        song_data_entries.push(format!(
            "SongData {{ track_notes: SONG_{}_TRACK_NOTES, ticks_per_q: {}, default_tempo: {}, filename: \"{}\", name: \"{}\" }}",
            song_idx, song.ticks_per_q, song.default_tempo, song.filename, song.name
        ));
        // TrackInfo for XML (use only available fields)
        let tracks = song.tracks.iter().map(|t| {
            let guess_str = format!("Some({:?}.to_string())", t.name);
            let program_val = format!("Some({})", t.program);
            format!(
                "TrackInfo {{ index: {}, program: {}, guess: {}, channels: vec![], note_count: {}, pitch_range: ({}, {}), sample_notes: vec!{:?} }}",
                t.index, program_val, guess_str, t.note_count, t.pitch_range.0, t.pitch_range.1, t.sample_notes)
        }).collect::<Vec<_>>().join(",");
        // Build track_index_map for this song (MusicXML)
        let mut track_index_map = String::from("{");
        for (dense_idx, t) in song.tracks.iter().enumerate() {
            track_index_map.push_str(&format!("let mut m = std::collections::HashMap::new(); m.insert({}, {});", t.index, dense_idx));
        }
        track_index_map.push_str("m}");
        song_info_entries.push(format!(
            "SongInfo {{ filename: \"{}\".to_string(), name: \"{}\".to_string(), tracks: vec![{}], default_tempo: {}, ticks_per_q: Some({}), song_type: SongType::MusicXml, source: SongSource::None, track_index_map: {} }}",
            song.filename, song.name, tracks, song.default_tempo, song.ticks_per_q, track_index_map));
        song_idx += 1;
    }
    // --- Add support for OGG, MP3, MP4, and .url (YouTube) files ---
    use std::fs;
    let midi_dir = Path::new("midi");
    let crate_root = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    for entry in fs::read_dir(midi_dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path().canonicalize().unwrap();
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
        let fname = path.file_name().and_then(|f| f.to_str()).unwrap_or("");
        match ext.as_str() {
            "ogg" => {
                let var_name = format!("SONG_{}_OGG_DATA", song_idx);
                let rel_path = path.strip_prefix(&crate_root).unwrap_or(&path);
                let rel_path_str = rel_path.to_str().unwrap().replace("\\", "/");
                writeln!(out, "static {}: &[u8] = include_bytes!(\"{}\");", var_name, rel_path_str).unwrap();
                song_info_entries.push(format!(
                    "SongInfo {{ filename: \"{}\".to_string(), name: \"{}\".to_string(), tracks: vec![], default_tempo: 0, ticks_per_q: None, song_type: SongType::Ogg, source: SongSource::EmbeddedOgg({}), track_index_map: std::collections::HashMap::new() }}",
                    fname, fname, var_name
                ));
                song_idx += 1;
            }
            "mp3" => {
                let var_name = format!("SONG_{}_MP3_DATA", song_idx);
                let rel_path = path.strip_prefix(&crate_root).unwrap_or(&path);
                let rel_path_str = rel_path.to_str().unwrap().replace("\\", "/");
                writeln!(out, "static {}: &[u8] = include_bytes!(\"{}\");", var_name, rel_path_str).unwrap();
                song_info_entries.push(format!(
                    "SongInfo {{ filename: \"{}\".to_string(), name: \"{}\".to_string(), tracks: vec![], default_tempo: 0, ticks_per_q: None, song_type: SongType::Mp3, source: SongSource::EmbeddedMp3({}), track_index_map: std::collections::HashMap::new() }}",
                    fname, fname, var_name
                ));
                song_idx += 1;
            }
            "mp4" => {
                let var_name = format!("SONG_{}_MP4_DATA", song_idx);
                let rel_path = path.strip_prefix(&crate_root).unwrap_or(&path);
                let rel_path_str = rel_path.to_str().unwrap().replace("\\", "/");
                writeln!(out, "static {}: &[u8] = include_bytes!(\"{}\");", var_name, rel_path_str).unwrap();
                song_info_entries.push(format!(
                    "SongInfo {{ filename: \"{}\".to_string(), name: \"{}\".to_string(), tracks: vec![], default_tempo: 0, ticks_per_q: None, song_type: SongType::Mp4, source: SongSource::EmbeddedMp4({}), track_index_map: std::collections::HashMap::new() }}",
                    fname, fname, var_name
                ));
                song_idx += 1;
            }
            "url" => {
                // Parse the .url file for a YouTube link or ID
                if let Ok(url_text) = fs::read_to_string(&path) {
                    let video_id = url_text.trim()
                        .rsplit('/')
                        .next()
                        .and_then(|s| s.split('?').next())
                        .unwrap_or("");
                    song_info_entries.push(format!(
                        "SongInfo {{ filename: \"{}\".to_string(), name: \"{}\".to_string(), tracks: vec![], default_tempo: 0, ticks_per_q: None, song_type: SongType::YouTube, source: SongSource::YouTube {{ video_id: \"{}\", start: None, end: None }}, track_index_map: std::collections::HashMap::new() }}",
                        fname, fname, video_id
                    ));
                    song_idx += 1;
                }
            }
            "webm" => {
                let var_name = format!("SONG_{}_WEBM_DATA", song_idx);
                let rel_path = path.strip_prefix(&crate_root).unwrap_or(&path);
                let rel_path_str = rel_path.to_str().unwrap().replace("\\", "/");
                writeln!(out, "static {}: &[u8] = include_bytes!(\"{}\");", var_name, rel_path_str).unwrap();
                song_info_entries.push(format!(
                    "SongInfo {{ filename: \"{}\".to_string(), name: \"{}\".to_string(), tracks: vec![], default_tempo: 0, ticks_per_q: None, song_type: SongType::Webm, source: SongSource::EmbeddedWebm({}), track_index_map: std::collections::HashMap::new() }}",
                    fname, fname, var_name
                ));
                song_idx += 1;
            }
            _ => {}
        }
    }
    // Write SongData static array
    // REMOVE struct SongData generation (it's defined in lib.rs)
    writeln!(out, "static SONG_DATA: &[SongData] = &[" ).unwrap();
    for entry in &song_data_entries {
        writeln!(out, "    {},", entry).unwrap();
    }
    writeln!(out, "];").unwrap();
    // Write get_songs()
    writeln!(out, "\npub fn get_songs() -> Vec<SongInfo> {{").unwrap();
    writeln!(out, "    vec![").unwrap();
    for entry in &song_info_entries {
        writeln!(out, "        {},", entry).unwrap();
    }
    writeln!(out, "    ]").unwrap();
    writeln!(out, "}}\n").unwrap();

    // Write get_events_for_song_tracks (real implementation)
    writeln!(out, "pub fn get_events_for_song_tracks(song_index: usize, track_indices: &[usize], tempo_bpm: u32) -> Vec<crate::Note> {{").unwrap();
    writeln!(out, "    if song_index >= SONG_DATA.len() {{").unwrap();
    writeln!(out, "        eprintln!(\"[e_midi] Invalid song_index {{}} in get_events_for_song_tracks (max is {{}})\", song_index, SONG_DATA.len().saturating_sub(1));").unwrap();
    writeln!(out, "        return Vec::new();").unwrap();
    writeln!(out, "    }}").unwrap();
    writeln!(out, "    let song_data = &SONG_DATA[song_index];").unwrap();
    writeln!(out, "    let ticks_per_q = song_data.ticks_per_q;").unwrap();
    writeln!(out, "    let mut events = Vec::new();").unwrap();
    writeln!(out, "    let track_notes = song_data.track_notes;").unwrap();
    writeln!(out, "    let mut debug_count = 0;").unwrap();
    writeln!(out, "    let tempo_usec_per_q = 60_000_000 / tempo_bpm;").unwrap();
    writeln!(out, "    for &track_idx in track_indices {{").unwrap();
    writeln!(out, "        if let Some(track) = track_notes.get(track_idx) {{").unwrap();
    writeln!(out, "            for note in (*track).iter() {{").unwrap();
    writeln!(out, "                let start_ms = ((note.start_ms as f64) * 60_000.0 / (tempo_bpm as f64 * ticks_per_q as f64)) as u32;").unwrap();
    writeln!(out, "                let mut dur_ms = ((note.dur_ms as f64) * 60_000.0 / (tempo_bpm as f64 * ticks_per_q as f64)) as u32;").unwrap();
    writeln!(out, "                dur_ms = dur_ms.max(50);").unwrap();
    // writeln!(out, "                if debug_count < 10 {{").unwrap();
    // writeln!(out, "                    println!(\"[DEBUG] song={{}} track={{}} start_ticks={{}} dur_ticks={{}} ticks_per_q={{}} tempo_bpm={{}} => start_ms={{}} dur_ms={{}} pitch={{}}\", song_index, track_idx, note.start_ms, note.dur_ms, ticks_per_q, tempo_bpm, start_ms, dur_ms, note.pitch);").unwrap();
    // writeln!(out, "                    debug_count += 1;").unwrap();
    // writeln!(out, "                }}").unwrap();
    writeln!(out, "                events.push(crate::Note {{ start_ms, dur_ms, chan: note.chan, pitch: note.pitch, vel: note.vel, track: note.track }});").unwrap();
    writeln!(out, "            }}").unwrap();
    writeln!(out, "        }}").unwrap();
    writeln!(out, "    }}").unwrap();
    writeln!(out, "    events.sort_by_key(|n| n.start_ms);").unwrap();
    writeln!(out, "    events").unwrap();
    writeln!(out, "}}\n").unwrap();

    // Write play_embedded_audio_bytes function for OGG/MP3/MP4
    writeln!(out, "/// Returns the embedded audio bytes for a static song index (OGG/MP3/MP4)\npub fn get_embedded_audio_bytes(song_index: usize, _song_type: &SongType) -> Option<&'static [u8]> {{
    match song_index {{
").unwrap();
    let mut audio_idx = 0;
    for entry in &song_info_entries {
        // Only match OGG/MP3/MP4/WebM
        if entry.contains("SongType::Ogg") {
            writeln!(out, "        {} => Some(SONG_{}_OGG_DATA),", audio_idx, audio_idx).unwrap();
        } else if entry.contains("SongType::Mp3") {
            writeln!(out, "        {} => Some(SONG_{}_MP3_DATA),", audio_idx, audio_idx).unwrap();
        } else if entry.contains("SongType::Mp4") {
            writeln!(out, "        {} => Some(SONG_{}_MP4_DATA),", audio_idx, audio_idx).unwrap();
        } else if entry.contains("SongType::Webm") {
            writeln!(out, "        {} => Some(SONG_{}_WEBM_DATA),", audio_idx, audio_idx).unwrap();
        }
        audio_idx += 1;
    }
    writeln!(out, "        _ => None,").unwrap();
    writeln!(out, "    }}\n}}\n").unwrap();

    use std::fs::OpenOptions;
    use std::io::Write;
    let debug_path = std::env::var("CARGO_TARGET_DIR").unwrap_or_else(|_| "target".to_string());
    let debug_file_path = Path::new(&debug_path).join("musicxml_part_debug.txt");
    if let Some(parent_dir) = debug_file_path.parent() {
    std::fs::create_dir_all(parent_dir).unwrap();
    }
    // Write debug info to file
    let mut debug_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&debug_file_path)
        .unwrap();
    for song in &xml_songs {
        writeln!(debug_file, "[BUILD DEBUG] MusicXML file: {}", song.filename).unwrap();
        for track in &song.tracks {
            writeln!(debug_file, "  part_idx: {}  name: '{}'  midi_program: {}  note_count: {}", track.index, track.name, track.program, track.note_count).unwrap();
        }
        debug_file.flush().unwrap();
    }
}
