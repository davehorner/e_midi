//! Inter-process communication module using iceoryx2
//!
//! This module provides lock-free, zero-copy communication between the MIDI player
//! and other applications in the e_* ecosystem (e_grid, state server, etc.)

pub mod events;
pub mod publisher;
pub mod service;
pub mod subscriber;
pub mod types;

pub use events::*;
pub use publisher::*;
pub use service::*;
pub use subscriber::*;
pub use types::*;

use std::error::Error;
use std::fmt;

impl fmt::Display for IpcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IpcError::NodeCreation(msg) => write!(f, "Node creation failed: {}", msg),
            IpcError::ServiceCreation(msg) => write!(f, "Service creation failed: {}", msg),
            IpcError::PublisherCreation(msg) => write!(f, "Publisher creation failed: {}", msg),
            IpcError::SubscriberCreation(msg) => write!(f, "Subscriber creation failed: {}", msg),
            IpcError::SendError(msg) => write!(f, "Send failed: {}", msg),
            IpcError::ReceiveError(msg) => write!(f, "Receive failed: {}", msg),
            IpcError::SerializationError(msg) => write!(f, "Serialization failed: {}", msg),
            IpcError::DeserializationError(msg) => write!(f, "Deserialization failed: {}", msg),
            IpcError::PayloadTooLarge(msg) => write!(f, "Payload too large: {}", msg),
        }
    }
}

impl Error for IpcError {}
