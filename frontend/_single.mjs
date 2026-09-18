/**
 * 把 dist 的 CSS / JS 内联进单文件 HTML —— 便于双击直接用 file:// 打开
 * （inline module script 不受 file:// 的 CORS 限制）
 */
import { readFileSync, writeFileSync, readdirSync } from 'node:fs'
import path from 'node:path'

const root = 'C:/Users/Administrator/Desktop/Bcrmapp/style-lab'
const dist = path.join(root, 'dist')
const assets = path.join(dist, 'assets')

const files = readdirSync(assets)
const cssFile = files.find((f) => f.endsWith('.css'))
const jsFile = files.find((f) => f.endsWith('.js'))
console.log('css =', cssFile, ' js =', jsFile)

let css = readFileSync(path.join(assets, cssFile), 'utf8')
let js = readFileSync(path.join(assets, jsFile), 'utf8')
let html = readFileSync(path.join(dist, 'index.html'), 'utf8')

// 安全转义：避免提前闭合 script/style
js = js.replace(/<\/script>/gi, '<\\/script>')
css = css.replace(/<\/style>/gi, '<\\/style>')

// 移除原有 link / script 标签
html = html.replace(/<link[^>]*rel="stylesheet"[^>]*>/gi, '')
html = html.replace(/<script[^>]*src="[^"]*"[^>]*><\/script>/gi, '')
html = html.replace(/<link[^>]*rel="modulepreload"[^>]*>/gi, '')

const inject = `<style>\n${css}\n</style>\n<script type="module">\n${js}\n</script>\n`

// 注意：必须用「函数式」replacement。
// 若直接写 html.replace('</body>', inject + '</body>')，
// inject 里 JS/CSS 中出现的 $& / $` / $' / $1 会被当成替换模式展开，
// 导致代码被破坏（曾出现 $& 被替换成 '</body>'，引发 "Unexpected token '<'"）。
if (html.includes('</body>')) html = html.replace('</body>', () => inject + '</body>')
else html += inject

const out = path.join(root, 'index.single.html')
writeFileSync(out, html, 'utf8')
console.log('written =', out, ' size =', (Buffer.byteLength(html) / 1024).toFixed(1) + ' KB')
console.log('contains <style> =', html.includes('<style>'), ' contains module =', html.includes('type="module"'))
