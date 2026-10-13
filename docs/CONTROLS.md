# Controls

| Pad | Keyboard | Action |
| --- | --- | --- |
| D-pad / stick | Arrows | Navigate; left/right switch tabs or step a setting |
| A | Enter | Send to a device, select a file, accept, repeat a send, type |
| B | Esc | Back, decline, cancel, leave the keyboard |
| X | X / Backspace | Add a device by IP, pick where a transfer lands, delete a history row, erase a character, take a folder |
| Y | Y | Pin or unpin the row under the cursor |
| Start | F1 | Confirm a send, OK on the keyboard |
| Select | Tab / F5 | Refresh the radar, switch roots, switch the keyboard layer |
| L1 / R1 | PgUp / PgDn | Switch tabs, page the file browser |

On the PS Vita, A, B, X and Y are Cross, Circle, Square and Triangle, and L1 / R1
are L / R.

A, B, X and Y follow SDL's positions: A south, B east, X west, Y north. On a pad
printed the Nintendo way, with A east and X north, **Settings → Swap A/B and
X/Y** (or `[input] swap_abxy = true`) trades the pairs so the hints name the
buttons as printed; on the Vita it makes Circle confirm.

## Touch

Tap a row to act on it. Tap a hint in the footer to press the button it names.
Only the screen taps: the Vita's rear touchpad is ignored.

## Quitting

| System | Quit |
| --- | --- |
| OnionOS, Allium, spruceOS (Miyoo Mini) | MENU |
| PortMaster, spruceOS (aarch64) | Select + Start |
| muOS, NextUI, Android | B on the home screen |
| Knulli | B on the home screen, or Hotkey + Start |
| PS Vita | Close it from the LiveArea |
| Desktop | Close the window |

## Keyboard layout

The Miyoo Mini's pad arrives as keys. The layout is picked from the video driver;
`RETSEND_KEYMAP=miyoo|desktop` overrides it ([Command line](CLI.md)).
