# Idee: Mundwerk auf Android

Stand: Diskussion, nicht begonnen (5. November 2026).

Ziel: auf dem Handy in jede App diktieren, mit der eigenen STT-Suite statt der Spracheingabe von Gboard, mit
Ersetzungsliste und Verlauf wie unter Windows.

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

## Empfehlung

Weg 1 als Grundlage (sauber, offiziell, jede App). Die Blase später, falls sie wirklich gebraucht wird.

## Netz

- Zu Hause: wie unter Windows, übers WLAN an die STT-Suite, mit ihrem Token.
- Unterwegs: nur verschlüsselt (HTTPS), also über einen Reverse-Proxy vor der STT-Suite. Der Proxy darf dafür keine
  eigene Anmeldung mit Login-Seite verlangen (die kann eine App nicht ausfüllen); die Absicherung übernimmt das Token der
  Suite. Das wäre eine eigene Ausnahme im Proxy, nur für `/transcribe`.

## Offene Fragen

- Welche Tastatur auf dem Handy: bei Gboard bleiben (Wechsel per Tastatursymbol) oder HeliBoard (Mikrofon-Knopf
  startet Mundwerk)?
- Reicht zu Hause, oder soll es von Anfang an unterwegs gehen (dann zuerst der Proxy)?
- Android Auto: gewünscht? Dann Weg 2 und seine Grenzen dort genauer ansehen.
- Bauen auf dem Mini: Android-SDK und Build-Tools wären neu zu installieren.
