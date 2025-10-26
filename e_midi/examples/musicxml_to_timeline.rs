// Example: Scan a directory for MusicXML files and print timeline info like build.rs does for MIDI
// Usage: cargo run --example musicxml_to_timeline -- [directory]

use musicxml::*;
use std::env;
use std::fs::read_dir;
use std::io::Write;
use std::path::Path;

#[derive(Debug)]
pub struct NoteEvent {
    pub start_time: u32, // in divisions or ms (depending on context)
    pub duration: u32,   // in divisions or ms
    pub pitch: u8,       // MIDI pitch
    pub velocity: u8,    // MIDI velocity (default 64)
    pub voice: u8,       // Voice/track
}

#[derive(Debug, Clone)]
pub struct TrackInfo {
    pub index: usize,
    pub guess: Option<String>,
    pub note_count: usize,
    pub pitch_range: (u8, u8),
    pub sample_notes: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct SongInfo {
    pub filename: String,
    pub name: String,
    pub tracks: Vec<TrackInfo>,
    pub default_tempo: u32,
}

fn process_musicxml_file(path: &Path) -> Option<SongInfo> {
    let score = match read_score_partwise(&path.to_string_lossy()) {
        Ok(s) => s,
        Err(_) => return None,
    };
    let mut tracks = Vec::new();
    let default_tempo = 120u32; // MusicXML may not always specify tempo
    for (part_idx, part) in score.content.part.iter().enumerate() {
        let mut note_count = 0;
        let mut min_pitch = 127u8;
        let mut max_pitch = 0u8;
        let mut sample_notes = Vec::new();
        let mut timeline = Vec::new();
        let mut current_time = 0u32;
        let mut _divisions = 1u32; // Default to 1 if not found
                                   // Find divisions from the first measure with attributes
        for elem in &part.content {
            if let musicxml::elements::PartElement::Measure(measure) = elem {
                for m_elem in &measure.content {
                    if let musicxml::elements::MeasureElement::Attributes(attr) = m_elem {
                        if let Some(div) = &attr.content.divisions {
                            _divisions = div.content.0;
                        }
                    }
                }
                break;
            }
        }
        // Now process all measures and notes
        for elem in &part.content {
            if let musicxml::elements::PartElement::Measure(measure) = elem {
                let mut local_time = current_time;
                for m_elem in &measure.content {
                    if let musicxml::elements::MeasureElement::Note(note) = m_elem {
                        if let musicxml::elements::NoteType::Normal(ref normal) = note.content.info
                        {
                            if let musicxml::elements::AudibleType::Pitch(ref pitch) =
                                normal.audible
                            {
                                let step_val = step_to_midi(&pitch.content.step);
                                let octave_val = pitch.content.octave.content.0;
                                let alter_val = pitch
                                    .content
                                    .alter
                                    .as_ref()
                                    .map(|a| a.content.0 as i8)
                                    .unwrap_or(0);
                                let midi_pitch =
                                    (step_val as i8 + alter_val + (octave_val as i8) * 12) as u8;
                                let duration = normal.duration.content.0;
                                let velocity = 64u8; // MusicXML rarely encodes velocity
                                let voice = 1u8; // Default to 1, as NormalInfo does not have a voice field
                                timeline.push(NoteEvent {
                                    start_time: local_time,
                                    duration,
                                    pitch: midi_pitch,
                                    velocity,
                                    voice,
                                });
                                if midi_pitch < min_pitch {
                                    min_pitch = midi_pitch;
                                }
                                if midi_pitch > max_pitch {
                                    max_pitch = midi_pitch;
                                }
                                if sample_notes.len() < 5 {
                                    sample_notes.push(midi_pitch);
                                }
                                note_count += 1;
                                local_time += duration;
                            }
                        }
                    }
                }
                current_time = local_time;
            }
        }
        println!("      Timeline for Track {}:", part_idx);
        println!(
            "pub const SONG_{}_TRACK_{}: &[NoteEvent] = &[",
            part_idx, part_idx
        );
        for ev in &timeline {
            println!(
                "    NoteEvent {{ start_time: {}, duration: {}, pitch: {}, velocity: {}, voice: {} }},",
                ev.start_time, ev.duration, ev.pitch, ev.velocity, ev.voice
            );
        }
        println!("];");
        tracks.push(TrackInfo {
            index: part_idx,
            guess: None, // Instrument guessing from MusicXML is nontrivial
            note_count,
            pitch_range: (min_pitch, max_pitch),
            sample_notes,
        });
    }
    Some(SongInfo {
        filename: path.file_name().unwrap().to_string_lossy().to_string(),
        name: path.file_stem().unwrap().to_string_lossy().to_string(),
        tracks,
        default_tempo,
    })
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let dir = if args.len() > 1 { &args[1] } else { "midi" };
    let dir_path = Path::new(dir);
    if !dir_path.exists() {
        eprintln!("Directory '{}' does not exist", dir);
        std::process::exit(1);
    }
    let mut xml_files = Vec::new();
    for entry in read_dir(dir_path).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
            if ext.eq_ignore_ascii_case("xml") || ext.eq_ignore_ascii_case("musicxml") {
                xml_files.push(path);
            }
        }
    }
    if xml_files.is_empty() {
        println!("No MusicXML files found in {}", dir);
        return;
    }
    // Write to OUT_DIR/generated_musicxml.rs or fallback to ./generated_musicxml.rs
    let out_path =
        std::env::var("OUT_DIR").unwrap_or_else(|_| ".".to_string()) + "/generated_musicxml.rs";
    let mut out = std::fs::File::create(&out_path).expect("Failed to create output file");
    writeln!(
        out,
        "// Auto-generated by musicxml_to_timeline.rs\nuse crate::NoteEvent;\n"
    )
    .unwrap();
    let mut all_arrays = Vec::new();
    for (idx, path) in xml_files.iter().enumerate() {
        if let Some(song) = process_musicxml_file(path) {
            for (track_idx, _track) in song.tracks.iter().enumerate() {
                let array_name = format!("SONG_{}_TRACK_{}", idx, track_idx);
                writeln!(out, "pub const {}: &[NoteEvent] = &[", array_name).unwrap();
                // Re-parse timeline for this track
                let score = read_score_partwise(&path.to_string_lossy()).unwrap();
                let part = &score.content.part[track_idx];
                let mut current_time = 0u32;
                for elem in &part.content {
                    if let musicxml::elements::PartElement::Measure(measure) = elem {
                        let mut local_time = current_time;
                        for m_elem in &measure.content {
                            if let musicxml::elements::MeasureElement::Note(note) = m_elem {
                                if let musicxml::elements::NoteType::Normal(ref normal) =
                                    note.content.info
                                {
                                    if let musicxml::elements::AudibleType::Pitch(ref pitch) =
                                        normal.audible
                                    {
                                        let step_val = step_to_midi(&pitch.content.step);
                                        let octave_val = pitch.content.octave.content.0;
                                        let alter_val = pitch
                                            .content
                                            .alter
                                            .as_ref()
                                            .map(|a| a.content.0 as i8)
                                            .unwrap_or(0);
                                        let midi_pitch =
                                            (step_val as i8 + alter_val + (octave_val as i8) * 12)
                                                as u8;
                                        let duration = normal.duration.content.0;
                                        let velocity = 64u8;
                                        let voice = 1u8;
                                        writeln!(out, "    NoteEvent {{ start_time: {}, duration: {}, pitch: {}, velocity: {}, voice: {} }},", local_time, duration, midi_pitch, velocity, voice).unwrap();
                                        local_time += duration;
                                    }
                                }
                            }
                        }
                        current_time = local_time;
                    }
                }
                writeln!(out, "];").unwrap();
                all_arrays.push(array_name);
            }
        }
    }
    // Write a static list of all arrays
    writeln!(out, "\npub const ALL_MUSICXML_SONGS: &[&[NoteEvent]] = &[").unwrap();
    for name in &all_arrays {
        writeln!(out, "    {},", name).unwrap();
    }
    writeln!(out, "];").unwrap();
    println!("Wrote generated Rust arrays to {}", out_path);
}

fn step_to_midi(step: &dyn std::fmt::Debug) -> u8 {
    let s = format!("{:?}", step);
    match s.as_str() {
        "C" => 0,
        "D" => 2,
        "E" => 4,
        "F" => 5,
        "G" => 7,
        "A" => 9,
        "B" => 11,
        _ => 0,
    }
}
