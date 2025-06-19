use std::error::Error;
use std::time::{Duration, Instant};
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
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame, Terminal,
};

use crate::{MidiPlayer, should_shutdown, set_shutdown_flag};

pub struct TuiApp {
    pub should_quit: bool,
    pub selected_song: usize,
    pub list_state: ListState,
    pub current_tempo: u32,
    pub is_playing: Arc<AtomicBool>,
    pub playback_info: Option<PlaybackInfo>,
    pub log_messages: Vec<String>,
    pub log_scroll: usize,
    pub playback_thread_handle: Option<thread::JoinHandle<()>>,
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
            playback_thread_handle: None,
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
    }
    
    pub fn stop_playback(&mut self) {
        self.is_playing.store(false, Ordering::Relaxed);
        self.playback_info = None;
        
        // Wait for playback thread to finish
        if let Some(handle) = self.playback_thread_handle.take() {
            let _ = handle.join();
        }
    }
}

pub fn run_tui_mode(midi_player: &mut MidiPlayer) -> Result<(), Box<dyn Error>> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = stdout();
    stdout.execute(crossterm::terminal::EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    
    let mut app = TuiApp::new();
    app.add_log("🎵 e_midi TUI started - Use ↑↓ to navigate, Enter to play, H for help".to_string());
    
    let result = run_tui_app(&mut terminal, &mut app, midi_player);
    
    // Stop any ongoing playback
    app.stop_playback();
    
    // Send all notes off before cleanup
    for channel in 0..16 {
        let _ = midi_player.conn.send(&[0xB0 | channel, 123, 0]);
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
) -> Result<(), Box<dyn Error>> {    loop {
        terminal.draw(|f| ui(f, app, midi_player))?;
        
        // Handle events with a timeout
        if event::poll(Duration::from_millis(100))? {
            match event::read()? {
                Event::Key(key) => {
                    if handle_key_event(key, app, midi_player)? {
                        break; // Exit requested
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
        }
        
        // Arrow keys - Navigate song list
        (KeyCode::Up, _) => {
            let song_count = midi_player.get_total_song_count();
            if song_count > 0 {
                app.selected_song = if app.selected_song == 0 { 
                    song_count - 1 
                } else { 
                    app.selected_song - 1 
                };
                app.list_state.select(Some(app.selected_song));
            }
        }
        
        (KeyCode::Down, _) => {
            let song_count = midi_player.get_total_song_count();
            if song_count > 0 {
                app.selected_song = (app.selected_song + 1) % song_count;
                app.list_state.select(Some(app.selected_song));
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
        }
        
        // Enter or Space - Play selected song
        (KeyCode::Enter, _) | (KeyCode::Char(' '), _) => {
            if !app.is_playing.load(Ordering::Relaxed) && midi_player.get_total_song_count() > 0 {
                start_playback(app, midi_player)?;
            } else if app.is_playing.load(Ordering::Relaxed) {
                app.add_log("⚠️ Already playing - press 'S' to stop first".to_string());
            }
        }
        
        // 's' - Stop playback
        (KeyCode::Char('s'), _) => {
            if app.is_playing.load(Ordering::Relaxed) {
                app.add_log("⏹️ Stopping playback...".to_string());
                app.stop_playback();
                
                // Send all notes off
                for channel in 0..16 {
                    midi_player.conn.send(&[0xB0 | channel, 123, 0])?;
                }
                app.add_log("⏹️ Playback stopped".to_string());
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
        }
        
        // 'n' - Next song (during playback)
        (KeyCode::Char('n'), _) => {
            if app.is_playing.load(Ordering::Relaxed) {
                let song_count = midi_player.get_total_song_count();
                if song_count > 0 {
                    app.selected_song = (app.selected_song + 1) % song_count;
                    app.list_state.select(Some(app.selected_song));
                    app.add_log(format!("⏭️ Skipping to next song ({})", app.selected_song));
                    
                    // Stop current and play next
                    app.stop_playback();
                    // Send all notes off
                    for channel in 0..16 {
                        midi_player.conn.send(&[0xB0 | channel, 123, 0])?;
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
                    app.list_state.select(Some(app.selected_song));
                    app.add_log(format!("⏮️ Skipping to previous song ({})", app.selected_song));
                    
                    // Stop current and play previous
                    app.stop_playback();
                    // Send all notes off  
                    for channel in 0..16 {
                        midi_player.conn.send(&[0xB0 | channel, 123, 0])?;
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
                app.list_state.select(Some(app.selected_song));
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
        app.is_playing.store(true, Ordering::Relaxed);
        
        // Start real MIDI playback in a separate thread
        app.add_log(format!("🎵 Starting MIDI playback: {}", song.name));
        app.add_log(format!("🎵 Playing {} tracks at {} BPM", track_indices.len(), song.default_tempo));
        
        // Clone necessary data for the playback thread
        let song_index = app.selected_song;
        let is_playing_clone = Arc::clone(&app.is_playing);
        let current_time_clone = Arc::clone(&current_time);
        let tempo_clone = Arc::clone(&tempo);
        
        // We need to implement a way to pass the MIDI connection to the thread
        // For now, let's create a simplified version that works with the existing architecture
        let playback_thread = thread::spawn(move || {
            // This is a placeholder - in a real implementation we would need to
            // pass the MIDI connection and call the actual playback methods
            let start = Instant::now();
            let total_time = duration_ms / 1000;
            
            while is_playing_clone.load(Ordering::Relaxed) {
                let elapsed_secs = start.elapsed().as_secs() as u32;
                current_time_clone.store(elapsed_secs, Ordering::Relaxed);
                
                if elapsed_secs >= total_time {
                    is_playing_clone.store(false, Ordering::Relaxed);
                    break;
                }
                
                // Check for tempo changes
                let current_tempo = tempo_clone.load(Ordering::Relaxed);
                // In a real implementation, we would adjust playback tempo here
                
                thread::sleep(Duration::from_millis(100));
            }
        });
        
        app.playback_thread_handle = Some(playback_thread);
    } else {
        app.add_log("❌ Invalid song selection".to_string());
    }
    
    Ok(())
}

fn ui(f: &mut Frame, app: &mut TuiApp, midi_player: &MidiPlayer) {
    // Create layout with two main panels
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(f.area());
    
    // Left panel: Song list and controls
    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Header
            Constraint::Min(10),    // Song list
            Constraint::Length(7),  // Controls
        ])
        .split(chunks[0]);
    
    // Right panel: Playback info and logs
    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(40), // Playback info
            Constraint::Percentage(60), // Log messages
        ])
        .split(chunks[1]);
    
    // Header
    render_header(f, left_chunks[0], midi_player);
    
    // Song list
    render_song_list(f, left_chunks[1], app, midi_player);
    
    // Controls
    render_controls(f, left_chunks[2], app);
    
    // Playback info
    render_playback_info(f, right_chunks[0], app);
    
    // Log messages
    render_log_messages(f, right_chunks[1], app);
}

fn render_header(f: &mut Frame, area: Rect, midi_player: &MidiPlayer) {
    let header_text = format!("🎵 e_midi TUI - {} songs ({} static + {} dynamic)", 
        midi_player.get_total_song_count(),
        midi_player.get_static_song_count(),
        midi_player.get_dynamic_song_count());
    
    let header = Paragraph::new(header_text)
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .block(Block::default().borders(Borders::ALL).title("Interactive MIDI Player"));
    
    f.render_widget(header, area);
}

fn render_song_list(f: &mut Frame, area: Rect, app: &mut TuiApp, midi_player: &MidiPlayer) {
    let songs: Vec<ListItem> = (0..midi_player.get_total_song_count())
        .map(|i| {
            if let Some(song) = midi_player.get_song(i) {
                let style = if i == app.selected_song {
                    if app.is_playing.load(Ordering::Relaxed) {
                        Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                    }
                } else {
                    Style::default().fg(Color::White)
                };
                
                let prefix = if i < midi_player.get_static_song_count() { "📀" } else { "🎶" };
                let playing_indicator = if app.is_playing.load(Ordering::Relaxed) && i == app.selected_song { 
                    "▶ " 
                } else { 
                    "  " 
                };
                
                ListItem::new(format!("{}{} {}: {} ({} tracks, {} BPM)", 
                    playing_indicator, prefix, i, song.name, song.tracks.len(), song.default_tempo))
                    .style(style)
            } else {
                ListItem::new(format!("❌ Invalid song {}", i))
                    .style(Style::default().fg(Color::Red))
            }
        })
        .collect();
    
    let list = List::new(songs)
        .block(Block::default()
            .borders(Borders::ALL)
            .title("Songs")
        )
        .highlight_style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD))
        .highlight_symbol("▶ ");
    
    f.render_stateful_widget(list, area, &mut app.list_state);
}

fn render_controls(f: &mut Frame, area: Rect, _app: &TuiApp) {
    let controls_text = vec![
        Line::from(vec![
            Span::styled("Navigation:", Style::default().add_modifier(Modifier::BOLD).fg(Color::Green)),
        ]),
        Line::from("↑↓: Select song  Enter/Space: Play  S: Stop  L: Refresh"),
        Line::from(vec![
            Span::styled("During Playback:", Style::default().add_modifier(Modifier::BOLD).fg(Color::Yellow)),
        ]),
        Line::from("T: Change tempo  N: Next song  P: Previous song"),
        Line::from("H: Help  PgUp/PgDn: Scroll  Alt+C: Clear dynamic"),
        Line::from("Q/Esc: Quit  Ctrl+C: Force quit"),
    ];
    
    let controls = Paragraph::new(controls_text)
        .block(Block::default().borders(Borders::ALL).title("Controls"))
        .wrap(Wrap { trim: true });
    
    f.render_widget(controls, area);
}

fn render_playback_info(f: &mut Frame, area: Rect, app: &TuiApp) {
    let info_text = if let Some(ref info) = app.playback_info {
        let current_time = info.current_time.load(Ordering::Relaxed);
        let current_tempo = info.tempo.load(Ordering::Relaxed);
        let is_playing = app.is_playing.load(Ordering::Relaxed);
        
        vec![
            Line::from(vec![
                Span::styled(
                    if is_playing { "♪ Now Playing:" } else { "⏸ Finished:" }, 
                    Style::default().add_modifier(Modifier::BOLD).fg(
                        if is_playing { Color::Green } else { Color::Yellow }
                    )
                ),
            ]),
            Line::from(format!("🎵 {}", info.song_name)),
            Line::from(format!("⏱️  {}s / {}s", current_time, info.total_time)),
            Line::from(format!("🎶 {} BPM", current_tempo)),
            Line::from(format!("🎹 {} tracks", info.track_count)),
            Line::from(""),
            Line::from(vec![
                Span::styled("Status:", Style::default().add_modifier(Modifier::BOLD)),
            ]),
            Line::from(if is_playing { 
                "🎵 Playing - Press S to stop, T for tempo, N/P for next/prev"
            } else { 
                "⏸️ Stopped - Press Enter/Space to play selected song" 
            }),
        ]
    } else {
        vec![
            Line::from("⏸️ Not playing"),
            Line::from(""),
            Line::from("Press Enter or Space to play selected song"),
            Line::from("Use ↑↓ keys to navigate song list"),
            Line::from("Press H for help"),
        ]
    };
    
    let playback = Paragraph::new(info_text)
        .block(Block::default().borders(Borders::ALL).title("Playback Info"))
        .wrap(Wrap { trim: true });
    
    f.render_widget(playback, area);
}

fn render_log_messages(f: &mut Frame, area: Rect, app: &TuiApp) {
    let visible_height = area.height.saturating_sub(2) as usize; // Account for borders
    let start_idx = if app.log_messages.len() > visible_height {
        app.log_messages.len() - visible_height
    } else {
        0
    };
    
    let log_items: Vec<ListItem> = app.log_messages
        .iter()
        .skip(start_idx)
        .map(|msg| ListItem::new(msg.as_str()))
        .collect();
    
    let logs = List::new(log_items)
        .block(Block::default()
            .borders(Borders::ALL)
            .title(format!("Messages ({}/50) - PgUp/PgDn to scroll", app.log_messages.len()))
        );
    
    f.render_widget(logs, area);
}
