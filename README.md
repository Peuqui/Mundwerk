# Mundwerk

Dictate into any text field: press a key or a mouse button, speak, and the text appears where the cursor is. The speech
recognition does not run on this machine but in the [STT-Suite](https://github.com/Peuqui/STT-Suite) on another one,
which decides with [GPU-Orc](https://github.com/Peuqui/GPU-Orc) whether the graphics card or the processor transcribes.
On the machine you dictate at, Mundwerk takes no graphics memory.

[Deutsche Version](README.de.md)

> Status: being built. Done: the core (settings, triggers, replacement list, the space between dictations, the call to
> the STT-Suite with fallback, history, WAV) with tests; the user interface (Tauri) follows.

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
6. The **history** keeps the last dictations: paste again or copy one that landed in the wrong field.

A small **overlay** shows listening and transcribing; it can be moved anywhere, other screens included, and never takes
the focus. Or only the tray icon shows the state. Starting with the system: a setting.

Off unless set: stopping after silence, a maximum recording length, Enter at the end.

## Settings

Every setting with its default is in [`crates/mundwerk-core/default_config.toml`](crates/mundwerk-core/default_config.toml).
On the first start Mundwerk writes it to the user's settings folder (on Windows `%APPDATA%\Mundwerk\config.toml`); from
then on that file counts. Every field must be in it, and an unknown one is an error, so a typo does not get lost quietly.

The STT-Suite needs its token for calls from another machine (`server.token`, see its Security section).

## Layout

- `crates/mundwerk-core`: the logic without a user interface, testable on any machine (`cargo test`).
- The Tauri app (follows): tray, overlay, settings window, keyboard and mouse hooks, microphone, typing. Built for
  Windows, and without detours for Linux (X11) and macOS as far as possible. Under Wayland the system blocks global keys
  and typing into other windows; that needs its own way later.
- `assets/sounds`: ping, pong and break from FreeEchoDot2 (`audio_samples/`, at 37190a9, made by
  `tools/gen_notify_sounds.py`).

## Development

```
cargo test
cargo clippy --all-targets
cargo fmt --check
```

Against a running STT-Suite (skipped otherwise):

```
MUNDWERK_LIVE_URL=http://127.0.0.1:5080 MUNDWERK_LIVE_TOKEN_FILE=~/.config/stt-suite/token MUNDWERK_LIVE_WAV=~/Projekte/STT-Suite/burn_in_speech.wav cargo test -p mundwerk-core --test live_stt -- --ignored
```

## License

[PolyForm Noncommercial 1.0.0](LICENSE). The sounds in `assets/sounds` come from FreeEchoDot2 (MIT, © 2026 Peuqui).
