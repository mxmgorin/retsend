# retsend

Transfer files between handhelds, phones and PCs over Wi-Fi, using the
[LocalSend](https://localsend.org) protocol — no cable, no SSH. Works with the
official LocalSend apps.

## Setup

1. Install [LocalSend](https://localsend.org) on your phone or PC.
2. Connect both devices to the same Wi-Fi network.
3. Launch retsend — nearby devices appear on the radar.

Send files in any direction, including handheld to handheld. Received ROMs land
in the console folder they belong to; everything else goes to the ROMs root.
Change both in Settings.

## Features

- Radar of nearby devices, or add one by IP on networks that block multicast
- Accept or decline incoming transfers, with speed, ETA and a countdown
- Gamepad file browser with multi-select, whole folders and pinned favourites
- Save routes: ROMs sorted into console folders by extension
- Transfer history; repeat a send in one press
- HTTPS by default, matching the official apps

## Controls

| Button         | Action                                                                                  |
|----------------|-----------------------------------------------------------------------------------------|
| D-pad          | Navigate · left/right switch tabs or step a setting                                     |
| A              | Send to a device · select a file · accept · repeat a send · type (keyboard)             |
| B              | Back · decline · cancel · leave the keyboard                                            |
| X              | Add a device by IP · pick where a transfer lands · delete a history row · take a folder · erase (keyboard) |
| Y              | Pin / unpin the row under the cursor                                                    |
| Start          | Confirm a send · OK (keyboard)                                                          |
| Select         | Refresh the radar · switch roots · layer (keyboard)                                     |
| L1 / R1        | Switch tabs · page the file browser                                                     |
| Select + Start | Quit                                                                                    |

## Credits

- Developed and ported by [mxmgorin](https://github.com/mxmgorin)
- Implements the [LocalSend](https://localsend.org) protocol
- Source and issues: https://github.com/mxmgorin/retsend
