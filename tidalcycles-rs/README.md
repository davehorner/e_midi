# tidalcycles-rs

Rust tools and utilities for interacting with [TidalCycles](https://tidalcycles.org/) and [SuperCollider](https://supercollider.github.io/), including OSC, installation, and process management. This crate helps you get up and running with live-coding music as quickly as possible.

## Features
- Launch and control TidalCycles (via GHCi) and SuperCollider from Rust
- Send OSC messages to TidalCycles or SuperCollider/SuperDirt
- Automate booting, pattern sending, and live-coding workflows
- Async and multi-threaded support via Tokio
- Example code for sending patterns, arpeggios, and more
- Windows automation for installing all dependencies - MacOS/Linux may come in the future.
- Downloads and installs the latest github [sc3-plugins](https://github.com/supercollider/sc3-plugins/).
- Automatically installs the [TidalLooper Quark](https://github.com/thgrund/tidal-looper) for SuperDirt live sampling/looping.

The core concept is: run tidalcycles-rs, it installs the required software (if needed) and it makes noise without a lot of additional effort.

## Quick Start
1. **Install Rust** ([rustup.rs](https://rustup.rs/))
2. **Clone this repo**
3. **Run the main automation binary:**
   ```sh
   cargo run --bin tidalcycles-rs
   ```
   This will:
   - Automatically check for and install (if needed) GHC, Cabal, TidalCycles, and SuperCollider.
   - Launch SuperCollider in headless mode with SuperDirt, setting up OSC code evaluation and sample loading.
   - Launch TidalCycles (GHCi) with a robust boot script, exposing all standard pattern aliases (`d1`-`d8`, `hush`, etc.).
   - Start an OSC server on port 57126, allowing you to send Tidal code from other programs (like the example sender).
   - Ensure all processes are cleaned up on exit (Ctrl+C), including SuperCollider and GHCi.
   - Print all SuperCollider and TidalCycles output to your terminal for debugging and live feedback.

4. **In a separate terminal, run the example sender:**
   ```sh
   cargo run --bin send_tidal_patterns
   ```
   This will send a series of Tidal patterns (including an arpeggiated chord progression) to the running TidalCycles instance via OSC.

---

## About the `tidalcycles-rs` Main Binary

The `tidalcycles-rs` binary is a robust automation tool for live-coding with TidalCycles and SuperDirt. It:
- **Automates installation**: Checks for and installs GHC, Cabal, TidalCycles, and SuperCollider if missing (Windows support is most complete).
- **Boots SuperCollider headlessly**: Starts SuperCollider with a custom startup script that loads SuperDirt, sets up OSC code evaluation, and prints debug output.
- **Boots TidalCycles (GHCi)**: Launches GHCi with a modern boot script, exposing all standard Tidal pattern aliases and helpers.
- **OSC Server for Tidal**: Listens on UDP port 57126 for `/tidal` OSC messages, allowing you to inject Tidal code from any OSC-capable client.
- **Process management**: Tracks and cleans up all child processes (SuperCollider, GHCi) on exit or Ctrl+C.
- **Live feedback**: Prints all SuperCollider and TidalCycles output (stdout/stderr) to your terminal for easy debugging and monitoring.
- **Extensible**: Designed for headless, automated, and programmatic workflows—ideal for scripting, testing, or integrating Tidal into larger systems.

You can use this binary as a drop-in Tidal/SuperDirt server for automated or remote control, or as a foundation for your own Rust-based live-coding tools.

---

## Requirements
- [TidalCycles](https://tidalcycles.org/) (including SuperDirt and GHCi)
- [SuperCollider](https://supercollider.github.io/)
- [GHC (Glasgow Haskell Compiler)](https://www.haskell.org/ghc/)
- [Cabal](https://www.haskell.org/cabal/)
- Rust (latest stable)
- TidalCycles must be available in your PATH (or specify the path in your code)

### TidalCycles Requirements
- [SuperDirt](https://github.com/musikinformatik/SuperDirt) (SuperCollider extension for TidalCycles)
- A compatible text editor (e.g., Atom, VSCode, Emacs) with TidalCycles integration

## Platform Support
On Windows, this project automates the installation of all required components for TidalCycles and its dependencies, making setup easier. Support for additional platforms may be added in the future.

## How It Works
The `tidalcycles-rs` binary automatically starts SuperCollider with SuperDirt and launches TidalCycles, providing a ready-to-use environment for live coding. You can send patterns via OSC or automate workflows from Rust.

## Binaries in `src/bin`

This crate provides several example and utility binaries for interacting with TidalCycles and SuperDirt:

- **tcrs_dirt_osc.rs**: Minimal example that sends alternating OSC messages to SuperDirt, triggering the built-in "bd" and "sn" samples directly via `/dirt/play`.
- **tcrs_dirt_sample_iter.rs**: Scans your Dirt-Samples directory, iterates over all sample banks, and plays either the first or all samples in each bank via OSC to SuperDirt. Useful for exploring available samples.
- **tcrs_interactive_tidal.rs**: Interactive command-line shell for sending arbitrary TidalCycles code or commands (e.g. `d1 $ s "bd sn"`, `hush`) to the custom OSC Tidal server (port 57126). Sends your input exactly as typed.
- **tcrs_osc_tidal_patterns.rs**: Sends a series of example Tidal patterns (including arpeggios and effects) to the custom OSC Tidal server (port 57126) for automated demo/testing.
- **tcrs_supercolider_osc_eval.rs**: Sends arbitrary SuperCollider code to SuperDirt's `/eval` OSC handler, allowing you to trigger synths, play patterns, or evaluate code remotely.
- **tcrs_tidal_ghci.rs**: Demonstrates how to launch a TidalCycles GHCi session from Rust, inject boot code, and set up pattern aliases programmatically.

Each binary is self-contained and can be run with:

```sh
cargo run --bin <binary_name>
```

See the source of each file in `src/bin/` for more details and usage examples.


## Troubleshooting
- If `ghci` or `tidal` is not found, ensure GHC and TidalCycles are installed and in your PATH.
- For more help, see the [TidalCycles documentation](https://tidalcycles.org/) and [SuperDirt README](https://github.com/musikinformatik/SuperDirt).

## License
MIT OR Apache-2.0

## Author
David Horner 6/25

