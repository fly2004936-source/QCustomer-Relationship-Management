import fs from 'node:fs'
const files = process.argv.slice(2)
for (const f of files) {
  if (!fs.existsSync(f)) {
    console.log(`--- ${f}: MISSING ---`)
    continue
  }
  const buf = fs.readFileSync(f)
  let txt
  try {
    txt = new TextDecoder('gbk').decode(buf)
  } catch {
    txt = buf.toString('utf8')
  }
  txt = txt.replace(/\u0000/g, '')
  console.log(`--- ${f} (${buf.length} bytes) ---`)
  console.log(txt.slice(0, 3000))
  console.log('--- end ---')
}
