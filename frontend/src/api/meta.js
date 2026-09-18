/**
 * 表结构元数据缓存
 *
 * 后端的 /meta/tables 与 /meta/tables/{table} 返回的是白名单元数据
 * （列名、类型、枚举、外键、是否 JSON、是否布尔），
 * 因此「数据管理」页面可以完全由元数据驱动：
 *   · 表格显示哪些列、列宽、对齐 —— 读元数据
 *   · 编辑表单的控件类型（下拉 / 开关 / 多行 / JSON / 外键选择）—— 读元数据
 *   · DDL 增删字段后无需改前端，重新拉一次元数据即可
 *
 * 元数据在会话内缓存，避免每次进页面都往返。
 */

import { api } from './client'

const tableCache = new Map()

/* ---------------------------------------------------------------------------
   表清单
   ------------------------------------------------------------------------ */

export async function loadTables() {
  if (tableCache.has('__list__')) return tableCache.get('__list__')
  const data = await api.tables()
  // 后端可能返回数组，也可能返回 { items } / { tables }，这里都兼容
  const list = Array.isArray(data) ? data : (data?.items ?? data?.tables ?? [])
  const norm = list.map((t) =>
    typeof t === 'string' ? { name: t } : { name: t.name, group: t.group, groupCode: t.groupCode },
  )
  tableCache.set('__list__', norm)
  tableCache.set('__index__', new Map(norm.map((t) => [t.name, t])))
  return norm
}

export function cachedTable(name) {
  return tableCache.get('__index__')?.get(name) ?? null
}

/* ---------------------------------------------------------------------------
   单表元数据
   ------------------------------------------------------------------------ */

export async function loadTableMeta(table) {
  if (tableCache.has(table)) return tableCache.get(table)
  const raw = await api.tableMeta(table)
  const meta = normalizeMeta(table, raw)
  tableCache.set(table, meta)
  return meta
}

function normalizeMeta(table, raw) {
  /*
   * ⚠️ 外键信息在**表级** `foreign_keys` 里，列级 `references` 实测恒为 null：
   *     foreign_keys: [{ columns:['customer_id'], refTable:'customer', refColumns:['id'], onDelete:'CASCADE' }]
   *     而 columns[i].references === null
   * 早期版本只读列级 references，导致 fkColumns 永远是空数组 ——
   * 表现是「外键下拉框没出现、表格里外键列只显示一串数字」，而且不报任何错。
   * 这里先把表级外键摊平成 { 列名: {table, column, onDelete} }，再合并到每一列上。
   */
  const fkByColumn = {}
  for (const f of raw?.foreign_keys ?? []) {
    const refTable = f.refTable ?? f.table
    const refColumn = (f.refColumns ?? [])[0] ?? 'id'
    for (const col of f.columns ?? []) {
      fkByColumn[col] = { table: refTable, column: refColumn, onDelete: f.onDelete ?? null }
    }
  }

  const columns = (raw?.columns ?? []).map((c) => ({
    name: c.name,
    sqlType: c.sqlType ?? c.type ?? 'TEXT',
    type: c.type ?? 'TEXT',
    primaryKey: !!c.primaryKey,
    autoIncrement: !!c.autoIncrement,
    notNull: !!c.notNull,
    unique: !!c.unique,
    enum: Array.isArray(c.enum) && c.enum.length ? c.enum : null,
    boolean: !!c.boolean,
    json: !!c.json,
    hasDefault: c.default !== null && c.default !== undefined,
    // 表级外键优先，列级 references 作为兼容兜底
    references: fkByColumn[c.name]
      ? fkByColumn[c.name]
      : c.references
        ? { table: c.references.refTable ?? c.references.table, column: (c.references.refColumns ?? [])[0] ?? 'id', onDelete: c.references.onDelete ?? null }
        : null,
  }))

  const pk = columns.filter((c) => c.primaryKey).map((c) => c.name)
  /** 唯一约束（单列 + 复合），表格里给这些列加个标记 */
  const uniqueKeys = (raw?.unique_keys ?? []).map((uk) => (Array.isArray(uk) ? uk : [uk]))
  const uniqueCols = new Set(uniqueKeys.flat())

  return {
    table,
    group: raw?.group ?? '',
    groupCode: raw?.groupCode ?? '',
    columns,
    primaryKey: pk.length ? pk : ['id'],
    uniqueKeys,
    uniqueCols,
    /** 外键列名集合，用于表单里做下拉 */
    fkColumns: columns.filter((c) => c.references),
    /** 子表清单，children 路由用得上 */
    hasMany: raw?.has_many ?? [],
  }
}

/** 清缓存（DDL 变更后手动刷新时用） */
export function clearMetaCache(table) {
  if (table) tableCache.delete(table)
  else tableCache.clear()
}

/* ---------------------------------------------------------------------------
   值的呈现与解析
   --------------------------------------------------------------------------- */

/** 单元格显示文本 */
export function displayValue(col, v) {
  if (v === null || v === undefined || v === '') return { text: '—', null: true }
  if (col.boolean) return { text: Number(v) === 1 ? '是' : '否' }
  if (col.json) {
    const s = typeof v === 'string' ? v : JSON.stringify(v)
    return { text: s, json: true, title: s }
  }
  if (typeof v === 'object') {
    const s = JSON.stringify(v)
    return { text: s, json: true, title: s }
  }
  return { text: String(v) }
}

/** 表单控件类型 */
export function controlOf(col) {
  if (col.primaryKey && col.autoIncrement) return 'readonly'
  if (col.boolean) return 'boolean'
  if (col.enum) return 'enum'
  if (col.references) return 'fk'
  if (col.json) return 'json'
  if (col.sqlType === 'INTEGER' || col.sqlType === 'REAL' || col.sqlType === 'NUMERIC') return 'number'
  // 长文本启发式：名称里带 remark / desc / review / summary / note / content / plan 的给多行
  if (/remark|desc|review|summary|note|content|plan|detail|issue|intel|analysis/i.test(col.name)) return 'text'
  return 'text'
}

/** 表单值 → 提交给后端的值（做类型规整，避免 "1" 被当成字符串存进 INTEGER 列） */
export function toSubmit(col, raw) {
  if (raw === '' || raw === undefined) return null
  if (col.boolean) return raw === true || raw === 1 || raw === '1' ? 1 : 0
  if (col.json) {
    if (typeof raw !== 'string') return raw
    const s = raw.trim()
    if (!s) return null
    // 允许写裸词，自动补成 JSON 字符串数组，减少 400 报错
    if (!(s.startsWith('{') || s.startsWith('[') || s.startsWith('"'))) {
      return JSON.stringify(s.split(/[,，]/).map((x) => x.trim()).filter(Boolean))
    }
    return s
  }
  if (col.sqlType === 'INTEGER' && !col.enum) {
    const n = Number(raw)
    return Number.isFinite(n) ? Math.trunc(n) : null
  }
  if (col.sqlType === 'REAL') {
    const n = Number(raw)
    return Number.isFinite(n) ? n : null
  }
  return typeof raw === 'string' ? raw : String(raw)
}

/** 后端行 → 表单初值 */
export function toForm(col, row) {
  const v = row?.[col.name]
  if (v === null || v === undefined) {
    if (col.boolean) return false
    return ''
  }
  if (col.boolean) return Number(v) === 1
  if (col.json) {
    if (typeof v === 'string') {
      // 已经是 JSON 文本就美化一下，方便编辑
      try {
        return JSON.stringify(JSON.parse(v), null, 1)
      } catch {
        return v
      }
    }
    return JSON.stringify(v, null, 1)
  }
  return v
}

/* ---------------------------------------------------------------------------
   查询操作符目录
   ---------------------------------------------------------------------------
   与后端 `backend/src/query/ast.rs` 的操作符白名单一一对应（共 20 个）。
   前端不该自己发明操作符：写错一个后端会返回"不支持的操作符"，
   更糟的是界面看着能用、结果永远为空。

   语法：`filter[列名__操作符]=值`；不带操作符 = eq。
   ⚠️ 两个高频坑（都在后端验证里踩过）：
     · `__in` / `__between` 用**英文逗号分隔的多值**，不是 JSON 数组
     · `__json_contains` / `__json_has_all` 收的是**裸元素**（写 VF），
       写成 `"VF"` （带引号）反而查不到
   ------------------------------------------------------------------------ */

/** 不需要填值的操作符 */
export const VALUELESS_OPS = new Set(['null', 'isnull', 'notnull', 'isnotnull'])

/**
 * 该操作符是否需要两个值（逗号分隔）
 * @type {Set<string>}
 */
export const RANGE_OPS = new Set(['between'])

/** 该操作符是否接受多个值（逗号分隔） */
export const MULTI_OPS = new Set(['in', 'nin', 'between', 'json_has_all'])

const OP_LABEL = {
  eq: '等于',
  ne: '不等于',
  gt: '大于',
  gte: '大于等于',
  lt: '小于',
  lte: '小于等于',
  like: '包含',
  nlike: '不包含',
  prefix: '以…开头',
  suffix: '以…结尾',
  in: '在列表中',
  nin: '不在列表中',
  between: '区间',
  null: '为空',
  notnull: '不为空',
  json_contains: 'JSON 含元素',
  json_has_all: 'JSON 含全部',
  json_eq: 'JSON 完全等于',
  json_like: 'JSON 文本包含',
  json_len_gte: 'JSON 长度≥',
}
export const opLabel = (op) => OP_LABEL[op] ?? op

const TEXT_OPS = ['eq', 'ne', 'like', 'nlike', 'prefix', 'suffix', 'in', 'nin', 'null', 'notnull']
const NUM_OPS = ['eq', 'ne', 'gt', 'gte', 'lt', 'lte', 'between', 'in', 'nin', 'null', 'notnull']
const BOOL_OPS = ['eq', 'ne', 'null', 'notnull']
const JSON_OPS = ['json_contains', 'json_has_all', 'json_eq', 'json_like', 'json_len_gte', 'null', 'notnull']

/**
 * 某一列可用的操作符列表（按类型收窄，避免给出后端根本不接受的组合）。
 * 枚举/布尔/外键列只给等值类操作符 —— 对枚举做 like 没有意义，也容易让用户困惑。
 */
export function operatorsFor(col) {
  if (!col) return TEXT_OPS
  if (col.json) return JSON_OPS
  if (col.boolean) return BOOL_OPS
  if (col.enum && col.enum.length) return ['eq', 'ne', 'in', 'nin', 'null', 'notnull']
  if (col.references) return NUM_OPS
  if (col.sqlType === 'INTEGER' || col.sqlType === 'REAL' || col.sqlType === 'NUMERIC') return NUM_OPS
  return TEXT_OPS
}

/** 该列的输入控件形态，决定筛选器里用什么控件填值 */
export function inputKindOf(col) {
  if (!col) return 'text'
  if (col.boolean) return 'boolean'
  if (col.enum && col.enum.length) return 'enum'
  if (col.references) return 'fk'
  if (col.json) return 'json'
  if (/^\d{4}-\d{2}-\d{2}/.test(col.name) || /_date$|_at$/.test(col.name)) return 'date'
  if (col.sqlType === 'INTEGER' || col.sqlType === 'REAL' || col.sqlType === 'NUMERIC') return 'number'
  return 'text'
}
