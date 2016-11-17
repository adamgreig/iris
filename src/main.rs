mod tts;
mod portfire;
mod script;

fn main() {

    let s = script::Script::from_file("./script.txt").unwrap();
    println!("{:?}", s);

    /*
    tts::init();

    tts::say("Discovering portfires");
    let boards = portfire::autodiscover().unwrap();

    match boards.len() {
        0 => { tts::say("No boards found"); return; },
        1 => tts::say("One board found"),
        i => tts::say(&format!("{} boards found", i)),
    }

    for board in boards {

        tts::say("Pinging");
        match board.ping() {
            Ok(_) => tts::say("OK"),
            Err(_) => tts::say("Error"),
        }

        tts::say("Checking bus voltage");
        match board.bus_voltage() {
            Ok(v) => tts::say(&format!("{:.0} volt", v)),
            Err(_) => tts::say("Error"),
        }

        tts::say("Checking continuities");
        let conts = board.continuities().unwrap();
        let channels: Vec<String> = conts.iter()
                                        .enumerate()
                                        .filter(|&(_, cont)| *cont != 255)
                                        .map(|(idx, _)| (idx+1).to_string())
                                        .collect();
        if channels.len() == 0 {
            tts::say("No channels connected");
        } else {
            tts::say(&format!("channels {} connected", channels.join(",")));
        }

        tts::say("Arming");
        match board.arm() {
            Ok(_) => tts::say("OK"),
            Err(_) => tts::say("Error"),
        }

        tts::say("Checking bus voltage");
        match board.bus_voltage() {
            Ok(v) => tts::say(&format!("{:.0} volt", v)),
            Err(_) => tts::say("Error"),
        }

        tts::say("Disarming");
        match board.disarm() {
            Ok(_) => tts::say("OK"),
            Err(_) => tts::say("Error"),
        }
    }
    */
}

