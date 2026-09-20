/**
 * 运行 npm 命令并输出 UTF-8 日志（Windows 中文控制台默认 GBK，需要转码）
 * 用法：node run.mjs <npm 子命令...>
 */
import { spawnSync } from 'node:child_process'
import fs from 'node:fs'
import path from 'node:path'

const root = path.resolve('.').replace(/\\/g, '/')
const args = process.argv.slice(2)
const logName = args[0] === 'build' ? 'build.log' : args[0] === 'install' ? 'install.log' : 'npm.log'

const r = spawnSync('npm', args, {
  cwd: root,
  encoding: 'buffer',
  shell: true,
  env: { ...process.env, npm_config_color: 'false', npm_config_fund: 'false', npm_config_audit: 'false' },
})

const dec = (buf) => {
  if (!buf) return ''
  try {
    return new TextDecoder('gbk').decode(buf)
  } catch {
    return buf.toString('utf8')
  }
}

const text = [`EXIT=${r.status}`, '===== STDOUT =====', dec(r.stdout), '===== STDERR =====', dec(r.stderr)].join('\n')
fs.writeFileSync(path.join(root, logName), text, 'utf8')
console.log('exit=' + r.status + ' log=' + logName)
