# Mundwerk

Diktieren in jedes Textfeld: Taste oder Maustaste drücken, sprechen, und der Text steht dort, wo der Cursor ist. Die
Spracherkennung läuft nicht auf dem eigenen Rechner, sondern in der [STT-Suite](https://github.com/Peuqui/STT-Suite) auf
einem anderen. Die entscheidet zusammen mit [GPU-Orc](https://github.com/Peuqui/GPU-Orc), ob die Grafikkarte oder der
Prozessor transkribiert. Auf dem Rechner, an dem diktiert wird, belegt Mundwerk keinen Grafikspeicher.

[English version](README.md)

> Stand: erste Fassung. Der Kern (Einstellungen, Auslöser, Ersetzungsliste, Leerzeichen zwischen Diktaten, Aufruf der
> STT-Suite mit Rückfall, Verlauf) ist getestet, der STT-Aufruf auch live. Die Windows-App baut und ist geprüft
> (Clippy), aber noch nicht auf Windows ausprobiert.

## Wie es arbeitet

1. Ein **Auslöser** startet die Aufnahme: eine Taste, eine Tastenkombination oder eine Maustaste (Mitte, X1, X2). Je
   Auslöser wählbar: **gedrückt halten** (Aufnahme, solange gehalten) oder **an/aus** (einmal drücken zum Start, einmal
   zum Ende). Wahlweise erreicht der Druck das Programm darunter nicht, damit eine Seitentaste nicht zugleich im Browser
   zurückblättert. Ein kurzer Fehldruck (unter `min_press_ms`) wird verworfen, Esc bricht ab.
2. Ein **Ton** markiert Start, Ende und Abbruch: dieselben wie beim Echo Dot (ping, pong, break aus FreeEchoDot2), zur
   Wahl auch dessen Benachrichtigungstöne Dong-Ding und Ding-Dong oder eine eigene WAV-Datei; im Einstellungsfenster
   lässt sich jeder vorab anhören.
3. Die Aufnahme geht als WAV an `POST /transcribe` der STT-Suite, standardmäßig mit `device=auto`. Mehrere Server
   werden **der Reihe nach** versucht, etwa zuerst die Direktverbindung, dann das Heimnetz. Weitergegangen wird nur,
   wenn ein Server nicht erreichbar ist. Antwortet einer mit einem Fehler, wird dieser angezeigt.
4. Die **Ersetzungsliste** korrigiert, was die Erkennung falsch versteht, vor allem Namen: Varianten als ganze Wörter
   ohne Rücksicht auf Groß- und Kleinschreibung (`Komm mit | Kommit → commit`), für Sonderfälle ein regulärer Ausdruck.
5. Der Text wird ins **aktive Feld** geschrieben: getippt (Unicode, mit Umlauten, ohne die Zwischenablage zu berühren)
   oder eingefügt (Strg+V, danach wird die Zwischenablage zurückgestellt). Die Methode lässt sich je Programm
   festlegen. Folgt ein Diktat einem anderen im selben Fenster, kommt ein **Leerzeichen** dazwischen, wie bei Agent-Orc.
   Am Ende wahlweise **Enter** zum sofortigen Absenden.
6. Der **Verlauf** im Menü des Infobereichs hält die letzten Diktate. Ein Klick kopiert eines, wenn es im falschen Feld
   gelandet ist.

Ein kleines **Overlay** zeigt „hört zu“ und „transkribiert“ und nimmt dem Textfeld nie den Fokus. Standardmäßig
erscheint es dort, wo man schreibt: unten rechts neben der Schreibmarke, auf deren Bildschirm. Viele Programme (Browser,
Electron und andere, die ihren Text selbst zeichnen) sagen Windows nicht, wo ihre Schreibmarke steht; dann steht es neben
dem Mauszeiger. Alternativ ein fester Platz, mit der Maus verschiebbar, auch auf andere Bildschirme. Im Menü des
Infobereichs lässt es sich abschalten, dann zeigt nur das Symbol (blau bereit, rot hört zu, orange transkribiert) den
Zustand. Solange diktiert wird, läuft das **Logo der STT-Suite** oben rechts am Mauszeiger mit (darunter erscheinen Tooltips), wie bei Agent-Orc
und SpeechPulse: Beim Zuhören bewegen sich seine Balken, beim Transkribieren steht es still und grau (abschaltbar; der
Mauszeiger selbst bleibt unberührt). Mit dem System starten: einstellbar.

Aus, solange nicht anders eingestellt: Anhalten nach Stille, Höchstdauer einer Aufnahme, Enter am Ende.

Mundwerk spricht Deutsch und Englisch (`system.language`: `auto` folgt der Systemsprache, oder `de`, `en`; oben im
Einstellungsfenster umschaltbar). Alle Texte stehen in einer Datei, [`ui/i18n.json`](ui/i18n.json), die die Fenster
laden und die App eingebaut hat; ein Test prüft, dass beide Sprachen dieselben Schlüssel haben und jeder benutzte
Schlüssel existiert. Eine weitere Sprache ist ein weiterer Block in dieser Datei.

## Einstellungen

Alle Einstellungen mit ihren Standardwerten stehen in
[`crates/mundwerk-core/default_config.toml`](crates/mundwerk-core/default_config.toml). Beim ersten Start schreibt Mundwerk
sie in den Einstellungsordner des Benutzers (unter Windows `%APPDATA%\de.peuqui.mundwerk\config.toml`), ab dann gilt
diese Datei. Jedes Feld muss darin stehen, ein unbekanntes ist ein Fehler, damit sich ein Tippfehler nicht still
verliert. Das Einstellungsfenster (Menü im Infobereich) ist ein Formular; Speichern prüft die Einstellungen, schreibt sie
in die Datei (deren Kommentare bleiben) und setzt sie sofort in Kraft. Auslöser werden live aufgenommen: „Aufnehmen“, dann
die Taste, Kombination oder Maustaste drücken; die Tasten erscheinen, während sie gedrückt werden. Tasten liest dabei das
Einstellungsfenster selbst, denn solange es vorne ist, gibt Windows sie Mundwerks Tastatur-Hook nicht; Maustasten kommen
vom Hook. Eine Kombination, die die Software einer Maus sendet (etwa Strg+Umschalt+Alt+D auf einer Seitentaste), zählt
wie getippt; eine so belegte Seitentaste sieht Windows selbst nie, nur ihre Tasten. Weil ein Makro die Tasten sofort
wieder loslässt, braucht sie „An und aus“. Links und rechts unterscheidet Mundwerk bei Strg, Umschalt und Alt nicht. Engine und Modell wählt die STT-Suite selbst, Mundwerk schickt nur Gerät und Sprache. Geht
die Datei beim Start nicht, läuft Mundwerk nicht mit Ersatzwerten, sondern öffnet das Einstellungsfenster mit dem Fehler
(dort lässt sich mit Standardwerten neu beginnen) und startet nach dem Speichern neu.

Die STT-Suite braucht für Aufrufe von einem anderen Rechner ihr Token (`server.token`, siehe deren Abschnitt
Sicherheit).

## Aufbau

- `crates/mundwerk-core`: die Logik ohne Oberfläche, auf jedem Rechner testbar (`cargo test -p mundwerk-core`).
- `src-tauri`: die App (Infobereich, Overlay, Einstellungsfenster, Hook, Mikrofon, Töne, Tippen) und `ui/` ihre Seiten
  (HTML, CSS, JavaScript ohne Bauschritt). Unter Windows ein eigener Low-Level-Hook: rdev ruft bei jeder Taste
  `ToUnicodeEx` auf und zerstört damit die Tottasten (^ ´ `). Unter macOS und Linux (X11) rdev. Unter Wayland sperrt
  das System globale Tasten und das Tippen in fremde Fenster; dort braucht es später einen eigenen Weg.
- `assets/sounds`: ping, pong und break aus FreeEchoDot2 (`audio_samples/`, Stand 37190a9, erzeugt mit
  `tools/gen_notify_sounds.py`), Ding-Dong und Dong-Ding ebenfalls von dort, ebenfalls algorithmisch erzeugt.

## Entwickeln

```
cargo test -p mundwerk-core
cargo clippy -p mundwerk-core --all-targets
cargo fmt --check
```

Die Windows-App entsteht auf Linux mit [cargo-xwin](https://github.com/rust-cross/cargo-xwin), das die Bibliotheken
von Windows selbst lädt (`cargo install --locked cargo-xwin tauri-cli`, `rustup target add x86_64-pc-windows-msvc`;
dazu clang und llvm, hier die Version 17 unter `/usr/lib/llvm-17/bin`):

```
PATH="/usr/lib/llvm-17/bin:$PATH" cargo xwin clippy --target x86_64-pc-windows-msvc -p mundwerk
cd src-tauri && PATH="/usr/lib/llvm-17/bin:$PATH" cargo tauri build --runner cargo-xwin --target x86_64-pc-windows-msvc --no-bundle
```

Das Ergebnis ist `target/x86_64-pc-windows-msvc/release/mundwerk.exe`, eine Datei ohne Installation (Windows 10 und
11 haben WebView2). Ein Installer (`--no-bundle` weglassen) braucht zusätzlich `nsis`.

Gegen eine laufende STT-Suite (wird sonst übersprungen):

```
MUNDWERK_LIVE_URL=http://127.0.0.1:5080 MUNDWERK_LIVE_TOKEN_FILE=~/.config/stt-suite/token MUNDWERK_LIVE_WAV=~/Projekte/STT-Suite/burn_in_speech.wav cargo test -p mundwerk-core --test live_stt -- --ignored
```

## Lizenz

[PolyForm Noncommercial 1.0.0](LICENSE). Die Töne in `assets/sounds` stammen aus FreeEchoDot2 (MIT, © 2026 Peuqui).
