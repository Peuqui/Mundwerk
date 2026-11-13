// The badge moves along with the pointer (badge.rs moves the window); this page only shows whether it listens (the
// bars move, the rim is red) or transcribes (still and grey).
const STATUS_EVENT = 'mundwerk://status'
const badge = document.querySelector('.dictation-badge')

window.__TAURI__.event.listen(STATUS_EVENT, ({ payload }) => {
  badge.classList.toggle('listening', payload.state === 'listening')
  badge.classList.toggle('transcribing', payload.state !== 'listening')
})
