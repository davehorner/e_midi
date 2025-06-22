use e_midi::MidiPlayer;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut player = MidiPlayer::new()?;
    let numsongs = player.get_total_song_count();
    println!("Total songs: {}", player.get_total_song_count());
    // Play the first song (index 0)
    // Loop through all songs and play each one
    for i in 0..numsongs {
        println!("Playing song {}", i + 1);
        player.play_song(i, None, None)?;
        // Wait until the song finishes before playing the next one
        while player.is_playing() {
            std::thread::sleep(std::time::Duration::from_millis(500));
        }
    }
    // Keep the program alive while the song plays
    Ok(())
}
