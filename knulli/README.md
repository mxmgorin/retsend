# RetSend for Knulli

A dedicated Knulli (Batocera-fork) AArch64 build of [RetSend](https://github.com/mxmgorin/retsend),
a LocalSend-compatible file sender and receiver for retro handhelds.

**Candidate build.** This artifact has not been tested on a real device and
makes no compatibility claims. It is not an installable Knulli App Store
package yet.

## What's inside

| File | Purpose |
| --- | --- |
| `RetSend.sh` | Launcher; sets the constrained paths below |
| `retsend` | AArch64 executable (links the device's own SDL2) |
| `LICENSE` | GPL-3.0, the licence of RetSend |
| `NOTICES` | Third-party components carried by the binary |

## Where files go

- Program files: the directory you unzipped this into (typically
  `/userdata/roms/tools/RetSend`).
- Config, transfer history and TLS identity:
  `/userdata/system/configs/retsend` — survives reinstalling or removing the
  program folder. Delete that directory to fully reset identity and settings.
- Received files: `/userdata/roms/retsend-inbox` (created on first launch).

The in-app file browser is rooted at `/userdata/roms` and the inbox only.

## Running

Launch `RetSend.sh` from the folder on the device (or wire it into your
frontend's menu of choice). Knulli shares SDL2 with the rest of the system;
no PortMaster helpers are involved.

Transfers use LocalSend's protocol: devices discover each other over LAN
multicast (UDP 53317), and transfers go over HTTPS with self-signed
certificates. Both devices must be on the same network.
