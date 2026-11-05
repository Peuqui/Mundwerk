// The overlay shows what Mundwerk reports (status.rs): listening, transcribing, an error. Rust shows and hides
// the window; this page only draws the state.
const { invoke } = window.__TAURI__.core
const { listen } = window.__TAURI__.event

const STATUS_EVENT = 'mundwerk://status'
const TEXTS = { listening: 'Hört zu …', transcribing: 'Transkribiert …', idle: '' }

const pill = document.getElementById('pill')
const text = document.getElementById('text')

listen(STATUS_EVENT, async ({ payload }) => {
  pill.className = payload.state
  text.textContent = payload.state === 'error' ? payload.detail : TEXTS[payload.state]
  // Read each time: the settings may have changed it.
  pill.style.opacity = await invoke('overlay_opacity')
})
