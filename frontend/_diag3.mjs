/** 比对 dist JS 与内联 JS，定位 </body> 注入点 */
import { readFileSync, readdirSync } from 'node:fs'
import path from 'node:path'

const root = 'C:/Users/Administrator/Desktop/Bcrmapp/style-lab'
const assets = path.join(root, 'dist/assets')
const jsFile = readdirSync(assets).find((f) => f.endsWith('.js'))
const src = readFileSync(path.join(assets, jsFile), 'utf8')
const single = readFileSync(path.join(root, 'index.single.html'), 'utf8')
const html = readFileSync(path.join(root, 'dist/index.html'), 'utf8')

console.log('=== dist/index.html ===')
console.log(JSON.stringify(html))

const count = (s, sub) => s.split(sub).length - 1
console.log('\ndist JS:  "</body>" =', count(src, '</body>'), ' "</style>" =', count(src, '</style>'), ' "</script>" =', count(src, '</script>'))
console.log('dist JS  len =', src.length)

const open = single.indexOf('<script type="module">') + '<script type="module">'.length
const close = single.lastIndexOf('</script>')
const inline = single.slice(open, close)
console.log('inline len =', inline.length, ' diff =', inline.length - src.length)
console.log('inline  "</body>" =', count(inline, '</body>'))

// 找第一个差异点
let i = 0
while (i < Math.min(src.length, inline.length) && src[i] === inline[i]) i++
console.log('\n首个差异 @', i)
console.log('src   :', JSON.stringify(src.slice(i - 60, i + 60)))
console.log('inline:', JSON.stringify(inline.slice(i - 60, i + 60)))
