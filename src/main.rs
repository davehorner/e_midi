use std::error::Error;
use e_midi::cli::run_cli;
use e_midi::set_shutdown_flag;

fn main() -> Result<(), Box<dyn Error>> {
    // Set up Ctrl+C handler
    ctrlc::set_handler(move || {
        println!("\n🛑 Received Ctrl+C, shutting down gracefully...");
        set_shutdown_flag();
        std::process::exit(0);
    })?;
    
    run_cli()
}
