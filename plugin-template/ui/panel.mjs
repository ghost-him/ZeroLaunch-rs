// Hello World plugin panel
export default function mount(rootEl, host) {
  rootEl.innerHTML = `
    <div style="padding: 16px; font-family: system-ui;">
      <h2>Hello World</h2>
      <p>这是一個第三方插件面板示例。</p>
      <div id="data-display"></div>
    </div>
  `

  host.onDataUpdate((data, actions) => {
    const display = rootEl.querySelector('#data-display')
    if (display) {
      display.textContent = JSON.stringify({ data, actions }, null, 2)
    }
  })
}
