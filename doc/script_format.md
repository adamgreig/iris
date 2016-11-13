# IRIS Script Format

A script file contains many cues, one per line.

Each cue contains an optional time and one action.

If the time is not specified, the time of the previous cue in the file is used.
If the time is specified, it is given in seconds since the script starts.
In either case, the time occupies the first four columns of the cue, followed 
by a space.

Each action may be one of:

`say <text>`: speak the text, starting at the cue time

`fire <board_id> <channels>`: send a fire command for the given board and
comma separated channels, must be one to three channels

`print <text>`: display the text in the message window

`pause`: wait for user to continue the script

Example:

```
0000 say hello
     print Show Starting
     pause
0005 print Here We Go
0006 fire 001 10,11
```
