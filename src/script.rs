use std::io;
use std::io::prelude::*;
use std::io::BufReader;
use std::path::Path;
use std::fs::File;
use std::collections::HashMap;

#[derive(Debug)]
pub enum ScriptError {
    Io(io::Error),
    Parse { lineno: usize, error: &'static str },
    DuplicateBoardId { lineno: usize, board_id: String },
    DuplicateBoardMac { lineno: usize, mac_address: [u8; 6] },
    DuplicateChannel { lineno: usize, name: String },
    InvalidChannelNum { lineno: usize, num: u8 },
    UnknownBoardId { lineno: usize, board_id: String },
    ChannelRefired { lineno: usize, channel: String },
    UndefinedChannel { lineno: usize, channel: String },
    ExcessChannelsPerBoard { lineno: usize, board_id: String },
    FireWithoutSleep { lineno: usize },
}

impl From<io::Error> for ScriptError {
    fn from(err: io::Error) -> ScriptError {
        ScriptError::Io(err)
    }
}

pub type ScriptResult<T> = Result<T, ScriptError>;

impl ScriptError {
    fn parse_err<T>(lineno: usize, error: &'static str) -> ScriptResult<T> {
        Err(ScriptError::Parse { lineno: lineno, error: error })
    }

    fn parse_err_numargs<T>(lineno: usize) -> ScriptResult<T> {
        Err(ScriptError::Parse { lineno: lineno,
                                 error: "Wrong number of arguments" })
    }
}

#[derive(Debug, PartialEq)]
pub enum Cue {
    Board { board_id: String, mac_address: [u8; 6] },
    Channel { name: String, board_id: String, num: u8 },
    Fire { channels: Vec<String> },
    Say { message: String },
    Print { message: String },
    Sleep { time: u64 },
    Pause,
}

#[derive(Debug, PartialEq)]
pub struct Script {
    pub cues: Vec<Cue>,
    pub boards: HashMap<String, [u8; 6]>,
    pub channels: HashMap<String, (String, u8)>,
    pub duration: u64,
}

impl Cue {
    fn from_line(line: &String, lineno: usize) -> ScriptResult<Option<Cue>> {
        let args: Vec<&str> = line.split_whitespace().collect();
        match args.first() {
            // Empty lines and comments are ignored
            None | Some(&"#") => Ok(None),

            Some(&word) => match word {

                // Parse a "say" command, where the entire rest of the line
                // is the message to say.
                "say" => {
                    if args.len() < 2 {
                        Ok(Some(Cue::Say { message: "".to_string() } ))
                    } else {
                        let (_, message) = line.split_at(4);
                        Ok(Some(Cue::Say{ message: String::from(message) }))
                    }
                },

                // Parse a "print" command. Like a "say" command, the rest of
                // the line is the message to print.
                "print" => {
                    if args.len() < 2 {
                        Ok(Some(Cue::Print { message: "".to_string() }))
                    } else {
                        let (_, message) = line.split_at(6);
                        Ok(Some(Cue::Print{ message: String::from(message) }))
                    }
                },

                // Parse a "sleep" command. The single argument must be an
                // integer number of seconds to sleep.
                "sleep" => {
                    if args.len() != 2 {
                        return ScriptError::parse_err_numargs(lineno);
                    }

                    match args[1].parse() {
                        Ok(time) => Ok(Some(Cue::Sleep { time: time })),
                        _ => ScriptError::parse_err(lineno, "Invalid sleep time")
                    }
                }

                // Parse a "board" command. There's a board_id and a
                // colon-delimited MAC address.
                "board" => {
                    if args.len() != 3 {
                        return ScriptError::parse_err_numargs(lineno);
                    }

                    let octets: Vec<u8> = args[2].split(":")
                                                 .filter_map(|x| u8::from_str_radix(x, 16).ok())
                                                 .collect();
                    if octets.len() != 6 {
                        return ScriptError::parse_err(lineno, "Invalid MAC address")
                    }

                    Ok(Some(Cue::Board {
                        board_id: String::from(args[1]),
                        mac_address: [octets[0], octets[1], octets[2],
                                      octets[3], octets[4], octets[5]]
                    }))
                },

                // Parse a "channel" command. There's a channel name and a
                // mapped board_id and channel_num.
                "channel" => {
                    if args.len() != 4 {
                        return ScriptError::parse_err_numargs(lineno);
                    }

                    match args[3].parse() {
                        Ok(num) => Ok(Some(Cue::Channel {
                            name: String::from(args[1]),
                            board_id: String::from(args[2]),
                            num: num
                        })),
                        _ => ScriptError::parse_err(lineno, "Invalid firing channel")
                    }
                },

                // Parse a "fire" command. Each argument is a firing channel name.
                "fire" => {
                    if args.len() < 2 {
                        return ScriptError::parse_err_numargs(lineno);
                    }

                    let channels: Vec<String> = args[1..].iter().map(|s| s.to_string()).collect();

                    Ok(Some(Cue::Fire { channels: channels }))
                },

                // Parse a "pause" command. No arguments.
                "pause" => {
                    if args.len() != 1 {
                        return ScriptError::parse_err_numargs(lineno);
                    }

                    Ok(Some(Cue::Pause))
                },

                // Any other command is an error.
                _ => ScriptError::parse_err(lineno, "Invalid command")
            }
        }
    }
}

impl Script {
    pub fn from_file<P: AsRef<Path>>(path: P) -> ScriptResult<Script> {
        let f = File::open(path)?;
        let bf = BufReader::new(&f);
        Script::from_bufreader(bf)
    }

    pub fn from_string(script: String) -> ScriptResult<Script> {
        let bf = BufReader::new(script.as_bytes());
        Script::from_bufreader(bf)
    }

    fn from_bufreader<B: BufRead>(bf: B) -> ScriptResult<Script> {
        let mut cues: Vec<Cue> = Vec::new();
        let mut duration = 0;
        let mut boards: HashMap<String, [u8; 6]> = HashMap::new();
        let mut channels: HashMap<String, (String, u8)> = HashMap::new();
        let mut channels_fired: Vec<String> = Vec::new();
        let mut sleep_since_fire = true;

        for (lineno, line) in bf.lines().enumerate() {
            match Cue::from_line(&line?, lineno+1)? {
                Some(cue) => {

                    match &cue {
                        // For board cues, add the board to the script
                        &Cue::Board { ref board_id, ref mac_address } => {
                            // Check board name not already used
                            if boards.contains_key(board_id) {
                                return Err(ScriptError::DuplicateBoardId {
                                    lineno: lineno+1, board_id: board_id.clone()
                                });
                            }

                            // Check board MAC not already used
                            for &mac in boards.values() {
                                if mac == *mac_address {
                                    return Err(ScriptError::DuplicateBoardMac {
                                        lineno: lineno+1, mac_address: *mac_address
                                    });
                                }
                            }

                            boards.insert(board_id.clone(), mac_address.clone());
                        },

                        // For channel cues, add the channel to the script
                        &Cue::Channel { ref name, ref board_id, ref num } => {
                            // Check channel name not already used
                            if channels.contains_key(name) {
                                return Err(ScriptError::DuplicateChannel {
                                    lineno: lineno+1, name: name.clone()
                                });
                            }

                            // Check board+num not already used
                            for &(ref ch_board, ref ch_num) in channels.values() {
                                if *board_id == *ch_board && *num == *ch_num {
                                    return Err(ScriptError::DuplicateChannel {
                                        lineno: lineno+1, name: name.clone()
                                    });
                                }
                            }

                            // Check board exists
                            if !boards.contains_key(board_id) {
                                return Err(ScriptError::UnknownBoardId {
                                    lineno: lineno+1,
                                    board_id: board_id.clone()
                                });
                            }

                            // Check num is 1..30
                            if *num == 0 || *num > 30 {
                                return Err(ScriptError::InvalidChannelNum {
                                    lineno: lineno+1,
                                    num: *num
                                });
                            }

                            channels.insert(name.clone(), (board_id.clone(), *num));
                        },

                        // For sleep cues, accumulate total time slept,
                        // and record that we've seen a sleep since the
                        // last fire cue.
                        &Cue::Sleep { time } => {
                            duration += time;
                            sleep_since_fire = true;
                        },

                        // For pause cues, just update the sleep_since_fire.
                        &Cue::Pause => sleep_since_fire = true,

                        // For fire cues, check all channel names are defined,
                        // check no more than three channels fired per board,
                        // and check there has been a sleep cue since the last
                        // fire cue.
                        &Cue::Fire { channels: ref cue_channels } =>
                        {
                            // Check we've slept since the previous Fire cue
                            if !sleep_since_fire {
                                return Err(ScriptError::FireWithoutSleep {
                                    lineno: lineno+1
                                });
                            }

                            // Store a count of how many channels have been fired on each
                            // board, so we can enforce the 3-per-go limit.
                            let mut board_counts: HashMap<String, usize> = HashMap::new();
                            for board in boards.keys() {
                                board_counts.insert(board.clone(), 0);
                            }

                            for channel in cue_channels {
                                // Check channel hasn't already been fired
                                if channels_fired.contains(channel) {
                                    return Err(ScriptError::ChannelRefired {
                                        lineno: lineno+1, channel: channel.clone()
                                    });
                                }

                                // Check channel has been defined
                                if !channels.contains_key(channel) {
                                    return Err(ScriptError::UndefinedChannel {
                                        lineno: lineno+1, channel: channel.clone()
                                    });
                                }

                                // Check board fire count
                                let (ref board_id, _) = channels[channel];
                                *board_counts.entry(board_id.clone()).or_insert(0) += 1;
                                if board_counts[board_id] > 3 {
                                    return Err(ScriptError::ExcessChannelsPerBoard {
                                        lineno: lineno+1, board_id: board_id.clone()
                                    });
                                }

                                channels_fired.push(channel.clone());
                            }

                            sleep_since_fire = false;
                        },

                        // Don't care about any other cue types specifically
                        _ => {},
                    };

                    cues.push(cue);
                },
                None => {},
            }
        }

        Ok(Script { cues: cues, boards: boards, duration: duration, channels: channels })
    }
}

#[cfg(test)]
mod tests {
    use super::{Script, Cue};
    use std::collections::HashMap;

    #[test]
    fn empty_script() {
        let script_string = "".to_string();
        let script = Script::from_string(script_string).unwrap();
        assert_eq!(script,
            Script { cues: vec![], boards: HashMap::new(), channels: HashMap::new(), duration: 0 })
    }

    #[test]
    #[should_panic(expected="Invalid command")]
    fn invalid_cmd() {
        let script_string = "
        invalid command
        ".to_string();

        Script::from_string(script_string).unwrap();
    }

    #[test]
    fn ignore_comments() {
        let script_string_1 = "
        say hello
        sleep 5
        ".to_string();

        let script_string_2 = "
        # this is a comment
        say hello
        # another comment
        sleep 5
        ".to_string();

        assert_eq!(
            Script::from_string(script_string_1).unwrap(),
            Script::from_string(script_string_2).unwrap()
        );
    }

    #[test]
    fn duration() {
        let script_string = "
        sleep 5
        sleep 3
        sleep 2
        ".to_string();

        assert_eq!(
            Script::from_string(script_string).unwrap().duration,
            10
        );
    }

    #[test]
    #[should_panic(expected="Invalid sleep time")]
    fn invalid_sleep_time() {
        let script_string = "
        sleep a
        ".to_string();
        Script::from_string(script_string).unwrap();
    }

    #[test]
    fn boards() {
        let script_string = "
        board 001 00:01:02:03:04:05
        board 002 aa:bb:cc:dd:ee:ff
        ".to_string();

        let mut boards = HashMap::new();
        boards.insert("001".to_string(), [0, 1, 2, 3, 4, 5]);
        boards.insert("002".to_string(), [170, 187, 204, 221, 238, 255]);

        assert_eq!(Script::from_string(script_string).unwrap().boards, boards);
    }

    #[test]
    #[should_panic(expected="DuplicateBoardId")]
    fn duplicate_board_name() {
        let script_string = "
        board 001 00:01:02:03:04:05
        board 001 aa:bb:cc:dd:ee:ff
        ".to_string();
        Script::from_string(script_string).unwrap();
    }

    #[test]
    #[should_panic(expected="DuplicateBoardMac")]
    fn duplicate_board_mac() {
        let script_string = "
        board 001 00:01:02:03:04:05
        board 002 00:01:02:03:04:05
        ".to_string();
        Script::from_string(script_string).unwrap();
    }

    #[test]
    #[should_panic(expected="Invalid MAC address")]
    fn invalid_mac_address() {
        let script_string = "
        board 001 00:01:02:03:04:ZZ
        ".to_string();
        Script::from_string(script_string).unwrap();
    }

    #[test]
    #[should_panic(expected="Invalid MAC address")]
    fn invalid_mac_address_2() {
        let script_string = "
        board 001 hello
        ".to_string();
        Script::from_string(script_string).unwrap();
    }

    #[test]
    fn channels() {
        let script_string = "
        board 001 00:00:00:00:00:00
        channel ch1 001 1
        channel ch2 001 2
        ".to_string();

        let mut channels = HashMap::new();
        channels.insert("ch1".to_string(), ("001".to_string(), 1));
        channels.insert("ch2".to_string(), ("001".to_string(), 2));

        assert_eq!(Script::from_string(script_string).unwrap().channels, channels);
    }

    #[test]
    #[should_panic(expected="DuplicateChannel")]
    fn duplicate_channels() {
        let script_string = "
        board 001 00:00:00:00:00:00
        channel ch1 001 1
        channel ch1 001 2
        ".to_string();
        Script::from_string(script_string).unwrap();
    }

    #[test]
    #[should_panic(expected="DuplicateChannel")]
    fn duplicate_board_channels() {
        let script_string = "
        board 001 00:00:00:00:00:00
        channel ch1 001 1
        channel ch2 001 1
        ".to_string();
        Script::from_string(script_string).unwrap();
    }

    #[test]
    #[should_panic(expected="UnknownBoardId")]
    fn unknown_channel_board() {
        let script_string = "
        channel ch1 001 1
        ".to_string();
        Script::from_string(script_string).unwrap();
    }

    #[test]
    #[should_panic(expected="Invalid firing channel")]
    fn invalid_firing_channel() {
        let script_string = "
        board 001 00:00:00:00:00:00
        channel ch1 001 a
        ".to_string();
        Script::from_string(script_string).unwrap();
    }

    #[test]
    #[should_panic(expected="InvalidChannelNum")]
    fn invalid_firing_channel_2() {
        let script_string = "
        board 001 00:00:00:00:00:00
        channel ch1 001 31
        ".to_string();
        Script::from_string(script_string).unwrap();
    }

    #[test]
    #[should_panic(expected="FireWithoutSleep")]
    fn fire_without_sleep() {
        let script_string = "
        board 001 00:00:00:00:00:00
        channel ch1 001 1
        channel ch2 001 2
        sleep 1
        fire ch1
        fire ch2
        sleep 1
        ".to_string();
        Script::from_string(script_string).unwrap();
    }

    #[test]
    #[should_panic(expected="ChannelRefired")]
    fn channel_refired() {
        let script_string = "
        board 001 00:00:00:00:00:00
        channel ch1 001 1
        fire ch1
        sleep 1
        fire ch1
        ".to_string();
        Script::from_string(script_string).unwrap();
    }

    #[test]
    #[should_panic(expected="UndefinedChannel")]
    fn undefined_channel() {
        let script_string = "
        fire ch1
        ".to_string();
        Script::from_string(script_string).unwrap();
    }

    #[test]
    #[should_panic(expected="ExcessChannelsPerBoard")]
    fn excess_channels_per_board() {
        let script_string = "
        board 001 00:00:00:00:00:00
        channel ch1 001 1
        channel ch2 001 2
        channel ch3 001 3
        channel ch4 001 4
        fire ch1 ch2 ch3 ch4
        ".to_string();
        Script::from_string(script_string).unwrap();
    }

    #[test]
    fn complete_script() {
        let script_string = "
        board 001 00:00:00:00:00:01\n\
        board 002 00:00:00:00:00:02\n\
        channel ch1 001 1\n\
        channel ch2 001 2\n\
        channel ch3 001 3\n\
        channel ch4 001 4\n\
        channel ch5 001 5\n\
        channel chA 002 1\n\
        channel chB 002 2\n\
        channel chC 002 3\n\
        print\n\
        print Hello\n\
        say Hello\n\
        say\n\
        pause\n\
        sleep 1\n\
        fire ch1 chA chB chC\n\
        sleep 2\n\
        fire ch2 ch3 ch4\n\
        pause\n\
        fire ch5\n\
        ".to_string();
        let script = Script::from_string(script_string).unwrap();

        let cues = vec![
            Cue::Board { board_id: "001".to_string(), mac_address: [0, 0, 0, 0, 0, 1] },
            Cue::Board { board_id: "002".to_string(), mac_address: [0, 0, 0, 0, 0, 2] },
            Cue::Channel { name: "ch1".to_string(), board_id: "001".to_string(), num: 1 },
            Cue::Channel { name: "ch2".to_string(), board_id: "001".to_string(), num: 2 },
            Cue::Channel { name: "ch3".to_string(), board_id: "001".to_string(), num: 3 },
            Cue::Channel { name: "ch4".to_string(), board_id: "001".to_string(), num: 4 },
            Cue::Channel { name: "ch5".to_string(), board_id: "001".to_string(), num: 5 },
            Cue::Channel { name: "chA".to_string(), board_id: "002".to_string(), num: 1 },
            Cue::Channel { name: "chB".to_string(), board_id: "002".to_string(), num: 2 },
            Cue::Channel { name: "chC".to_string(), board_id: "002".to_string(), num: 3 },
            Cue::Print { message: "".to_string() },
            Cue::Print { message: "Hello".to_string() },
            Cue::Say { message: "Hello".to_string() },
            Cue::Say { message: "".to_string() },
            Cue::Pause,
            Cue::Sleep { time: 1 },
            Cue::Fire { channels: vec!["ch1".to_string(), "chA".to_string(),
                                       "chB".to_string(), "chC".to_string()] },
            Cue::Sleep { time: 2 },
            Cue::Fire { channels: vec!["ch2".to_string(), "ch3".to_string(), "ch4".to_string() ] },
            Cue::Pause,
            Cue::Fire { channels: vec!["ch5".to_string()] }
        ];

        let mut boards: HashMap<String, [u8; 6]> = HashMap::new();
        boards.insert("001".to_string(), [0, 0, 0, 0, 0, 1]);
        boards.insert("002".to_string(), [0, 0, 0, 0, 0, 2]);

        let mut channels: HashMap<String, (String, u8)> = HashMap::new();
        channels.insert("ch1".to_string(), ("001".to_string(), 1));
        channels.insert("ch2".to_string(), ("001".to_string(), 2));
        channels.insert("ch3".to_string(), ("001".to_string(), 3));
        channels.insert("ch4".to_string(), ("001".to_string(), 4));
        channels.insert("ch5".to_string(), ("001".to_string(), 5));
        channels.insert("chA".to_string(), ("002".to_string(), 1));
        channels.insert("chB".to_string(), ("002".to_string(), 2));
        channels.insert("chC".to_string(), ("002".to_string(), 3));

        assert_eq!(script.cues, cues);
        assert_eq!(script.boards, boards);
        assert_eq!(script.channels, channels);
        assert_eq!(script.duration, 3);
    }
}
