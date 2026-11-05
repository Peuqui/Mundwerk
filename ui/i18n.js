// The words of the pages, from i18n.json (the app has the same file built in); which language, Mundwerk says.
let texts = null
let language = 'en'

// setting: 'auto', 'de' or 'en' to show that (the settings page while it is changed), or null for the app's.
async function loadTexts(setting = null) {
  if (!texts) texts = await (await fetch('i18n.json')).json()
  language = await window.__TAURI__.core.invoke('ui_language', { setting })
  document.documentElement.lang = language
}

// The text for key; a key the file lacks is a bug the core's tests catch (every_key_the_code_uses_exists).
function t(key) {
  const text = texts[language][key]
  if (text === undefined) throw new Error(`no text ${key} in i18n.json`)
  return text
}

// The text for key if the file has one: for names of which only some are written out (key caps: "Strg" for Ctrl,
// while a letter or F9 is shown as it is).
function tIfAny(key) {
  return texts[language][key]
}
