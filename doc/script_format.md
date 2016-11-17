# IRIS Script Format

A script file contains many cues, one per line.

Alternatively a line may begin with the # character, in which case it is
ignored.

Each cue may be one of:

`say <text>`: speak the text out loud

`fire <board_id> <channels>`: send a fire command for the given board and
comma separated channels, must be one to three channels

`print <text>`: display the text in the message window

`sleep <time>`: sleep for the given number of seconds

`pause`: wait for user to continue the script
