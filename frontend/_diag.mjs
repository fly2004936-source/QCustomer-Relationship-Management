/** 诊断单文件中的危险序列 */
import { readFileSync } from 'node:fs'

const p = 'C:/Users/Administrator/Desktop/Bcrmapp/style-lab/index.single.html'
const h = readFileSync(p, 'utf8')

const danger = [
  [/<\/script/gi, '</script'],
  [/<\/style/gi, '</style'],
  [/<!--/g, '<!--'],
  [/-->/g, '-->'],
  [/<script/gi, '<script'],
]
for (const [re, name] of danger) {
  const m = h.match(re)
  console.log(name.padEnd(12), '=>', m ? m.length : 0)
}

const lines = h.split('\n')
console.log('\ntotal lines =', lines.length)
lines.forEach((l, i) => {
  if (/<\/script|<\/style|<!--/i.test(l)) console.log(`line ${i + 1} len=${l.length}: ${l.slice(0, 160)}`)
})

// 定位第 29 行
if (lines[28]) {
  const l = lines[28]
  console.log('\n--- line 29 (len ' + l.length + ') 前后各 200 字符 @ 10161 ---')
  console.log(l.slice(9950, 10400))
}
