const url = process.argv[2] ?? 'http://127.0.0.1:4173/'
try {
  const ctrl = new AbortController()
  const t = setTimeout(() => ctrl.abort(), 8000)
  const res = await fetch(url, { signal: ctrl.signal })
  clearTimeout(t)
  const text = await res.text()
  console.log('STATUS=' + res.status)
  console.log('LEN=' + text.length)
  console.log(text.slice(0, 1200))
} catch (e) {
  console.log('FETCH_FAILED: ' + (e?.message ?? String(e)))
}
