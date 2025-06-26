//! MusicXML embedding logic for build.rs and other tools
// Extracts MusicXML timelines and metadata for static embedding
// Update the path below if these types are defined elsewhere, e.g., in a module named 'model' or 'common'.
// use crate::model::{SongInfo, TrackInfo, Note, SongType};
// use crate::common::{SongInfo, TrackInfo, Note, SongType};
// If not present, create a types.rs file in src/ with the required definitions and add:
use musicxml::read_score_partwise;
use quick_xml::Reader;
use quick_xml::events::Event;
use std::collections::HashMap;
use std::fs;
use std::io::BufReader;
use std::path::Path;
use crate::types::{XmlTrackInfo, XmlSongInfo};

pub fn extract_musicxml_songs(xml_dir: &Path) -> Vec<XmlSongInfo> {
    let mut songs = Vec::new();
    if xml_dir.exists() {
        for entry in fs::read_dir(xml_dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            // Only process .xml files
            if path.extension().and_then(|s| s.to_str()) == Some("xml") {
                // Use quick-xml to extract part id -> (name, program) mapping
                let part_map = extract_part_list_mapping(&path);
                eprintln!("[DEBUG] MusicXML part mapping for {:?}:", path.file_name().unwrap());
                for (pid, (name, prog)) in &part_map {
                    eprintln!("  part_id: {}  name: '{}'  midi_program: {}", pid, name, prog);
                }
                let score = match read_score_partwise(path.to_str().unwrap()) {
                    Ok(s) => s,
                    Err(_) => continue,
                };
                let mut track_infos = Vec::new();
                let mut all_track_notes = Vec::new();
                let mut default_tempo = 120u32;
                let mut part_idx = 0;
                let mut ticks_per_q = 1u32;
                for part in &score.content.part {
                    // Get part id from attributes
                    let part_id = part.attributes.id.0.clone();
                    // Look up instrument name/program from mapping
                    let (instrument_name, program) = part_map.get(&part_id)
                        .map(|(n, p)| (n.clone(), *p))
                        .unwrap_or_else(|| (format!("Part{}", part_idx), 0));
                    eprintln!("[DEBUG] Assigning part_id '{}' -> '{}' (program {})", part_id, instrument_name, program);
                    let mut note_count = 0;
                    let mut min_pitch = 127u8;
                    let mut max_pitch = 0u8;
                    let mut sample_notes = Vec::new();
                    let mut timeline = Vec::new();
                    let mut current_time = 0u32;
                    let mut divisions = 1u32;
                    // Find divisions from the first measure with attributes
                    for elem in &part.content {
                        if let musicxml::elements::PartElement::Measure(measure) = elem {
                            for m_elem in &measure.content {
                                if let musicxml::elements::MeasureElement::Attributes(attr) = m_elem {
                                    if let Some(div) = &attr.content.divisions {
                                        divisions = div.content.0 as u32;
                                    }
                                }
                            }
                            break;
                        }
                    }
                    if part_idx == 0 {
                        ticks_per_q = divisions;
                    }
                    // Now process all measures and notes
                    for elem in &part.content {
                        if let musicxml::elements::PartElement::Measure(measure) = elem {
                            let mut local_time = current_time;
                            for m_elem in &measure.content {
                                if let musicxml::elements::MeasureElement::Note(note) = m_elem {
                                    if let musicxml::elements::NoteType::Normal(ref normal) = note.content.info {
                                        if let musicxml::elements::AudibleType::Pitch(ref pitch) = normal.audible {
                                            let step_val = step_to_midi(&pitch.content.step);
                                            let octave_val = pitch.content.octave.content.0 as u8;
                                            let alter_val = pitch.content.alter.as_ref().map(|a| a.content.0 as i8).unwrap_or(0);
                                            let midi_pitch = (step_val as i8 + alter_val + ((octave_val as i8 + 1) * 12)) as u8;
                                            let duration = normal.duration.content.0 as u32;
                                            let velocity = 64u8;
                                            // Use the MusicXML voice field if present, otherwise default to 1
                                            let voice = note.content.voice.as_ref()
                                                .and_then(|v| v.content.to_string().parse::<u8>().ok())
                                                .unwrap_or(1u8);
                                            timeline.push((local_time, duration, voice, midi_pitch, velocity));
                                            if midi_pitch < min_pitch { min_pitch = midi_pitch; }
                                            if midi_pitch > max_pitch { max_pitch = midi_pitch; }
                                            if sample_notes.len() < 5 { sample_notes.push(midi_pitch); }
                                            note_count += 1;
                                            local_time += duration;
                                        }
                                    }
                                }
                            }
                            current_time = local_time;
                        }
                    }
                    // Assign default channel 0 if none specified (MusicXML usually doesn't specify channels)
                    let mut channels = Vec::new();
                    // If you ever parse real channel info, push it here. For now, always push 0.
                    channels.push(0);
                    track_infos.push(XmlTrackInfo {
                        index: part_idx,
                        name: instrument_name,
                        note_count,
                        pitch_range: (min_pitch, max_pitch),
                        sample_notes,
                        program,
                        channels,
                    });
                    all_track_notes.push(timeline);
                    part_idx += 1;
                }
                let filename = path.file_name().unwrap().to_str().unwrap().to_string();
                let song_name = filename.replace(".xml", "").replace("_", " ");
                songs.push(XmlSongInfo {
                    filename,
                    name: song_name,
                    tracks: track_infos,
                    default_tempo,
                    ticks_per_q,
                    track_notes: all_track_notes,
                });
            }
        }
    }
    songs
}

// Helper for step to midi number
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

fn instrument_name_to_program(name: &str) -> u8 {
    // General MIDI mapping (add more as needed)
    match name.to_ascii_lowercase().as_str() {
        "acoustic grand piano" | "piano" => 0,
        "bright acoustic piano" => 1,
        "electric grand piano" => 2,
        "honky-tonk piano" => 3,
        "electric piano 1" | "rhodes" => 4,
        "electric piano 2" => 5,
        "harpsichord" => 6,
        "clavinet" => 7,
        "celesta" => 8,
        "glockenspiel" => 9,
        "music box" => 10,
        "vibraphone" => 11,
        "marimba" => 12,
        "xylophone" => 13,
        "tubular bells" => 14,
        "dulcimer" => 15,
        "drawbar organ" => 16,
        "percussive organ" => 17,
        "rock organ" => 18,
        "church organ" | "organ" => 19,
        "reed organ" => 20,
        "accordion" => 21,
        "harmonica" => 22,
        "tango accordion" => 23,
        "guitar" | "acoustic guitar" => 24,
        "electric guitar (jazz)" => 26,
        "electric guitar (clean)" => 27,
        "electric guitar (muted)" => 28,
        "overdriven guitar" => 29,
        "distortion guitar" => 30,
        "guitar harmonics" => 31,
        "acoustic bass" => 32,
        "electric bass (finger)" => 33,
        "electric bass (pick)" => 34,
        "fretless bass" => 35,
        "slap bass 1" => 36,
        "slap bass 2" => 37,
        "synth bass 1" => 38,
        "synth bass 2" => 39,
        "violin" => 40,
        "viola" => 41,
        "cello" | "violoncello" => 42,
        "contrabass" | "double bass" => 43,
        "tremolo strings" => 44,
        "pizzicato strings" => 45,
        "orchestral harp" => 46,
        "timpani" => 47,
        "string ensemble 1" => 48,
        "string ensemble 2" => 49,
        "synth strings 1" => 50,
        "synth strings 2" => 51,
        "choir aahs" => 52,
        "voice oohs" => 53,
        "synth voice" => 54,
        "orchestra hit" => 55,
        "trumpet" => 56,
        "trombone" => 57,
        "tuba" => 58,
        "muted trumpet" => 59,
        "french horn" | "horn" => 60,
        "brass section" => 61,
        "synth brass 1" => 62,
        "synth brass 2" => 63,
        "soprano sax" => 64,
        "alto sax" => 65,
        "tenor sax" => 66,
        "baritone sax" => 67,
        "oboe" => 68,
        "english horn" => 69,
        "bassoon" => 70,
        "clarinet" => 71,
        "piccolo" => 72,
        "flute" => 73,
        "recorder" => 74,
        "pan flute" => 75,
        "blown bottle" => 76,
        "shakuhachi" => 77,
        "whistle" => 78,
        "ocarina" => 79,
        "lead 1 (square)" => 80,
        "lead 2 (sawtooth)" => 81,
        "lead 3 (calliope)" => 82,
        "lead 4 (chiff)" => 83,
        "lead 5 (charang)" => 84,
        "lead 6 (voice)" => 85,
        "lead 7 (fifths)" => 86,
        "lead 8 (bass + lead)" => 87,
        "pad 1 (new age)" => 88,
        "pad 2 (warm)" => 89,
        "pad 3 (polysynth)" => 90,
        "pad 4 (choir)" => 91,
        "pad 5 (bowed)" => 92,
        "pad 6 (metallic)" => 93,
        "pad 7 (halo)" => 94,
        "pad 8 (sweep)" => 95,
        "fx 1 (rain)" => 96,
        "fx 2 (soundtrack)" => 97,
        "fx 3 (crystal)" => 98,
        "fx 4 (atmosphere)" => 99,
        "fx 5 (brightness)" => 100,
        "fx 6 (goblins)" => 101,
        "fx 7 (echoes)" => 102,
        "fx 8 (sci-fi)" => 103,
        "sitar" => 104,
        "banjo" => 105,
        "shamisen" => 106,
        "koto" => 107,
        "kalimba" => 108,
        "bag pipe" => 109,
        "fiddle" => 110,
        "shanai" => 111,
        "tinkle bell" => 112,
        "agogo" => 113,
        "steel drums" => 114,
        "woodblock" => 115,
        "taiko drum" => 116,
        "melodic tom" => 117,
        "synth drum" => 118,
        "reverse cymbal" => 119,
        "guitar fret noise" => 120,
        "breath noise" => 121,
        "seashore" => 122,
        "bird tweet" => 123,
        "telephone ring" => 124,
        "helicopter" => 125,
        "applause" => 126,
        "gunshot" => 127,
        // Add more mappings as needed
        _ => 0, // Default to Acoustic Grand Piano
    }
}

/// Extracts a mapping from part id to (instrument name, MIDI program) from the <part-list> section of a MusicXML file.
pub fn extract_part_list_mapping(xml_path: &Path) -> HashMap<String, (String, u8)> {
    let mut mapping = HashMap::new();
    let file = match std::fs::File::open(xml_path) {
        Ok(f) => f,
        Err(_) => return mapping,
    };
    let mut reader = Reader::from_reader(BufReader::new(file));
    let mut buf = Vec::new();
    let mut in_part_list = false;
    let mut in_score_part = false;
    let mut current_id = None;
    let mut current_name = None;
    let mut current_virtual_name = None;
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                match e.name().as_ref() {
                    b"part-list" => in_part_list = true,
                    b"score-part" if in_part_list => {
                        in_score_part = true;
                        current_id = e.attributes()
                            .filter_map(|a| a.ok())
                            .find(|a| a.key.as_ref() == b"id")
                            .and_then(|a| std::str::from_utf8(&a.value).ok().map(|s| s.to_string()));
                        current_name = None;
                        current_virtual_name = None;
                    }
                    b"part-name" if in_score_part => {
                        if let Ok(Event::Text(t)) = reader.read_event_into(&mut buf) {
                            let name = t.unescape().unwrap_or_default();
                            current_name = Some(name.trim().to_string());
                        }
                    }
                    b"virtual-name" if in_score_part => {
                        if let Ok(Event::Text(t)) = reader.read_event_into(&mut buf) {
                            let vname = t.unescape().unwrap_or_default();
                            current_virtual_name = Some(vname.trim().to_string());
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::End(ref e)) => {
                match e.name().as_ref() {
                    b"score-part" if in_score_part => {
                        if let Some(id) = current_id.take() {
                            // Prefer part-name if it maps to a known program, else try virtual-name
                            let mut name = current_name.take().unwrap_or_else(|| "Unknown".to_string());
                            let mut program = instrument_name_to_program(&name);
                            if program == 0 && name.to_ascii_lowercase() != "acoustic grand piano" {
                                if let Some(vname) = current_virtual_name.take() {
                                    let vprog = instrument_name_to_program(&vname);
                                    if vprog != 0 || vname.to_ascii_lowercase() == "flute" {
                                        name = vname;
                                        program = vprog;
                                    }
                                }
                            }
                            mapping.insert(id, (name, program));
                        }
                        in_score_part = false;
                    }
                    b"part-list" => break,
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            _ => {}
        }
        buf.clear();
    }
    mapping
}

// --- Conversion helper: XmlSongInfo -> SongInfo/Note (for main player integration) ---
// Types are available in the crate root; no import needed.

// All MusicXML logic moved to musicxml.rs

