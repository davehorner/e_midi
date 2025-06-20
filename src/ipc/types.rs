//! Type definitions for IPC communication
//! 
//! All types used in inter-process communication must be serializable
//! and designed for lock-free, zero-copy transmission.

use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// Unique identifier for events
pub type EventId = u64;

/// Generate a unique event ID based on timestamp and sequence
pub fn generate_event_id() -> EventId {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64
}

/// Application identifier in the e_* ecosystem
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AppId {
    EMidi,
    EGrid,
    StateServer,
    Unknown,
}

impl Default for AppId {
    fn default() -> Self {
        Self::Unknown
    }
}

/// Window state information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowState {
    pub app_id: AppId,
    pub window_id: String,
    pub title: String,
    pub focused: bool,
    pub visible: bool,
    pub position: (i32, i32),
    pub size: (u32, u32),
    pub timestamp: u64,
}

impl Default for WindowState {
    fn default() -> Self {
        Self {
            app_id: AppId::default(),
            window_id: String::new(),
            title: String::new(),
            focused: false,
            visible: true,
            position: (0, 0),
            size: (800, 600),
            timestamp: generate_event_id(),
        }
    }
}

/// MIDI playback state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MidiPlaybackState {
    pub is_playing: bool,
    pub current_song_index: Option<usize>,
    pub current_song_name: String,
    pub progress_ms: u32,
    pub total_duration_ms: u32,
    pub tempo_bpm: u32,
    pub volume: f32,
    pub timestamp: u64,
}

impl Default for MidiPlaybackState {
    fn default() -> Self {
        Self {
            is_playing: false,
            current_song_index: None,
            current_song_name: String::new(),
            progress_ms: 0,
            total_duration_ms: 0,
            tempo_bpm: 120,
            volume: 1.0,
            timestamp: generate_event_id(),
        }
    }
}

/// MIDI song information for IPC
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcSongInfo {
    pub index: usize,
    pub name: String,
    pub filename: String,
    pub track_count: usize,
    pub default_tempo: u32,
    pub duration_ms: Option<u32>,
    pub is_dynamic: bool,
}

/// Grid state information (for future e_grid integration)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GridState {
    pub grid_id: String,
    pub cells: Vec<Vec<String>>,
    pub selected_cell: Option<(usize, usize)>,
    pub timestamp: u64,
}

impl Default for GridState {
    fn default() -> Self {
        Self {
            grid_id: String::new(),
            cells: Vec::new(),
            selected_cell: None,
            timestamp: generate_event_id(),
        }
    }
}

// Use a fixed-size array for zero-copy IPC payloads
pub const MAX_PAYLOAD_SIZE: usize = 4096;
pub type IpcPayload = [u8; MAX_PAYLOAD_SIZE];

pub type IpcResult<T> = Result<T, IpcError>;

#[derive(Debug, Clone)]
pub enum IpcError {
    NodeCreation(String),
    ServiceCreation(String),
    PublisherCreation(String),
    SubscriberCreation(String),
    SendError(String),
    ReceiveError(String),
    SerializationError(String),
    DeserializationError(String),
    PayloadTooLarge(String),
}

/// Convert serializable data to IPC payload
pub fn serialize_to_payload<T: serde::Serialize>(data: &T) -> IpcResult<IpcPayload> {
    let json_bytes = serde_json::to_vec(data)
        .map_err(|e| IpcError::SerializationError(format!("Serialization failed: {:?}", e)))?;
    
    if json_bytes.len() > MAX_PAYLOAD_SIZE {
        return Err(IpcError::PayloadTooLarge(format!(
            "Payload size {} exceeds maximum {}", 
            json_bytes.len(), 
            MAX_PAYLOAD_SIZE
        )));
    }
    
    let mut payload = [0u8; MAX_PAYLOAD_SIZE];
    payload[..json_bytes.len()].copy_from_slice(&json_bytes);
    Ok(payload)
}

/// Extract size from IPC payload and deserialize
pub fn deserialize_from_payload<T: serde::de::DeserializeOwned>(
    payload: &IpcPayload,
    size: usize,
) -> IpcResult<T> {
    if size > MAX_PAYLOAD_SIZE {
        return Err(IpcError::DeserializationError(format!(
            "Payload size {} exceeds maximum {}", 
            size, 
            MAX_PAYLOAD_SIZE
        )));
    }
    
    let json_bytes = &payload[..size];
    serde_json::from_slice(json_bytes)
        .map_err(|e| IpcError::DeserializationError(format!("Deserialization failed: {:?}", e)))
}
