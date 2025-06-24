//! Subscriber module for receiving events via iceoryx2
//!
//! Provides lock-free, zero-copy subscription to events from publishers

use iceoryx2::port::subscriber::Subscriber;
use iceoryx2::prelude::*;
use iceoryx2::service::ipc::Service;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use super::{AppId, Event, IpcError, IpcPayload, IpcResult};

/// Lock-free event subscriber
///
#[derive(Debug)]
pub struct EventSubscriber {
    subscriber: Subscriber<Service, IpcPayload, ()>,
    app_id: AppId,
    is_active: Arc<AtomicBool>,
    event_buffer: VecDeque<Event>,
    last_heartbeat: Instant,
}

impl EventSubscriber {
    /// Create a new event subscriber for events from the specified app
    pub fn new(source_app: AppId, subscriber_app: AppId) -> IpcResult<Self> {
        let service_name = format!("e_ecosystem_events_{:?}", source_app).to_lowercase();

        // Create node (suppress debug output)
        let node = NodeBuilder::new()
            .create::<Service>()
            .map_err(|_| IpcError::NodeCreation(format!("Node creation failed")))?;

        // Try to open existing service first, if that fails, create it
        let service = match node
            .service_builder(
                &ServiceName::new(&service_name)
                    .map_err(|_| IpcError::ServiceCreation(format!("Invalid service name")))?,
            )
            .publish_subscribe::<IpcPayload>()
            .open()
        {
            Ok(service) => service,
            Err(_) => {
                // If opening fails, try to create the service
                node.service_builder(
                    &ServiceName::new(&service_name)
                        .map_err(|_| IpcError::ServiceCreation(format!("Invalid service name")))?,
                )
                .publish_subscribe::<IpcPayload>()
                .open_or_create()
                .map_err(|_| IpcError::ServiceCreation(format!("Failed to create service")))?
            }
        };
        let subscriber = service
            .subscriber_builder()
            .create()
            .map_err(|_| IpcError::SubscriberCreation(format!("Failed to create subscriber")))?;

        Ok(Self {
            subscriber,
            app_id: subscriber_app,
            is_active: Arc::new(AtomicBool::new(true)),
            event_buffer: VecDeque::new(),
            last_heartbeat: Instant::now(),
        })
    }

    /// Try to receive events (non-blocking, lock-free)
    pub fn try_receive(&mut self) -> IpcResult<Vec<Event>> {
        if !self.is_active.load(Ordering::Relaxed) {
            return Ok(Vec::new());
        }

        let mut events = Vec::new(); // Check for new samples
        while let Some(sample) = self
            .subscriber
            .receive()
            .map_err(|_| IpcError::ReceiveError(format!("Failed to receive")))?
        {
            // Deserialize the received data
            let json_data = sample.payload();

            // Try to deserialize as single event first
            if let Ok(event) = serde_json::from_slice::<Event>(json_data) {
                events.push(event);
            } else if let Ok(batch) = serde_json::from_slice::<Vec<Event>>(json_data) {
                // If single event fails, try as batch
                events.extend(batch);
            } else {
                return Err(IpcError::DeserializationError(
                    "Failed to deserialize event data".to_string(),
                ));
            }
        }

        // Update heartbeat tracking
        if !events.is_empty() {
            self.last_heartbeat = Instant::now();
        }

        Ok(events)
    }

    /// Receive events with timeout (blocking)
    pub fn receive_timeout(&mut self, timeout: Duration) -> IpcResult<Vec<Event>> {
        let start = Instant::now();

        while start.elapsed() < timeout {
            let events = self.try_receive()?;
            if !events.is_empty() {
                return Ok(events);
            }

            // Small sleep to prevent busy waiting
            std::thread::sleep(Duration::from_millis(1));
        }

        Ok(Vec::new())
    }

    /// Check if subscriber is active
    pub fn is_active(&self) -> bool {
        self.is_active.load(Ordering::Relaxed)
    }

    /// Deactivate subscriber
    pub fn deactivate(&self) {
        self.is_active.store(false, Ordering::Relaxed);
    }

    /// Get time since last received event (for heartbeat monitoring)
    pub fn time_since_last_event(&self) -> Duration {
        self.last_heartbeat.elapsed()
    }

    /// Check if the source appears to be alive (received event recently)
    pub fn source_is_alive(&self, timeout: Duration) -> bool {
        self.time_since_last_event() < timeout
    }

    /// Get the app ID this subscriber represents
    pub fn app_id(&self) -> AppId {
        self.app_id
    }
}

impl Drop for EventSubscriber {
    fn drop(&mut self) {
        self.deactivate();
    }
}

// Subscriber is Send + Sync for lock-free usage across threads
unsafe impl Send for EventSubscriber {}
unsafe impl Sync for EventSubscriber {}

/// Event filter for processing specific event types
pub struct EventFilter {
    midi_events: bool,
    window_events: bool,
    grid_events: bool,
    system_events: bool,
}

impl EventFilter {
    pub fn new() -> Self {
        Self {
            midi_events: true,
            window_events: true,
            grid_events: true,
            system_events: true,
        }
    }

    pub fn midi_only() -> Self {
        Self {
            midi_events: true,
            window_events: false,
            grid_events: false,
            system_events: false,
        }
    }

    pub fn system_only() -> Self {
        Self {
            midi_events: false,
            window_events: false,
            grid_events: false,
            system_events: true,
        }
    }
    /// Filter events based on the filter settings
    pub fn filter(&self, events: Vec<Event>) -> Vec<Event> {
        events
            .into_iter()
            .filter(|event| {
                match event {
                    // MIDI command events (TUI to player)
                    Event::MidiCommandPlay { .. }
                    | Event::MidiCommandStop { .. }
                    | Event::MidiCommandPause { .. }
                    | Event::MidiCommandResume { .. }
                    | Event::MidiCommandNext { .. }
                    | Event::MidiCommandPrevious { .. }
                    | Event::MidiCommandSetTempo { .. }
                    | Event::MidiCommandSongListRequest { .. } => self.midi_events,

                    // MIDI status events (player to TUI)
                    Event::MidiPlaybackStarted { .. }
                    | Event::MidiPlaybackStopped { .. }
                    | Event::MidiPlaybackPaused { .. }
                    | Event::MidiPlaybackResumed { .. }
                    | Event::MidiTempoChanged { .. }
                    | Event::MidiSongChanged { .. }
                    | Event::MidiProgressUpdate { .. }
                    | Event::MidiSongListUpdated { .. } => self.midi_events,

                    Event::WindowFocused { .. }
                    | Event::WindowClosed { .. }
                    | Event::WindowResized { .. } => self.window_events,

                    Event::GridCellSelected { .. }
                    | Event::GridCellUpdated { .. }
                    | Event::GridStateChanged { .. } => self.grid_events,

                    Event::SystemShutdown { .. }
                    | Event::SystemHeartbeat { .. }
                    | Event::StateRequest { .. }
                    | Event::StateResponse { .. } => self.system_events,
                }
            })
            .collect()
    }
}

impl Default for EventFilter {
    fn default() -> Self {
        Self::new()
    }
}
