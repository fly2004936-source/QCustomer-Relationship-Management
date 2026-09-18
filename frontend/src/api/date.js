/**
 * 日期 / 时间工具
 *
 * 单独成文件是因为工作台的多个卡片都要用（顶部时钟、待办按天、番茄钟按天、资讯按天）。
 * 统一用**本地时区**拼字符串，不用 toISOString()——后者是 UTC，晚上 8 点之后会算成第二天。
 */

const p2 = (n) => String(n).padStart(2, '0')

/** 本地日期 → 'YYYY-MM-DD' */
export function dateKey(d = new Date()) {
  return `${d.getFullYear()}-${p2(d.getMonth() + 1)}-${p2(d.getDate())}`
}

/** 本地时间 → 'YYYY-MM-DD HH:mm:ss' */
export function timeKey(d = new Date()) {
  return `${dateKey(d)} ${p2(d.getHours())}:${p2(d.getMinutes())}:${p2(d.getSeconds())}`
}

/** 'YYYY-MM-DD HH:mm' */
export function minuteKey(d = new Date()) {
  return `${dateKey(d)} ${p2(d.getHours())}:${p2(d.getMinutes())}`
}

/** ISO 周序号（含跨年修正：12 月 31 日可能属于次年第 1 周） */
export function isoWeek(d = new Date()) {
  const t = new Date(Date.UTC(d.getFullYear(), d.getMonth(), d.getDate()))
  const day = t.getUTCDay() || 7
  t.setUTCDate(t.getUTCDate() + 4 - day)
  const yearStart = new Date(Date.UTC(t.getUTCFullYear(), 0, 1))
  return Math.ceil(((t - yearStart) / 86400000 + 1) / 7)
}

export const WEEKDAY_CN = ['周日', '周一', '周二', '周三', '周四', '周五', '周六']

/** 'YYYY-MM-DD' → 'M月D日 周五' */
export function formatDateCN(key) {
  const [y, m, d] = String(key ?? '').split('-').map(Number)
  if (!y) return key ?? ''
  return `${m}月${d}日 ${WEEKDAY_CN[new Date(y, m - 1, d).getDay()]}`
}

/** 相对今天的自然语言 */
export function relativeDay(key) {
  const k = String(key ?? '').slice(0, 10)
  if (!k) return ''
  const today = dateKey()
  if (k === today) return '今天'
  const diff = Math.round((new Date(`${today}T00:00:00`) - new Date(`${k}T00:00:00`)) / 86400000)
  if (diff === 1) return '昨天'
  if (diff === 2) return '前天'
  if (diff > 0 && diff < 7) return `${diff} 天前`
  if (diff < 0) return '未来'
  const [y, m, d] = k.split('-').map(Number)
  return `${m}月${d}日`
}

export { p2 }
