# Command line

```
retsend [--receive] [FILE...]
```

| Argument | Effect |
| --- | --- |
| `FILE...` | Pre-select these files in the send browser. |
| `--receive` | Headless receiver: no screen, auto-accept into `save_dir`, progress on stdout. For SSH and scripting. |

## Environment variables

Launchers set these; on the desktop they are for testing.

| Variable | Effect |
| --- | --- |
| `RETSEND_DATA_DIR` | Data dir for the config, history and TLS identity. |
| `RETSEND_CONFIG` | Path of `config.toml`. |
| `RETSEND_SAVE_DIR` | Default folder for received files. |
| `RETSEND_BROWSER_ROOTS` | Extra file-browser roots, `:`-separated. |
| `RETSEND_ALIAS` | Default device name. |
| `RETSEND_SCALE` | UI scale base, replacing the fit to the screen; `[display] scale` applies over it. |
| `RETSEND_GLES=0\|1` | Prefer desktop GL or GLES. |
| `RETSEND_SOFTWARE=1` | Skip GL; draw with SDL's renderer. |
| `RETSEND_BLIT=1` | Present the UI as one texture copy per frame, for drivers that show nothing else (Miyoo Mini). |
| `RETSEND_KEYMAP=miyoo\|desktop` | Keyboard layout of the pad. |
| `RETSEND_BACK_QUIT=1` | B on the home screen quits. |
| `RETSEND_MENU_QUIT=1` | MENU quits (Miyoo Mini layout). |
| `RETSEND_LOG_LEVEL` | `error`, `warn`, `info` (default), `debug`, `trace`. |
| `RETSEND_LOG_STYLE` | `always` (default), `auto`, `never`. |
| `RETSEND_LOG_FILE` | Write the log to this file. |
| `RETSEND_PANIC_FILE` | Where a panic is written; default `retsend-panic.log` in the working dir. |

## Two instances on one machine

They find each other over multicast loopback:

```sh
RETSEND_DATA_DIR=/tmp/ls-a cargo run &
RETSEND_DATA_DIR=/tmp/ls-b cargo run
```
