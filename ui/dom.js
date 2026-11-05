// Building the page: h(tag, attributes, ...children). An attribute starting with "on" is an event handler.
function h(tag, attributes = {}, ...children) {
  const element = document.createElement(tag)
  for (const [name, value] of Object.entries(attributes)) {
    if (value === undefined || value === null || value === false) continue
    if (name.startsWith('on')) element.addEventListener(name.slice(2), value)
    else if (name === 'value') element.value = value
    else if (name === 'checked') element.checked = value
    else element.setAttribute(name, value === true ? '' : value)
  }
  element.append(...children.flat().filter((child) => child !== null && child !== undefined && child !== false))
  return element
}
