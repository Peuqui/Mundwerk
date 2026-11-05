// The overlay shows what Mundwerk reports (status.rs): listening, transcribing, an error. Rust shows and hides
// the window; this page only draws the state.
const { invoke } = window.__TAURI__.core
const { listen } = window.__TAURI__.event

const STATUS_EVENT = 'mundwerk://status'

const pill = document.getElementById('pill')
const text = document.getElementById('text')

listen(STATUS_EVENT, async ({ payload }) => {
  // Read each time: the settings may have changed language and opacity.
  await loadTexts()
  pill.className = payload.state
  const words = { listening: t('overlay.listening'), transcribing: t('overlay.transcribing'), error: payload.detail, idle: '' }
  text.textContent = words[payload.state]
  pill.style.opacity = await invoke('overlay_opacity')
})
