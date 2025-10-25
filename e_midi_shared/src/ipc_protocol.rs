//! C-compatible, zero-copy IPC protocol definitions for e_midi
//!
//! This module defines the PlaySongAtHeartbeat and TrackVoiceOverride structs
//! for robust, synchronized, multi-client MIDI playback.
//!
//! All structs are #[repr(C)] and use only fixed-size, ABI-stable types.
use iceoryx2::prelude::*;

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, ZeroCopySend)]
pub struct MidiNoteEvent {
    pub channel: u8,
    pub pitch: u8,
    pub velocity: u8, // 0 for NoteOff
    pub kind: u8,     // 0 = NoteOn, 1 = NoteOff
    pub timestamp: u64,
    pub _reserved: [u8; 4], // for alignment
}
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, ZeroCopySend)]
pub struct TrackVoiceOverride {
    /// Track index (0-based)
    pub track_index: u8,
    /// MIDI voice/program number (0-127)
    pub voice: u8,
    /// Reserved for alignment/future use
    pub _reserved: [u8; 2],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, ZeroCopySend)]
pub struct PlaySongAtHeartbeat {
    /// Song index to play
    pub song_index: u16,
    /// Start at this heartbeat (global sync)
    pub start_heartbeat: u32,
    /// Stop at this heartbeat (optional, 0 = ignore)
    pub stop_heartbeat: u32,
    /// Play for this many ms (optional, 0 = ignore)
    pub play_for_duration_ms: u32,
    /// Number of track overrides in the array below
    pub num_track_overrides: u8,
    /// Reserved for alignment/future use
    pub _reserved: [u8; 3],
    /// Per-track voice overrides (max 16 tracks)
    pub track_overrides: [TrackVoiceOverride; 16],
}

impl Default for TrackVoiceOverride {
    fn default() -> Self {
        Self {
            track_index: 0,
            voice: 0,
            _reserved: [0; 2],
        }
    }
}

impl Default for PlaySongAtHeartbeat {
    fn default() -> Self {
        Self {
            song_index: 0,
            start_heartbeat: 0,
            stop_heartbeat: 0,
            play_for_duration_ms: 0,
            num_track_overrides: 0,
            _reserved: [0; 3],
            track_overrides: [TrackVoiceOverride::default(); 16],
        }
    }
}

/// SAFETY: These helpers allow zero-copy conversion between the struct and a byte array.
/// The struct must be #[repr(C)] and contain no pointers or references.
impl PlaySongAtHeartbeat {
    pub const BYTE_SIZE: usize = std::mem::size_of::<PlaySongAtHeartbeat>();

    /// Convert the struct to a byte array (for IPC transmission)
    pub fn as_bytes(&self) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts(
                (self as *const PlaySongAtHeartbeat) as *const u8,
                PlaySongAtHeartbeat::BYTE_SIZE,
            )
        }
    }

    /// Construct from a byte array (received from IPC)
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() != PlaySongAtHeartbeat::BYTE_SIZE {
            return None;
        }
        let mut s = PlaySongAtHeartbeat::default();
        unsafe {
            std::ptr::copy_nonoverlapping(
                bytes.as_ptr(),
                &mut s as *mut PlaySongAtHeartbeat as *mut u8,
                PlaySongAtHeartbeat::BYTE_SIZE,
            );
        }
        Some(s)
    }
}

impl TrackVoiceOverride {
    pub const BYTE_SIZE: usize = std::mem::size_of::<TrackVoiceOverride>();
    pub fn as_bytes(&self) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts(
                (self as *const TrackVoiceOverride) as *const u8,
                TrackVoiceOverride::BYTE_SIZE,
            )
        }
    }
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() != TrackVoiceOverride::BYTE_SIZE {
            return None;
        }
        let mut s = TrackVoiceOverride::default();
        unsafe {
            std::ptr::copy_nonoverlapping(
                bytes.as_ptr(),
                &mut s as *mut TrackVoiceOverride as *mut u8,
                TrackVoiceOverride::BYTE_SIZE,
            );
        }
        Some(s)
    }
}
