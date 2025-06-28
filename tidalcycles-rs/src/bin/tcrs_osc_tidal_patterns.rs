use rosc::encoder;
use rosc::OscMessage;
use rosc::OscPacket;
use std::net::UdpSocket;
use std::thread;
use std::time::Duration;

fn main() {
    let addr = "127.0.0.1:57126";
    let sock = UdpSocket::bind("0.0.0.0:0").expect("could not bind local socket");
    println!("Sending Tidal patterns to {}", addr);

    let patterns = [
        "d1 $ s \"bd sn\"",
        "d1 $ s \"cp*4\" # speed 2",
        "d1 $ s \"hh*8\" # pan (slow 4 $ sine) # gain 0.7",
        "d1 $ every 3 (rev) $ s \"bd*2 sn*2\"",
        "d1 $ fast 2 $ s \"arpy*8\" # up 1",
        r#"d1 $ slow 2 $ n (arp "updown" "[0,4,7] [0,3,7] [0,4,7] [0,4,7,11]") # s "pluck""#,
        "hush",
    ];

    for (i, pat) in patterns.iter().enumerate() {
        let msg = OscMessage {
            addr: "/tidal".to_string(),
            args: vec![rosc::OscType::String(pat.to_string())],
        };
        let packet = OscPacket::Message(msg);
        let buf = encoder::encode(&packet).unwrap();
        sock.send_to(&buf, addr).expect("could not send OSC message");
        println!("Sent pattern {}: {}", i + 1, pat);
        thread::sleep(Duration::from_secs(2));
    }
    println!("Done sending patterns.");
}
