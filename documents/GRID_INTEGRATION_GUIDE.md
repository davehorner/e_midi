# e_midi Integration Guide: Using Improved GridClient

## Quick Start

This guide shows how e_midi can integrate with the improved GridClient to receive window focus events for spatial music control.

## Basic Integration

### 1. Add e_grid Dependency
```toml
# In e_midi's Cargo.toml
[dependencies]
e_grid = { path = "../e_grid" }
```

### 2. Create Grid Integration Module
```rust
// src/integration/grid_integration.rs
use e_grid::{GridClient, GridClientResult, ipc::WindowFocusEvent};
use std::sync::Arc;
use crate::midi::MidiController; // Your MIDI controller

pub struct GridMidiIntegration {
    grid_client: GridClient,
    midi_controller: Arc<MidiController>,
}

impl GridMidiIntegration {
    pub fn new(midi_controller: Arc<MidiController>) -> GridClientResult<Self> {
        let mut grid_client = GridClient::new()?;
        
        // Register focus callback for music control
        let midi_controller_clone = midi_controller.clone();
        grid_client.set_focus_callback(move |focus_event| {
            Self::handle_focus_change(&midi_controller_clone, focus_event);
        })?;
        
        Ok(Self {
            grid_client,
            midi_controller,
        })
    }
    
    pub fn start_monitoring(&mut self) -> GridClientResult<()> {
        self.grid_client.start_background_monitoring()
    }
    
    fn handle_focus_change(midi_controller: &MidiController, focus_event: WindowFocusEvent) {
        let app_name = String::from_utf8_lossy(
            &focus_event.app_name[..focus_event.app_name_len.min(256) as usize]
        );
        
        if focus_event.is_focused {
            // Window gained focus - start/resume music
            if let Some(song) = midi_controller.get_or_assign_song(&app_name) {
                midi_controller.play_song(song, focus_event.hwnd);
                println!("🎵 Playing {} for app: {}", song.title, app_name);
            }
        } else {
            // Window lost focus - pause music
            midi_controller.pause_for_window(focus_event.hwnd);
            println!("⏸️ Paused music for app: {}", app_name);
        }
    }
}
```

### 3. Initialize in Main Application
```rust
// src/main.rs or src/lib.rs
use crate::integration::GridMidiIntegration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize your MIDI controller
    let midi_controller = Arc::new(MidiController::new()?);
    
    // Create grid integration
    let mut grid_integration = GridMidiIntegration::new(midi_controller.clone())?;
    
    // Start monitoring window focus events
    grid_integration.start_monitoring()?;
    
    println!("🎵 e_midi with spatial window focus control started!");
    
    // Your existing e_midi main loop
    run_midi_loop()?;
    
    Ok(())
}
```

## Advanced Usage

### Error Handling
```rust
use e_grid::{GridClientError, retry_with_backoff, RetryConfig};

impl GridMidiIntegration {
    pub fn robust_start(&mut self) -> GridClientResult<()> {
        let retry_config = RetryConfig {
            max_attempts: 3,
            base_delay_ms: 1000,
            backoff_multiplier: 2.0,
        };
        
        retry_with_backoff(|| {
            self.grid_client.start_background_monitoring()
        }, &retry_config)
    }
    
    pub fn handle_grid_error(&self, error: GridClientError) {
        match error {
            GridClientError::IpcError(msg) => {
                println!("🔌 Grid IPC issue: {} - continuing with basic mode", msg);
                // Fall back to non-spatial music mode
            }
            GridClientError::FocusCallbackError(msg) => {
                println!("🎯 Focus callback issue: {} - registering fallback", msg);
                // Register a simpler callback
            }
            _ => {
                println!("⚠️ Grid client error: {} - check configuration", error);
            }
        }
    }
}
```

### Song Management Integration
```rust
use std::collections::HashMap;

pub struct SpatialMidiController {
    // Your existing MIDI fields
    songs: HashMap<String, MidiSong>,
    app_to_song: HashMap<String, String>,
    current_playback: HashMap<u64, PlaybackState>, // HWND -> PlaybackState
}

impl SpatialMidiController {
    pub fn get_or_assign_song(&mut self, app_name: &str) -> Option<&MidiSong> {
        // Check if app already has a song assigned
        if let Some(song_name) = self.app_to_song.get(app_name) {
            return self.songs.get(song_name);
        }
        
        // Assign a new song based on your logic
        let song_name = self.choose_song_for_app(app_name);
        self.app_to_song.insert(app_name.to_string(), song_name.clone());
        self.songs.get(&song_name)
    }
    
    pub fn play_song(&mut self, song: &MidiSong, hwnd: u64) {
        // Resume if we have previous state, otherwise start from beginning
        if let Some(state) = self.current_playback.get(&hwnd) {
            self.resume_from_position(song, state.position);
        } else {
            self.start_song(song);
            self.current_playback.insert(hwnd, PlaybackState::new());
        }
    }
    
    pub fn pause_for_window(&mut self, hwnd: u64) {
        if let Some(state) = self.current_playback.get_mut(&hwnd) {
            state.position = self.get_current_position();
            state.is_paused = true;
            self.pause_playback();
        }
    }
}
```

### Configuration
```rust
// config.toml
[grid_integration]
enabled = true
auto_assign_songs = true
spatial_mode = "focus_based"  # or "position_based" for future

[song_assignment]
# Map applications to specific songs
"Visual Studio Code" = "coding_ambient.mid"
"Firefox" = "web_browsing.mid"
"Terminal" = "terminal_beats.mid"
default = "general_background.mid"
```

## Testing Your Integration

### 1. Create a Test Example
```rust
// examples/test_e_midi_integration.rs
use e_midi::integration::GridMidiIntegration;
use e_midi::midi::MockMidiController;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let midi_controller = Arc::new(MockMidiController::new());
    let mut integration = GridMidiIntegration::new(midi_controller)?;
    
    integration.start_monitoring()?;
    
    println!("🧪 Test integration running - focus different windows to test");
    
    // Keep running
    std::thread::park();
    Ok(())
}
```

### 2. Run the Test
```bash
cd /path/to/e_midi
cargo run --example test_e_midi_integration
```

## Troubleshooting

### Common Issues

1. **"Failed to create IPC node"**
   - Ensure e_grid server is running
   - Check firewall/permissions

2. **"Focus callback error"**
   - Verify callback function doesn't panic
   - Add error handling in callback

3. **"Invalid coordinates"**
   - Check grid configuration matches server
   - Validate coordinates before assignment

### Debug Mode
```rust
// Enable detailed logging
env_logger::init();
log::set_max_level(log::LevelFilter::Debug);

let mut grid_client = GridClient::new()?;
// GridClient will now output detailed debug info
```

## Performance Considerations

- Focus callbacks are called frequently - keep them lightweight
- Consider batching MIDI commands if receiving many focus events
- Use async processing for heavy operations in callbacks

## Next Steps

1. Test basic focus-based music control
2. Add application-specific song mapping
3. Implement position-based spatial audio (future enhancement)
4. Add persistence for app-to-song mappings
5. Create user interface for configuration

This integration provides the foundation for spatial music control based on window focus, enabling e_midi to create an immersive desktop music experience!
