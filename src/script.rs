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
    ChannelReused { board_id: String, channel: u8 },
}

impl From<io::Error> for ScriptError {
    fn from(err: io::Error) -> ScriptError {
        ScriptError::Io(err)
    }
}

pub type ScriptResult<T> = Result<T, ScriptError>;

#[derive(Debug)]
pub enum Cue {
    Fire { board_id: String, channels: Vec<u8> },
    Say { message: String },
    Print { message: String },
    Sleep { time: i32 },
    Pause,
}

#[derive(Debug)]
pub struct Script {
    pub cues: Vec<Cue>,
    pub duration: i32,
    pub num_channels: usize,
    pub channels: HashMap<String, Vec<u8>>,
}

impl Cue {
    fn from_line(line: &String, lineno: usize) -> ScriptResult<Option<Cue>> {
        let args: Vec<&str> = line.split_whitespace().collect();
        match args.first() {
            None | Some(&"#") => Ok(None),
            Some(word) => match word {

                // Parse a "say" command, where the entire rest of the line
                // is the message to say.
                &"say" => {
                    let (_, message) = line.split_at(4);
                    Ok(Some(Cue::Say{ message: String::from(message) }))
                },

                // Parse a "print" command. Like a "say" command, the rest of
                // the line is the message to print.
                &"print" => {
                    let (_, message) = line.split_at(6);
                    Ok(Some(Cue::Print{ message: String::from(message) }))
                },

                // Parse a "sleep" command. The single argument must be an
                // integer number of seconds to sleep.
                &"sleep" => match args.len() {
                    2 => match args[1].parse() {
                        Ok(time) => Ok(Some(Cue::Sleep { time: time })),
                        Err(_) => Err(ScriptError::Parse {
                            lineno: lineno, error: "Invalid sleep time" })
                    },
                    _ => Err(ScriptError::Parse {
                        lineno: lineno, error: "Too many arguments" })
                },

                // Parse a "fire" command. The first argument is a string board
                // ID, and the second argument is a comma-separated list of
                // firing channel numbers.
                &"fire" => match args.len() {
                    3 => {
                        let mut channels = Vec::new();
                        for channel in args[2].split(",") {
                            match channel.parse() {
                                Ok(channel) => channels.push(channel),
                                Err(_) => return Err(ScriptError::Parse {
                                    lineno: lineno, error: "Invalid firing channel"
                                })
                            }
                        }
                        Ok(Some(Cue::Fire { board_id: String::from(args[1]),
                                            channels: channels }))
                    },
                    _ => Err(ScriptError::Parse {
                        lineno: lineno, error: "Wrong number of arguments"
                    })
                },

                // Parse a "pause" command. No arguments.
                &"pause" => match args.len() {
                    1 => Ok(Some(Cue::Pause)),
                    _ => Err(ScriptError::Parse {
                        lineno: lineno, error: "Wrong number of arguments"
                    })
                },

                // Any other command is an error.
                _ => Err(ScriptError::Parse {
                    lineno: lineno, error: "Unknown command" }),
            }
        }
    }
}

impl Script {
    pub fn from_file<P: AsRef<Path>>(path: P) -> ScriptResult<Script> {
        let mut cues: Vec<Cue> = Vec::new();
        let mut duration = 0;
        let mut num_channels = 0;
        let mut channels: HashMap<String, Vec<u8>> = HashMap::new();

        let f = File::open(path)?;
        let bf = BufReader::new(&f);

        for (lineno, line) in bf.lines().enumerate() {
            match Cue::from_line(&line?, lineno+1)? {
                Some(cue) => {

                    match &cue {
                        // For sleep cues, accumulate total time slept
                        &Cue::Sleep { time } => {
                            duration += time;
                        },

                        // For fire cues, accumulate total number of firing
                        // channels, and keep a vec of all channels used on
                        // each different board_id.
                        &Cue::Fire { ref board_id, channels: ref cue_channels } =>
                        {
                            num_channels += cue_channels.len();

                            for channel in cue_channels {
                                // Add this board to the hash map if needed
                                if !channels.contains_key(board_id) {
                                    channels.insert(board_id.clone(), Vec::new());
                                }

                                // If we've already seen this channel, return
                                // an error about it.
                                if channels.get(board_id).unwrap().contains(channel) {
                                    return Err(ScriptError::ChannelReused {
                                        board_id: board_id.clone(),
                                        channel: *channel
                                    });
                                }

                                channels.get_mut(board_id).unwrap().push(*channel);
                            }
                        },

                        // Don't care about any other cue types
                        _ => {},
                    };

                    cues.push(cue);
                },
                None => {},
            }
        }

        Ok(Script { cues: cues, duration: duration, num_channels: num_channels,
                    channels: channels })
    }
}
