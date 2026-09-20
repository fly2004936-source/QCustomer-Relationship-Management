import { spawnSync } from 'node:child_process'
import fs from 'node:fs'
import path from 'node:path'

const root = path.resolve('.').replace(/\\/g, '/')
const r = spawnSync('npm', ['install', '--no-audit', '--no-fund', '--loglevel=notice'], {
  cwd: root,
  encoding: 'buffer',
  shell: true,
  env: { ...process.env, npm_config_color: 'false' },
})
const dec = (buf) => {
  if (!buf) return ''
  try {
    return new TextDecoder('gbk').decode(buf)
  } catch {
    return buf.toString('utf8')
  }
}
const lines = []
lines.push('STATUS=' + r.status)
lines.push('SIGNAL=' + r.signal)
lines.push('--- STDOUT ---')
lines.push(dec(r.stdout).slice(0, 4000))
lines.push('--- STDERR ---')
lines.push(dec(r.stderr).slice(0, 4000))
fs.writeFileSync(path.join(root, '_install.txt'), lines.join('\n'), 'utf8')
console.log('done status=' + r.status)
