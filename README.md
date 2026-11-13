# Mundwerk

Dictate into any text field: press a key or a mouse button, speak, press again, and the text appears where the cursor
is, in any program. The speech recognition does not run on the machine you dictate at but in the
[STT-Suite](https://github.com/Peuqui/STT-Suite) on another one, which decides with
[GPU-Orc](https://github.com/Peuqui/GPU-Orc) whether a graphics card or the processor transcribes. So Mundwerk takes no
graphics memory where you work, and one speech recognition serves every machine in the house.

Mundwerk is a small tray app for Windows (Rust and [Tauri](https://tauri.app)), built to be portable to macOS and Linux.

[Deutsche Version](README.de.md)

## What it does

- **Triggers:** any key, key combination or mouse button (middle, side buttons), each either *hold* (listens while held)
  or *toggle* (one press starts, the next stops). Taken live in the settings window: press it, and it is set. A
  combination a mouse's own software sends counts too.
- **Sounds** at start, end and cancel (ping, pong, break, or a WAV file of your own), each to listen to in the settings.
- **Writes into the focused field**, typed (Unicode, umlauts included, the clipboard untouched) or pasted, chosen per
  program; a space between two dictations into the same window; Enter at the end if you like (sends a chat message at
  once).
- **Replacement list** for what the recognition gets wrong, names above all: variants as whole words
  (`Komm mit | Kommit → commit`), or a regular expression.
- **Overlay** next to where you write, and the **STT-Suite's logo by the mouse pointer** while dictating (its bars move
  and its rim is red while it listens, it turns grey while it transcribes), as in Agent-Orc and SpeechPulse.
- **Nothing gets lost:** a history of the last dictations in the tray (a click copies one), a key that writes the last
  dictation again where the cursor is now, and a recording the STT-Suite could not transcribe waits in the tray to be
  sent again.
- **Several servers in order**, e.g. a direct cable first and the home network second; the next one only when one cannot
  be reached.
- German and English, switchable; starts with the system if you like.

## Getting started

**1. The STT-Suite.** Mundwerk needs a running [STT-Suite](https://github.com/Peuqui/STT-Suite) it can reach. For calls
from another machine the suite needs its token and has to listen beyond its own machine (`STT_HOST=0.0.0.0`); its
README's section *Security* says how. The token is in `~/.config/stt-suite/token` on the suite's machine.

**2. Mundwerk.** Until there is a release, build `mundwerk.exe` as described under [Building](#building). It is a single
file without installation; Windows 10 and 11 bring the WebView2 it needs. Windows SmartScreen warns about an unsigned
program the first time: *More info*, then *Run anyway*.

**3. First start.** No window opens: Mundwerk sits in the tray (the yellow-orange sound waves of the STT-Suite, maybe behind the
`^`). Its menu has the settings, the overlay switch, the history, *send again*, the log and *quit*. In the settings:

- under *STT-Suite*, the addresses of your suite (tried top to bottom) and its **token**;
- under *Triggers*, click *Record* and press the key or mouse button you want; choose *hold* or *on and off*;
- *Save*: it is in effect at once, and the window closes.

Then click into any text field, press your trigger, speak, press again (or let go, for *hold*): ping, pong, and the text
is there.

## Using it

**Hold or toggle.** *Hold* listens while the key is held; a press shorter than the shortest press (300 ms) counts as a
slip and is thrown away without a sound. *On and off* starts with one press and stops with the next. A mouse button
with a **macro** (its software sends, say, Ctrl+Shift+Alt+D) needs *on and off*: the macro lets go of the keys at once.
For Mundwerk such a button *is* that combination; Windows never sees the button itself, only its keys. *Pass on* lets
the program below get the press too; usually it stays off, so a side button does not also page back in the browser.

**Cancel.** Escape (or the key you set) throws a running recording away, with the break sound. It is swallowed while
a recording runs and passes untouched otherwise.

**Where the text goes.** Into the field that has the focus when the text comes back from the STT-Suite. Two dictations
in a row into the same window get a space between them.

**A dictation that went nowhere** (no text field had the focus): click into the right field and press the key for
*last dictation once more* (set it in the settings under *Writing*), or copy it from the history in the tray.

**When the STT-Suite cannot be reached** or refuses: the overlay says why for a few seconds, and the recording waits in
the tray under *Send the last recording again*.

**Writing into a terminal.** Mundwerk "types" the text: it hands Windows key presses as if they came from a keyboard
(`SendInput` with Unicode characters), and Windows delivers them to the window in front, a terminal as much as an
editor. Windows Terminal and most programs take that as typing. A program that does not (some remote desktops, older
consoles) gets the text pasted instead: under *Writing*, *Programs with their own method*, its process name (e.g.
`mstsc.exe`) with *through the clipboard*; Mundwerk then puts the text into the clipboard, presses Ctrl+V and puts back
what the clipboard held. The text is written only once Ctrl, Shift, Alt and the Windows key are let go: otherwise a
letter would arrive as a shortcut (a console shows `^T`), for instance while a mouse button's macro still holds them
and the text is already back.

## Overlay and badge

The **overlay** (listening, transcribing, errors) never takes the focus from the text field. By default it shows below
and right of the text cursor, on that screen. Many programs (browsers, Electron and others that draw their own text) do
not tell Windows where their text cursor is; then it shows by the mouse pointer. Or a fixed place, moved with the mouse,
on any screen. The tray menu switches it off; then the tray icon alone shows the state: the badge below in its state (yellow rim ready, red rim listening,
grey transcribing, dark red rim and grey bars an error).

The **badge** by the mouse pointer is the dictation badge of every program (its one source is STT-Suite/ui/badge/): the STT-Suite's logo in a round badge, above and right of the
pointer's tip, moving with it while a dictation runs: its bars move and its rim is red while it listens (red also where
motion is reduced), it is still and grey while it transcribes. The pointer itself stays as it is. The tray and program icons
(`src-tauri/icons/`, `ui/icon.png`) are the same badge, made from that source by the STT-Suite's `scripts/render_badge_icons.py`;
none of them is edited by hand. It can be switched
off.

The **sounds** come in an order that suits a headset too: the start sound plays to its end before the microphone opens,
the end and cancel sounds only once it is closed (an open microphone turns many headsets to their telephone mode, where
sounds come quiet, cut or with an echo).

## Settings

The settings window is a form; saving checks everything, writes the file and puts it into effect at once. The file
itself is `%APPDATA%\de.peuqui.mundwerk\config.toml`; every setting is explained in it. Its defaults, with the same
explanations, are in [`crates/mundwerk-core/default_config.toml`](crates/mundwerk-core/default_config.toml).

| Section | What |
|---|---|
| Language | automatic (the system's), German, English |
| Triggers | keys, combinations, mouse buttons; hold or on and off; pass on |
| STT-Suite | servers in order, token, how long to wait for a server |
| Recognition | device (automatic, GPU only, CPU only) and language of the dictations; engine and model are the suite's |
| Recording | microphone, cancel key, shortest press, stop after silence, longest recording (both off unless set) |
| Sounds | on or off, start, end, cancel, volume |
| Writing | typing or clipboard, per program; space between dictations; Enter at the end; key for the last dictation once more |
| Replacements | words or regular expressions, and what to write instead |
| Display | overlay or tray only, its place, the badge by the pointer, opacity, how long an error shows |
| History and system | how many dictations the history keeps; start with the system |

Every field must be in the file, and one Mundwerk does not know is an error, so a typo does not get lost quietly. If
the file cannot work at the start, Mundwerk does not run on stand-ins: the settings window opens with the reason (and
can start again from the defaults); saving restarts Mundwerk.

## When something does not work

The tray menu's *Open the log* opens `%LOCALAPPDATA%\de.peuqui.mundwerk\logs\mundwerk.log`. It has every trigger, every
state, every transcription with the server and the device, every error, and while a trigger is being recorded, every
raw key and mouse button the system hook sees.

- **"STT-Suite refused (401 unauthorized)"**: the token is missing or wrong.
- **"no STT-Suite reachable"**: none of the addresses answered; the recording waits in the tray.
- **Only a ping, and the overlay goes again**: a trigger on *hold* was let go before the shortest press, typical for a
  mouse button with a macro: set it to *on and off*.

## Building

Windows is built on Linux with [cargo-xwin](https://github.com/rust-cross/cargo-xwin), which fetches the Windows
libraries itself (`cargo install --locked cargo-xwin tauri-cli`, `rustup target add x86_64-pc-windows-msvc`, plus clang
and llvm, here version 17 in `/usr/lib/llvm-17/bin`):

```
PATH="/usr/lib/llvm-17/bin:$PATH" cargo xwin clippy --target x86_64-pc-windows-msvc -p mundwerk
cd src-tauri && PATH="/usr/lib/llvm-17/bin:$PATH" cargo tauri build --runner cargo-xwin --target x86_64-pc-windows-msvc --no-bundle
```

The result is `target/x86_64-pc-windows-msvc/release/mundwerk.exe`. An installer (leave out `--no-bundle`) also needs
`nsis`. The logic has its tests on any machine:

```
cargo test -p mundwerk-core
cargo clippy -p mundwerk-core --all-targets
cargo fmt --check
```

and against a running STT-Suite (skipped otherwise):

```
MUNDWERK_LIVE_URL=http://127.0.0.1:5080 MUNDWERK_LIVE_TOKEN_FILE=~/.config/stt-suite/token MUNDWERK_LIVE_WAV=~/Projekte/STT-Suite/burn_in_speech.wav cargo test -p mundwerk-core --test live_stt -- --ignored
```

## Inside

- `crates/mundwerk-core`: the logic without a user interface, tested on any machine: settings, triggers, when a dictation
  starts and ends, taking a trigger live, replacements, the space between dictations, the STT-Suite call with its
  fallback, history, the texts in both languages.
- `src-tauri`: the app: tray, overlay, badge, settings window, the system hook, microphone, sounds, typing. On Windows a
  low-level hook of its own: the usual library (rdev) calls `ToUnicodeEx` on every key and so breaks the dead keys
  (^ ´ `). On macOS and Linux (X11) rdev; under Wayland the system blocks global keys and typing into other windows, which
  will need a way of its own.
- `ui/`: the pages (HTML, CSS, JavaScript without a build step) and `i18n.json`, all words of Mundwerk in German and
  English; a test checks that both languages have the same keys and that every key in use exists. Another language is
  another block in that file.

## Ideas

- [Mundwerk on Android](docs/android.de.md) (German): a voice input keyboard or a floating bubble, with one's own
  STT-Suite instead of Gboard's. A discussion, not started.

## Star History

![Star History](https://raw.githubusercontent.com/Peuqui/Traffic/main/data/Mundwerk/star-history.svg)

<sub>Recorded daily in [Peuqui/Traffic](https://github.com/Peuqui/Traffic), together with views, clones and downloads.</sub>

## License

[PolyForm Noncommercial 1.0.0](LICENSE)
