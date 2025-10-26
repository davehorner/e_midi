//! Event definitions for IPC communication
//!
//! Events are designed to be lightweight, serializable, and suitable for
//! lock-free transmission between processes.

use crate::ipc::types::{generate_event_id, AppId, EventId};
use serde::{Deserialize, Serialize};
/// Base event trait for all IPC events
pub trait IpcEvent: Send + Sync {
    fn event_id(&self) -> EventId;
    fn timestamp(&self) -> u64;
    fn source_app(&self) -> AppId;
}
pub type IpcEventSender = std::sync::mpsc::Sender<Event>;
/// Core event types in the system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Event {
    /// Window management events
    WindowFocused {
        window_id: String,
        app_id: AppId,
        timestamp: u64,
    },
    WindowClosed {
        window_id: String,
        app_id: AppId,
        timestamp: u64,
    },
    WindowResized {
        window_id: String,
        size: (u32, u32),
        timestamp: u64,
    },
    /// MIDI command events (from TUI to player)
    MidiCommandPlay {
        song_index: usize,
        timestamp: u64,
    },
    MidiCommandStop {
        timestamp: u64,
    },
    MidiCommandPause {
        timestamp: u64,
    },
    MidiCommandResume {
        timestamp: u64,
    },
    MidiCommandNext {
        timestamp: u64,
    },
    MidiCommandPrevious {
        timestamp: u64,
    },
    MidiCommandSetTempo {
        new_tempo: u32,
        timestamp: u64,
    },
    MidiCommandSongListRequest {
        timestamp: u64,
    },

    /// MIDI status events (from player to TUI)
    MidiPlaybackStarted {
        song_index: usize,
        song_name: String,
        timestamp: u64,
    },
    MidiPlaybackStopped {
        timestamp: u64,
    },
    MidiPlaybackPaused {
        timestamp: u64,
    },
    MidiPlaybackResumed {
        timestamp: u64,
    },
    MidiTempoChanged {
        new_tempo: u32,
        timestamp: u64,
    },
    MidiSongChanged {
        song_index: usize,
        song_name: String,
        timestamp: u64,
    },
    MidiProgressUpdate {
        progress_ms: u32,
        total_ms: u32,
        timestamp: u64,
    },
    MidiSongListUpdated {
        song_count: usize,
        timestamp: u64,
    },

    /// Grid events (for future e_grid integration)
    GridCellSelected {
        grid_id: String,
        cell: (usize, usize),
        timestamp: u64,
    },
    GridCellUpdated {
        grid_id: String,
        cell: (usize, usize),
        value: String,
        timestamp: u64,
    },
    GridStateChanged {
        grid_id: String,
        timestamp: u64,
    },

    /// System events
    SystemShutdown {
        timestamp: u64,
    },
    SystemHeartbeat {
        app_id: AppId,
        timestamp: u64,
    },

    /// State synchronization events
    StateRequest {
        requesting_app: AppId,
        state_type: StateType,
        timestamp: u64,
    },
    StateResponse {
        state_type: StateType,
        data: Vec<u8>,
        timestamp: u64,
    },

    /// MIDI note events
    MidiNoteOn {
        channel: u8,
        pitch: u8,
        velocity: u8,
        timestamp: u64,
    },
    MidiNoteOff {
        channel: u8,
        pitch: u8,
        timestamp: u64,
    },
    MidiProgramChange {
        channel: u8,
        program: u8,
        timestamp: u64,
    },
}

/// Types of state that can be synchronized
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StateType {
    WindowStates,
    MidiPlayback,
    MidiSongList,
    GridState(String), // Grid ID
    AllStates,
}

impl Event {
    /// Get the event ID (based on timestamp for uniqueness)
    pub fn event_id(&self) -> EventId {
        self.timestamp()
    }

    /// Get the timestamp of the event
    pub fn timestamp(&self) -> u64 {
        match self {
            Event::WindowFocused { timestamp, .. } => *timestamp,
            Event::WindowClosed { timestamp, .. } => *timestamp,
            Event::WindowResized { timestamp, .. } => *timestamp,
            Event::MidiCommandPlay { timestamp, .. } => *timestamp,
            Event::MidiCommandStop { timestamp } => *timestamp,
            Event::MidiCommandPause { timestamp } => *timestamp,
            Event::MidiCommandResume { timestamp } => *timestamp,
            Event::MidiCommandNext { timestamp } => *timestamp,
            Event::MidiCommandPrevious { timestamp } => *timestamp,
            Event::MidiCommandSetTempo { timestamp, .. } => *timestamp,
            Event::MidiCommandSongListRequest { timestamp } => *timestamp,
            Event::MidiPlaybackStarted { timestamp, .. } => *timestamp,
            Event::MidiPlaybackStopped { timestamp } => *timestamp,
            Event::MidiPlaybackPaused { timestamp } => *timestamp,
            Event::MidiPlaybackResumed { timestamp } => *timestamp,
            Event::MidiTempoChanged { timestamp, .. } => *timestamp,
            Event::MidiSongChanged { timestamp, .. } => *timestamp,
            Event::MidiProgressUpdate { timestamp, .. } => *timestamp,
            Event::MidiSongListUpdated { timestamp, .. } => *timestamp,
            Event::GridCellSelected { timestamp, .. } => *timestamp,
            Event::GridCellUpdated { timestamp, .. } => *timestamp,
            Event::GridStateChanged { timestamp, .. } => *timestamp,
            Event::SystemShutdown { timestamp } => *timestamp,
            Event::SystemHeartbeat { timestamp, .. } => *timestamp,
            Event::StateRequest { timestamp, .. } => *timestamp,
            Event::StateResponse { timestamp, .. } => *timestamp,
            Event::MidiNoteOn { timestamp, .. } => *timestamp,
            Event::MidiNoteOff { timestamp, .. } => *timestamp,
            Event::MidiProgramChange { timestamp, .. } => *timestamp,
        }
    }
    /// Determine which app typically generates this event
    pub fn typical_source(&self) -> AppId {
        match self {
            Event::MidiCommandPlay { .. }
            | Event::MidiCommandStop { .. }
            | Event::MidiCommandPause { .. }
            | Event::MidiCommandResume { .. }
            | Event::MidiCommandNext { .. }
            | Event::MidiCommandPrevious { .. }
            | Event::MidiCommandSetTempo { .. }
            | Event::MidiCommandSongListRequest { .. } => AppId::EMidi, // TUI commands

            Event::MidiPlaybackStarted { .. }
            | Event::MidiPlaybackStopped { .. }
            | Event::MidiPlaybackPaused { .. }
            | Event::MidiPlaybackResumed { .. }
            | Event::MidiTempoChanged { .. }
            | Event::MidiSongChanged { .. }
            | Event::MidiProgressUpdate { .. }
            | Event::MidiSongListUpdated { .. } => AppId::EMidi, // Player status

            Event::GridCellSelected { .. }
            | Event::GridCellUpdated { .. }
            | Event::GridStateChanged { .. } => AppId::EGrid,

            Event::WindowFocused { app_id, .. } | Event::WindowClosed { app_id, .. } => *app_id,

            Event::StateRequest { requesting_app, .. } => *requesting_app,

            _ => AppId::Unknown,
        }
    }
}

/// Helper functions for creating events
impl Event {
    pub fn midi_playback_started(song_index: usize, song_name: String) -> Self {
        Event::MidiPlaybackStarted {
            song_index,
            song_name,
            timestamp: generate_event_id(),
        }
    }

    pub fn midi_playback_stopped() -> Self {
        Event::MidiPlaybackStopped {
            timestamp: generate_event_id(),
        }
    }

    pub fn midi_tempo_changed(new_tempo: u32) -> Self {
        Event::MidiTempoChanged {
            new_tempo,
            timestamp: generate_event_id(),
        }
    }

    pub fn midi_progress_update(progress_ms: u32, total_ms: u32) -> Self {
        Event::MidiProgressUpdate {
            progress_ms,
            total_ms,
            timestamp: generate_event_id(),
        }
    }

    pub fn system_heartbeat(app_id: AppId) -> Self {
        Event::SystemHeartbeat {
            app_id,
            timestamp: generate_event_id(),
        }
    }

    /// Helper functions to create MIDI command events
    pub fn midi_command_play(song_index: usize) -> Self {
        Self::MidiCommandPlay {
            song_index,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        }
    }

    pub fn midi_command_stop() -> Self {
        Self::MidiCommandStop {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        }
    }

    pub fn midi_command_next() -> Self {
        Self::MidiCommandNext {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        }
    }

    pub fn midi_command_previous() -> Self {
        Self::MidiCommandPrevious {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        }
    }

    pub fn midi_command_set_tempo(new_tempo: u32) -> Self {
        Self::MidiCommandSetTempo {
            new_tempo,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        }
    }
}
