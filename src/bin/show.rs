use std::env;
use std::collections::HashMap;
use std::thread;
use std::time::Duration;
use std::io;

extern crate iris;
use iris::portfire;
use iris::script::{self, Cue};
use iris::tts::TTS;

fn main() {
    let args: Vec<_> = env::args().collect();
    if args.len() != 2 {
        println!("Usage: {} <script file>", args[0]);
        return;
    }

    // Read script
    let script = script::Script::from_file(&args[1]).unwrap();

    // Find Portfires and map to script
    let mut discovered_portfires = portfire::autodiscover().unwrap();
    let mut portfires: HashMap<String, portfire::Board> = HashMap::new();
    for (board_id, mac) in script.boards.iter() {
        let mut board_found = false;
        for portfire in &mut discovered_portfires {
            if portfire.mac == *mac {
                portfires.insert(board_id.clone(), portfire.clone());
                board_found = true;
            }
        }
        if !board_found {
            println!("Didn't find board {} {:2X}:{:2X}:{:2X}:{:2X}:{:2X}:{:2X}",
                     board_id, mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]);
            return;
        }
    }

    // Check all portfires are behaving and have correct continuities
    let mut got_error = false;
    for (board_id, board) in portfires.iter() {
        // Check ping
        board.ping().unwrap();

        // Bus voltage while disarmed should be 0
        let v = board.bus_voltage().unwrap();
        if v > 1.0 {
            println!("Board {} bus voltage {}v ERROR", board_id, v);
            got_error = true;
        }

        // Check all continuities are correct
        let conts = board.continuities().unwrap();
        for (ch, &(ref ch_bid, ref ch_num)) in script.channels.iter() {
            if ch_bid == board_id {
                let ch_cont = conts[*ch_num as usize - 1];
                if ch_cont == 255 {
                    println!("Board {} ch#{} '{}' not connected, ERROR",
                             board_id, ch_num, ch);
                    got_error = true;
                }
            }
        }

        // Check all unassigned channels are not connected
        for num in 1..31 {
            let mut channel_used = false;
            for &(ref ch_bid, ref ch_num) in script.channels.values() {
                if ch_bid == board_id && *ch_num == num {
                    channel_used = true;
                }
            }
            if !channel_used && conts[num as usize - 1] != 255 {
                println!("Board {} unused channel #{:02} connected, ERROR",
                         board_id, num);
                got_error = true;
            }
        }

        // Check the continuity test voltage is not pulled down
        if conts[30] < 30 {
            println!("Board {} continuity voltage {}, ERROR",
                     board_id, conts[30]);
            got_error = true;
        }

        // Check the boards arm and the bus voltage comes up
        board.arm().unwrap();
        let v = board.bus_voltage().unwrap();
        if v < 3.0 {
            println!("Board {} arm voltage {}, ERROR", board_id, v);
            got_error = true;
        }
    }

    // Quit early if anything went wrong in setup
    if got_error {
        println!("An error occurred, disarming and quitting.");
        for board in portfires.values() {
            board.disarm().unwrap();
        }
        return;
    }

    // Start up the TTS engine
    let tts = TTS::new();

    // Run the show!
    for cue in script.cues {
        match cue {
            Cue::Sleep { time } => {
                thread::sleep(Duration::from_secs(time));
            },

            Cue::Pause => {
                let mut l = String::new();
                let _ = io::stdin().read_line(&mut l);
            },

            Cue::Print { message } => {
                println!("{}", message);
            },

            Cue::Say { message } => {
                tts.say(&message);
            },

            Cue::Fire { channels } => {
                // Accumulate numerical channels to fire on each board
                let mut board_channels: HashMap<String, Vec<u8>> = HashMap::new();
                for channel in channels {
                    let (ref board_id, ref ch_num) = script.channels[&channel];
                    let chs = board_channels.entry(board_id.clone())
                                            .or_insert(Vec::new());
                    (*chs).push(*ch_num);
                }

                // Send the fire commands
                for (ref board_id, ref chans) in board_channels.iter() {
                    let mut firing_chans = [0u8; 3];
                    for (idx, chan) in chans.iter().enumerate() {
                        firing_chans[idx] = *chan;
                    }
                    let portfire = &portfires[&board_id.to_string()];
                    portfire.fire(firing_chans).unwrap();
                }
            },

            _ => {},
        }
    }

    // Show over, disarm
    for board in portfires.values() {
        board.disarm().unwrap();
    }

    // Wait for final user input before quitting, in case of pending TTS
    let mut l = String::new();
    let _ = io::stdin().read_line(&mut l);
}
