# Configuration

Settings live in `config.toml` in the data dir (`SDL_GetPrefPath`, e.g.
`~/.local/share/mxmgorin/retsend/` on Linux, `ux0:/data/mxmgorin/retsend/` on the
PS Vita), or wherever `RETSEND_CONFIG` points. It is written with the defaults on
first run. The Settings screen edits everything except:

| Key | Effect |
| --- | --- |
| `[network] https = false` | Use the protocol's plain-HTTP mode. |
| `[network] announce_interval_secs` | Multicast announce interval. |
| `[transfer] browser_roots` | Extra file-browser roots. |
| `[transfer] history_limit` | Transfers kept in History (default 200). |
| `[transfer] pinned_paths`, `last_send_dir` | Written by Y and by sending. |
| `[display] scale` | UI size over the screen's own (0.6–1.6). Every screen is drawn for 640×480 and fitted. |

Environment variables are in [Command line](CLI.md#environment-variables).

## Save routes

Received ROMs go to the console folder they belong to, among those already in
`save_dir`: `.gba` into `gba`, `.sfc` into `snes`.

- Nothing is created: a missing folder gets no route.
- Ambiguous extensions (`.zip`, `.iso`, `.bin`, `.md`) are never routed.
- Off with **Settings → Auto save routes** or `[transfer] auto_routes = false`.

Per-extension overrides win over detected routes. Set them in **Settings → Save
routes**, or:

```toml
[transfer.routes]
gbc = "gb"                    # relative: <save_dir>/gb
gba = "/roms/gba"             # absolute: used as-is
png = "/roms/screenshots"
```

Extensions match case-insensitively, folders are created on demand, and anything
unrouted lands in `save_dir`.

## Sending folders

X on a folder row takes its whole tree under the folder's own name:
`roms/gb/zelda.gbc` arrives as `roms/gb/zelda.gbc`.

- The footer totals the folder before Start.
- Dotfiles and symlinks are left out, as in the listing.
- A send stops at 2048 files; a toast says when a pick ran past it.
- X anywhere else takes the files of the current folder, without subfolders.
- History repeats a send from its folder, so a resend picks up what is there now.

## Receiving folders

A folder sent from the official app is rebuilt under `save_dir`:
`Zelda/saves/Zelda.sav` arrives at `<save_dir>/Zelda/saves/Zelda.sav`.

- Files in a folder skip the save routes; loose files are routed as usual.
- Sender paths are sanitized and never leave `save_dir`.
- Off with **Settings → Received folders** or `[transfer] keep_folders = false`:
  every file lands flat and is routed.
