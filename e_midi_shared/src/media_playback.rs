// Unified media playback for e_midi, supporting rodio and gstreamer backends
// mpv is always attempted first if available, then GStreamer, then rodio

#[cfg(feature = "uses_gstreamer")]
use gstreamer as gst;
use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use which::which;

/// Play audio/video using the selected backend. mpv is always attempted first if available, then GStreamer, then rodio.
pub fn play_media_file(
    song_name: &str,
    file_path: Option<&str>,
    bytes: Option<&[u8]>,
    stop_flag: Arc<AtomicBool>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Try to use mpv if available
    if let Ok(mpv_path) = which("mpv") {
        println!("[e_midi] Using mpv for playback: {}", mpv_path.display());
        let (uri, tmp_path): (String, Option<std::path::PathBuf>) = if let Some(path) = file_path {
            // Replace manual prefix strip:
            // if path.starts_with("file://") {
            //     (path[7..].to_string(), None) // strip file://
            // }
            if let Some(stripped) = path.strip_prefix("file://") {
                (stripped.to_string(), None) // strip file://
            } else {
                (path.to_string(), None)
            }
        } else {
            let tmp_dir = std::env::temp_dir();
            // Replace unnecessary rsplitn:
            // let ext = song_name.rsplitn(2, '.').next().unwrap_or("dat");
            let ext = song_name.rsplit('.').next().unwrap_or("dat");
            let tmp_path = tmp_dir.join(format!(
                "e_midi_tmp_{}.{}",
                song_name.replace(' ', "_"),
                ext
            ));
            let mut f = std::fs::File::create(&tmp_path)?;
            let data = bytes.ok_or("No data for playback")?;
            f.write_all(data)?;
            let metadata = std::fs::metadata(&tmp_path)?;
            println!(
                "[mpv] Temp file: {} ({} bytes)",
                tmp_path.display(),
                metadata.len()
            );
            (tmp_path.to_string_lossy().to_string(), Some(tmp_path))
        };
        let stop_flag_clone = stop_flag.clone();
        let song_name = song_name.to_string();
        let handle = std::thread::spawn(move || {
            let mut child = Command::new(mpv_path)
                .arg("--no-terminal")
                .arg("--force-window=yes")
                .arg("--ontop")
                .arg("--no-border")
                .arg("--no-osc")
                .arg("--loop=no")
                .arg("--keep-open=no") // Ensure mpv does not loop
                .arg(&uri)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .expect("Failed to start mpv");
            while !stop_flag_clone.load(std::sync::atomic::Ordering::Relaxed) {
                if let Ok(Some(_)) = child.try_wait() {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            // Ensure child process is waited on to avoid zombies
            let _ = child.wait();
            let _ = child.kill();
            if let Some(tmp_path) = tmp_path {
                let _ = std::fs::remove_file(&tmp_path);
            }
            println!("[mpv] Playback finished: {}", song_name);
        });
        handle.join().ok();
        return Ok(());
    }
    #[cfg(feature = "uses_gstreamer")]
    {
        use glib;
        use gst::prelude::*;
        use std::env;
        use std::fs;
        use std::io::Write;
        use std::path::PathBuf;
        use std::sync::atomic::Ordering;

        gst::init()?;
        let main_loop = glib::MainLoop::new(None, false);
        let main_loop_ref = main_loop.clone();
        let (uri, tmp_path): (String, Option<PathBuf>) = if let Some(path) = file_path {
            if path.starts_with("file://") {
                (path.to_string(), None)
            } else {
                (format!("file://{}", path), None)
            }
        } else {
            // If only bytes are provided, write to a temp file
            let tmp_dir = env::temp_dir();
            let ext = song_name.rsplitn(2, '.').next().unwrap_or("dat");
            let tmp_path = tmp_dir.join(format!(
                "e_midi_tmp_{}.{}",
                song_name.replace(' ', "_"),
                ext
            ));
            let mut f = fs::File::create(&tmp_path)?;
            let data = bytes.ok_or("No data for playback")?;
            f.write_all(data)?;
            let metadata = fs::metadata(&tmp_path)?;
            println!(
                "[GStreamer] Temp file: {} ({} bytes)",
                tmp_path.display(),
                metadata.len()
            );
            (
                format!("file://{}", tmp_path.to_string_lossy()),
                Some(tmp_path),
            )
        };
        let playbin = gst::ElementFactory::make("playbin")
            .build()
            .map_err(|_| "Failed to create playbin element")?;
        // Explicitly set audio and video sinks for Windows compatibility
        let audio_sink = gst::ElementFactory::make("autoaudiosink").build().ok();
        let video_sink = gst::ElementFactory::make("autovideosink").build().ok();
        if let Some(audio_sink) = audio_sink {
            playbin.set_property("audio-sink", &audio_sink);
        }
        if let Some(video_sink) = video_sink {
            playbin.set_property("video-sink", &video_sink);
        }
        playbin.set_property("uri", &uri);
        let bus = playbin.bus().unwrap_or_else(|| gst::Bus::new());
        playbin.set_state(gst::State::Playing)?;
        println!("▶️  Playing (GStreamer/playbin): {}", song_name);
        // Listen for EOS or ERROR
        let is_playing = Arc::new(AtomicBool::new(true));
        let is_playing_cb = is_playing.clone();
        let stop_flag_cb = stop_flag.clone();
        std::thread::spawn(move || {
            for msg in bus.iter_timed(gst::ClockTime::NONE) {
                use gst::MessageView;
                match msg.view() {
                    MessageView::Eos(..) => {
                        println!("[GStreamer] End of stream.");
                        is_playing_cb.store(false, Ordering::Relaxed);
                        main_loop_ref.quit();
                        break;
                    }
                    MessageView::Error(err) => {
                        eprintln!("[GStreamer ERROR] {}", err.error());
                        is_playing_cb.store(false, Ordering::Relaxed);
                        main_loop_ref.quit();
                        break;
                    }
                    _ => {}
                }
                if stop_flag_cb.load(Ordering::Relaxed) {
                    is_playing_cb.store(false, Ordering::Relaxed);
                    main_loop_ref.quit();
                    break;
                }
            }
        });
        main_loop.run();
        playbin.set_state(gst::State::Null)?;
        // Clean up temp file if used
        if let Some(tmp_path) = tmp_path {
            let _ = fs::remove_file(&tmp_path);
        }
        Ok(())
    }
    #[cfg(all(not(feature = "uses_gstreamer"), feature = "uses_rodio"))]
    {
        use rodio::Decoder;
        use rodio::OutputStream;
        use rodio::Sink;
        use std::io::Cursor;
        let (_stream, stream_handle) = OutputStream::try_default()?;
        let sink = Arc::new(Sink::try_new(&stream_handle)?);
        let source: Box<dyn rodio::Source<Item = f32> + Send> = if let Some(bytes) = bytes {
            Box::new(Decoder::new(Cursor::new(bytes))?.convert_samples())
        } else if let Some(path) = file_path {
            Box::new(Decoder::new(std::fs::File::open(path)?)?.convert_samples())
        } else {
            return Err("No data for rodio playback".into());
        };
        sink.append(source);
        sink.play();
        println!("▶️  Playing (rodio): {}", song_name);
        while !stop_flag.load(std::sync::atomic::Ordering::Relaxed) && !sink.empty() {
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        Ok(())
    }
    #[cfg(not(any(feature = "uses_gstreamer", feature = "uses_rodio")))]
    {
        Err("No audio backend enabled. Enable 'uses_rodio' or 'uses_gstreamer'.".into())
    }
}
