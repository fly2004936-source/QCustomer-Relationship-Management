import fs from 'node:fs'
import path from 'node:path'

const root = path.resolve('.').replace(/\\/g, '/')
const nm = path.join(root, 'node_modules')
console.log('node_modules_exists=' + fs.existsSync(nm))
if (fs.existsSync(nm)) {
  const names = fs.readdirSync(nm)
  console.log('entries=' + names.length)
  console.log('first=' + names.slice(0, 20).join('|'))
  console.log('vue=' + fs.existsSync(path.join(nm, 'vue', 'package.json')))
  console.log('vite=' + fs.existsSync(path.join(nm, 'vite', 'package.json')))
  console.log('plugin_vue=' + fs.existsSync(path.join(nm, '@vitejs', 'plugin-vue', 'package.json')))
}
const logPath = path.join(root, 'install.log')
if (fs.existsSync(logPath)) {
  const raw = fs.readFileSync(logPath, 'utf8').replace(/\u0000/g, '')
  console.log('LOG_BYTES=' + fs.statSync(logPath).size)
  console.log('LOG_START')
  console.log(raw.slice(0, 1500))
  console.log('LOG_END')
}
