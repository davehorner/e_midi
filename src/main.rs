use std::error::Error;
use e_midi::cli::run_cli;

fn main() -> Result<(), Box<dyn Error>> {
    run_cli()
}
