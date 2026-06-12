
;(function () {
  function __tauriDeepFreeze(object) {
    const props = Object.getOwnPropertyNames(object)

    for (const prop of props) {
      if (typeof object[prop] === 'object') {
        __tauriDeepFreeze(object[prop])
      }
    }

    return Object.freeze(object)
  }

  Object.defineProperty(window.__TAURI_INTERNALS__, '__TAURINO_PATTERN__', {
    value: __tauriDeepFreeze(__TEMPLATE_pattern__)
  })
})()