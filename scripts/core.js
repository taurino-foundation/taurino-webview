;(function () {
  function uid() {
    return window.crypto.getRandomValues(new Uint32Array(1))[0]
  }

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

  if (!window.__TAURINO_INTERNALS__) {
    defineValue(window, '__TAURINO_INTERNALS__', {
      plugins: {}
    })
  }

  const osName = __TEMPLATE_os_name__
  const protocolScheme = __TEMPLATE_protocol_scheme__

  defineValue(window.__TAURINO_INTERNALS__, 'convertFileSrc', function (
    filePath,
    protocol = 'asset'
  ) {
    const path = encodeURIComponent(filePath)

    return osName === 'windows' || osName === 'android'
      ? `${protocolScheme}://${protocol}.localhost/${path}`
      : `${protocol}://localhost/${path}`
  })

  const callbacks = new Map()

  function registerCallback(callback, once) {
    const identifier = uid()

    callbacks.set(identifier, (data) => {
      if (once) {
        unregisterCallback(identifier)
      }

      return callback && callback(data)
    })

    return identifier
  }

  function unregisterCallback(id) {
    callbacks.delete(id)
  }

  function runCallback(id, data) {
    const callback = callbacks.get(id)

    if (callback) {
      callback(data)
    } else {
      console.warn(
        `[TAURINO] Couldn't find callback id ${id}. This can happen when the app is reloaded while Rust is running an asynchronous operation.`
      )
    }
  }

  defineValue(window.__TAURINO_INTERNALS__, 'transformCallback', registerCallback)
  defineValue(window.__TAURINO_INTERNALS__, 'unregisterCallback', unregisterCallback)
  defineValue(window.__TAURINO_INTERNALS__, 'runCallback', runCallback)
  defineValue(window.__TAURINO_INTERNALS__, 'callbacks', callbacks)

  const ipcQueue = []
  let isWaitingForIpc = false

  function waitForIpc() {
    if ('ipc' in window.__TAURINO_INTERNALS__) {
      while (ipcQueue.length > 0) {
        const action = ipcQueue.shift()
        action()
      }

      return
    }

    setTimeout(waitForIpc, 50)
  }

  function invoke(cmd, payload = {}, options) {
    return new Promise(function (resolve, reject) {
      const callback = registerCallback((response) => {
        resolve(response)
        unregisterCallback(error)
      }, true)

      const error = registerCallback((errorValue) => {
        reject(errorValue)
        unregisterCallback(callback)
      }, true)

      const action = () => {
        window.__TAURINO_INTERNALS__.ipc({
          cmd,
          callback,
          error,
          payload,
          options
        })
      }

      if ('ipc' in window.__TAURINO_INTERNALS__) {
        action()
      } else {
        ipcQueue.push(action)

        if (!isWaitingForIpc) {
          isWaitingForIpc = true
          waitForIpc()
        }
      }
    })
  }

  defineValue(window.__TAURINO_INTERNALS__, 'invoke', invoke)

  const tauriApi = window.__TAURI__ || {}
  const coreApi = tauriApi.core || {}

  defineValue(coreApi, 'invoke', invoke)
  defineValue(tauriApi, 'core', coreApi)
  defineValue(window, '__TAURI__', tauriApi)
})()