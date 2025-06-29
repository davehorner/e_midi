use rosc::{OscMessage, OscPacket, OscType};
use std::io::{self, Write};
use std::net::UdpSocket;

fn main() -> io::Result<()> {
    let addr = "127.0.0.1:57126"; // Use our custom OSC Tidal server port
    let socket = UdpSocket::bind("0.0.0.0:0")?;

    println!(
        "TidalCycles OSC interactive shell (custom OSC server on 57126). Type 'quit' to exit."
    );
    println!("Type any TidalCycles code or pattern (e.g. d1 $ s \"bd sn\"), or a command like hush, and press Enter.");
    println!("Your input will be sent exactly as typed to the OSC server.");

    // Send a sample pattern to d1 on startup
    let sample_pattern = "d1 $ s \"bd sn\" # gain \"0.8\" # orbit \"0\"";
    let msg = OscMessage {
        addr: "/tidal".to_string(),
        args: vec![OscType::String(sample_pattern.to_string())],
    };
    let packet = OscPacket::Message(msg);
    let buf = rosc::encoder::encode(&packet).unwrap();
    socket.send_to(&buf, addr)?;
    println!("[debug] Sent to {}: /tidal -> {}", addr, sample_pattern);

    loop {
        print!("Enter Tidal code (or 'quit'): ");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();
        if input.eq_ignore_ascii_case("quit") {
            break;
        }
        if input.is_empty() {
            continue;
        }
        let msg = OscMessage {
            addr: "/tidal".to_string(),
            args: vec![OscType::String(input.to_string())],
        };
        let packet = OscPacket::Message(msg);
        let buf = rosc::encoder::encode(&packet).unwrap();
        socket.send_to(&buf, addr)?;
        println!("[debug] Sent to {}: /tidal -> {}", addr, input);
    }
    Ok(())
}
