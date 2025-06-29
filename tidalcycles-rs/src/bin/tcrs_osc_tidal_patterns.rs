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

    //credits to https://github.com/tedthetrumpet/tidal
    let patterns = [
        // ab: nice subtle drum sounds
        "d1 $ slow 2 $ s \"ab\" <| n (run 12)",
        // ade: various long samples
        "d1 $ s \"ade\" <| n (run 10) # cut 1",
        // ades2: meh, short quiet noisy sounds
        "d1 $ s \"ades2\" <| n (run 9) # gain 1.3",
        // ades3: short noisy sounds, lowish pitch
        "d1 $ s \"ades3\" <| n (run 7)",
        // ades4: short high pitched sounds
        "d1 $ s \"ades4\" <| n (run 6)",
        // amencutup: wisott
        "d1 $ slow 2 $ s \"amencutup\" <| n (shuffle 8 $ run 32) # speed \"{1,2,3}%8\"",
        // armora: probably useless low pitched noise
        "d1 $ slow 4 $ s \"armora\" <| n (run 7)",
        // arp: two synth notes, low and high, both c#?!?
        "d1 $ s \"arp\" <| n (run 2)",
        // superpiano: slow 4, C major scale
        "d1 $ slow 4 $ s \"superpiano\" <| n \"c d f g a c6 d6 f6 g6 a6 c7\"",
        // arpy: aha!
        "d1 $ s \"arpy\" <| up \"c d e f g a b c6\"",
        // arpy: in estuary arpy comes out a tone too high in D major! can subtract 2 maybe fixed now
        "d1 $ s \"arpy\"",
        // baa: sheep sounds, why?!?
        "d1 $ slow 4 $ s \"baa\" <| n (run 7)",
        // baa2: rather simlar to the above? same?
        "d1 $ slow 4 $ s \"baa2\" <| n (run 7)",
        // bass: four short bass sounds, nasty abrupt release
        "d1 $ slow 2 $ s \"bass\" <| n (run 4)",
        // bass0: one highly distorted bass drum, plus?!?!?
        "d1 $ s \"bass0\" <| n (run 3)",
        // bass1: thirty synth bass sounds, some long, f or c
        "d1 $ slow 8 $ s \"bass1\" <| n (run 30)",
        // bass2: five aggressive tonal kicks
        "d1 $ s \"bass2\" <| n \"[ 0 .. 4 ]\"",
        // bass3: eleven bass sounds, odd mix of pitches
        "d1 $ slow 4 $ s \"bass3!44\" # n (run 11)",
        // bassdm: 24 rather similar acoustic-ish kicks
        "d1 $ slow 4 $ s \"bassdm\" <| n (run 24)",
        // bassfoo: same bank as bass0
        "d1 $ s \"bassfoo\" <| n (run 3)",
        // bd: lots of electo kicks, mostly quite similar
        "d1 $ slow 4 $ s \"bd\" <| n (run 24)",
        // bend: four subtle noisy sounds
        "d1 $ s \"bend\" <| n (run 4)",
        // bin: two dustbin hits, kind of ok, could be a snare
        "d1 $ s \"bin\" <| n (run 2)",
        // birds: chaffinches, nightingales etc
        "d1 $ slow 4 $ s \"birds\" <| n (run 10)",
        // birds3: very short noisy sounds, highish pitch
        "d1 $ slow 2 $ s \"birds3\" <| n (run 19)",
        // bleep: rtd2 ish, loud!
        "d1 $ s \"bleep\" <| n (run 13)",
        // blip: two short pitched sounds, minor seventh apart
        "d1 $ s \"blip\" <| n (run 13)",
        // bottle: short sounds, might be a bottle
        "d1 $ slow 2 $ s \"bottle\" <| n (run 13)",
        // can: iya
        "d1 $ s \"can\" <| n (run 16) # speed \"0.125 1!15\"",
        // casio: just three cheapo 'drum' sounds
        "d1 $ s \"casio\" <| n (run 3)",
        // casio: tak
        "d1 $ fast 2 $ s \"casio\" <| n \"1 2 3 2\" # speed 0.25 # cut 1",
        // cb: omg what is that sound, so familiar! iya -- nearly same as 808:0
        "d1 $ s \"cb\"",
        // chin: very quiet synthetic clicks
        "d1 $ s \"chin\" <| n (run 4) # gain 2",
        // circus: three strange and pointless sounds
        "d1 $ s \"circus\" <| n (run 3)",
        // clak: two quiet typewriters clicks, or clock ticks?
        "d1 $ s \"clak\" <| n (run 2) # gain 2",
        // click: four glitch sounds, maybe useful
        "d1 $ s \"click\" <| n (run 4)",
        // e: 8 short and quiet glitchy sounds, similar
        "d1 $ s \"e\" <| n (run 8)",
        // east: 9 'world' drum sounds, ok
        "d1 $ slow 2 $ s \"east\" <| n (run 9)",
        // em2: six longer sounds, kalimba, flute, loon?
        "d1 $ slow 4 $ s \"em2\" <| n (run 6)",
        // feel: quite nice bank of 7 drum sounds
        "d1 $ s \"feel\" <| n (run 7)",
        // feelfx: varied effected sounds, bit longer, ok
        "d1 $ slow 2 $ s \"feelfx\" <| n (run 8)",
        // fm: whole bank of loops! inc '31 seconds…'
        "d1 $ slow 16 $ s \"fm\" <| n (run 17)",
        // gab: bitcrushed hits
        "d1 $ slow 2 $ s \"gab\" <| n (run 10)",
        // gabba: bitcrushed kit, four sounds
        "d1 $ s \"gabba\" <| n (run 4)",
        // gabbaloud: wisott
        "d1 $ s \"gabbaloud\" <| n (run 4)",
        // glitch: iya Eb/Ab stab at 5
        "d1 $ s \"glitch\" <| n (run 8)",
        // glitch2: same?!?
        "d1 $ s \"glitch2\" <| n (run 8)",
        // gtr: three long C notes elect guitar
        "d1 $ slow 4 $ s \"gtr\" <| n (run 3)",
        // h: short baby sounds?
        "d1 $ s \"h\" <| n (run 7)",
        // hand: mix of quiet clap sounds, some longer
        "d1 $ slow 8 $ s \"hand\" <| n (run 17)",
        // hardkick: 6 rather loud crushed kicks
        "d1 $ s \"hardkick\" <| n (run 6)",
        // haw: 6 odd short hits
        "d1 $ s \"haw\" <| n (run 6)",
        // hc: 6 closed hats
        "d1 $ s \"hc\" <| n (run 6)",
        // hmm: female voice saying 'hmm'
        "d1 $ s \"hmm\"",
        // hoover: six loud hoover bass soundss
        "d1 $ every 2 (fast 2) $ s \"hoover\" <| n (shuffle 6 $ run 6)",
        // house: quite a nice kit, one pitched sound at 5 ~ Ebm
        "d1 $ s \"house\" <| n (run 8)",
        // if: five bitcrushed hits
        "d1 $ s \"if\" <| n (run 5)",
        // industrial: iya mix of metallic percussive sounds
        "d1 $ slow 2 $  s \"industrial\" <| n (run 32)",
        // jazz: totally not jazzy at all kit!
        "d1 $ s \"jazz\" <| n (run 8)",
        // jungbass: mostly longish sub-bass kind of sounds
        "d1 $ slow 8 $ s \"jungbass\" <| n (run 20)",
        // jungle: quiet 'jungle' kit, amen-ish
        "d1 $ s \"jungle\" <| n (run 13)",
        // juno: lead/pad notes and chords, C/Cminor
        "d1 $ slow 4 $ s \"juno\" <| n (run 12)",
        // jvbass: selection synth notes, black notes starting Gb
        "d1 $ slow 4 $ s \"jvbass\" <| n (run 13)",
        // koy: two koyaanisqatsi long samples, more or less sample
        "d1 $ slow 4 $ s \"koy\" <| n 1",
        // kurt: vocal samples with telephone eq?
        "d1 $ slow 4 $ s \"kurt\" <| n (run 7)",
        // latibro: pentatonic selection of open 12th synth samples
        "d1 $ slow 2 $ s \"latibro\" <| n (run 8)",
        // lighter: short quiet noisy hits high pitch meh
        "d1 $ slow 4 $ s \"lighter\" <| n (run 33)",
        // linnhats: wisott
        "d1 $ s \"linnhats\" <| n (run 6)",
        // mash: low synth tom sound and sort of glitch sound, why
        "d1 $ s \"mash\" <| n (run 2)",
        // mash2: longish low syntom sounds
        "d1 $ s \"mash2\" <| n (run 4)",
        // metal: a tiny high metal tink at 10 pitches
        "d1 $ s \"metal\" <| n (run 10)",
        // metal: iya
        "d1 $ s \"metal\" <| n (run 10) # up (-24)",
        // miniyeah: very short glitchy sounds, better -24
        "d1 $ s \"miniyeah\" <| n (run 4) # up (-24)",
        // moog: long low synth notes, various pitches
        "d1 $ slow 8 $ s \"moog\" <| n (run 7)",
        // mouth: iya short vocal sounds?
        "d1 $ s \"mouth\" <| n (run 15)",
        // msg: subtle quiet hits
        "d1 $ s \"msg\" <| n (run 9)",
        // noise2: 8 short noise hits, three much louder than the others
        "d1 $ s \"noise2\" <| n (run 8)",
        // notes: same as newnotes, sines
        "d1 $ s \"notes\" <| n (run 15)",
        // numbers: female voice individual numbers
        "d1 $ slow 4 $ s \"numbers\" <| n (run 9)",
        // off: single short glitchy bass note C#
        "d1 $ s \"off\"",
        // peri: collection of synth hits, ok
        "d1 $ s \"peri\" <| n (run 15)",
        // popkick: kicks, but also tuned-ish in there
        "d1 $ s \"popkick\" <| n (run 10)",
        // print: dot matrix printer sounds, ok!
        "d1 $ slow 4 $ s \"print\" <| n (run 11)",
        // sine: sines with blunt envelopes, some very low
        "d1 $ s \"sine\" <| n (run 6)",
        // stab: polysynth/fm hits, sort of pitched not really
        "d1 $ slow 4 $ s \"stab\" <| n (run 23)",
        // stomp: mostly kicks
        "d1 $ s \"stomp\" <| n (run 10)",
        // tabla: both hits and gestures
        "d1 $ slow 8 $ s \"tabla\" <| n (run 26)",
        // tabla2: multisampled single hits
        "d1 $ slow 8 $ s \"tabla2\" <| n (run 46)",
        // v: 6 mixed electronic sounds, kind of a kit
        "d1 $ s \"v\" <| n (run 6)",
        // voodoo: actually quite a nice five sound kit
        "d1 $ s \"voodoo\" <| n (run 5)",
        // xmas: voice saying 'merry christmas'
        "d1 $ s \"xmas\"",
        // yeah: big selection of short clicks and pops, usable
        "d1 $ slow 2 $ s \"yeah\" <| n (run 31)",
        "hush",
    ];

    for (i, pat) in patterns.iter().enumerate() {
        let msg = OscMessage {
            addr: "/tidal".to_string(),
            args: vec![rosc::OscType::String(pat.to_string())],
        };
        let packet = OscPacket::Message(msg);
        let buf = encoder::encode(&packet).unwrap();
        sock.send_to(&buf, addr)
            .expect("could not send OSC message");
        println!("Sent pattern {}: {}", i + 1, pat);
        thread::sleep(Duration::from_secs(2));
    }
    println!("Done sending patterns.");
}
