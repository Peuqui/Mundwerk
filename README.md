# Mundwerk

Dictate into any text field: press a key or a mouse button, speak, and the text appears where the cursor is. The speech
recognition does not run on this machine but in the [STT-Suite](https://github.com/Peuqui/STT-Suite) on another one,
which decides with [GPU-Orc](https://github.com/Peuqui/GPU-Orc) whether the graphics card or the processor transcribes.
On the machine you dictate at, Mundwerk takes no graphics memory.

[Deutsche Version](README.de.md)

> Status: first version. The core (settings, triggers, replacement list, the space between dictations, the call to the
> STT-Suite with fallback, history) is tested, the STT call also live. The Windows app builds and passes Clippy but has
> not been tried on Windows yet.

## How it works

1. A **trigger** starts the recording: a key, a combination or a mouse button (middle, X1, X2). Per trigger: **hold**
   (records while held) or **toggle** (press once to start, once more to stop). Optionally the press does not reach the
   program below, so a side button does not also page back in the browser. A slip shorter than `min_press_ms` is thrown
   away; Escape cancels.
2. A **sound** marks start, end and cancel: the Echo Dot's (ping, pong, break from FreeEchoDot2).
3. The recording goes as WAV to the STT-Suite's `POST /transcribe`, by default with `device=auto`. Several servers are
   tried **in order**, e.g. the direct cable first, the home network second; only a server that cannot be reached is
   skipped. One that answers with an error has answered, and that error is shown.
4. The **replacement list** fixes what the recognition gets wrong, names above all: variants as whole words, case
   ignored (`Komm mit | Kommit → commit`), a regular expression for special cases.
5. The text goes into the **focused field**: typed (Unicode, umlauts included, the clipboard untouched) or pasted
   (Ctrl+V, then the clipboard is put back), settable per program. A dictation that follows another in the same window
   gets a **space** before it, as in Agent-Orc. Optionally **Enter** at the end to send at once.
6. The **history** in the tray menu keeps the last dictations; a click copies one that landed in the wrong field.

A small **overlay** shows listening and transcribing. It can be moved anywhere with the mouse, other screens included,
remembers its place and never takes the focus from the text field. The tray menu switches it off; then only the icon
(blue ready, red listening, orange transcribing) shows the state. Starting with the system: a setting.

Off unless set: stopping after silence, a maximum recording length, Enter at the end.

## Settings

Every setting with its default is in [`crates/mundwerk-core/default_config.toml`](crates/mundwerk-core/default_config.toml).
On the first start Mundwerk writes it to the user's settings folder (on Windows `%APPDATA%\de.peuqui.mundwerk\config.toml`);
from then on that file counts. Every field must be in it, and an unknown one is an error, so a typo does not get lost
quietly. The settings window (tray menu) is a form; saving checks the settings, writes them into the file (its comments
stay) and puts them into effect at once. Triggers are taken live: "Aufnehmen", then press the key, combination or mouse
button. That goes through Mundwerk's system hook, so a combination a mouse's software sends counts too (say
Ctrl+Shift+Alt+D on a mouse button); Mundwerk does not tell left from right Ctrl, Shift and Alt. The STT-Suite chooses
engine and model itself; Mundwerk sends only device and language. If the file cannot work at the start, Mundwerk does not
run on stand-ins: it opens the settings window with the error (where one can start again from the defaults) and starts
anew after saving.

The STT-Suite needs its token for calls from another machine (`server.token`, see its Security section).

## Layout

- `crates/mundwerk-core`: the logic without a user interface, testable on any machine (`cargo test -p mundwerk-core`).
- `src-tauri`: the app (tray, overlay, settings window, hook, microphone, sounds, typing) and `ui/` its pages (HTML, CSS,
  JavaScript without a build step). On Windows a low-level hook of its own: rdev calls `ToUnicodeEx` on every key and so
  breaks the dead keys (^ ´ `). On macOS and Linux (X11) rdev. Under Wayland the system blocks global keys and typing
  into other windows; that needs its own way later.
- `assets/sounds`: ping, pong and break from FreeEchoDot2 (`audio_samples/`, at 37190a9, made by
  `tools/gen_notify_sounds.py`).

## Development

```
cargo test -p mundwerk-core
cargo clippy -p mundwerk-core --all-targets
cargo fmt --check
```

The Windows app is built on Linux with [cargo-xwin](https://github.com/rust-cross/cargo-xwin), which fetches the
Windows libraries itself (`cargo install --locked cargo-xwin tauri-cli`, `rustup target add x86_64-pc-windows-msvc`;
plus clang and llvm, here version 17 in `/usr/lib/llvm-17/bin`):

```
PATH="/usr/lib/llvm-17/bin:$PATH" cargo xwin clippy --target x86_64-pc-windows-msvc -p mundwerk
cd src-tauri && PATH="/usr/lib/llvm-17/bin:$PATH" cargo tauri build --runner cargo-xwin --target x86_64-pc-windows-msvc --no-bundle
```

The result is `target/x86_64-pc-windows-msvc/release/mundwerk.exe`, a single file without installation (Windows 10 and
11 have WebView2). An installer (leave out `--no-bundle`) also needs `nsis`.

Against a running STT-Suite (skipped otherwise):

```
MUNDWERK_LIVE_URL=http://127.0.0.1:5080 MUNDWERK_LIVE_TOKEN_FILE=~/.config/stt-suite/token MUNDWERK_LIVE_WAV=~/Projekte/STT-Suite/burn_in_speech.wav cargo test -p mundwerk-core --test live_stt -- --ignored
```

## License

[PolyForm Noncommercial 1.0.0](LICENSE). The sounds in `assets/sounds` come from FreeEchoDot2 (MIT, © 2026 Peuqui).
