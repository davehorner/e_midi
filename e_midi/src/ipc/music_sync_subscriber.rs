//! MusicSyncSubscriber: Subscribes to PlaySongAtHeartbeat messages via iceoryx2
//!
//! Provides lock-free, zero-copy subscription to music sync messages for synchronized playback

use iceoryx2::port::subscriber::Subscriber;
use iceoryx2::prelude::*;
use iceoryx2::service::ipc::Service;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use e_midi_shared::ipc_protocol::PlaySongAtHeartbeat;

pub const E_MIDI_MUSIC_SYNC_SERVICE: &str = "e_midi_music_sync";

#[derive(Debug)]
pub struct MusicSyncSubscriber {
    subscriber: Subscriber<Service, PlaySongAtHeartbeat, ()>,
    is_active: Arc<AtomicBool>,
    last_heartbeat: Instant,
}

impl MusicSyncSubscriber {
    /// Create a new music sync subscriber
    pub fn new() -> Result<Self, String> {
        let node = NodeBuilder::new()
            .create::<Service>()
            .map_err(|e| format!("Node creation failed: {e:?}"))?;
        let service = node
            .service_builder(&ServiceName::new(E_MIDI_MUSIC_SYNC_SERVICE).map_err(|e| format!("Invalid service name: {e:?}"))?)
            .publish_subscribe::<PlaySongAtHeartbeat>()
            .open_or_create()
            .map_err(|e| format!("Failed to create/open service: {e:?}"))?;
        let subscriber = service
            .subscriber_builder()
            .create()
            .map_err(|e| format!("Failed to create subscriber: {e:?}"))?;
        Ok(Self {
            subscriber,
            is_active: Arc::new(AtomicBool::new(true)),
            last_heartbeat: Instant::now(),
        })
    }

    /// Try to receive PlaySongAtHeartbeat messages (non-blocking)
    pub fn try_receive(&mut self) -> Result<Vec<PlaySongAtHeartbeat>, String> {
        if !self.is_active.load(Ordering::Relaxed) {
            return Ok(Vec::new());
        }
        let mut messages = Vec::new();
        while let Some(sample) = self.subscriber.receive().map_err(|e| format!("Receive error: {e:?}"))? {
            messages.push(*sample);
        }
        if !messages.is_empty() {
            self.last_heartbeat = Instant::now();
        }
        Ok(messages)
    }

    /// Deactivate subscriber
    pub fn deactivate(&self) {
        self.is_active.store(false, Ordering::Relaxed);
    }

    /// Get time since last received message
    pub fn time_since_last_message(&self) -> Duration {
        self.last_heartbeat.elapsed()
    }
}

unsafe impl Send for MusicSyncSubscriber {}
unsafe impl Sync for MusicSyncSubscriber {}
