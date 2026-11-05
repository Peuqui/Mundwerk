// The badge moves along with the pointer (badge.rs moves the window); this page only shows whether it listens (the
// bars move) or transcribes (still and grey).
const STATUS_EVENT = 'mundwerk://status'
const badge = document.getElementById('badge')

window.__TAURI__.event.listen(STATUS_EVENT, ({ payload }) => {
  badge.classList.toggle('still', payload.state !== 'listening')
})
