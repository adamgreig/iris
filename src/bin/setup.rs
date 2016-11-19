extern crate iris;
use iris::{script, portfire};

use std::env;
use std::collections::HashMap;

fn main() {
    let args: Vec<_> = env::args().collect();
    if args.len() != 2 {
        println!("Usage: {} <script file>", args[0]);
        return;
    }

    println!("Reading script...");
    let script = script::Script::from_file(&args[1]).unwrap();
    println!("    {} boards", script.boards.len());
    println!("    {} channels", script.channels.len());
    println!("    {} cues", script.cues.len());
    println!("    {}s duration", script.duration);

    println!("Autodiscovering portfires...");
    let mut discovered_portfires = portfire::autodiscover().unwrap();
    println!("    Found {} boards, expected {}", discovered_portfires.len(), script.boards.len());

    println!("Matching boards to script...");
    let mut portfires: HashMap<String, portfire::Board> = HashMap::new();
    for (board_id, mac) in script.boards.iter() {
        let mut board_found = false;
        for portfire in &mut discovered_portfires {
            if portfire.mac == *mac {
                println!("    {} -> {}", board_id, portfire.ip);
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

    println!("\nChecking boards individually...\n");
    let mut got_error = false;

    for (board_id, board) in portfires.iter() {
        println!("");
        println!("Board {} =========================", board_id);

        board.ping().unwrap();
        println!("Ping..........................OK");

        let v = board.bus_voltage().unwrap();
        print!("Bus voltage {:.<18.2}", v);
        if v < 1.0 {
            println!("OK");
        } else {
            println!("ERROR");
            got_error = true;
        }

        let conts = board.continuities().unwrap();

        // Check all assigned channels are connected
        for (ch, &(ref ch_bid, ref ch_num)) in script.channels.iter() {
            if ch_bid == board_id {
                let ch_cont = conts[*ch_num as usize - 1];
                print!("Channel #{:02} {: <12} ", ch_num, ch);
                if ch_cont == 255 {
                    println!("not connected, ERROR");
                    got_error = true;
                } else {
                    println!("{: >3}Ω OK", ch_cont);
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
                println!("Unused channel #{:02} connected  ERROR", num);
                got_error = true;
            }
        }

        print!("Continuity voltage {:.<11.1}", conts[30] as f32 /10.0);
        if conts[30] > 30 {
            println!("OK");
        } else {
            println!("ERROR");
            got_error = true;
        }

        board.arm().unwrap();
        let v = board.bus_voltage().unwrap();
        print!("Arm voltage {:.<18.2}", v);
        if v > 3.0 {
            println!("OK");
        } else {
            println!("ERROR");
            got_error = true;
        }
        board.disarm().unwrap();
    }

    println!("");

    if got_error {
        println!("An error occurred, quitting.");
    } else {
        println!("No errors, good to go!");
    }
}
