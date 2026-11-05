// The settings as the text of their file: Rust checks it on saving (an error names the field) and puts it into
// effect at once. Comments stay as they are written.
const { invoke } = window.__TAURI__.core

const text = document.getElementById('text')
const message = document.getElementById('message')
const microphoneList = document.getElementById('microphone-list')

function say(words, kind) {
  message.textContent = words
  message.className = kind
  message.hidden = false
}

async function load() {
  const settings = await invoke('settings_text')
  document.getElementById('path').textContent = settings.path
  text.value = settings.text
  const startupError = await invoke('startup_error')
  if (startupError) say(`Mundwerk läuft nicht, weil diese Einstellungen nicht gehen:\n${startupError}\nNach dem Speichern startet es neu.`, 'error')
  else message.hidden = true
}

document.getElementById('save').addEventListener('click', async () => {
  try {
    await invoke('save_settings', { text: text.value })
    say('Gespeichert und in Kraft.', 'ok')
  } catch (error) {
    say(`Nicht gespeichert: ${error}`, 'error')
  }
})

document.getElementById('reload').addEventListener('click', () => load().catch((error) => say(String(error), 'error')))

document.getElementById('defaults').addEventListener('click', async () => {
  text.value = await invoke('default_settings_text')
  say('Standardwerte eingesetzt, noch nicht gespeichert. Achtung: das Token und eigene Einträge sind darin leer.', 'ok')
})

document.getElementById('microphones').addEventListener('click', async () => {
  try {
    const names = await invoke('microphones')
    microphoneList.replaceChildren(...names.map((name) => Object.assign(document.createElement('li'), { textContent: name })))
    microphoneList.hidden = false
  } catch (error) {
    say(`Mikrofone: ${error}`, 'error')
  }
})

load().catch((error) => say(String(error), 'error'))
