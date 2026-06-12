;(function () {
  function defineValue(target, key, value) {
    try {
      Object.defineProperty(target, key, {
        value,
        configurable: true,
        enumerable: false,
        writable: false
      })
    } catch (_) {
      target[key] = value
    }
  }

  function __taurinoDeepFreeze(object) {
    if (!object || typeof object !== 'object') {
      return object
    }

    const props = Object.getOwnPropertyNames(object)

    for (const prop of props) {
      const value = object[prop]

      if (value && typeof value === 'object') {
        __taurinoDeepFreeze(value)
      }
    }

    return Object.freeze(object)
  }

  if (!window.__TAURINO_INTERNALS__) {
    defineValue(window, '__TAURINO_INTERNALS__', {
      plugins: {}
    })
  }

  defineValue(
    window.__TAURINO_INTERNALS__,
    '__TAURINO_PATTERN__',
    __taurinoDeepFreeze(__TEMPLATE_pattern__)
  )
})()