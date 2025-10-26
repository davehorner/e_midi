//! MusicSyncPublisher: Publishes PlaySongAtHeartbeat messages via iceoryx2
//!
//! Provides lock-free, zero-copy publishing of music sync messages for synchronized playback

use iceoryx2::port::publisher::Publisher;
use iceoryx2::prelude::*;
// use iceoryx2::service::ipc::Service;

use crate::ipc_protocol::PlaySongAtHeartbeat;

use super::music_sync_subscriber::E_MIDI_MUSIC_SYNC_SERVICE;

#[derive(Debug)]
pub struct MusicSyncPublisher {
    publisher: Publisher<ipc::Service, PlaySongAtHeartbeat, ()>,
}

impl MusicSyncPublisher {
    /// Create a new music sync publisher
    pub fn new() -> Result<Self, String> {
        let node = NodeBuilder::new()
            .create::<ipc::Service>()
            .map_err(|e| format!("Node creation failed: {e:?}"))?;
        let service = node
            .service_builder(
                &ServiceName::new(E_MIDI_MUSIC_SYNC_SERVICE)
                    .map_err(|e| format!("Invalid service name: {e:?}"))?,
            )
            .publish_subscribe::<PlaySongAtHeartbeat>()
            .open_or_create()
            .map_err(|e| format!("Failed to create/open service: {e:?}"))?;
        let publisher = service
            .publisher_builder()
            .create()
            .map_err(|e| format!("Failed to create publisher: {e:?}"))?;
        Ok(Self { publisher })
    }

    /// Publish a PlaySongAtHeartbeat message (zero-copy)
    pub fn publish(&mut self, msg: &PlaySongAtHeartbeat) -> Result<(), String> {
        self.publisher
            .send_copy(*msg)
            .map(|_| ())
            .map_err(|e| format!("Failed to publish: {e:?}"))
    }
}

unsafe impl Send for MusicSyncPublisher {}
unsafe impl Sync for MusicSyncPublisher {}
