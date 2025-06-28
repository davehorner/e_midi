use std::net::UdpSocket;
use std::thread::sleep;
use std::time::Duration;

fn main() {
    // Compose OSC message to /eval with arbitrary code
    // This code will play a drum pattern and stop itself after 4 seconds using an envelope
    // https://supercollider.github.io/examples - Thor Magnusson, 2006
    let auto_stop_code = r#"{
        var snare, bdrum, hihat, env;
        var tempo = 4;
        tempo = Impulse.ar(tempo); // for a drunk drummer replace Impulse with Dust !!!
        snare = WhiteNoise.ar(Decay2.ar(PulseDivider.ar(tempo, 4, 2), 0.005, 0.5));
        bdrum = SinOsc.ar(Line.ar(120,60, 1), 0, Decay2.ar(PulseDivider.ar(tempo, 4, 0), 0.005, 0.5));
        hihat = HPF.ar(WhiteNoise.ar(1), 10000) * Decay2.ar(tempo, 0.005, 0.5);
        // Envelope: 4 seconds duration, doneAction: 2 auto-frees the synth
        env = EnvGen.kr(Env.linen(0.01, 4, 0.1), doneAction: 2);
        Out.ar(0, (snare + bdrum + hihat) * 0.4 * env ! 2)
    }.play; // This synth will stop itself after 4 seconds
    "#;
    let auto_stop_packet = rosc::encoder::encode(&rosc::OscPacket::Message(
        rosc::OscMessage {
            addr: "/eval".to_string(),
            args: vec![rosc::OscType::String(auto_stop_code.to_string())],
        },
    ))
    .unwrap();
    let osc_addr = "127.0.0.1:57120";
    let sock = UdpSocket::bind("0.0.0.0:0").expect("bind");
    println!("Sending /eval OSC message to SuperCollider (auto-stop drum pattern):");
    println!("{}", auto_stop_code);
    sock.send_to(&auto_stop_packet, osc_addr).unwrap();
    println!("Waiting 3 seconds for auto-stop drum pattern...");
    sleep(Duration::from_secs(3));

    // Compose OSC message to /eval with arbitrary code (manual stop example)
    let code = "s = { SinOsc.ar(SinOsc.kr([1, 3]).exprange(100, 2e3), 0, 0.2) }.play;";
    let osc_packet = rosc::encoder::encode(&rosc::OscPacket::Message(
        rosc::OscMessage {
            addr: "/eval".to_string(),
            args: vec![rosc::OscType::String(code.to_string())],
        },
    ))
    .unwrap();
    println!("Sending /eval OSC message to SuperCollider (manual stop example):");
    println!("{}", code);
    sock.send_to(&osc_packet, osc_addr).unwrap();
    println!("Waiting 10 seconds for sound to play...");
    sleep(Duration::from_secs(10));
    // Send another OSC message to stop all sound
    let stop_code = "s.free;";
    let stop_packet = rosc::encoder::encode(&rosc::OscPacket::Message(
        rosc::OscMessage {
            addr: "/eval".to_string(),
            args: vec![rosc::OscType::String(stop_code.to_string())],
        },
    ))
    .unwrap();
    sock.send_to(&stop_packet, osc_addr).unwrap();
    println!("Sent stop command to SuperCollider.");
    println!("Done.");
}
