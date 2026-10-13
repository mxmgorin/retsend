# Controls

| Pad | Keyboard | Keypad phone | Action |
| --- | --- | --- | --- |
| D-pad / stick | Arrows / H J K L / W A S D | D-pad / 2 4 6 8 | Navigate; left/right switch tabs or step a setting |
| A | Enter / V | OK | Send to a device, select a file, accept, repeat a send, type |
| B | Esc / Z | Back | Back, decline, cancel, leave the keyboard |
| X | X / Backspace | 7 / Del | Add a device by IP, pick where a transfer lands, delete a history row, erase a character, take a folder |
| Y | Y / C | 9 | Pin or unpin the row under the cursor |
| Start | Space / Alt | Right soft key / Call / 5 | Confirm a send, OK on the keyboard |
| Select | Shift / Tab | Left soft key / 0 | Refresh the radar, switch roots, switch the keyboard layer |
| L1 / R1 | PgUp / PgDn, Q / E | 1 / 3 | Switch tabs, page the file browser |

Footer hints show keys after a key press and pad buttons after a button press.

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

## Remapping keys

**Settings → Keys** (not on the Miyoo Mini):

- **Remap all keys**: one key per button, Up to R1. B keeps Esc and Back.
- A on a row adds a key, X resets the row.
- **Reset all keys** restores the defaults.
- Esc or Back cancels; 10 seconds without a key cancels too.
- The last key of the D-pad, A or B cannot be moved.

`[input.keys]` sets the keys per button, by SDL key name:

```toml
[input.keys]
a = ["Return", "Select"]
select = []                 # unbound
```

Buttons: `up`, `down`, `left`, `right`, `a`, `b`, `x`, `y`, `start`, `select`,
`l1`, `r1`, `menu`. A button left out keeps its default keys. On screen, `Select`
shows as OK and `AC Back` as Back.
