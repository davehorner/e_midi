use rosc::{OscPacket, OscMessage, OscType};
use std::net::UdpSocket;
use std::thread::sleep;
use std::time::Duration;

fn main() -> std::io::Result<()> {
    let addr = "127.0.0.1:57120";
    let socket = UdpSocket::bind("0.0.0.0:0")?;

    let pattern = ["bd", "sn"];
    let mut i = 0;
    loop {
        let msg = OscMessage {
            addr: "/dirt/play".to_string(),
            args: vec![
                OscType::String("s".to_string()), OscType::String(pattern[i % 2].to_string()),
                OscType::String("gain".to_string()), OscType::Float(0.8),
                OscType::String("orbit".to_string()), OscType::Int(0),
            ],
        };
        let packet = OscPacket::Message(msg);
        let buf = rosc::encoder::encode(&packet).unwrap();

        socket.send_to(&buf, addr)?;
        println!("Sent: {}", pattern[i % 2]);
        i += 1;
        sleep(Duration::from_millis(500));
    }
}