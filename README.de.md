# Mundwerk

Diktieren in jedes Textfeld: Taste oder Maustaste drücken, sprechen, noch einmal drücken, und der Text steht dort, wo
der Cursor ist, in jedem Programm. Die Spracherkennung läuft nicht auf dem Rechner, an dem du diktierst, sondern in der
[STT-Suite](https://github.com/Peuqui/STT-Suite) auf einem anderen. Die entscheidet zusammen mit
[GPU-Orc](https://github.com/Peuqui/GPU-Orc), ob eine Grafikkarte oder der Prozessor transkribiert. Mundwerk belegt also
keinen Grafikspeicher dort, wo du arbeitest, und eine Spracherkennung bedient jeden Rechner im Haus.

Mundwerk ist ein kleines Programm im Infobereich für Windows (Rust und [Tauri](https://tauri.app)), angelegt, um auch auf
macOS und Linux zu laufen.

[English version](README.md)

## Was es kann

- **Auslöser:** jede Taste, Tastenkombination oder Maustaste (Mitte, Seitentasten), jeweils *gedrückt halten* (hört zu,
  solange gehalten) oder *an und aus* (ein Druck startet, der nächste beendet). Im Einstellungsfenster live aufgenommen:
  drücken, und er ist gesetzt. Auch eine Kombination, die die Software einer Maus sendet, zählt.
- **Töne** bei Start, Ende und Abbruch (ping, pong und break des Echo Dot, sein Dong-Ding und Ding-Dong oder eine eigene
  WAV-Datei), jeder in den Einstellungen anhörbar.
- **Schreibt ins aktive Feld**, getippt (Unicode, mit Umlauten, ohne die Zwischenablage zu berühren) oder eingefügt, je
  Programm wählbar; ein Leerzeichen zwischen zwei Diktaten ins selbe Fenster; auf Wunsch Enter am Ende (schickt eine
  Chat-Nachricht sofort ab).
- **Ersetzungsliste** für das, was die Erkennung falsch versteht, vor allem Namen: Varianten als ganze Wörter
  (`Komm mit | Kommit → commit`) oder ein regulärer Ausdruck.
- **Overlay** neben der Stelle, an der du schreibst, und das **Logo der STT-Suite am Mauszeiger**, solange diktiert
  wird (beim Zuhören bewegen sich seine Balken, beim Transkribieren wird es grau), wie bei Agent-Orc und SpeechPulse.
- **Nichts geht verloren:** ein Verlauf der letzten Diktate im Infobereich (ein Klick kopiert eines), eine Taste, die das
  letzte Diktat noch einmal dorthin schreibt, wo der Cursor jetzt steht, und eine Aufnahme, die die STT-Suite nicht
  transkribieren konnte, wartet im Infobereich darauf, erneut gesendet zu werden.
- **Mehrere Server der Reihe nach**, etwa zuerst ein Direktkabel und dann das Heimnetz; der nächste nur, wenn einer nicht
  erreichbar ist.
- Deutsch und Englisch, umschaltbar; auf Wunsch mit dem System starten.

## Loslegen

**1. Die STT-Suite.** Mundwerk braucht eine laufende [STT-Suite](https://github.com/Peuqui/STT-Suite), die es erreicht. Für
Aufrufe von einem anderen Rechner braucht die Suite ihr Token und muss über ihren eigenen Rechner hinaus lauschen
(`STT_HOST=0.0.0.0`); wie, steht in ihrem README im Abschnitt *Sicherheit*. Das Token liegt auf dem Rechner der Suite in
`~/.config/stt-suite/token`.

**2. Mundwerk.** Bis es ein Release gibt, `mundwerk.exe` bauen, wie unter [Bauen](#bauen) beschrieben. Es ist eine
einzelne Datei ohne Installation; Windows 10 und 11 bringen das nötige WebView2 mit. Windows SmartScreen warnt beim
ersten Start vor einem nicht signierten Programm: *Weitere Informationen*, dann *Trotzdem ausführen*.

**3. Erster Start.** Es öffnet sich kein Fenster: Mundwerk sitzt im Infobereich (eine blaue Sprechblase mit Mikrofon,
eventuell hinter dem `^`). Sein Menü hat die Einstellungen, den Overlay-Schalter, den Verlauf, *erneut senden*, das
Protokoll und *Beenden*. In den Einstellungen:

- unter *STT-Suite* die Adressen deiner Suite (von oben nach unten versucht) und ihr **Token**;
- unter *Auslöser* auf *Aufnehmen* klicken und die gewünschte Taste oder Maustaste drücken; *gedrückt halten* oder *an
  und aus* wählen;
- *Speichern*: Es gilt sofort, und das Fenster schließt sich.

Dann in irgendein Textfeld klicken, den Auslöser drücken, sprechen, noch einmal drücken (oder bei *gedrückt halten*
loslassen): Ping, Pong, und der Text steht da.

## Bedienung

**Halten oder an und aus.** *Gedrückt halten* hört zu, solange die Taste gehalten wird; ein Druck kürzer als die
Mindestdauer (300 ms) gilt als Versehen und wird ohne Ton verworfen. *An und aus* startet mit einem Druck und endet mit dem
nächsten. Eine Maustaste mit **Makro** (ihre Software sendet zum Beispiel Strg+Umschalt+Alt+D) braucht *an und aus*: Das
Makro lässt die Tasten sofort wieder los. Für Mundwerk *ist* so eine Taste diese Kombination; die Taste selbst sieht
Windows nie, nur ihre Tasten. *Durchlassen* gibt den Druck zusätzlich an das Programm darunter; normalerweise bleibt das
aus, damit eine Seitentaste nicht nebenbei im Browser zurückblättert.

**Abbrechen.** Esc (oder die eingestellte Taste) verwirft eine laufende Aufnahme, mit dem Break-Ton. Während einer
Aufnahme wird die Taste geschluckt, sonst geht sie unberührt durch.

**Wohin der Text geht.** In das Feld, das den Fokus hat, wenn der Text von der STT-Suite zurückkommt. Zwei Diktate
hintereinander ins selbe Fenster bekommen ein Leerzeichen dazwischen.

**Ein Diktat, das ins Leere ging** (kein Textfeld hatte den Fokus): ins richtige Feld klicken und die Taste für *letztes
Diktat noch einmal* drücken (in den Einstellungen unter *Einfügen*), oder es aus dem Verlauf im Infobereich kopieren.

**Wenn die STT-Suite nicht erreichbar ist** oder ablehnt: Das Overlay sagt ein paar Sekunden lang warum, und die
Aufnahme wartet im Infobereich unter *Letzte Aufnahme erneut senden*.

**Ins Terminal schreiben.** Mundwerk „tippt“ den Text: Es gibt Windows Tastendrücke, als kämen sie von einer Tastatur
(`SendInput` mit Unicode-Zeichen), und Windows stellt sie dem Fenster vorne zu, einem Terminal genauso wie einem Editor.
Windows Terminal und die meisten Programme nehmen das als Tippen. Ein Programm, das das nicht tut (manche Remote-Desktops,
ältere Konsolen), bekommt den Text stattdessen eingefügt: unter *Einfügen*, *Programme mit eigener Methode*, seinen
Prozessnamen (etwa `mstsc.exe`) mit *über die Zwischenablage*. Mundwerk legt den Text dann in die Zwischenablage, drückt
Strg+V und stellt zurück, was die Zwischenablage vorher hatte.

## Overlay und Logo

Das **Overlay** (hört zu, transkribiert, Fehler) nimmt dem Textfeld nie den Fokus. Standardmäßig erscheint es unten
rechts neben der Schreibmarke, auf deren Bildschirm. Viele Programme (Browser, Electron und andere, die ihren Text selbst
zeichnen) sagen Windows nicht, wo ihre Schreibmarke steht; dann erscheint es neben dem Mauszeiger. Alternativ ein fester
Platz, mit der Maus verschiebbar, auf jedem Bildschirm. Im Menü des Infobereichs lässt es sich abschalten; dann zeigt nur
das Symbol den Zustand (blau bereit, rot hört zu, orange transkribiert).

Das **Logo am Mauszeiger** ist das Diktat-Abzeichen von Agent-Orc: das Logo der STT-Suite in Bernstein, oben rechts an der
Spitze des Zeigers, und es wandert mit, solange diktiert wird. Der Mauszeiger selbst bleibt, wie er ist. Abschaltbar.

## Einstellungen

Das Einstellungsfenster ist ein Formular; Speichern prüft alles, schreibt die Datei und setzt sie sofort in Kraft. Die
Datei selbst ist `%APPDATA%\de.peuqui.mundwerk\config.toml`; jede Einstellung ist darin erklärt. Ihre Standardwerte, mit
denselben Erklärungen, stehen in [`crates/mundwerk-core/default_config.toml`](crates/mundwerk-core/default_config.toml).

| Abschnitt | Was |
|---|---|
| Sprache | automatisch (Systemsprache), Deutsch, Englisch |
| Auslöser | Tasten, Kombinationen, Maustasten; halten oder an und aus; durchlassen |
| STT-Suite | Server der Reihe nach, Token, wie lange auf einen Server gewartet wird |
| Erkennung | Gerät (automatisch, nur GPU, nur CPU) und Sprache der Diktate; Engine und Modell stellt die Suite ein |
| Aufnahme | Mikrofon, Abbruchtaste, Mindestdauer, Anhalten nach Stille, Höchstdauer (beide aus, solange nicht gesetzt) |
| Töne | an oder aus, Start, Ende, Abbruch, Lautstärke |
| Einfügen | Tippen oder Zwischenablage, je Programm; Leerzeichen zwischen Diktaten; Enter am Ende; Taste für das letzte Diktat noch einmal |
| Ersetzungen | Wörter oder reguläre Ausdrücke, und was stattdessen geschrieben wird |
| Anzeige | Overlay oder nur das Symbol, sein Platz, das Logo am Mauszeiger, Deckkraft, wie lange ein Fehler stehen bleibt |
| Verlauf und System | wie viele Diktate der Verlauf hält; mit dem System starten |

Jedes Feld muss in der Datei stehen, und eines, das Mundwerk nicht kennt, ist ein Fehler, damit sich ein Tippfehler nicht
still verliert. Geht die Datei beim Start nicht, läuft Mundwerk nicht mit Ersatzwerten: Das Einstellungsfenster öffnet
sich mit dem Grund (und kann mit Standardwerten neu beginnen); Speichern startet Mundwerk neu.

## Wenn etwas nicht geht

*Protokoll öffnen* im Menü des Infobereichs öffnet `%LOCALAPPDATA%\de.peuqui.mundwerk\logs\mundwerk.log`. Darin stehen
jeder Auslöser, jeder Zustand, jede Transkription mit Server und Gerät, jeder Fehler und, während ein Auslöser
aufgenommen wird, jede rohe Taste und Maustaste, die der System-Hook sieht.

- **„STT-Suite refused (401 unauthorized)“**: Das Token fehlt oder stimmt nicht.
- **„no STT-Suite reachable“**: Keine der Adressen hat geantwortet; die Aufnahme wartet im Infobereich.
- **Nur ein Ping, und das Overlay verschwindet gleich wieder**: Ein Auslöser auf *gedrückt halten* wurde vor der
  Mindestdauer losgelassen, typisch für eine Maustaste mit Makro: auf *an und aus* stellen.

## Bauen

Windows wird auf Linux mit [cargo-xwin](https://github.com/rust-cross/cargo-xwin) gebaut, das die Bibliotheken von Windows
selbst lädt (`cargo install --locked cargo-xwin tauri-cli`, `rustup target add x86_64-pc-windows-msvc`, dazu clang und
llvm, hier die Version 17 unter `/usr/lib/llvm-17/bin`):

```
PATH="/usr/lib/llvm-17/bin:$PATH" cargo xwin clippy --target x86_64-pc-windows-msvc -p mundwerk
cd src-tauri && PATH="/usr/lib/llvm-17/bin:$PATH" cargo tauri build --runner cargo-xwin --target x86_64-pc-windows-msvc --no-bundle
```

Das Ergebnis ist `target/x86_64-pc-windows-msvc/release/mundwerk.exe`. Ein Installer (`--no-bundle` weglassen) braucht
zusätzlich `nsis`. Die Logik hat ihre Tests auf jedem Rechner:

```
cargo test -p mundwerk-core
cargo clippy -p mundwerk-core --all-targets
cargo fmt --check
```

und gegen eine laufende STT-Suite (sonst übersprungen):

```
MUNDWERK_LIVE_URL=http://127.0.0.1:5080 MUNDWERK_LIVE_TOKEN_FILE=~/.config/stt-suite/token MUNDWERK_LIVE_WAV=~/Projekte/STT-Suite/burn_in_speech.wav cargo test -p mundwerk-core --test live_stt -- --ignored
```

## Innen

- `crates/mundwerk-core`: die Logik ohne Oberfläche, auf jedem Rechner getestet: Einstellungen, Auslöser, wann ein Diktat
  beginnt und endet, das Live-Aufnehmen eines Auslösers, Ersetzungen, das Leerzeichen zwischen Diktaten, der Aufruf der
  STT-Suite mit Rückfall, Verlauf, die Texte in beiden Sprachen.
- `src-tauri`: die App: Infobereich, Overlay, Logo, Einstellungsfenster, der System-Hook, Mikrofon, Töne, Tippen. Unter
  Windows ein eigener Low-Level-Hook: Die übliche Bibliothek (rdev) ruft bei jeder Taste `ToUnicodeEx` auf und zerstört
  damit die Tottasten (^ ´ `). Unter macOS und Linux (X11) rdev; unter Wayland sperrt das System globale Tasten und das
  Tippen in fremde Fenster, dafür braucht es später einen eigenen Weg.
- `ui/`: die Seiten (HTML, CSS, JavaScript ohne Bauschritt) und `i18n.json`, alle Texte von Mundwerk auf Deutsch und
  Englisch; ein Test prüft, dass beide Sprachen dieselben Schlüssel haben und jeder benutzte Schlüssel existiert. Eine
  weitere Sprache ist ein weiterer Block in dieser Datei.
- `assets/sounds`: die Töne, aus FreeEcho.2 (Peuquis Echo-Dot-Projekt, nicht öffentlich), dort algorithmisch erzeugt.

## Star History

![Star History](https://raw.githubusercontent.com/Peuqui/Traffic/main/data/Mundwerk/star-history.svg)

<sub>Täglich erhoben in [Peuqui/Traffic](https://github.com/Peuqui/Traffic), zusammen mit Aufrufen, Klonen und Downloads.</sub>

## Lizenz

[PolyForm Noncommercial 1.0.0](LICENSE). Die Töne in `assets/sounds` stammen aus FreeEcho.2 (MIT, © 2026 Peuqui).
