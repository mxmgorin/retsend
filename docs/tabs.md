# Tabs — design

Status: **implemented**. Three persistent top-level tabs —
**Send · Receive · Settings** — replacing the earlier precedence-stacked modal
surfaces.

## 1. Motivation

Today every surface is a modal ranked by precedence (`src/overlay/mod.rs:13`
`Focus`, resolved in `src/app/mod.rs:124` `focus()`). The radar is the base
screen; **Start** opens Settings as a modal over it; picking a peer opens the
Browser; an incoming request is a Prompt modal; a live transfer takes the
screen over.

Two problems fall out of that:

- **Receiving is invisible.** The device's whole reason to exist is to *receive*
  files pushed from a phone (see the README's "Why?"), yet there is no screen
  that says "you're discoverable as `<alias>`, ready." That state only leaks
  into the radar header (`src/ui/home.rs:33`) and the momentary Prompt.
- **Send and identity are conflated.** The one radar screen is simultaneously
  "who can I send to" and "here's me." Splitting them gives each concern a home.

LocalSend — whose protocol and default UX this app mirrors — uses exactly these
three tabs. Matching them keeps the mental model familiar to anyone who runs
the phone/desktop app on the other end.

## 2. Current architecture (as-is)

Base-screen selection, drawn in `src/ui/mod.rs:124`:

```
if browser.open        → browser
else if settings.open  → settings
else if transfer       → transfer
else                   → home (radar)
```

Input precedence, `src/app/mod.rs:124` `focus()`:

```
Osk > Prompt > Browser > Settings > Transfer > Home
```

The command router (`src/app/mod.rs:142` `execute_command`) is the single place
that decides "what does button X do right now", matching on
`(Focus, AppCommand)`. Input handlers stay focus-agnostic and just emit
`AppCommand`s (`src/app/command.rs:1`) — this is the seam the tab design leans
on.

Physical → command map (`src/event/gamepad.rs:55`, `src/event/keyboard.rs:8`):

| Pad      | Keyboard      | `AppCommand`     |
|----------|---------------|------------------|
| D-pad/stick | Arrows     | `Nav(dir)`       |
| A        | Enter         | `Confirm`        |
| B        | Esc/Backspace | `Back`           |
| Start    | F1            | `ToggleSettings` |
| Select   | Tab / F5      | `ReAnnounce`     |
| L1       | PgUp          | `PageUp`         |
| R1       | PgDn          | `PageDown`       |

Settings is a modal: **Start** sets `settings.open = true`
(`src/app/mod.rs:267`); leaving it runs `close_settings()`
(`src/app/mod.rs:335`) which persists config and restarts the net stack if the
port changed.

## 3. Target model

### 3.1 The tab

```rust
// src/overlay/mod.rs
pub enum Tab { Send, Receive, Settings }
```

Held as one field on the tab state (replacing `Settings::open`). **Default
landing tab: `Receive`** — the device's primary role is to be received-to, and
it's the natural home for the branded idle screen. (One-line change if we'd
rather land on `Send`; see §7.)

### 3.2 Focus precedence (revised)

```
Osk > Prompt > Browser > Transfer > Tabs
```

`Settings` leaves the precedence ladder — it is no longer a modal, it's the
Settings tab. `Focus::Home` and `Focus::Settings` collapse into a single
`Focus::Tabs`; the router then branches on the *active tab* for tab-specific
input. Modal/sub-screens (Osk, Prompt, Browser, Transfer) are unchanged and
still outrank the tabs, so:

- A Browser opened from **Send** (pick files) or from the **Settings** tab
  (save-dir picker) draws above the tab bar, exactly as it does today.
- A live Transfer still takes the screen over; when it closes
  (`transfer.close()`) we fall back to the active tab.
- The incoming Prompt and the accept/decline flow are independent of which tab
  is active.

### 3.3 Switching tabs

No new `AppCommand` variants. The input layer stays focus-agnostic; the router
reinterprets existing commands at `Focus::Tabs`:

- **L1 / R1** (`PageUp` / `PageDown`) → previous / next tab, cycling
  `Send ⇄ Receive ⇄ Settings` with wraparound (mirrors `Settings::move_cursor`'s
  `rem_euclid`, `src/overlay/settings.rs:44`). Only when `focus() == Tabs`;
  inside the Browser they keep paging the file list.
- **Start** (`ToggleSettings`) → jump to the Settings tab; pressed *on* the
  Settings tab, return to the previously-active tab. Preserves the current
  "Start = settings" toggle muscle memory.
- **Select** (`ReAnnounce`) → re-announce, unchanged. Meaningful on Send and
  Receive.

D-pad Left/Right are deliberately **not** tab switchers: they stay the value
adjuster on the Settings tab (`adjust_setting`, `src/app/mod.rs:315`) and are
unused on Send/Receive, so tab switching lives entirely on L1/R1 + Start with no
per-tab exception.

Costs of reusing L1/R1 (both acceptable):

- Radar list **page-jump is dropped** — hold the D-pad to scroll instead. The
  device list is short in practice.
- Settings **port ±100** shortcut is dropped — Left/Right still steps ±1
  (`adjust_setting`, `src/app/mod.rs:315`).

### 3.4 Persisting settings

Leaving the Settings tab (via L1/R1, Start, or B) triggers the old modal-close
behaviour, now `leave_settings()` (`src/app/mod.rs`): save config, and restart
the net stack if `port_dirty`. `Settings::open` is removed; `cursor` and
`port_dirty` stay.

## 4. Per-tab layout

A shared **tab bar** (a `Panel::top`) renders `[Send] [Receive] [Settings]`
with the active tab in `theme::ACCENT`, on all three tab screens. It is hidden
during Browser / Transfer / Prompt / Osk takeovers.

### Send
- Central: the current radar device list (`src/ui/home.rs:83`).
- Empty state: "No devices found — open LocalSend on your phone or PC on the
  same network." (The branded wordmark hero moves to Receive.)
- **A** → open Browser for the selected device. **Select** → refresh.
- Footer hint: `A Send · L1/R1 Tabs · Select Refresh`.

### Receive
- Central: the wordmark hero (`add_wordmark`, `src/ui/home.rs:143`, moved here),
  "Ready to receive as `<alias>`", the endpoint line (`HTTPS · ip:port`, from
  `endpoint_line` `src/ui/mod.rs:282`), a quick-save on/off badge, and
  "Waiting for a sender…".
- No list to navigate; **Nav**/**A** do nothing. **Select** → refresh.
- An incoming request pops the Prompt modal over this tab, unchanged.
- Footer hint: `L1/R1 Tabs · Select Refresh`.

### Settings
- Central: today's settings rows (`src/ui/settings.rs`, `src/overlay/settings.rs`).
- **A** edits a row, **Left/Right** adjusts a value — unchanged.
- **B** → back to the previously-active tab (also persists, §3.4).
- Footer hint: `A Edit · ←/→ Adjust · L1/R1 Tabs`.

## 5. Rendering changes

- New tab-bar renderer — `src/ui/tabs.rs` (or a helper in `src/ui/mod.rs`),
  drawn on the three tab screens.
- Split `src/ui/home.rs`: the device-list body stays for **Send**; the
  identity header + idle hero + endpoint become the **Receive** renderer
  (`src/ui/receive.rs`).
- `src/ui/mod.rs` base-screen selection (`:124`) becomes: draw the tab bar +
  the active tab's body; Browser / Transfer / Prompt / Osk overlays as today.

## 6. Implementation checklist

- [x] `src/overlay/mod.rs`: add `Tab`; collapse `Focus::{Home,Settings}` into
      `Focus::Tabs`.
- [x] Tab state in `src/overlay/tabs.rs`, holding the active tab + the previous
      tab (for Start toggle), with unit tests.
- [x] `src/overlay/settings.rs`: drop `open`; keep `cursor`, `port_dirty`.
- [x] `src/app/mod.rs`: `focus()` returns `Tabs`; `execute_command` branches on
      active tab; `PageUp/PageDown`→tab switch, `ToggleSettings`→Settings toggle,
      persist-on-leave. `Nav(Left/Right)` unchanged (Settings value-adjust; no-op
      on Send/Receive).
- [x] `src/ui/mod.rs`, `src/ui/tabs.rs`, `src/ui/receive.rs`, `src/ui/home.rs`:
      renderers per §5.
- [x] README controls table — L1/R1 now "switch tabs"; Start "Settings tab";
      page-jump/port-±100 removed.
- [x] Tests: `Tab` cycling / Start-toggle logic unit-tested in
      `src/overlay/tabs.rs`.

## 7. Decisions

- **Default tab** — **Receive** (the device's primary role is to be received-to).
- **Transfer** — a full-screen takeover above the tabs; on close it returns to
  the active tab. Keeps the send/accept flow untouched.
- **Select** — kept as refresh, not repurposed for tab switching.
