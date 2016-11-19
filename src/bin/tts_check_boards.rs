extern crate iris;

use iris::portfire;

#[cfg(feature="tts")]
use iris::tts;

fn say(message: &str) {
    #[cfg(feature="tts")]
    tts::say(message);
    #[cfg(not(feature="tts"))]
    println!("SAYING: {}", message);
}

fn main() {

    #[cfg(feature="tts")]
    tts::init();

    say("Discovering portfires");
    let boards = portfire::autodiscover().unwrap();

    match boards.len() {
        0 => { say("No boards found"); return; },
        1 => say("One board found"),
        i => say(&format!("{} boards found", i)),
    }

    for board in boards {

        say("Pinging");
        match board.ping() {
            Ok(_) => say("OK"),
            Err(_) => say("Error"),
        }

        say("Checking bus voltage");
        match board.bus_voltage() {
            Ok(v) => say(&format!("{:.0} volt", v)),
            Err(_) => say("Error"),
        }

        say("Checking continuities");
        let conts = board.continuities().unwrap();
        let channels: Vec<String> = conts.iter()
                                        .enumerate()
                                        .filter(|&(_, cont)| *cont != 255)
                                        .map(|(idx, _)| (idx+1).to_string())
                                        .collect();
        if channels.len() == 1 {
            say("No channels connected");
        } else {
            say(&format!("channels {} connected", channels.join(",")));
        }

        say("Arming");
        match board.arm() {
            Ok(_) => say("OK"),
            Err(_) => say("Error"),
        }

        say("Checking bus voltage");
        match board.bus_voltage() {
            Ok(v) => say(&format!("{:.0} volt", v)),
            Err(_) => say("Error"),
        }

        say("Disarming");
        match board.disarm() {
            Ok(_) => say("OK"),
            Err(_) => say("Error"),
        }
    }
}

