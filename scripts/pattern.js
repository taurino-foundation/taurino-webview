;(function () {
  function __taurinoDeepFreeze(object) {
    const props = Object.getOwnPropertyNames(object)

    for (const prop of props) {
      if (typeof object[prop] === 'object') {
        __taurinoDeepFreeze(object[prop])
      }
    }

    return Object.freeze(object)
  }

  Object.defineProperty(window.__TAURINO_INTERNALS__, '__TAURINO_PATTERN__', {
    value: __taurinoDeepFreeze(__TEMPLATE_pattern__)
  })
})()