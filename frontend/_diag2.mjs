/** 提取内联 JS 做语法检查 */
import { readFileSync, writeFileSync } from 'node:fs'
import { spawnSync } from 'node:child_process'
import path from 'node:path'

const root = 'C:/Users/Administrator/Desktop/Bcrmapp/style-lab'
const h = readFileSync(path.join(root, 'index.single.html'), 'utf8')

const open = h.indexOf('<script type="module">')
const close = h.lastIndexOf('</script>')
console.log('script open at', open, ' close at', close, ' inner len =', close - open - '<script type="module">'.length)

const js = h.slice(open + '<script type="module">'.length, close)
const tmp = path.join(root, '_inline_check.mjs')
writeFileSync(tmp, js, 'utf8')

const r = spawnSync(process.execPath, ['--check', tmp], { encoding: 'utf8' })
console.log('node --check exit =', r.status)
if (r.stderr) {
  console.log('--- stderr ---')
  console.log(r.stderr.slice(0, 2000))
}

// 找出 js 中所有 "未在字符串内" 的可疑 < 位置
const idxs = []
for (let i = 0; i < js.length; i++) if (js[i] === '<') idxs.push(i)
console.log('\n"<" 出现次数 =', idxs.length)
// 打印每个 '<' 附近 60 字符
idxs.slice(0, 40).forEach((i) => {
  console.log(`@${i}: ...${js.slice(Math.max(0, i - 40), i + 40).replace(/\n/g, '\\n')}...`)
})
