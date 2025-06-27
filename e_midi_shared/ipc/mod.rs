// Moved from e_midi/src/ipc/mod.rs

pub mod events;

pub use events::Event;
pub use events::IpcEventSender;
pub use events::IpcEventReceiver;

use std::sync::OnceLock;
use std::sync::mpsc::{Sender, Receiver, channel};
use crate::ipc::service::IpcServiceManager;

/// Global event sender for static/background publishing
pub static IPC_EVENT_SENDER: OnceLock<Sender<Event>> = OnceLock::new();

/// Start the global IPC event relay thread
pub fn start_ipc_event_relay(ipc_manager: IpcServiceManager) {
    let (tx, rx): (Sender<Event>, Receiver<Event>) = channel();
    let _ = IPC_EVENT_SENDER.set(tx);
    std::thread::spawn(move || {
        while let Ok(event) = rx.recv() {
            let _ = ipc_manager.publish_event(event);
        }
    });
}
