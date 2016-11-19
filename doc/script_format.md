# IRIS Script Format

A script file contains many cues, one per line.

Blank lines and lines beginning with `# ` are ignored.

Each cue may be one of:

`board <board_id> <mac_address>`: configure a board ID to MAC address mapping. 
MAC address should be in usual XX:XX:XX:XX:XX:XX format.

`channel <channel_name> <board_id> <channel_num>`: configure a mapping between 
a board and channel number (physical channel) and a name used for firing.

`say <text>`: speak the text out loud

`fire <channel> [channel]...`: send a fire command for one or more
space-separated channel names. Note you must have sleep/pause cues between fire 
cues, and you are limited to three channels on the same board per cue.

`print <text>`: display the text in the message window

`sleep <time>`: sleep for the given number of seconds

`pause`: wait for user to continue the script
