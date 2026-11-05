// The settings as a form. Rust loads them, checks them on saving (an error names the field), writes them into the
// commented file and puts them into effect at once. A trigger is taken live through Mundwerk's system hook, so a
// combination a mouse's software sends counts as well.
const { invoke } = window.__TAURI__.core
const { listen } = window.__TAURI__.event

const CAPTURE_EVENT = 'mundwerk://capture'
const BUILTIN_PREFIX = 'builtin:'
const BUILTIN_SOUNDS = [['ping', 'Ping (hoch)'], ['pong', 'Pong (tiefer)'], ['break', 'Break (hoch, dann tief)']]
const OWN_FILE = 'file'

const form = document.getElementById('form')
const message = document.getElementById('message')

let config = null
let microphones = []
// The field a running capture writes into: { update(update), cancel() }, or null.
let capturing = null

// The names on the keys of a German keyboard, for the names the settings use.
const KEY_NAMES = {
  Ctrl: 'Strg', Shift: 'Umschalt', Alt: 'Alt', Meta: 'Win', Space: 'Leertaste', Escape: 'Esc', Enter: 'Eingabe',
  Backspace: 'Rücktaste', Delete: 'Entf', Insert: 'Einfg', Home: 'Pos1', End: 'Ende', PageUp: 'Bild ↑', PageDown: 'Bild ↓',
  Up: '↑', Down: '↓', Left: '←', Right: '→', CapsLock: 'Feststelltaste', ScrollLock: 'Rollen', PrintScreen: 'Druck',
  'Mouse:X1': 'Maus-Seitentaste hinten', 'Mouse:X2': 'Maus-Seitentaste vorne', 'Mouse:Middle': 'Mausrad-Klick',
}

// A trigger as key caps: "Ctrl+Shift+Alt+D" → [Strg] + [Umschalt] + [Alt] + [D].
function keyCaps(trigger) {
  if (!trigger) return [h('kbd', { class: 'empty' }, 'keine')]
  const parts = trigger.startsWith('Mouse:') ? [trigger] : trigger.split('+')
  return parts.flatMap((part, index) => [...(index ? [h('span', { class: 'plus' }, '+')] : []), h('kbd', {}, KEY_NAMES[part] ?? part)])
}

function say(words, kind) {
  message.textContent = words
  message.className = kind
  message.hidden = false
}

// Into Mundwerk's log (tray: "Protokoll öffnen"), to see on another machine what this page did.
function note(text) {
  // A log line that cannot be sent must not raise an error of its own (it would report itself again).
  invoke('page_log', { message: text }).catch(() => {})
}

// Any error of this page shows at the bottom and goes into the log.
function report(text) {
  say(`Fehler in der Seite: ${text}`, 'error')
  note(`error: ${text}`)
}
window.addEventListener('error', (event) => report(`${event.message} (${event.filename}:${event.lineno})`))
window.addEventListener('unhandledrejection', (event) => report(String(event.reason)))

// ── Fields ────────────────────────────────────────────────────────────────────────────────────────────────────

function text(object, key, attributes = {}) {
  return h('input', { type: 'text', value: object[key], spellcheck: 'false', ...attributes, oninput: (event) => { object[key] = event.target.value } })
}

function number(object, key, attributes = {}) {
  return h('input', { type: 'number', min: 0, step: 1, value: object[key], ...attributes, oninput: (event) => { object[key] = event.target.valueAsNumber } })
}

function check(object, key, label) {
  return h('label', { class: 'check' }, h('input', { type: 'checkbox', checked: object[key], onchange: (event) => { object[key] = event.target.checked } }), label)
}

function choice(object, key, options, onchange) {
  const select = h('select', { onchange: (event) => { object[key] = event.target.value; if (onchange) onchange() } },
    options.map(([value, label]) => h('option', { value }, label)))
  select.value = object[key]
  return select
}

function percent(object, key, lowest = 0) {
  const shown = h('span', { class: 'muted' }, `${object[key]} %`)
  return h('span', { class: 'inline' },
    h('input', { type: 'range', min: lowest, max: 100, step: 1, value: object[key], oninput: (event) => { object[key] = event.target.valueAsNumber; shown.textContent = `${object[key]} %` } }),
    shown)
}

function field(label, control, hint) {
  return [h('label', {}, label), control, hint ? h('div', { class: 'hint' }, hint) : null]
}

function section(title, hint, ...content) {
  return h('section', {}, h('h2', {}, title), hint ? h('p', { class: 'hint' }, hint) : null, ...content)
}

// A key, combination or mouse button, taken live: the button arms the capture; while it waits the keys held show up
// as they are pressed, and the next key or mouse button completes it.
function triggerField(object, key) {
  const caps = h('span', { class: 'caps' }, keyCaps(object[key]))
  const warning = h('span', { class: 'note' })
  // This field's capture while it runs: one click handler arms it or, while it runs, cancels it.
  let mine = null
  const button = h('button', { type: 'button', class: 'small', onclick: () => (capturing && capturing === mine ? disarm() : arm()) }, 'Aufnehmen')
  async function arm() {
    note(`capture: armed for ${key} (was ${object[key] || 'none'})`)
    if (capturing) await disarm()
    mine = { update, cancel: () => { caps.replaceChildren(...keyCaps(object[key])); done() } }
    capturing = mine
    caps.replaceChildren(h('kbd', { class: 'waiting' }, 'Taste drücken …'))
    warning.textContent = ''
    button.textContent = 'Abbrechen'
    button.classList.add('listening')
    await invoke('start_capture')
  }
  function update({ kind, trigger }) {
    note(`capture: ${kind} ${trigger}`)
    if (kind === 'done') {
      object[key] = trigger
      caps.replaceChildren(...keyCaps(trigger))
      done()
    } else if (kind === 'held') {
      caps.replaceChildren(...(trigger ? [...keyCaps(trigger), h('span', { class: 'plus' }, '+ …')] : [h('kbd', { class: 'waiting' }, 'Taste drücken …')]))
    } else {
      warning.textContent = `${trigger}: diese Taste geht nicht, bitte eine andere`
    }
  }
  function done() {
    capturing = null
    mine = null
    warning.textContent = ''
    button.textContent = 'Aufnehmen'
    button.classList.remove('listening')
  }
  return h('span', { class: 'trigger' }, caps, button, warning)
}

async function disarm() {
  if (!capturing) return
  const running = capturing
  note('capture: cancelled')
  await invoke('stop_capture')
  running.cancel()
}

// The keys of a capture come from this page: while Mundwerk's own window is in front, Windows does not hand them to
// its system hook (seen on Aragon: the mouse reached the hook, no key did). Mouse buttons still come from the hook.
// While a capture runs, a key does nothing else here.
const MODIFIER_CODES = ['ControlLeft', 'ControlRight', 'ShiftLeft', 'ShiftRight', 'AltLeft', 'AltRight', 'MetaLeft', 'MetaRight']
// KeyboardEvent.code as the settings name keys (letters, digits and F1-F24 below).
const CODE_NAMES = {
  Space: 'Space', Enter: 'Enter', NumpadEnter: 'Enter', Tab: 'Tab', Escape: 'Escape', Backspace: 'Backspace', Insert: 'Insert',
  Delete: 'Delete', Home: 'Home', End: 'End', PageUp: 'PageUp', PageDown: 'PageDown', ArrowUp: 'Up', ArrowDown: 'Down',
  ArrowLeft: 'Left', ArrowRight: 'Right', Pause: 'Pause', ScrollLock: 'ScrollLock', CapsLock: 'CapsLock', PrintScreen: 'PrintScreen',
}

function keyName(code) {
  if (/^Key[A-Z]$/.test(code)) return code.slice(3)
  if (/^Digit[0-9]$/.test(code)) return code.slice(5)
  if (/^F([1-9]|1[0-9]|2[0-4])$/.test(code)) return code
  return CODE_NAMES[code] ?? null
}

window.addEventListener('keydown', (event) => {
  if (!capturing) return
  event.preventDefault()
  event.stopPropagation()
  // The order of the settings: Ctrl, Shift, Alt, Meta.
  const held = [event.ctrlKey && 'Ctrl', event.shiftKey && 'Shift', event.altKey && 'Alt', event.metaKey && 'Meta'].filter(Boolean).join('+')
  if (MODIFIER_CODES.includes(event.code)) return capturing.update({ kind: 'held', trigger: held })
  const name = keyName(event.code)
  const trigger = [held, name ?? event.code].filter(Boolean).join('+')
  if (!name) return capturing.update({ kind: 'unusable', trigger })
  // The hook's capture ends too, or it would take the next press somewhere else.
  invoke('stop_capture')
  capturing.update({ kind: 'done', trigger })
}, true)

window.addEventListener('keyup', (event) => {
  if (!capturing) return
  event.preventDefault()
  const held = [event.ctrlKey && 'Ctrl', event.shiftKey && 'Shift', event.altKey && 'Alt', event.metaKey && 'Meta'].filter(Boolean).join('+')
  if (MODIFIER_CODES.includes(event.code)) capturing.update({ kind: 'held', trigger: held })
}, true)

listen(CAPTURE_EVENT, ({ payload }) => {
  if (capturing) capturing.update(payload)
  else note(`capture event without a capture: ${JSON.stringify(payload)}`)
}).then(() => note('listening for captures'))

// A list of rows: draw(item, index) gives a row's controls; add() a new item.
function rows(list, draw, add, addLabel) {
  const container = h('div', { class: 'rows' })
  function render() {
    container.replaceChildren(...list.map((item, index) => h('div', { class: 'row' },
      draw(item, index, render),
      h('button', { type: 'button', class: 'icon', title: 'Nach oben', disabled: index === 0, onclick: () => { list.splice(index - 1, 0, ...list.splice(index, 1)); render() } }, '↑'),
      h('button', { type: 'button', class: 'icon', title: 'Entfernen', onclick: () => { list.splice(index, 1); render() } }, '✕'))))
  }
  render()
  return [container, h('button', { type: 'button', class: 'add', onclick: () => { list.push(add()); render(); note(`added: ${addLabel}`) } }, addLabel)]
}

// ── Sections ──────────────────────────────────────────────────────────────────────────────────────────────────

function triggers() {
  return section('Auslöser', null,
    h('div', { class: 'explain' },
      h('p', {}, h('b', {}, 'Aufnehmen'), ' klicken und dann die Taste, Tastenkombination oder Maustaste drücken. Die Tasten erscheinen, während du sie drückst.'),
      h('p', {}, h('b', {}, 'Gedrückt halten'), ': Mundwerk hört zu, solange du die Taste hältst. ',
        h('b', {}, 'An und aus'), ': ein Druck startet, der nächste beendet. Eine Maustaste mit Makro (etwa Strg + Umschalt + Alt + D) braucht „An und aus“, weil das Makro die Tasten sofort wieder loslässt.'),
      h('p', {}, h('b', {}, 'Durchlassen'), ': normalerweise aus. Dann bekommt nur Mundwerk den Druck, die Seitentaste blättert also nicht zusätzlich im Browser zurück. An: das Programm, in dem du schreibst, bekommt ihn auch.')),
    ...rows(config.triggers, (trigger) => [
      triggerField(trigger, 'input'),
      choice(trigger, 'mode', [['hold', 'Gedrückt halten'], ['toggle', 'An und aus']]),
      check(trigger, 'pass_through', 'Durchlassen'),
    ], () => ({ input: '', mode: 'toggle', pass_through: false }), '+ Auslöser'))
}

function server() {
  const token = h('input', { type: 'password', value: config.server.token, spellcheck: 'false', autocomplete: 'off', oninput: (event) => { config.server.token = event.target.value } })
  const reveal = h('button', { type: 'button', class: 'small', onclick: () => { token.type = token.type === 'password' ? 'text' : 'password' } }, 'zeigen')
  const urls = config.server.urls
  return section('STT-Suite', 'Die Server werden der Reihe nach versucht; der nächste nur, wenn einer nicht erreichbar ist.',
    ...rows(urls, (url, index) => [h('input', { type: 'text', class: 'grow', value: url, spellcheck: 'false', oninput: (event) => { urls[index] = event.target.value } })],
      () => 'http://', '+ Server'),
    h('div', { class: 'fields spaced' },
      field('Token', h('span', { class: 'inline' }, token, reveal), 'Das Token der STT-Suite (auf dem Mini in ~/.config/stt-suite/token).'),
      field('Verbindungsaufbau', h('span', { class: 'inline' }, number(config.server, 'connect_timeout_ms'), 'ms'), 'So lange wartet Mundwerk auf einen Server, bevor es den nächsten versucht.')))
}

function recognition() {
  return section('Erkennung', 'Engine und Modell stellt die STT-Suite selbst ein.',
    h('div', { class: 'fields' },
      field('Gerät', choice(config.transcription, 'device', [['auto', 'Automatisch (GPU oder CPU, wie die STT-Suite entscheidet)'], ['cuda', 'Nur GPU'], ['cpu', 'Nur CPU']])),
      field('Sprache', text(config.transcription, 'language', { class: 'short' }), 'Sprachcode, z. B. de oder en.')))
}

function recording() {
  const options = [['', 'Systemstandard'], ...microphones.map((name) => [name, name])]
  if (config.recording.microphone && !microphones.includes(config.recording.microphone)) options.push([config.recording.microphone, `${config.recording.microphone} (nicht gefunden)`])
  return section('Aufnahme', null,
    h('div', { class: 'fields' },
      field('Mikrofon', choice(config.recording, 'microphone', options)),
      field('Abbrechen mit', triggerField(config.recording, 'cancel_key'), 'Verwirft eine laufende Aufnahme, ohne etwas zu schreiben.'),
      field('Mindestdauer', h('span', { class: 'inline' }, number(config.recording, 'min_press_ms'), 'ms'), 'Kürzer gehalten gilt als Versehen und wird verworfen.'),
      field('Anhalten nach Stille', h('span', { class: 'inline' }, number(config.recording, 'stop_after_silence_s'), 's'), '0 = nie.'),
      field('Stille heißt leiser als', percent(config.recording, 'silence_level_percent'), 'Nur für „Anhalten nach Stille“.'),
      field('Höchstdauer', h('span', { class: 'inline' }, number(config.recording, 'max_duration_s'), 's'), '0 = unbegrenzt.')))
}

function soundField(key) {
  const sounds = config.sounds
  const builtin = sounds[key].startsWith(BUILTIN_PREFIX)
  const state = { kind: builtin ? sounds[key].slice(BUILTIN_PREFIX.length) : OWN_FILE }
  const path = h('input', { type: 'text', value: builtin ? '' : sounds[key], placeholder: 'C:\\Pfad\\zum\\Ton.wav', hidden: builtin, oninput: (event) => { sounds[key] = event.target.value } })
  const select = choice(state, 'kind', [...BUILTIN_SOUNDS, [OWN_FILE, 'Eigene WAV-Datei …']], () => {
    path.hidden = state.kind !== OWN_FILE
    sounds[key] = state.kind === OWN_FILE ? path.value : BUILTIN_PREFIX + state.kind
  })
  return h('span', { class: 'inline' }, select, path)
}

function sounds() {
  return section('Töne', null,
    h('div', { class: 'fields' },
      field('Töne', check(config.sounds, 'enabled', 'spielen')),
      field('Start', soundField('start')),
      field('Ende', soundField('stop')),
      field('Abbruch', soundField('cancel')),
      field('Lautstärke', percent(config.sounds, 'volume_percent'))))
}

function insert() {
  const methods = [['type', 'Tippen'], ['clipboard', 'Über die Zwischenablage']]
  config.insert.per_program ??= []
  return section('Einfügen', 'Tippen schreibt Zeichen für Zeichen, die Zwischenablage bleibt unberührt. Einfügen legt den Text kurz in die Zwischenablage und drückt Strg+V.',
    h('div', { class: 'fields' },
      field('Methode', choice(config.insert, 'method', methods)),
      field('Zwischen zwei Diktaten', check(config.insert, 'space_between_dictations', 'ein Leerzeichen, wenn es ins selbe Fenster geht')),
      field('Am Ende', check(config.insert, 'press_enter', 'Enter drücken (sofort absenden)')),
      field('Zwischenablage zurück nach', h('span', { class: 'inline' }, number(config.insert, 'clipboard_restore_delay_ms'), 'ms'))),
    h('h3', { class: 'muted subheading' }, 'Programme mit eigener Methode'),
    ...rows(config.insert.per_program, (rule) => [
      text(rule, 'process', { class: 'grow', placeholder: 'Prozessname, z. B. mstsc.exe' }),
      choice(rule, 'method', methods),
    ], () => ({ process: '', method: 'clipboard' }), '+ Programm'))
}

function replacements() {
  return section('Ersetzungen', 'Was die Erkennung falsch versteht, vor allem Namen. Wörter: ganze Wörter, Groß- und Kleinschreibung egal, mehrere Varianten mit | trennen.',
    ...rows(config.replacements, (rule, index, render) => {
      const kind = { value: rule.pattern === undefined || rule.pattern === null ? 'words' : 'pattern' }
      const input = kind.value === 'words'
        ? h('input', { type: 'text', class: 'grow', value: rule.variants.join(' | '), placeholder: 'Komm mit | Kommit', spellcheck: 'false', oninput: (event) => { rule.variants = event.target.value.split('|').map((variant) => variant.trim()).filter(Boolean) } })
        : h('input', { type: 'text', class: 'grow', value: rule.pattern, placeholder: '(?i)\\bagent[ -]?org?\\b', spellcheck: 'false', oninput: (event) => { rule.pattern = event.target.value } })
      return [
        choice(kind, 'value', [['words', 'Wörter'], ['pattern', 'Regulärer Ausdruck']], () => {
          if (kind.value === 'words') { rule.variants = rule.pattern ? [rule.pattern] : []; delete rule.pattern } else { rule.pattern = rule.variants.join('|'); rule.variants = [] }
          render()
        }),
        input, h('span', {}, '→'), text(rule, 'replacement', { class: 'grow', placeholder: 'commit' }),
      ]
    }, () => ({ variants: [], replacement: '' }), '+ Ersetzung'))
}

function display() {
  return section('Anzeige', null,
    h('div', { class: 'fields' },
      field('Zustand zeigen', choice(config.overlay, 'mode', [['floating', 'Schwebendes Fenster (mit der Maus verschiebbar)'], ['tray_only', 'Nur das Symbol im Infobereich']])),
      field('Deckkraft', percent(config.overlay, 'opacity_percent', 10)),
      field('Fehler stehen lassen', h('span', { class: 'inline' }, number(config.overlay, 'error_shown_s'), 's'))))
}

function system() {
  return section('Verlauf und System', null,
    h('div', { class: 'fields' },
      field('Verlauf', h('span', { class: 'inline' }, number(config.history, 'size'), 'Diktate')),
      field('Mit Windows starten', check(config.system, 'autostart', 'ja'))))
}

// ── Loading and saving ────────────────────────────────────────────────────────────────────────────────────────

function draw(path) {
  form.replaceChildren(triggers(), server(), recognition(), recording(), sounds(), insert(), replacements(), display(), system(),
    h('p', { class: 'muted' }, 'Gespeichert in ', h('code', {}, path)))
}

function drawBroken(loaded) {
  form.replaceChildren(h('section', { class: 'banner' },
    h('h2', {}, 'Die Einstellungen gehen nicht'),
    h('p', {}, loaded.error),
    h('p', { class: 'muted' }, 'Datei: ', h('code', {}, loaded.path)),
    h('button', { type: 'button', onclick: async () => { config = await invoke('default_settings'); draw(loaded.path); say('Standardwerte eingesetzt – noch nicht gespeichert. Das Token ist darin leer.', 'ok') } },
      'Mit Standardwerten neu beginnen')))
}

async function load() {
  await disarm()
  const loaded = await invoke('load_settings')
  microphones = await invoke('microphones').catch(() => [])
  message.hidden = true
  if (loaded.settings) {
    config = loaded.settings
    draw(loaded.path)
    note(`loaded ${loaded.path}: ${config.triggers.length} triggers`)
  } else {
    config = null
    drawBroken(loaded)
  }
}

function numbersValid() {
  return [...form.querySelectorAll('input[type="number"]')].every((input) => Number.isInteger(input.valueAsNumber) && input.valueAsNumber >= 0)
}

document.getElementById('save').addEventListener('click', async () => {
  if (!config) return say('Erst „Mit Standardwerten neu beginnen“.', 'error')
  if (!numbersValid()) return say('Eine Zahl fehlt oder ist keine ganze Zahl ab 0.', 'error')
  try {
    note('save clicked')
    await invoke('save_settings', { settings: config })
    say('Gespeichert und in Kraft.', 'ok')
  } catch (error) {
    say(`Nicht gespeichert: ${error}`, 'error')
  }
})

document.getElementById('reload').addEventListener('click', () => load().catch((error) => say(String(error), 'error')))

load().catch((error) => say(String(error), 'error'))
