// The settings as a form. Rust loads them, checks them on saving (an error names the field), writes them into the
// commented file and puts them into effect at once. Every word comes from i18n.json (t), German or English.
const { invoke } = window.__TAURI__.core
const { listen } = window.__TAURI__.event

const CAPTURE_EVENT = 'mundwerk://capture'
const BUILTIN_PREFIX = 'builtin:'
const OWN_FILE = 'file'

const form = document.getElementById('form')
const message = document.getElementById('message')

let config = null
let path = ''
let microphones = []
// The field a running capture writes into: { update(update), cancel() }, or null.
let capturing = null

// The arrow keys look the same in every language.
const ARROWS = { Up: '↑', Down: '↓', Left: '←', Right: '→' }

// A key as its cap says it: "Strg" for Ctrl on a German keyboard; a letter or F9 as it is.
function keyLabel(part) {
  return ARROWS[part] ?? tIfAny(`key.${part}`) ?? part
}

// A trigger as key caps: "Ctrl+Shift+Alt+D" → [Strg] + [Umschalt] + [Alt] + [D].
function keyCaps(trigger) {
  if (!trigger) return [h('kbd', { class: 'empty' }, t('capture.none'))]
  const parts = trigger.startsWith('Mouse:') ? [trigger] : trigger.split('+')
  return parts.flatMap((part, index) => [...(index ? [h('span', { class: 'plus' }, '+')] : []), h('kbd', {}, keyLabel(part))])
}

function say(words, kind) {
  message.textContent = words
  message.className = kind
  message.hidden = false
}

// Into Mundwerk's log (tray: open the log), to see on another machine what this page did.
function note(text) {
  // A log line that cannot be sent must not raise an error of its own (it would report itself again).
  invoke('page_log', { message: text }).catch(() => {})
}

// Any error of this page shows at the bottom and goes into the log.
function report(text) {
  // Before the texts are loaded there is no translation yet; the error must still show.
  const lead = texts ? t('settings.page_error') : 'Error in the page'
  say(`${lead}: ${text}`, 'error')
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
  const button = h('button', { type: 'button', class: 'small', onclick: () => (capturing && capturing === mine ? disarm() : arm()) }, t('capture.record'))
  async function arm() {
    note(`capture: armed for ${key} (was ${object[key] || 'none'})`)
    if (capturing) await disarm()
    mine = { update, cancel: () => { caps.replaceChildren(...keyCaps(object[key])); done() } }
    capturing = mine
    caps.replaceChildren(h('kbd', { class: 'waiting' }, t('capture.press')))
    warning.textContent = ''
    button.textContent = t('capture.cancel')
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
      caps.replaceChildren(...(trigger ? [...keyCaps(trigger), h('span', { class: 'plus' }, '+ …')] : [h('kbd', { class: 'waiting' }, t('capture.press'))]))
    } else {
      warning.textContent = `${trigger}: ${t('capture.unusable')}`
    }
  }
  function done() {
    capturing = null
    mine = null
    warning.textContent = ''
    button.textContent = t('capture.record')
    button.classList.remove('listening')
  }
  return h('span', { class: 'trigger' }, caps, button, warning)
}

// A trigger that may be off: the field and a button that switches it off.
function optionalTrigger(object, key) {
  const box = h('span', { class: 'inline' })
  const render = () => box.replaceChildren(triggerField(object, key),
    h('button', { type: 'button', class: 'small', disabled: !object[key], onclick: () => { object[key] = ''; render() } }, t('common.off')))
  render()
  return box
}

async function disarm() {
  if (!capturing) return
  const running = capturing
  note('capture: cancelled')
  await invoke('stop_capture')
  running.cancel()
}

// The keys of a capture come from this page: while Mundwerk's own window is in front, Windows does not hand them to
// its system hook (seen on Windows 11: the mouse reached the hook, no key did). Mouse buttons still come from the hook.
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
      h('button', { type: 'button', class: 'icon', title: t('common.up'), disabled: index === 0, onclick: () => { list.splice(index - 1, 0, ...list.splice(index, 1)); render() } }, '↑'),
      h('button', { type: 'button', class: 'icon', title: t('common.remove'), onclick: () => { list.splice(index, 1); render() } }, '✕'))))
  }
  render()
  return [container, h('button', { type: 'button', class: 'add', onclick: () => { list.push(add()); render(); note(`added: ${addLabel}`) } }, addLabel)]
}

// ── Sections ──────────────────────────────────────────────────────────────────────────────────────────────────

function languageSection() {
  const options = [['auto', t('settings.language_auto')], ['de', 'Deutsch'], ['en', 'English']]
  const select = choice(config.system, 'language', options, async () => {
    // The page shows the chosen language at once; the tray and overlay follow on saving.
    await loadTexts(config.system.language)
    translateStatic()
    draw()
  })
  select.classList.add('language')
  return section(t('settings.language'), null, select)
}

function triggers() {
  return section(t('triggers.title'), null,
    h('div', { class: 'explain' },
      h('p', {}, h('b', {}, t('triggers.capture_lead')), t('triggers.capture_text')),
      h('p', {}, h('b', {}, t('triggers.hold_lead')), t('triggers.hold_text'), h('b', {}, t('triggers.toggle_lead')), t('triggers.toggle_text')),
      h('p', {}, h('b', {}, t('triggers.pass_lead')), t('triggers.pass_text'))),
    ...rows(config.triggers, (trigger) => [
      triggerField(trigger, 'input'),
      choice(trigger, 'mode', [['hold', t('triggers.hold')], ['toggle', t('triggers.toggle')]]),
      check(trigger, 'pass_through', t('triggers.pass')),
    ], () => ({ input: '', mode: 'toggle', pass_through: false }), t('triggers.add')))
}

function server() {
  const token = h('input', { type: 'password', value: config.server.token, spellcheck: 'false', autocomplete: 'off', oninput: (event) => { config.server.token = event.target.value } })
  const reveal = h('button', { type: 'button', class: 'small', onclick: () => { token.type = token.type === 'password' ? 'text' : 'password' } }, t('common.show'))
  const urls = config.server.urls
  return section('STT-Suite', t('server.hint'),
    ...rows(urls, (url, index) => [h('input', { type: 'text', class: 'grow', value: url, spellcheck: 'false', oninput: (event) => { urls[index] = event.target.value } })],
      () => 'http://', t('server.add')),
    h('div', { class: 'fields spaced' },
      field(t('server.token'), h('span', { class: 'inline' }, token, reveal), t('server.token_hint')),
      field(t('server.timeout'), h('span', { class: 'inline' }, number(config.server, 'connect_timeout_ms'), 'ms'), t('server.timeout_hint'))))
}

function recognition() {
  return section(t('recognition.title'), t('recognition.hint'),
    h('div', { class: 'fields' },
      field(t('recognition.device'), choice(config.transcription, 'device', [['auto', t('recognition.device_auto')], ['cuda', t('recognition.device_cuda')], ['cpu', t('recognition.device_cpu')]])),
      field(t('recognition.language'), text(config.transcription, 'language', { class: 'short' }), t('recognition.language_hint'))))
}

function recording() {
  const options = [['', t('recording.microphone_default')], ...microphones.map((name) => [name, name])]
  if (config.recording.microphone && !microphones.includes(config.recording.microphone)) options.push([config.recording.microphone, `${config.recording.microphone} (${t('recording.not_found')})`])
  return section(t('recording.title'), null,
    h('div', { class: 'fields' },
      field(t('recording.microphone'), choice(config.recording, 'microphone', options)),
      field(t('recording.cancel_key'), triggerField(config.recording, 'cancel_key'), t('recording.cancel_key_hint')),
      field(t('recording.min_press'), h('span', { class: 'inline' }, number(config.recording, 'min_press_ms'), 'ms'), t('recording.min_press_hint')),
      field(t('recording.silence'), h('span', { class: 'inline' }, number(config.recording, 'stop_after_silence_s'), 's'), t('recording.never')),
      field(t('recording.silence_level'), percent(config.recording, 'silence_level_percent'), t('recording.silence_level_hint')),
      field(t('recording.max'), h('span', { class: 'inline' }, number(config.recording, 'max_duration_s'), 's'), t('recording.unlimited'))))
}

function soundField(key) {
  const sounds = config.sounds
  const builtin = sounds[key].startsWith(BUILTIN_PREFIX)
  const state = { kind: builtin ? sounds[key].slice(BUILTIN_PREFIX.length) : OWN_FILE }
  const path = h('input', { type: 'text', value: builtin ? '' : sounds[key], placeholder: t('sounds.path'), hidden: builtin, oninput: (event) => { sounds[key] = event.target.value } })
  const options = [['ping', t('sounds.ping')], ['pong', t('sounds.pong')], ['break', t('sounds.break')], [OWN_FILE, t('sounds.own_file')]]
  const select = choice(state, 'kind', options, () => {
    path.hidden = state.kind !== OWN_FILE
    sounds[key] = state.kind === OWN_FILE ? path.value : BUILTIN_PREFIX + state.kind
  })
  // Plays what is chosen now, at the volume set now, also before saving.
  const listen = h('button', { type: 'button', class: 'icon', title: t('sounds.listen'), 'aria-label': t('sounds.listen'),
    onclick: () => invoke('play_sound', { source: sounds[key], volumePercent: sounds.volume_percent }).catch((error) => say(String(error), 'error')) }, '▶')
  return h('span', { class: 'inline' }, select, path, listen)
}

function sounds() {
  return section(t('sounds.title'), null,
    h('div', { class: 'fields' },
      field(t('sounds.title'), check(config.sounds, 'enabled', t('sounds.play'))),
      field(t('sounds.start'), soundField('start')),
      field(t('sounds.stop'), soundField('stop')),
      field(t('sounds.cancel'), soundField('cancel')),
      field(t('sounds.volume'), percent(config.sounds, 'volume_percent'))))
}

function insert() {
  const methods = [['type', t('insert.type')], ['clipboard', t('insert.clipboard')]]
  config.insert.per_program ??= []
  return section(t('insert.title'), t('insert.hint'),
    h('div', { class: 'fields' },
      field(t('insert.method'), choice(config.insert, 'method', methods)),
      field(t('insert.between'), check(config.insert, 'space_between_dictations', t('insert.between_text'))),
      field(t('insert.end'), check(config.insert, 'press_enter', t('insert.enter'))),
      field(t('insert.again'), optionalTrigger(config.insert, 'again_key'), t('insert.again_hint')),
      field(t('insert.restore'), h('span', { class: 'inline' }, number(config.insert, 'clipboard_restore_delay_ms'), 'ms'))),
    h('h3', { class: 'muted subheading' }, t('insert.programs')),
    ...rows(config.insert.per_program, (rule) => [
      text(rule, 'process', { class: 'grow', placeholder: t('insert.process') }),
      choice(rule, 'method', methods),
    ], () => ({ process: '', method: 'clipboard' }), t('insert.add')))
}

function replacements() {
  return section(t('replacements.title'), t('replacements.hint'),
    ...rows(config.replacements, (rule, index, render) => {
      const kind = { value: rule.pattern === undefined || rule.pattern === null ? 'words' : 'pattern' }
      const input = kind.value === 'words'
        ? h('input', { type: 'text', class: 'grow', value: rule.variants.join(' | '), placeholder: 'Komm mit | Kommit', spellcheck: 'false', oninput: (event) => { rule.variants = event.target.value.split('|').map((variant) => variant.trim()).filter(Boolean) } })
        : h('input', { type: 'text', class: 'grow', value: rule.pattern, placeholder: '(?i)\\bagent[ -]?org?\\b', spellcheck: 'false', oninput: (event) => { rule.pattern = event.target.value } })
      return [
        choice(kind, 'value', [['words', t('replacements.words')], ['pattern', t('replacements.pattern')]], () => {
          if (kind.value === 'words') { rule.variants = rule.pattern ? [rule.pattern] : []; delete rule.pattern } else { rule.pattern = rule.variants.join('|'); rule.variants = [] }
          render()
        }),
        input, h('span', {}, '→'), text(rule, 'replacement', { class: 'grow', placeholder: 'commit' }),
      ]
    }, () => ({ variants: [], replacement: '' }), t('replacements.add')))
}

function display() {
  return section(t('display.title'), null,
    h('div', { class: 'fields' },
      field(t('display.mode'), choice(config.overlay, 'mode', [['floating', t('display.floating')], ['tray_only', t('display.tray_only')]])),
      field(t('display.opacity'), percent(config.overlay, 'opacity_percent', 10)),
      field(t('display.error_shown'), h('span', { class: 'inline' }, number(config.overlay, 'error_shown_s'), 's'))))
}

function system() {
  return section(t('system.title'), null,
    h('div', { class: 'fields' },
      field(t('system.history'), h('span', { class: 'inline' }, number(config.history, 'size'), t('system.dictations'))),
      field(t('system.autostart'), check(config.system, 'autostart', t('common.yes')))))
}

// ── Loading and saving ────────────────────────────────────────────────────────────────────────────────────────

// The words outside the form: the window's title and the buttons at the bottom.
function translateStatic() {
  document.title = t('settings.title')
  document.getElementById('save').textContent = t('settings.save')
  document.getElementById('reload').textContent = t('settings.discard')
}

function draw() {
  form.replaceChildren(languageSection(), triggers(), server(), recognition(), recording(), sounds(), insert(), replacements(), display(), system(),
    h('p', { class: 'muted' }, `${t('settings.saved_in')} `, h('code', {}, path)))
}

function drawBroken(error) {
  form.replaceChildren(h('section', { class: 'banner' },
    h('h2', {}, t('settings.broken_title')),
    h('p', {}, error),
    h('p', { class: 'muted' }, `${t('settings.broken_file')}: `, h('code', {}, path)),
    h('button', { type: 'button', onclick: async () => { config = await invoke('default_settings'); draw(); say(t('settings.defaults_inserted'), 'ok') } },
      t('settings.start_over'))))
}

async function load() {
  await disarm()
  await loadTexts()
  translateStatic()
  const loaded = await invoke('load_settings')
  path = loaded.path
  microphones = await invoke('microphones').catch((error) => { say(`${t('settings.microphones_error')}: ${error}`, 'error'); return [] })
  if (loaded.settings) {
    message.hidden = true
    config = loaded.settings
    draw()
    note(`loaded ${path}: ${config.triggers.length} triggers`)
  } else {
    config = null
    drawBroken(loaded.error)
  }
}

function numbersValid() {
  return [...form.querySelectorAll('input[type="number"]')].every((input) => Number.isInteger(input.valueAsNumber) && input.valueAsNumber >= 0)
}

document.getElementById('save').addEventListener('click', async () => {
  if (!config) return say(t('settings.start_over_first'), 'error')
  if (!numbersValid()) return say(t('settings.numbers_invalid'), 'error')
  try {
    note('save clicked')
    await invoke('save_settings', { settings: config })
    say(t('settings.saved'), 'ok')
  } catch (error) {
    say(`${t('settings.not_saved')}: ${error}`, 'error')
  }
})

document.getElementById('reload').addEventListener('click', () => load().catch((error) => report(String(error))))

load().catch((error) => report(String(error)))
