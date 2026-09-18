/**
 * 后端 API 客户端
 *
 * 后端约定（详见《客户管理系统API接口文档.md》）：
 *  · 资源路径 = 表名本身，前缀 `/api/v1`，不做驼峰转换；参数名 = 列名
 *  · 统一响应体 `{ code, message, requestId, data }`，`code === 0` 为成功
 *  · 列表：`data = { items, page, page_size, total, page_count }`
 *  · 写操作优先 PATCH；`page_size` 上限 200；无条件 DELETE 会被拒绝
 */

/**
 * 后端地址。
 *
 * 默认是开发期的 `127.0.0.1:8787`（`cargo run` 起的独立服务）。
 * 但打包成桌面程序时端口不能写死：
 *   · 8787 可能被别的程序（或另一个实例）占着；
 *   · 一个 exe 里同时跑着 axum 服务，端口是由它自己挑的。
 *
 * 因此地址按下面的优先级取，从「运行时」到「构建时」再到「写死」：
 *
 *   1. `window.__BCRM_API_BASE__` —— Tauri 壳在页面脚本执行**之前**注入
 *      （打包后走这条，端口由内核随机分配，前端无需重新构建）。
 *   2. `import.meta.env.VITE_BCRM_API_BASE` —— 构建/启动时注入。
 *      根目录的「启动Web服务.ps1」会传 `/api/v1`（相对路径），
 *      请求经 vite 的同源反代转到后端 ⇒ **后端换端口也不用改产物**。
 *   3. `DEFAULT_BASE` —— 什么都没注入时的兜底（直接用浏览器打开 dist 的场景）。
 */
export const DEFAULT_BASE = 'http://127.0.0.1:8787/api/v1'

/** 宿主（Tauri 壳）运行时注入的地址 */
function injectedBase() {
  try {
    const v = globalThis.__BCRM_API_BASE__
    return typeof v === 'string' && v ? v : ''
  } catch {
    return ''
  }
}

/**
 * 构建/启动期注入的地址。
 *
 * ⚠️ 只在**非 Tauri**环境才该用它：桌面版必须走宿主注入的随机端口，
 * 而这里注入的 `/api/v1` 是相对路径，在 `tauri://localhost` 下会打错地方。
 * 好在优先级顺序已经保证了这一点（宿主注入非空时不会被覆盖）。
 */
function envBase() {
  try {
    const v = import.meta.env?.VITE_BCRM_API_BASE
    return typeof v === 'string' && v ? v : ''
  } catch {
    return ''
  }
}

/**
 * 同源兜底：页面本身由 HTTP 服务提供时，API 就在同一个源上。
 *
 * 这是 **Web 版（形态 A · 后端内嵌托管前端产物）** 走的路：页面与 API 同端口，
 * 用相对路径永远正确 —— 换端口、换主机名、换 IP 都不用重新构建产物。
 *
 * 必须排在 `DEFAULT_BASE` 之前：否则同样是从 http 打开的页面，却会去打
 * `127.0.0.1:8787` —— 服务端换台机器、换个端口就全盘失效。
 *
 * 桌面版不受影响：Tauri 的页面源是 `tauri://localhost`，命中不了这里，
 * 仍然走优先级最高的宿主注入。
 */
function sameOriginBase() {
  try {
    const p = globalThis.location?.protocol
    if (p === 'http:' || p === 'https:') return '/api/v1'
  } catch {
    /* 非浏览器环境 */
  }
  return ''
}

let baseUrl = injectedBase() || envBase() || sameOriginBase() || DEFAULT_BASE

export const getBaseUrl = () => baseUrl
export const setBaseUrl = (u) => {
  baseUrl = String(u || DEFAULT_BASE).replace(/\/+$/, '')
}

/** 统一错误类型，便于上层区分「连不上」与「后端报错」 */
export class ApiError extends Error {
  constructor(message, { status = 0, code = null, kind = 'unknown', cause = null } = {}) {
    super(message)
    this.name = 'ApiError'
    this.status = status
    this.code = code
    this.kind = kind // offline | timeout | http | api
    this.cause = cause
  }
}

const JSON_HEADERS = { 'content-type': 'application/json' }

/**
 * 把参数对象编码成后端认识的查询串。
 *
 * ⚠️ 这里必须**展开嵌套对象**：后端的过滤语法是 `filter[列名]` / `filter[列名__操作符]`，
 * 而不是 `filter=<json>`。早期版本用 `new URLSearchParams({filter:{...}})`，
 * 结果被 `String()` 成 `filter=[object Object]`，后端返回 40004 / 400，
 * 前端又用 safe() 静默兜住，表现为「界面在、数据全是空的」——极难排查。
 *
 * 支持的输入形态：
 *   { page_size: 200, order_by: 'id:desc', q: '关键字' }
 *   { filter: { customer_id: 3, name__like: '电力' } }        → filter[customer_id]=3 & filter[name__like]=电力
 *   { or: [ { status: 'A', status__ne: 'B' }, { x: 1 } ] }     → or[0][status]=A & or[1][x]=1
 *   { and: [ { a: 1 } ] }                                      → and[0][a]=1
 */
function qs(params) {
  const sp = new URLSearchParams()

  const add = (key, value) => {
    if (value === undefined || value === null || value === '') return
    if (Array.isArray(value)) {
      value.forEach((v) => add(key, v))
      return
    }
    sp.append(key, String(value))
  }

  const isPlainObject = (v) => v !== null && typeof v === 'object' && !Array.isArray(v)

  for (const [key, value] of Object.entries(params || {})) {
    if (value === undefined || value === null || value === '') continue

    // filter 对象 → filter[列名]=值
    if (key === 'filter' && isPlainObject(value)) {
      for (const [col, val] of Object.entries(value)) add(`filter[${col}]`, val)
      continue
    }

    // or / and 分组 → or[组号][列名]=值
    if ((key === 'or' || key === 'and') && Array.isArray(value)) {
      value.forEach((group, i) => {
        if (!isPlainObject(group)) return
        for (const [col, val] of Object.entries(group)) add(`${key}[${i}][${col}]`, val)
      })
      continue
    }

    add(key, value)
  }

  const s = sp.toString()
  return s ? `?${s}` : ''
}

/**
 * 把参数对象序列化成查询串，供「条件筛选器」预览实际请求用。
 * 与 `api.list` 内部走的是同一个函数，所以预览出来的就是真实请求，不会对不上。
 */
export const toQueryString = (params) => qs(params)

async function request(path, { method = 'GET', body, timeout = 9000 } = {}) {
  const ctrl = new AbortController()
  const timer = setTimeout(() => ctrl.abort(), timeout)
  try {
    const res = await fetch(baseUrl + path, {
      method,
      headers: body === undefined ? undefined : JSON_HEADERS,
      body: body === undefined ? undefined : JSON.stringify(body),
      signal: ctrl.signal,
    })

    let payload = null
    try {
      payload = await res.json()
    } catch {
      throw new ApiError(`响应不是合法 JSON（HTTP ${res.status}）`, {
        status: res.status,
        kind: 'http',
      })
    }

    if (!payload || typeof payload.code !== 'number') {
      throw new ApiError(`响应体缺少 code 字段（HTTP ${res.status}）`, {
        status: res.status,
        kind: 'http',
      })
    }

    if (payload.code !== 0) {
      throw new ApiError(payload.message || `接口返回 code=${payload.code}`, {
        status: res.status,
        code: payload.code,
        kind: 'api',
      })
    }

    return payload.data
  } catch (e) {
    if (e instanceof ApiError) throw e
    if (e.name === 'AbortError') {
      throw new ApiError(`请求超时（${timeout}ms）`, { kind: 'timeout', cause: e })
    }
    throw new ApiError(`连不上后端 ${baseUrl}`, { kind: 'offline', cause: e })
  } finally {
    clearTimeout(timer)
  }
}

export const api = {
  /* --- 元数据 --- */
  health: () => request('/meta/health', { timeout: 3500 }),
  tables: () => request('/meta/tables'),
  tableMeta: (table) => request(`/meta/tables/${encodeURIComponent(table)}`),
  enums: () => request('/meta/enums'),

  /* --- 查询 --- */
  list: (table, params) => request(`/${encodeURIComponent(table)}${qs(params)}`),
  byId: (table, id) => request(`/${encodeURIComponent(table)}/${encodeURIComponent(id)}`),
  query: (table, tree, params) =>
    request(`/${encodeURIComponent(table)}/query${qs(params)}`, { method: 'POST', body: tree }),
  count: (table, params) => request(`/${encodeURIComponent(table)}/count${qs(params)}`),

  /* --- 写入 --- */
  create: (table, row) => request(`/${encodeURIComponent(table)}`, { method: 'POST', body: row }),
  patch: (table, id, row) =>
    request(`/${encodeURIComponent(table)}/${encodeURIComponent(id)}`, { method: 'PATCH', body: row }),
  remove: (table, id) =>
    request(`/${encodeURIComponent(table)}/${encodeURIComponent(id)}`, { method: 'DELETE' }),
}

/**
 * 列表接口的 data 是 `{ items, page, page_size, total, page_count }`，
 * 这里统一取出数组，并在数据量超过一页时自动翻页补齐。
 *
 * 为什么必须翻页：`page_size` 上限是 200，而 internal_person / person 这类表
 * 在有真实数据后很容易超过 200 行，若只取第一页，网络图会"少人"且没有任何提示——
 * 这种静默截断是最难排查的 bug。maxPages 是安全阀，防止误传条件导致死循环。
 *
 * @param {string} table
 * @param {object} [params] 查询参数（filter / q / order_by / page_size …）
 * @param {{maxPages?: number}} [opts]
 */
export async function fetchAll(table, params = {}, { maxPages = 25 } = {}) {
  const pageSize = Math.min(200, Math.max(1, Number(params.page_size) || 200))
  const first = await api.list(table, { ...params, page_size: pageSize, page: 1 })

  // 后端若直接返回数组（兼容旧行为），就没有翻页可言
  if (Array.isArray(first)) return first

  const items = first?.items ?? []
  const total = Number(first?.total ?? items.length)
  const pageCount = Number(first?.page_count ?? 1)
  if (items.length >= total || pageCount <= 1) return items

  const out = [...items]
  const last = Math.min(pageCount, maxPages)
  for (let p = 2; p <= last; p += 1) {
    const d = await api.list(table, { ...params, page_size: pageSize, page: p })
    const chunk = Array.isArray(d) ? d : (d?.items ?? [])
    if (!chunk.length) break
    out.push(...chunk)
    if (out.length >= total) break
  }
  return out
}

const countCache = new Map()

/**
 * 单表行数（侧边栏用来显示「这张表里有几条数据」）。
 * 结果做进程内缓存，避免每次切视图都把 62 张表重新数一遍。
 */
export async function countTable(table, { refresh = false } = {}) {
  if (!refresh && countCache.has(table)) return countCache.get(table)
  const d = await api.count(table)
  const n = Number(d?.count ?? (Array.isArray(d) ? d.length : 0)) || 0
  countCache.set(table, n)
  return n
}

export const clearCountCache = () => countCache.clear()
