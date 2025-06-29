use std::net::UdpSocket;
use std::thread::sleep;
use std::time::Duration;

fn main() {
    // --- SuperPiano SynthDef trigger example ---
    let superpiano_code = r#"(
        // Play a C major chord using the 'superpiano' SynthDef (as Tidal does)
        [60, 64, 67, 72].do { |midi|
            Synth(\superpiano, [\freq, midi.midicps, \amp, 0.2, \sustain, 1.5]);
        };
    )"#;
    let osc_addr = "127.0.0.1:57120";
    let sock = UdpSocket::bind("0.0.0.0:0").expect("bind");
    let superpiano_packet = rosc::encoder::encode(&rosc::OscPacket::Message(rosc::OscMessage {
        addr: "/eval".to_string(),
        args: vec![rosc::OscType::String(superpiano_code.to_string())],
    }))
    .unwrap();
    println!("Sending /eval OSC message to SuperCollider (sc3-plugins SuperPiano example):");
    println!("{}", superpiano_code);
    sock.send_to(&superpiano_packet, osc_addr).unwrap();
    println!("Waiting 7 seconds to hear SuperPiano...");
    sleep(Duration::from_secs(7));

    // --- PitchShift example ---
    let pitchshift_code = r#"{
        var sig, shifted;
        sig = WhiteNoise.ar(0.2);
        // PitchShift is from sc3-plugins
        shifted = PitchShift.ar(sig, 0.2, 2.0, 0, 0.01);
        Out.ar(0, [sig, shifted]);
    }.play;
    "#;
    let pitchshift_packet = rosc::encoder::encode(&rosc::OscPacket::Message(rosc::OscMessage {
        addr: "/eval".to_string(),
        args: vec![rosc::OscType::String(pitchshift_code.to_string())],
    }))
    .unwrap();
    println!("Sending /eval OSC message to SuperCollider (sc3-plugins PitchShift example):");
    println!("{}", pitchshift_code);
    sock.send_to(&pitchshift_packet, osc_addr).unwrap();
    println!("Waiting 6 seconds to hear PitchShift...");
    sleep(Duration::from_secs(6));

    // Stop all sound on exit (free all nodes, including Synths and lingering effects)
    let stop_code = r#"s.defaultGroup.set(\gate, 0); s.freeAll;"#;
    let stop_packet = rosc::encoder::encode(&rosc::OscPacket::Message(rosc::OscMessage {
        addr: "/eval".to_string(),
        args: vec![rosc::OscType::String(stop_code.to_string())],
    }))
    .unwrap();
    sock.send_to(&stop_packet, osc_addr).unwrap();
    println!("Sent stop command to SuperCollider.");
    println!("Done.");
}
