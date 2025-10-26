#[derive(Debug, Clone)]
pub struct TrackInfo {
    pub index: usize,
    pub program: Option<u8>,
    pub guess: Option<String>,
    pub channels: Vec<u8>,
    pub note_count: usize,
    pub pitch_range: (u8, u8),
    pub sample_notes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SongType {
    Midi,
    MusicXml,
    Ogg,
    Mp3,
    Mp4,
    Webm,
    YouTube,
    TidalCycles, // New variant for .tidal files
    Other,
}

#[derive(Debug, Clone)]
pub enum SongSource {
    EmbeddedMidi(&'static [u8]),
    EmbeddedOgg(&'static [u8]),
    EmbeddedMp3(&'static [u8]),
    EmbeddedMp4(&'static [u8]),
    EmbeddedWebm(&'static [u8]),
    YouTube {
        video_id: &'static str,
        start: Option<u64>,
        end: Option<u64>,
    },
    EmbeddedTidalCycles(&'static str), // New variant for embedded .tidal files
    FilePath(&'static str),
    None,
}

impl SongSource {
    pub fn url(&self) -> Option<String> {
        match self {
            SongSource::YouTube { video_id, .. } => {
                Some(format!("https://www.youtube.com/watch?v={}", video_id))
            }
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SongInfo {
    pub filename: String,
    pub name: String,
    pub tracks: Vec<TrackInfo>,
    pub default_tempo: u32,
    pub ticks_per_q: Option<u32>,
    pub song_type: SongType,
    pub source: SongSource,
    pub track_index_map: std::collections::HashMap<usize, usize>, // user index -> dense index
    pub duration_ms: Option<u32>, // Duration in milliseconds, if known
}

#[derive(Clone, Debug)]
pub struct Note {
    pub start_ms: u32,
    pub dur_ms: u32,
    pub chan: u8,
    pub pitch: u8,
    pub vel: u8,
    pub track: u8, // New field for track index
}

#[derive(Debug, Clone)]
pub struct SongData {
    pub track_notes: &'static [&'static [Note]],
    pub ticks_per_q: u32,
    pub default_tempo: u32,
    pub filename: &'static str,
    pub name: &'static str,
}

#[derive(Debug, Clone)]
pub struct XmlTrackInfo {
    pub index: usize,
    pub name: String,
    pub note_count: usize,
    pub pitch_range: (u8, u8),
    pub sample_notes: Vec<u8>,
    pub program: u8,
    pub channels: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct XmlSongInfo {
    pub filename: String,
    pub name: String,
    pub tracks: Vec<XmlTrackInfo>,
    #[allow(clippy::type_complexity)]
    pub track_notes: Vec<Vec<(u32, u32, u8, u8, u8)>>,
    pub default_tempo: u32,
    pub ticks_per_q: u32,
}
