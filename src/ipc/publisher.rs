//! Publisher module for sending events via iceoryx2
//!
//! Provides lock-free, zero-copy publishing of events to subscribers

use iceoryx2::port::publisher::Publisher;
use iceoryx2::prelude::*;
use iceoryx2::service::ipc::Service;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use super::{serialize_to_payload, AppId, Event, IpcError, IpcPayload, IpcResult};

/// Lock-free event publisher
pub struct EventPublisher {
    publisher: Publisher<Service, IpcPayload, ()>,
    app_id: AppId,
    is_active: Arc<AtomicBool>,
}

impl EventPublisher {
    /// Create a new event publisher for the specified app
    pub fn new(app_id: AppId) -> IpcResult<Self> {
        let service_name = format!("e_ecosystem_events_{:?}", app_id).to_lowercase();

        // Create node (suppress debug output)
        let node = NodeBuilder::new()
            .create::<Service>()
            .map_err(|_| IpcError::NodeCreation(format!("Node creation failed")))?;

        let service = node
            .service_builder(
                &ServiceName::new(&service_name)
                    .map_err(|_| IpcError::ServiceCreation(format!("Invalid service name")))?,
            )
            .publish_subscribe::<IpcPayload>()
            .open_or_create()
            .map_err(|_| IpcError::ServiceCreation(format!("Failed to create service")))?;

        let publisher = service
            .publisher_builder()
            .create()
            .map_err(|_| IpcError::PublisherCreation(format!("Failed to create publisher")))?;

        Ok(Self {
            publisher,
            app_id,
            is_active: Arc::new(AtomicBool::new(true)),
        })
    }
    /// Publish an event (lock-free, zero-copy when possible)
    pub fn publish(&self, event: Event) -> IpcResult<()> {
        if !self.is_active.load(Ordering::Relaxed) {
            return Err(IpcError::SendError("Publisher is not active".to_string()));
        }

        // Serialize event to payload
        let payload = serialize_to_payload(&event)?;
        // Send via iceoryx2 using send_copy for simplicity
        self.publisher
            .send_copy(payload)
            .map_err(|_| IpcError::SendError(format!("Failed to send event")))?;

        Ok(())
    }
    /// Publish multiple events in batch (more efficient)
    pub fn publish_batch(&self, events: Vec<Event>) -> IpcResult<()> {
        if !self.is_active.load(Ordering::Relaxed) {
            return Err(IpcError::SendError("Publisher is not active".to_string()));
        }

        // Serialize all events to a single payload
        let payload = serialize_to_payload(&events)?;
        self.publisher
            .send_copy(payload)
            .map_err(|_| IpcError::SendError(format!("Failed to send batch")))?;

        Ok(())
    }

    /// Check if the publisher is active
    pub fn is_active(&self) -> bool {
        self.is_active.load(Ordering::Relaxed)
    }

    /// Deactivate the publisher (graceful shutdown)
    pub fn deactivate(&self) {
        self.is_active.store(false, Ordering::Relaxed);
    }

    /// Get the app ID this publisher represents
    pub fn app_id(&self) -> AppId {
        self.app_id
    }
}

impl Drop for EventPublisher {
    fn drop(&mut self) {
        self.deactivate();
    }
}

// Publisher is Send + Sync for lock-free usage across threads
unsafe impl Send for EventPublisher {}
unsafe impl Sync for EventPublisher {}

/// Convenience functions for common publishing patterns
impl EventPublisher {
    /// Publish a heartbeat event
    pub fn heartbeat(&self) -> IpcResult<()> {
        self.publish(Event::system_heartbeat(self.app_id))
    }

    /// Publish MIDI events
    pub fn midi_started(&self, song_index: usize, song_name: String) -> IpcResult<()> {
        self.publish(Event::midi_playback_started(song_index, song_name))
    }

    pub fn midi_stopped(&self) -> IpcResult<()> {
        self.publish(Event::midi_playback_stopped())
    }

    pub fn midi_tempo_changed(&self, new_tempo: u32) -> IpcResult<()> {
        self.publish(Event::midi_tempo_changed(new_tempo))
    }

    pub fn midi_progress(&self, progress_ms: u32, total_ms: u32) -> IpcResult<()> {
        self.publish(Event::midi_progress_update(progress_ms, total_ms))
    }
}
