import fs from 'node:fs'
import path from 'node:path'

const root = 'C:/Users/Administrator/Desktop/Bcrmapp'

function walk(dir, depth = 0, max = 2) {
  let out = []
  let ents
  try { ents = fs.readdirSync(dir, { withFileTypes: true }) } catch (e) { return ['  ENOENT ' + dir] }
  for (const e of ents) {
    if (e.name === 'node_modules' || e.name === '.git') { out.push('  '.repeat(depth) + e.name + '/ ...'); continue }
    const p = path.join(dir, e.name)
    let sz = 0, mt = ''
    try { const st = fs.statSync(p); sz = st.size; mt = st.mtime.toISOString().slice(0, 19) } catch {}
    out.push('  '.repeat(depth) + (e.isDirectory() ? e.name + '/' : e.name) + '  ' + sz + 'B  ' + mt)
    if (e.isDirectory() && depth < max) out = out.concat(walk(p, depth + 1, max))
  }
  return out
}

console.log('===== Bcrmapp (depth 2) =====')
console.log(walk(root, 0, 2).join('\n'))

console.log('\n===== style-lab (depth 3) =====')
console.log(walk(path.join(root, 'style-lab'), 0, 3).join('\n'))

console.log('\n===== Bcrm (depth 3) =====')
console.log(walk('C:/Users/Administrator/Desktop/Bcrm', 0, 3).join('\n'))
