/**
 * 直接调用 vite bin 构建 style-lab（绕过 npm / shell）
 */
import { spawnSync } from 'node:child_process'
import { existsSync, writeFileSync } from 'node:fs'
import path from 'node:path'

const root = 'C:/Users/Administrator/Desktop/Bcrmapp/style-lab'
const vite = path.join(root, 'node_modules/vite/bin/vite.js')
console.log('vite exists =', existsSync(vite))

const r = spawnSync(
  process.execPath,
  [vite, 'build', '--logLevel', 'info'],
  { cwd: root, encoding: 'buffer', env: { ...process.env, NODE_ENV: 'production' } },
)

const dec = (b) => { try { return new TextDecoder('gbk').decode(b) } catch { return String(b) } }
const out = [`EXIT=${r.status}`, '--- STDOUT ---', dec(r.stdout), '--- STDERR ---', dec(r.stderr)].join('\n')
writeFileSync(path.join(root, 'build.log'), out, 'utf8')
console.log('exit=' + r.status)
console.log(out.slice(0, 3000))
