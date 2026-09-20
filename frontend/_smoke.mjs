/**
 * 冒烟测试：
 *  1) 在 jsdom 中挂载构建产物，确认无运行时报错，且组织架构图渲染出节点与展开按钮
 *  2) 扫描产物与源码，确认前 8 套风格范围内不包含任何 emoji 字符
 */
import fs from 'node:fs'
import path from 'node:path'
import { JSDOM } from 'jsdom'

const root = path.resolve('.')
const distIndex = fs.readFileSync(path.join(root, 'dist/index.html'), 'utf8')
const jsRel = distIndex.match(/src="\.\/(assets\/[^"]+\.js)"/)[1]
const jsAbs = path.join(root, 'dist', jsRel)

const dom = new JSDOM('<!doctype html><html><body><div id="app"></div></body></html>', {
  pretendToBeVisual: true,
  url: 'http://localhost/',
})
const w = dom.window
for (const k of [
  'window',
  'document',
  'navigator',
  'HTMLElement',
  'SVGElement',
  'Element',
  'Node',
  'Event',
  'CustomEvent',
  'MouseEvent',
  'KeyboardEvent',
  'getComputedStyle',
  'DOMParser',
  'NodeFilter',
  'MutationObserver',
  'ResizeObserver',
]) {
  try {
    Object.defineProperty(global, k, { value: w[k], writable: true, configurable: true })
  } catch (e) {
    console.log('skip global ' + k + ': ' + e.message)
  }
}
global.requestAnimationFrame = (cb) => setTimeout(() => cb(Date.now()), 0)
global.cancelAnimationFrame = (id) => clearTimeout(id)

const errors = []
w.addEventListener('error', (e) => errors.push('window.error: ' + e.message))
w.addEventListener('unhandledrejection', (e) => errors.push('unhandled: ' + e.reason))
const realError = console.error
console.error = (...a) => {
  errors.push('console.error: ' + a.map((x) => (x && x.message ? x.message : String(x))).join(' ').slice(0, 400))
}
console.warn = (...a) => {
  errors.push('console.warn: ' + a.map((x) => String(x)).join(' ').slice(0, 400))
}

await import('file://' + jsAbs.replace(/\\/g, '/'))
await new Promise((r) => setTimeout(r, 400))

const doc = w.document
const line = (k, v) => console.log(`${k}=${v}`)
line('app_html_len', doc.querySelector('#app')?.innerHTML.length ?? 0)
line('shell', doc.querySelectorAll('.shell').length)
line('style_pills', doc.querySelectorAll('.stylepill').length)
line('sf_app', doc.querySelectorAll('.sf-app').length)
line('sf_app_data_style', doc.querySelector('.sf-app')?.getAttribute('data-style') ?? '')
line('sf_cards', doc.querySelectorAll('.sf-card').length)
line('og_svg', doc.querySelectorAll('.og-svg').length)
line('og_nodes_rendered', doc.querySelectorAll('.og-node').length)
line('og_edges_rendered', doc.querySelectorAll('.og-edge').length)
line('og_toggles', doc.querySelectorAll('.og-toggle-hit').length)
line('og_zone_labels', doc.querySelectorAll('.og-group-label').length)
line('table_rows', doc.querySelectorAll('.sf-table tbody tr').length)
line('errors', errors.length)
errors.slice(0, 12).forEach((e) => console.log('  ! ' + e))
console.error = realError

/* ---------------- emoji 扫描 ---------------- */
const EMOJI = /[\u{1F000}-\u{1FAFF}\u{1F1E6}-\u{1F1FF}\u{2600}-\u{27BF}\u{2B00}-\u{2BFF}\u{FE0F}\u{1F900}-\u{1F9FF}]/u
const scanTargets = [
  'src/styles/themes.css',
  'src/styles/components.css',
  'src/styles/registry.js',
  'src/data/mock.js',
  'src/components/DemoPage.vue',
  'src/components/MiniPreview.vue',
  'src/components/OrgGraph.vue',
  'src/components/AppIcon.vue',
  'dist/index.html',
]
console.log('--- emoji scan ---')
let emojiTotal = 0
for (const rel of scanTargets) {
  const p = path.join(root, rel)
  if (!fs.existsSync(p)) {
    console.log(`  ${rel}: MISSING`)
    continue
  }
  const txt = fs.readFileSync(p, 'utf8')
  const hits = [...txt.matchAll(new RegExp(EMOJI, 'gu'))].map((m) => m[0])
  emojiTotal += hits.length
  console.log(`  ${rel}: ${hits.length ? 'FOUND ' + JSON.stringify([...new Set(hits)]) : 'clean'}`)
}
console.log('emoji_total=' + emojiTotal)
