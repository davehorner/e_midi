use std::error::Error;
use std::time::Duration;
use std::io::stdout;
use std::sync::{Arc, atomic::{AtomicBool, AtomicU32, Ordering}};
use std::thread;

use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    terminal::{disable_raw_mode, enable_raw_mode},
    ExecutableCommand,
};

use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Line,    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame, Terminal,
};

use crate::{MidiPlayer, should_shutdown, set_shutdown_flag};
use crate::ipc::{Event as IpcEvent, EventSubscriber, EventPublisher, AppId};

pub struct TuiApp {
    pub should_quit: bool,
    pub selected_song: usize,
    pub list_state: ListState,
    pub current_tempo: u32,
    pub is_playing: Arc<AtomicBool>,
    pub playback_info: Option<PlaybackInfo>,
    pub log_messages: Vec<String>,
    pub log_scroll: usize,
    pub event_subscriber: Option<EventSubscriber>,
    pub command_publisher: Option<EventPublisher>,
}

#[derive(Clone)]
pub struct PlaybackInfo {
    pub song_name: String,
    pub current_time: Arc<AtomicU32>,
    pub total_time: u32,
    pub tempo: Arc<AtomicU32>,
    pub tracks: Vec<String>,
    pub track_count: usize,
}

impl TuiApp {
    pub fn new() -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));
          Self {
            should_quit: false,
            selected_song: 0,
            list_state,
            current_tempo: 120,
            is_playing: Arc::new(AtomicBool::new(false)),
            playback_info: None,
            log_messages: Vec::new(),
            log_scroll: 0,
            event_subscriber: None,
            command_publisher: None,
        }
    }
    
    pub fn add_log(&mut self, message: String) {
        self.log_messages.push(message);
        // Keep only last 50 messages
        if self.log_messages.len() > 50 {
            self.log_messages.remove(0);
        }
        // Auto-scroll to bottom
        self.log_scroll = self.log_messages.len().saturating_sub(1);
    }
    
    pub fn scroll_log_up(&mut self) {
        if self.log_scroll > 0 {
            self.log_scroll -= 1;
        }
    }
    
    pub fn scroll_log_down(&mut self) {
        if self.log_scroll < self.log_messages.len().saturating_sub(1) {
            self.log_scroll += 1;
        }
    }    pub fn stop_playback(&mut self) {
        // Don't modify is_playing here - it's controlled by the MIDI player
        self.playback_info = None;
    }pub fn init_event_subscriber(&mut self) -> Result<(), Box<dyn Error>> {
        match EventSubscriber::new(AppId::EMidi, AppId::EMidi) {
            Ok(subscriber) => {
                self.event_subscriber = Some(subscriber);
                // Silently initialized - no log message to avoid clutter
                Ok(())
            }
            Err(_) => {
                // Silently fail - IPC is optional for TUI operation
                Ok(())
            }
        }
    }

    pub fn init_command_publisher(&mut self) -> Result<(), Box<dyn Error>> {
        match EventPublisher::new(AppId::EMidi) {
            Ok(publisher) => {
                self.command_publisher = Some(publisher);
                self.add_log("🔗 IPC command publisher initialized".to_string());
                Ok(())
            }
            Err(_) => {
                self.add_log("⚠️ Failed to initialize IPC publisher - commands will be local only".to_string());
                Ok(())
            }
        }
    }
    
    pub fn init_command_subscriber(&mut self) -> Result<(), Box<dyn Error>> {
        match EventSubscriber::new(AppId::EMidi, AppId::EMidi) {
            Ok(subscriber) => {
                // We'll use this to subscribe to our own published commands
                // This enables the IPC demonstration while keeping everything in-process
                Ok(())
            }            Err(_) => {
                Ok(()) // Silently fail
            }
        }
    }
    
    pub fn process_ipc_events(&mut self) {
        if let Some(ref mut subscriber) = self.event_subscriber {
            match subscriber.try_receive() {
                Ok(events) => {
                    for event in events {
                        self.handle_ipc_event(event);
                    }
                }
                Err(_) => {
                    // No events available or error - continue silently
                }
            }
        }
    }
      fn handle_ipc_event(&mut self, event: IpcEvent) {
        match event {
            IpcEvent::MidiPlaybackStarted { song_index, song_name, .. } => {
                self.add_log(format!("🎵 Started: {} ({})", song_name, song_index));
                // Don't set is_playing here - it's controlled by the MIDI player
            }
            IpcEvent::MidiPlaybackStopped { .. } => {
                self.add_log("⏹️ Playback stopped".to_string());
                // Don't set is_playing here - it's controlled by the MIDI player
                self.playback_info = None;
            }
            IpcEvent::MidiTempoChanged { new_tempo, .. } => {
                self.add_log(format!("🎶 Tempo changed to {} BPM", new_tempo));
                self.current_tempo = new_tempo;
                if let Some(ref info) = self.playback_info {
                    info.tempo.store(new_tempo, Ordering::Relaxed);
                }
            }
            IpcEvent::MidiProgressUpdate { progress_ms, total_ms, .. } => {
                if let Some(ref info) = self.playback_info {
                    info.current_time.store(progress_ms / 1000, Ordering::Relaxed);
                    // Update total time if it's different
                    if info.total_time != total_ms / 1000 {
                        // We can't update total_time directly since it's not atomic
                        // But we could create a new PlaybackInfo if needed
                    }
                }
            }
            IpcEvent::SystemHeartbeat { .. } => {
                // Ignore heartbeat events in TUI
            }
            _ => {
                // Handle other event types as needed
            }
        }
    }
    
    pub fn publish_command(&mut self, event: IpcEvent) {
        if let Some(ref mut publisher) = self.command_publisher {
            match publisher.publish(event) {
                Ok(()) => {
                    // Command sent successfully - no log to avoid clutter
                }
                Err(_) => {
                    self.add_log("⚠️ Failed to send IPC command".to_string());
                }
            }
        } else {
            self.add_log("⚠️ No IPC publisher available - command not sent".to_string());
        }
    }
}

pub fn run_tui_mode(midi_player: &mut MidiPlayer) -> Result<(), Box<dyn Error>> {
    let mut app = TuiApp::new();
    
    // Initialize all IPC components BEFORE entering raw mode to prevent output corruption
    app.add_log("Initializing IPC components...".to_string());
    
    // Initialize IPC for bidirectional communication
    let _ = midi_player.init_ipc_publisher();
    
    // Initialize event subscriber for receiving status updates
    if let Err(_) = app.init_event_subscriber() {
        app.add_log("⚠️ IPC subscriber initialization failed - running without real-time updates".to_string());
    } else {
        app.add_log("✅ IPC subscriber initialized for real-time updates".to_string());
    }
    
    // Initialize command publisher for sending commands
    if let Err(_) = app.init_command_publisher() {
        app.add_log("⚠️ IPC publisher initialization failed - running in local mode".to_string());
    }
    
    // Setup terminal AFTER IPC initialization to prevent corruption
    enable_raw_mode()?;
    let mut stdout = stdout();
    stdout.execute(crossterm::terminal::EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    
    // Clear the screen first
    terminal.clear()?;
    
    // Use the shared playing state from MIDI player
    app.is_playing = midi_player.get_playing_state();
    
    app.add_log("Starting e_midi TUI...".to_string());
    app.add_log("Use Up/Down to navigate, Enter to play, H for help".to_string());
    app.add_log(format!("Found {} songs total", midi_player.get_total_song_count()));
    
    let result = run_tui_app(&mut terminal, &mut app, midi_player);
    
    // Stop any ongoing playback
    app.stop_playback();
      // Send all notes off before cleanup
    for channel in 0..16 {
        let _ = midi_player.send_midi_command(crate::MidiCommand::SendMessage(vec![0xB0 | channel, 123, 0]));
    }
    
    // Cleanup terminal
    disable_raw_mode()?;
    terminal.backend_mut().execute(crossterm::terminal::LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    
    result
}

fn run_tui_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut TuiApp,
    midi_player: &mut MidiPlayer,
) -> Result<(), Box<dyn Error>> {
    loop {
        // Process IPC events for real-time updates
        app.process_ipc_events();
          // Ensure list state is synchronized before each render
        let song_count = midi_player.get_total_song_count();
        if song_count > 0 {
            if app.selected_song >= song_count {
                app.selected_song = 0;
            }
            // Force the list state to be updated - this ensures the visual highlight moves
            app.list_state = ListState::default();
            app.list_state.select(Some(app.selected_song));
        } else {
            app.list_state.select(None);
        }
        
        // Draw the UI with better error handling
        if let Err(e) = terminal.draw(|f| ui(f, app, midi_player)) {
            app.add_log(format!("UI render error: {}", e));
            // Try to continue - the error might be temporary
            thread::sleep(Duration::from_millis(50));
        }
          // Handle events with a timeout
        if event::poll(Duration::from_millis(100))? {
            match event::read()? {
                Event::Key(key) => {
                    // Only process key press events, not key release events
                    if key.kind == crossterm::event::KeyEventKind::Press {
                        if handle_key_event(key, app, midi_player)? {
                            break; // Exit requested
                        }
                    }
                }
                _ => {}
            }
        }
        
        // Check for global shutdown
        if should_shutdown() {
            app.add_log("🛑 Shutdown requested".to_string());
            app.stop_playback();
            break;
        }
        
        if app.should_quit {
            break;
        }
    }
    
    Ok(())
}

fn handle_key_event(
    key: KeyEvent, 
    app: &mut TuiApp, 
    midi_player: &mut MidiPlayer
) -> Result<bool, Box<dyn Error>> {
    match (key.code, key.modifiers) {
        // Ctrl+C - Exit immediately
        (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
            app.add_log("🛑 Ctrl+C pressed, exiting".to_string());
            app.stop_playback();
            set_shutdown_flag();
            return Ok(true);
        }
        
        // Escape or 'q' - Exit
        (KeyCode::Esc, _) | (KeyCode::Char('q'), _) => {
            app.stop_playback();
            app.should_quit = true;
            return Ok(true);
        }        // Arrow keys - Navigate song list
        (KeyCode::Up, _) => {
            let song_count = midi_player.get_total_song_count();
            if song_count > 0 {
                let old_selection = app.selected_song;
                app.selected_song = if app.selected_song == 0 { 
                    song_count - 1 
                } else { 
                    app.selected_song - 1 
                };
                app.add_log(format!("🔼 Navigate: {} -> {}", old_selection, app.selected_song));
            }
        }

        (KeyCode::Down, _) => {
            let song_count = midi_player.get_total_song_count();
            if song_count > 0 {
                let old_selection = app.selected_song;
                app.selected_song = (app.selected_song + 1) % song_count;
                app.add_log(format!("🔽 Navigate: {} -> {}", old_selection, app.selected_song));
            }
        }
        
        // Page Up/Page Down for log scrolling
        (KeyCode::PageUp, _) => {
            for _ in 0..5 {
                app.scroll_log_up();
            }
        }
        
        (KeyCode::PageDown, _) => {
            for _ in 0..5 {
                app.scroll_log_down();
            }
        }        // Enter or Space - Play selected song
        (KeyCode::Enter, _) | (KeyCode::Char(' '), _) => {
            if !app.is_playing.load(Ordering::Relaxed) && midi_player.get_total_song_count() > 0 {
                app.add_log(format!("▶️ Playing song {}", app.selected_song));
                start_playback(app, midi_player)?;
            } else if app.is_playing.load(Ordering::Relaxed) {
                app.add_log("⚠️ Already playing - press 'S' to stop first".to_string());
            }
        }// 's' - Stop playback
        (KeyCode::Char('s'), _) => {
            if app.is_playing.load(Ordering::Relaxed) {
                app.add_log("⏹️ Sending stop command via IPC...".to_string());
                
                let stop_command = IpcEvent::MidiCommandStop {
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as u64
                };
                
                app.publish_command(stop_command.clone());
                
                // Execute the command locally (in a real distributed system, 
                // this would be handled by the main process subscriber)
                execute_midi_command(stop_command, app, midi_player)?;
            }
        }
        
        // 'l' - List songs (refresh)
        (KeyCode::Char('l'), _) => {
            app.add_log(format!("📀 {} total songs available ({} static + {} dynamic)", 
                midi_player.get_total_song_count(),
                midi_player.get_static_song_count(),
                midi_player.get_dynamic_song_count()));
        }
        
        // 't' - Tempo adjustment (during playback)
        (KeyCode::Char('t'), _) => {
            if app.is_playing.load(Ordering::Relaxed) {
                if let Some(ref info) = app.playback_info {
                    let current_tempo = info.tempo.load(Ordering::Relaxed);
                    let new_tempo = match current_tempo {
                        60..=89 => 90,
                        90..=119 => 120,
                        120..=149 => 150,
                        150..=179 => 180,
                        _ => 60,
                    };
                    info.tempo.store(new_tempo, Ordering::Relaxed);
                    app.current_tempo = new_tempo;
                    app.add_log(format!("🎶 Tempo changed to {} BPM", new_tempo));
                }
            } else {
                app.add_log("⚠️ Tempo can only be changed during playback".to_string());
            }
        }        // 'n' - Next song (during playback)
        (KeyCode::Char('n'), _) => {
            if app.is_playing.load(Ordering::Relaxed) {
                let song_count = midi_player.get_total_song_count();
                if song_count > 0 {
                    app.selected_song = (app.selected_song + 1) % song_count;
                    app.add_log(format!("⏭️ Skipping to next song ({})", app.selected_song));
                    
                    // Stop current playback properly
                    midi_player.stop_playback();
                    app.stop_playback();
                    // Send all notes off
                    for channel in 0..16 {
                        midi_player.send_midi_command(crate::MidiCommand::SendMessage(vec![0xB0 | channel, 123, 0]))?;
                    }
                    start_playback(app, midi_player)?;
                }
            } else {
                app.add_log("⚠️ Next song can only be used during playback".to_string());
            }
        }
        
        // 'p' - Previous song
        (KeyCode::Char('p'), _) => {
            if app.is_playing.load(Ordering::Relaxed) {
                let song_count = midi_player.get_total_song_count();
                if song_count > 0 {
                    app.selected_song = if app.selected_song == 0 { 
                        song_count - 1 
                    } else { 
                        app.selected_song - 1 
                    };
                    app.add_log(format!("⏮️ Skipping to previous song ({})", app.selected_song));
                    
                    // Stop current playback properly
                    midi_player.stop_playback();
                    app.stop_playback();
                    // Send all notes off  
                    for channel in 0..16 {
                        midi_player.send_midi_command(crate::MidiCommand::SendMessage(vec![0xB0 | channel, 123, 0]))?;
                    }
                    start_playback(app, midi_player)?;
                }
            } else {
                app.add_log("⚠️ Previous song can only be used during playback".to_string());
            }
        }
        
        // 'h' - Help
        (KeyCode::Char('h'), _) => {
            app.add_log("🆘 Navigation: ↑↓=select, Enter/Space=play, S=stop, L=refresh".to_string());
            app.add_log("🆘 Playback: T=tempo, N=next, P=prev, Q/Esc=quit, Ctrl+C=force".to_string());
            app.add_log("🆘 Scrolling: PgUp/PgDn=scroll logs, Alt+C=clear dynamic".to_string());
        }
          // Alt+C - Clear dynamic songs
        (KeyCode::Char('c'), KeyModifiers::ALT) => {
            midi_player.clear_dynamic_songs();
            app.add_log("🗑️ Dynamic songs cleared".to_string());
            
            // Adjust selection if needed
            if app.selected_song >= midi_player.get_total_song_count() {
                app.selected_song = midi_player.get_total_song_count().saturating_sub(1);
            }
        }
        
        _ => {}
    }
    
    Ok(false)
}

fn start_playback(app: &mut TuiApp, midi_player: &mut MidiPlayer) -> Result<(), Box<dyn Error>> {
    if let Some(song) = midi_player.get_song(app.selected_song) {
        app.add_log(format!("🎵 Starting playback: {}", song.name));
        
        // Calculate song duration for display
        let track_indices: Vec<usize> = song.tracks.iter().map(|t| t.index).collect();
        let events = midi_player.get_events_for_song(app.selected_song, &track_indices, song.default_tempo);
        let duration_ms = crate::calculate_song_duration_ms(&events);
        
        // Set up playback info with atomic values for thread-safe updates
        let current_time = Arc::new(AtomicU32::new(0));
        let tempo = Arc::new(AtomicU32::new(song.default_tempo));
        
        app.playback_info = Some(PlaybackInfo {
            song_name: song.name.clone(),
            current_time: Arc::clone(&current_time),
            total_time: duration_ms / 1000,
            tempo: Arc::clone(&tempo),
            tracks: song.tracks.iter().map(|t| 
                format!("Track {} ({})", t.index, 
                    t.guess.as_ref().unwrap_or(&"Unknown".to_string()))).collect(),
            track_count: song.tracks.len(),
        });
          app.current_tempo = song.default_tempo;
        // Don't set is_playing here - it's controlled by the MIDI player background thread
        
        // Publish IPC command to start playback
        app.add_log(format!("🎵 Sending play command via IPC: {}", song.name));
        app.add_log(format!("🎵 Song {} - {} tracks at {} BPM", app.selected_song, track_indices.len(), song.default_tempo));
        
        let play_command = IpcEvent::MidiCommandPlay {
            song_index: app.selected_song,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64
        };
          app.publish_command(play_command.clone());
        
        // Execute the command locally (in a real distributed system, 
        // this would be handled by the main process subscriber)
        execute_midi_command(play_command, app, midi_player)?;
        
    } else {
        app.add_log("❌ Invalid song selection".to_string());
    }
    
    Ok(())
}

/// Execute an IPC MIDI command locally (demonstrates the command processing)
/// In a distributed system, this would be handled by the main process
fn execute_midi_command(
    command: IpcEvent, 
    app: &mut TuiApp, 
    midi_player: &mut MidiPlayer
) -> Result<(), Box<dyn Error>> {
    match command {
        IpcEvent::MidiCommandPlay { song_index, .. } => {
            app.add_log(format!("🎵 Executing play command for song {}", song_index));
              // Use the actual MIDI player method with IPC publishing (non-blocking)
            if let Err(e) = midi_player.play_song_with_ipc_nonblocking(song_index) {
                app.add_log(format!("❌ Playback failed: {}", e));
            }
        }        IpcEvent::MidiCommandStop { .. } => {
            app.add_log("⏹️ Executing stop command".to_string());
            
            // Stop playback using the MIDI player's method
            midi_player.stop_playback();
            
            // Publish stop event
            midi_player.publish_midi_event(IpcEvent::MidiPlaybackStopped {
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64
            });
            
            app.stop_playback();
        }
        IpcEvent::MidiCommandSetTempo { new_tempo, .. } => {
            app.add_log(format!("🎶 Executing tempo change to {} BPM", new_tempo));
            app.current_tempo = new_tempo;
            
            // Publish tempo changed event
            midi_player.publish_midi_event(IpcEvent::MidiTempoChanged {
                new_tempo,
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64
            });
        }
        _ => {
            // Handle other commands as needed
        }
    }
    
    Ok(())
}

fn ui(f: &mut Frame, app: &mut TuiApp, midi_player: &MidiPlayer) {
    // Get terminal size and ensure we have minimum dimensions
    let size = f.area();
    if size.width < 80 || size.height < 24 {
        // Terminal too small - show error message
        let error_text = vec![
            Line::from("Terminal too small!"),
            Line::from(format!("Current: {}x{}", size.width, size.height)),
            Line::from("Minimum required: 80x24"),
            Line::from("Please resize your terminal"),
        ];
        
        let error_widget = Paragraph::new(error_text)
            .block(Block::default().borders(Borders::ALL).title("Error"))
            .wrap(Wrap { trim: true });
        
        f.render_widget(error_widget, size);
        return;
    }
    
    // Simple, robust two-panel layout
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(size);
    
    // Left panel: Header, song list, and controls  
    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),               // Header (fixed)
            Constraint::Min(8),                  // Song list (minimum space)
            Constraint::Length(9),               // Controls (fixed)
        ])
        .split(main_chunks[0]);
    
    // Right panel: Playback info and logs
    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(10),              // Playback info (fixed)
            Constraint::Min(5),                  // Logs (remaining space)
        ])
        .split(main_chunks[1]);
    
    // Render each section safely
    if left_chunks.len() >= 3 && right_chunks.len() >= 2 {
        render_header(f, left_chunks[0], midi_player);
        render_song_list(f, left_chunks[1], app, midi_player);
        render_controls(f, left_chunks[2], app);
        render_playback_info(f, right_chunks[0], app, midi_player);
        render_log_messages(f, right_chunks[1], app);
    } else {
        // Fallback: render a simple message if layout fails
        let fallback_text = vec![
            Line::from("Layout error - try resizing terminal"),
        ];
        
        let fallback_widget = Paragraph::new(fallback_text)
            .block(Block::default().borders(Borders::ALL).title("Layout Error"))
            .wrap(Wrap { trim: true });
        
        f.render_widget(fallback_widget, size);
    }
}

fn render_header(f: &mut Frame, area: Rect, midi_player: &MidiPlayer) {
    let header_text = format!("e_midi - {} songs ({} static + {} dynamic)", 
        midi_player.get_total_song_count(),
        midi_player.get_static_song_count(),
        midi_player.get_dynamic_song_count());
    
    let header = Paragraph::new(header_text)
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .block(Block::default().borders(Borders::ALL).title("MIDI Player TUI"))
        .wrap(Wrap { trim: true });
    
    f.render_widget(header, area);
}

fn render_song_list(f: &mut Frame, area: Rect, app: &mut TuiApp, midi_player: &MidiPlayer) {
    let song_count = midi_player.get_total_song_count();
    
    let songs: Vec<ListItem> = if song_count == 0 {
        vec![ListItem::new("No songs available").style(Style::default().fg(Color::Red))]
    } else {
        (0..song_count)
            .map(|i| {
                if let Some(song) = midi_player.get_song(i) {                    let style = if i == app.selected_song {
                        if midi_player.is_playing() {
                            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
                        } else {
                            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                        }
                    } else {
                        Style::default().fg(Color::White)
                    };
                    
                    let prefix = if i < midi_player.get_static_song_count() { "S" } else { "D" };
                    let playing_indicator = if midi_player.is_playing() && i == app.selected_song { 
                        ">" 
                    } else { 
                        " " 
                    };
                    
                    ListItem::new(format!("{} [{}] {}: {} ({} tracks)", 
                        playing_indicator, prefix, i, song.name, song.tracks.len()))
                        .style(style)
                } else {
                    ListItem::new(format!("Invalid song {}", i))
                        .style(Style::default().fg(Color::Red))
                }
            })
            .collect()
    };
    
    let list = List::new(songs)
        .block(Block::default()
            .borders(Borders::ALL)
            .title("Song List")
        )
        .highlight_style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD))
        .highlight_symbol(">> ");
    
    // Create a completely fresh ListState to ensure proper highlighting
    let mut fresh_list_state = ListState::default();
    if song_count > 0 && app.selected_song < song_count {
        fresh_list_state.select(Some(app.selected_song));
    }
    
    f.render_stateful_widget(list, area, &mut fresh_list_state);
}

fn render_controls(f: &mut Frame, area: Rect, _app: &TuiApp) {
    let controls_text = vec![
        Line::from("CONTROLS:"),
        Line::from("Up/Down: Navigate  Enter: Play  S: Stop"),
        Line::from("T: Tempo  N: Next  P: Previous"),
        Line::from("L: Refresh  H: Help  PgUp/PgDn: Scroll"),
        Line::from("Q/Esc: Quit  Ctrl+C: Force exit"),
        Line::from(""),
        Line::from("Legend: [S]=Static [D]=Dynamic >=Playing"),
    ];
    
    let controls = Paragraph::new(controls_text)
        .block(Block::default().borders(Borders::ALL).title("Controls"))
        .wrap(Wrap { trim: true });
    
    f.render_widget(controls, area);
}

fn render_playback_info(f: &mut Frame, area: Rect, app: &TuiApp, midi_player: &MidiPlayer) {
    let info_text = if let Some(ref info) = app.playback_info {
        let current_time = info.current_time.load(Ordering::Relaxed);
        let current_tempo = info.tempo.load(Ordering::Relaxed);
        let is_playing = midi_player.is_playing();
        let progress_pct = if info.total_time > 0 {
            (current_time * 100 / info.total_time).min(100)
        } else {
            0
        };
        
        vec![
            Line::from(format!("Status: {}", if is_playing { "PLAYING" } else { "STOPPED" })),
            Line::from(""),
            Line::from(format!("Song: {}", info.song_name)),
            Line::from(format!("Time: {}s/{}s ({}%)", current_time, info.total_time, progress_pct)),
            Line::from(format!("Tempo: {} BPM", current_tempo)),
            Line::from(format!("Tracks: {}", info.track_count)),
            Line::from(""),
            Line::from("Press T for tempo, N/P for next/prev"),
        ]
    } else {
        vec![
            Line::from("Status: IDLE"),
            Line::from(""),
            Line::from("No song playing"),
            Line::from(""),
            Line::from("Select a song and press Enter"),
            Line::from("Use Up/Down to navigate"),
            Line::from("Press H for help"),
            Line::from(""),
        ]
    };
    
    let playback = Paragraph::new(info_text)
        .block(Block::default().borders(Borders::ALL).title("Playback"))
        .wrap(Wrap { trim: true });
    
    f.render_widget(playback, area);
}

fn render_log_messages(f: &mut Frame, area: Rect, app: &TuiApp) {
    let visible_height = area.height.saturating_sub(2) as usize; // Account for borders
    
    let log_items: Vec<ListItem> = if app.log_messages.is_empty() {
        vec![ListItem::new("No messages yet...").style(Style::default().fg(Color::Gray))]
    } else {
        // Show the most recent messages
        let start_idx = if app.log_messages.len() > visible_height {
            app.log_messages.len() - visible_height
        } else {
            0
        };
        
        app.log_messages
            .iter()
            .skip(start_idx)
            .map(|msg| ListItem::new(msg.as_str()).style(Style::default().fg(Color::White)))
            .collect()
    };
    
    let logs = List::new(log_items)
        .block(Block::default()
            .borders(Borders::ALL)
            .title(format!("Messages ({}/50)", app.log_messages.len()))
        );
    
    f.render_widget(logs, area);
}
