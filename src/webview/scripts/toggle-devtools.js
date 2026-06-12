;(function () {
  function toggleDevtoolsHotkey() {
    const osName = __TEMPLATE_os_name__

    const isHotkey =
      osName === 'macos'
        ? (event) => event.metaKey && event.altKey && event.code === 'KeyI'
        : (event) => event.ctrlKey && event.shiftKey && event.code === 'KeyI'

    document.addEventListener('keydown', (event) => {
      if (isHotkey(event)) {
        window.__TAURINO_INTERNALS__.invoke(
          'plugin:webview|internal_toggle_devtools'
        )
      }
    })
  }

  if (
    document.readyState === 'complete'
    || document.readyState === 'interactive'
  ) {
    toggleDevtoolsHotkey()
  } else {
    window.addEventListener('DOMContentLoaded', toggleDevtoolsHotkey, true)
  }
})()