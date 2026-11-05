// The settings as a form. Rust loads them, checks them on saving (an error names the field), writes them into the
// commented file and puts them into effect at once. A trigger is taken live through Mundwerk's system hook, so a
// combination a mouse's software sends counts as well.
const { invoke } = window.__TAURI__.core
const { listen } = window.__TAURI__.event

const CAPTURED_EVENT = 'mundwerk://captured'
const BUILTIN_PREFIX = 'builtin:'
const BUILTIN_SOUNDS = [['ping', 'Ping (hoch)'], ['pong', 'Pong (tiefer)'], ['break', 'Break (hoch, dann tief)']]
const OWN_FILE = 'file'

const form = document.getElementById('form')
const message = document.getElementById('message')

let config = null
let microphones = []
// The field a running capture writes into: { set(text) }, or null.
let capturing = null

function say(words, kind) {
  message.textContent = words
  message.className = kind
  message.hidden = false
}

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
  return h('select', { onchange: (event) => { object[key] = event.target.value; if (onchange) onchange() } },
    options.map(([value, label]) => h('option', { value, selected: object[key] === value }, label)))
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

// A key, combination or mouse button, taken live: the button arms the capture, the next press fills it.
function triggerField(object, key) {
  const shown = h('kbd', {}, label(object[key]))
  const button = h('button', { type: 'button', class: 'small', onclick: () => arm() }, 'Aufnehmen')
  function label(trigger) {
    return trigger ? trigger.replace('Mouse:', 'Maus ') : 'keine'
  }
  async function arm() {
    if (capturing) await disarm()
    capturing = { set(trigger) { object[key] = trigger; shown.textContent = label(trigger); done() }, cancel: done }
    button.textContent = 'Jetzt drücken … (Klick = Abbrechen)'
    button.classList.add('listening')
    button.onclick = () => disarm()
    await invoke('start_capture')
  }
  function done() {
    capturing = null
    button.textContent = 'Aufnehmen'
    button.classList.remove('listening')
    button.onclick = () => arm()
  }
  return h('span', { class: 'inline' }, shown, button)
}

async function disarm() {
  if (!capturing) return
  const running = capturing
  await invoke('stop_capture')
  running.cancel()
}

listen(CAPTURED_EVENT, ({ payload }) => {
  if (capturing) capturing.set(payload)
})

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
  return [container, h('button', { type: 'button', class: 'add', onclick: () => { list.push(add()); render() } }, addLabel)]
}

// ── Sections ──────────────────────────────────────────────────────────────────────────────────────────────────

function triggers() {
  return section('Auslöser', 'Taste, Tastenkombination oder Maustaste (Mitte, Seitentasten). „Aufnehmen“ und dann die Taste drücken – auch eine Kombination, die die Maussoftware sendet.',
    ...rows(config.triggers, (trigger) => [
      triggerField(trigger, 'input'),
      choice(trigger, 'mode', [['hold', 'Gedrückt halten'], ['toggle', 'An und aus']]),
      check(trigger, 'pass_through', 'auch ans Programm weitergeben'),
      h('span', { class: 'grow' }),
    ], () => ({ input: '', mode: 'hold', pass_through: false }), '+ Auslöser'))
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
    await invoke('save_settings', { settings: config })
    say('Gespeichert und in Kraft.', 'ok')
  } catch (error) {
    say(`Nicht gespeichert: ${error}`, 'error')
  }
})

document.getElementById('reload').addEventListener('click', () => load().catch((error) => say(String(error), 'error')))

load().catch((error) => say(String(error), 'error'))
