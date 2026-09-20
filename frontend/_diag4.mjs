import { readFileSync } from 'node:fs'
const h = readFileSync('C:/Users/Administrator/Desktop/Bcrmapp/style-lab/index.single.html', 'utf8')
const lines = h.split('\n')
console.log('lines =', lines.length)
lines.forEach((l, i) => console.log(String(i + 1).padStart(3), 'len=' + String(l.length).padStart(7), '|', l.slice(0, 90).replace(/\n/g, '')))

const l29 = lines[28] || ''
console.log('\n--- line 29 around col 10161 ---')
console.log(JSON.stringify(l29.slice(10050, 10280)))

// 找到所有 < 在 line 29 的位置
const pos = []
for (let i = 0; i < l29.length; i++) if (l29[i] === '<') pos.push(i)
console.log('\n"<" positions in line29 =', pos.length, pos.slice(0, 20))
