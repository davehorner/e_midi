//! Service discovery and management for the e_* ecosystem
//!
//! Provides lock-free service registration and discovery

// use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use std::sync::OnceLock;
use serde::{Deserialize, Serialize};
use super::{AppId, EventPublisher, EventSubscriber, IpcResult};

/// Service information for discovery
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceInfo {
    pub app_id: AppId,
    pub service_name: String,
    pub version: String,
    pub capabilities: Vec<String>,
    pub last_heartbeat: u64,
    pub process_id: u32,
}

/// Service registry for managing available services
///
#[derive(Debug)]
pub struct ServiceRegistry {
    services: HashMap<AppId, ServiceInfo>,
    is_active: Arc<AtomicBool>,
    last_cleanup: Instant,
}

impl ServiceRegistry {
    pub fn new() -> Self {
        Self {
            services: HashMap::new(),
            is_active: Arc::new(AtomicBool::new(true)),
            last_cleanup: Instant::now(),
        }
    }

    /// Register a service (lock-free when possible)
    pub fn register_service(&mut self, service_info: ServiceInfo) {
        if self.is_active.load(Ordering::Relaxed) {
            self.services.insert(service_info.app_id, service_info);
        }
    }

    /// Get service information
    pub fn get_service(&self, app_id: AppId) -> Option<&ServiceInfo> {
        if self.is_active.load(Ordering::Relaxed) {
            self.services.get(&app_id)
        } else {
            None
        }
    }

    /// List all active services
    pub fn list_services(&self) -> Vec<&ServiceInfo> {
        if self.is_active.load(Ordering::Relaxed) {
            self.services.values().collect()
        } else {
            Vec::new()
        }
    }

    /// Update heartbeat for a service
    pub fn update_heartbeat(&mut self, app_id: AppId) {
        if let Some(service) = self.services.get_mut(&app_id) {
            service.last_heartbeat = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
        }
    }

    /// Clean up stale services (call periodically)
    pub fn cleanup_stale_services(&mut self, max_age: Duration) {
        if self.last_cleanup.elapsed() < Duration::from_secs(5) {
            return; // Don't cleanup too frequently
        }

        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let max_age_secs = max_age.as_secs();

        self.services
            .retain(|_, service| current_time - service.last_heartbeat < max_age_secs);

        self.last_cleanup = Instant::now();
    }

    /// Check if a service is alive
    pub fn is_service_alive(&self, app_id: AppId, max_age: Duration) -> bool {
        if let Some(service) = self.get_service(app_id) {
            let current_time = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();

            current_time - service.last_heartbeat < max_age.as_secs()
        } else {
            false
        }
    }

    pub fn deactivate(&self) {
        self.is_active.store(false, Ordering::Relaxed);
    }
}

/// IPC service manager for coordinating publishers and subscribers
///
#[derive(Debug)]
pub struct IpcServiceManager {
    app_id: AppId,
    publisher: Option<EventPublisher>,
    subscribers: HashMap<AppId, EventSubscriber>,
    registry: ServiceRegistry,
    heartbeat_interval: Duration,
    last_heartbeat: Instant,
    is_active: Arc<AtomicBool>,
}

impl IpcServiceManager {
    /// Create a new IPC service manager
    pub fn new(app_id: AppId) -> IpcResult<Self> {
        println!("[IPC SERVICE MANAGER DEBUG] Creating IpcServiceManager for app_id: {:?}", app_id);
        use std::io::Write;
        std::io::stdout().flush().ok();
        let (tx, rx) = std::sync::mpsc::channel();
        let _ = crate::ipc::IPC_EVENT_SENDER.set(tx);
        let manager = Arc::new(Self {
            app_id,
            publisher: None,
            subscribers: HashMap::new(),
            registry: ServiceRegistry::new(),
            heartbeat_interval: Duration::from_secs(5),
            last_heartbeat: Instant::now(),
            is_active: Arc::new(AtomicBool::new(true)),
        });
        let manager_clone = Arc::clone(&manager);
        std::thread::spawn(move || {
            while let Ok(event) = rx.recv() {
                let _ = manager_clone.publish_event(event);
            }
        });
        // Return Arc as plain reference for compatibility
        Ok(Arc::try_unwrap(manager).unwrap_or_else(|arc| (*arc).clone()))
    }

    /// Initialize publisher for this app
    pub fn init_publisher(&mut self) -> IpcResult<()> {
        if self.publisher.is_none() {
            self.publisher = Some(EventPublisher::new(self.app_id)?);
        }
        Ok(())
    }

    /// Subscribe to events from another app
    pub fn subscribe_to(&mut self, source_app: AppId) -> IpcResult<()> {
        if !self.subscribers.contains_key(&source_app) {
            let subscriber = EventSubscriber::new(source_app, self.app_id)?;
            self.subscribers.insert(source_app, subscriber);
        }
        Ok(())
    }

    /// Get publisher reference
    pub fn publisher(&self) -> Option<&EventPublisher> {
        self.publisher.as_ref()
    }

    /// Get subscriber reference
    pub fn subscriber(&mut self, source_app: AppId) -> Option<&mut EventSubscriber> {
        self.subscribers.get_mut(&source_app)
    }

    /// Send periodic heartbeat
    pub fn heartbeat(&mut self) -> IpcResult<()> {
        if self.last_heartbeat.elapsed() >= self.heartbeat_interval {
            if let Some(publisher) = &self.publisher {
                publisher.heartbeat()?;
                self.registry.update_heartbeat(self.app_id);
                self.last_heartbeat = Instant::now();
            }
        }
        Ok(())
    }

    /// Process all incoming events from all subscribers
    pub fn process_events(&mut self) -> IpcResult<Vec<(AppId, Vec<super::Event>)>> {
        let mut all_events = Vec::new();
        for (source_app, subscriber) in self.subscribers.iter_mut() {
            let events = subscriber.try_receive()?;
            if !events.is_empty() {
                all_events.push((*source_app, events));
            }
        }

        Ok(all_events)
    }

    /// Publish an event via the managed publisher
    pub fn publish_event(&self, event: super::Event) -> IpcResult<()> {
        if let Some(ref publisher) = self.publisher {
            publisher.publish(event)
        } else {
            Err(super::IpcError::PublisherCreation(
                "No publisher available".to_string(),
            ))
        }
    }

    /// Clean up and deactivate
    pub fn shutdown(&mut self) {
        self.is_active.store(false, Ordering::Relaxed);

        if let Some(publisher) = &self.publisher {
            publisher.deactivate();
        }

        for subscriber in self.subscribers.values() {
            subscriber.deactivate();
        }

        self.registry.deactivate();
    }

    pub fn is_active(&self) -> bool {
        self.is_active.load(Ordering::Relaxed)
    }

    /// Returns a clone of the global event sender, if set
    pub fn get_event_sender(&self) -> Option<std::sync::mpsc::Sender<super::Event>> {
        crate::ipc::IPC_EVENT_SENDER.get().cloned()
    }

    /// Publish an IPC event (static method)
    pub fn publish_ipc_event(event: super::Event) {
        // Try to get the global IpcServiceManager (if available)
        if let Some(ipc_manager) = crate::ipc::IPC_SERVICE_MANAGER.get() {
            println!("[IPC DEBUG] Publishing event: {:?}", event); // DEBUG
            let _ = ipc_manager.publish_event(event);
        } else {
            println!("[IPC DEBUG] IPC manager not initialized, skipping event publishing: {:?}", event);
        }
    }
}

impl Clone for IpcServiceManager {
    fn clone(&self) -> Self {
        IpcServiceManager {
            app_id: self.app_id,
            publisher: None, // Do not clone publisher (not needed for relay)
            subscribers: HashMap::new(), // Do not clone subscribers (not needed for relay)
            registry: ServiceRegistry::new(), // New registry for relay
            heartbeat_interval: self.heartbeat_interval,
            last_heartbeat: self.last_heartbeat,
            is_active: Arc::clone(&self.is_active),
        }
    }
}

impl Drop for IpcServiceManager {
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// Global reference to the main IpcServiceManager for static publishing
pub static IPC_SERVICE_MANAGER: OnceLock<IpcServiceManager> = OnceLock::new();
