# Idee: Mundwerk auf Android

Stand: Diskussion, nicht begonnen (5. November 2026).

Ziel: auf dem Handy in jede App diktieren, mit der eigenen STT-Suite statt der Spracheingabe von Gboard (die versteht
manches „ziemlich banane“, Peuqui), mit Ersetzungsliste und Verlauf wie unter Windows.

## Warum nicht einfach die Windows-App

Mundwerk unter Windows hängt sich mit einem System-Hook an Tastatur und Maus und „tippt“ den Text ins Fenster vorne. So
etwas lässt Android nicht zu: Keine App sieht die Tasten anderer Apps oder schreibt einfach in sie hinein. Android hat
dafür eigene, erlaubte Wege, und die verlangen eine eigene kleine App (Kotlin). Tauri kann zwar Android, aber keine
Eingabemethode und keinen Bedienungshilfe-Dienst; die Logik (Ersetzungen, Aufruf der STT-Suite, Verlauf) ist schlank
genug, um sie dort neu zu schreiben oder `mundwerk-core` einzubinden.

## Weg 1: eine Spracheingabe als Eingabemethode (Tastatur)

Eine Eingabemethode („IME“), die nur aus einem großen Mikrofon-Knopf besteht. Ins Feld tippen, auf „Mundwerk“
wechseln (Tastatursymbol unten oder Globus), sprechen; die STT-Suite transkribiert, Android schreibt den Text direkt ins
Feld (`InputConnection.commitText`), danach springt es zur vorigen Tastatur zurück.

- Der offizielle Weg; so arbeitet auch das Mikrofon von Gboard. Geht in jeder App, braucht keine heiklen Rechte.
- Gboard gibt sein Mikrofon nicht an fremde Spracheingaben weiter. Andere Tastaturen schon, etwa HeliBoard (die
  Open-Source-Fassung der AOSP-Tastatur): Dort startet ihr Mikrofon-Knopf die eingestellte Spracheingabe, also Mundwerk.
  Dann gäbe es den gewohnten Ablauf, nur mit der eigenen STT-Suite.
- Mit Gboard bliebe der Wechsel über das Tastatursymbol.

## Weg 2: eine schwebende Blase über allen Apps

Ein Knopf, der über jeder App schwebt (Recht „Über anderen Apps einblenden“); den Text schreibt ein
Bedienungshilfe-Dienst ins aktive Feld. Kommt dem Windows-Mundwerk am nächsten.

- Die Bedienungshilfe ist ein sehr mächtiges Recht; Android warnt deutlich, Google Play lässt es kaum zu (für eine App
  nur für sich selbst egal).
- Manche Apps nehmen nur „Text ersetzen“ an (`ACTION_SET_TEXT`, das ganze Feld) statt „einfügen“; dann über die
  Zwischenablage (`ACTION_PASTE`).
- In Android Auto sind solche Overlays stark eingeschränkt; wie es dort gehen kann, wäre zu klären.

**Die Tastatur, die beim Antippen eines Felds hochkommt** (Peuqui: nervig), kann genau dieser Dienst unten halten: Eine
Bedienungshilfe darf die Bildschirmtastatur ausblenden (`AccessibilityService.getSoftKeyboardController()` mit
`SHOW_MODE_HIDDEN`, seit Android 7). Solange der Diktat-Modus an ist, bleibt sie weg: ins Feld tippen, die Schreibmarke
steht, keine Tastatur, die Blase diktiert hinein. Ein langer Druck auf die Blase holt die Tastatur zurück, wenn doch
getippt werden soll.

**Die Blase nur, wenn sie gebraucht wird** (Peuqui: dauernd sichtbar würde sie stören und Bildschirm verdecken): Der
Dienst erfährt, wann ein Eingabefeld den Fokus bekommt und verliert (`TYPE_VIEW_FOCUSED`, Knoten `isEditable`). Die
Blase erscheint also nur, solange ein Textfeld aktiv ist, klein neben ihm oder am Rand, und geht mit dem Fokus wieder;
dazu verschiebbar und halb an den Rand zu schieben. Man sieht dann, was man jetzt sieht, nur ohne Tastatur und mit
Mikrofon.

**Oder ganz ohne Blase:** Der Dienst darf Hardwaretasten abfangen (`FLAG_REQUEST_FILTER_KEY_EVENTS`); ein langer Druck
auf „Leiser“ könnte das Diktat starten und beenden, wie unter Windows die Seitentaste der Maus. Beides lässt sich
verbinden.

**An der Schreibmarke einfügen:** `ACTION_SET_TEXT` ersetzt das ganze Feld. Mundwerk liest deshalb den Text des Felds
und die Stelle der Schreibmarke (`textSelectionStart`/`End`), setzt das Diktat dort ein (mit Leerzeichen davor, wenn
nötig) und stellt die Marke dahinter (`ACTION_SET_SELECTION`). Bei gewöhnlichen Eingabefeldern geht das; Apps mit
eigenen Eingabefeldern (Web-Oberflächen, manche Chat-Apps) können sich anders verhalten, dann über die Zwischenablage.

## Empfehlung

Peuqui findet die Blase am interessantesten, und mit der ausgeblendeten Tastatur passt sie am besten zu dem, was
Mundwerk unter Windows ist: tippen, diktieren, fertig, ohne Tastatur dazwischen. Also Weg 2, für eine App nur für sich
selbst (nicht über Google Play). Weg 1 bleibt die Rückfallebene, falls eine wichtige App das Einfügen per Bedienungshilfe
nicht mitmacht.

## Netz

- Zu Hause: wie unter Windows, übers WLAN an die STT-Suite, mit ihrem Token.
- Unterwegs: nur verschlüsselt (HTTPS), also über einen Reverse-Proxy vor der STT-Suite. Der Proxy darf dafür keine
  eigene Anmeldung mit Login-Seite verlangen (die kann eine App nicht ausfüllen); die Absicherung übernimmt das Token der
  Suite. Das wäre eine eigene Ausnahme im Proxy, nur für `/transcribe`.

## Offene Fragen

- Die Apps, um die es geht (Peuqui: alles, wo man sonst tippt): WhatsApp, E-Mail (FairEmail, Gmail), Browser. An ihnen
  zuerst prüfen, ob das Einfügen an der Schreibmarke geht oder die Zwischenablage nötig ist. Erwartung: WhatsApp und
  FairEmail haben gewöhnliche Eingabefelder (geht); Gmail Betreff und Empfänger ja, beim Textteil hängt es an der App;
  im Browser gewöhnliche Formularfelder meist ja, Web-Editoren, die ihren Text selbst zeichnen, über die Zwischenablage.
- Blase nur bei aktivem Textfeld, Leiser-Taste oder beides?
- Reicht zu Hause, oder soll es von Anfang an unterwegs gehen (dann zuerst der Proxy)?
- Android Auto: gewünscht? Dann Weg 2 und seine Grenzen dort genauer ansehen.
- Bauen auf dem Mini: Android-SDK und Build-Tools wären neu zu installieren.
