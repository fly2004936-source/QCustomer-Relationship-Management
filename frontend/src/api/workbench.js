/**
 * 工作台数据访问层
 *
 * 全部读写都落到后端真实表，因此「关掉再打开数据还在」：
 *   · daily_plan            —— 每天一句话目标（一天一行）
 *   · daily_todo            —— 今日待办
 *   · dashboard_note       —— 灵感便签（一行一条）
 *   · dashboard_shortcut   —— 快捷入口
 *   · focus_session         —— 番茄钟记录（2026-09-15 新增表）
 *   · news_item             —— 行业 / 政策 / 攻防 / AI 资讯
 *   · customer_news         —— 客户信息更新（is_new 区分存量/增量）
 *   · entity_history        —— 合作/竞争方动向
 *   · entsales_confrontation—— 竞争对抗记录（竞争策略与动作）
 *   · dashboard_update_log  —— 信息更新批次
 *
 * 约定：所有日期列都是 TEXT，格式 YYYY-MM-DD；时间列是 ISO8601 本地时间字符串。
 */

import { api, fetchAll } from './client'
import { dateKey, timeKey, minuteKey, isoWeek, WEEKDAY_CN, formatDateCN, relativeDay, p2 } from './date'

/* 日期工具统一放在 ./date，这里再导出一次，方便调用方只 import 一个模块 */
export { dateKey, timeKey, minuteKey, isoWeek, WEEKDAY_CN, formatDateCN, relativeDay, p2 }

/** 未填值时的统一占位 */
export const DASH = '—'

/* ---------------------------------------------------------------------------
   业务枚举映射
   ------------------------------------------------------------------------ */

export const NEWS_CATEGORY = {
  industry: '行业与市场',
  policy: '政策法规',
  incident: '攻防事件/漏洞',
  ai: 'AI安全',
}

/** 同属「行业与市场」的资讯靠 tags 再分流：网络安全 / 金融科技 */
export const NEWS_TAG = {
  sec: '网络安全',
  ai: 'AI动向',
  fintech: '金融科技',
}

export const CATEGORY_LABEL = {
  [NEWS_CATEGORY.industry]: '行业与市场',
  [NEWS_CATEGORY.policy]: '政策法规',
  [NEWS_CATEGORY.incident]: '攻防事件',
  [NEWS_CATEGORY.ai]: 'AI 安全',
}

/* ---------------------------------------------------------------------------
   读写
   ------------------------------------------------------------------------ */

const num = (v) => (v === null || v === undefined ? null : Number(v))

export const workbenchApi = {
  /* ---- 每日计划：一句话目标 + 下一步动作 ---- */
  async loadPlan(date = dateKey()) {
    const rows = await fetchAll('daily_plan', { filter: { plan_date: date } })
    return rows[0] ?? null
  },
  async savePlan({ id, plan_date = dateKey(), one_line_goal, next_action }) {
    const row = { plan_date, one_line_goal: one_line_goal ?? '', next_action: next_action ?? '' }
    return id ? api.patch('daily_plan', id, row) : api.create('daily_plan', row)
  },

  /* ---- 今日待办 ---- */
  async listTodos(date = dateKey()) {
    const rows = await fetchAll('daily_todo', {
      filter: { plan_date: date },
      order_by: 'id:asc',
    })
    return rows.map((r) => ({
      id: r.id,
      item: r.item ?? '',
      // status 是自由文本列（无 CHECK），这里只认 '已完成' 表示勾选
      done: r.status === '已完成',
      priority: r.priority ?? null,
      customer_id: num(r.customer_id),
      project_id: num(r.project_id),
    }))
  },
  addTodo({ item, priority = null, customer_id = null, project_id = null, date = dateKey() }) {
    return api.create('daily_todo', {
      plan_date: date,
      item,
      priority,
      customer_id,
      project_id,
      status: '未完成',
    })
  },
  setTodoDone(id, done) {
    return api.patch('daily_todo', id, { status: done ? '已完成' : '未完成' })
  },
  removeTodo(id) {
    return api.remove('daily_todo', id)
  },

  /* ---- 灵感便签 ---- */
  async listNotes() {
    const rows = await fetchAll('dashboard_note', { order_by: 'id:asc' })
    return rows.map((r) => ({ id: r.id, content: r.content ?? '', updated_at: r.updated_at }))
  },
  createNote(content = '') {
    return api.create('dashboard_note', { content })
  },
  saveNote(id, content) {
    return api.patch('dashboard_note', id, { content, updated_at: timeKey() })
  },
  removeNote(id) {
    return api.remove('dashboard_note', id)
  },

  /* ---- 快捷入口 ---- */
  async listShortcuts() {
    const rows = await fetchAll('dashboard_shortcut', { order_by: 'sort_order:asc,id:asc' })
    return rows.map((r) => ({
      id: r.id,
      title: r.title ?? '',
      url: r.url ?? '',
      icon: r.icon ?? '',
      sort_order: num(r.sort_order) ?? 0,
    }))
  },
  addShortcut({ title, url, icon = '', sort_order = 0 }) {
    return api.create('dashboard_shortcut', { title, url, icon, sort_order })
  },
  removeShortcut(id) {
    return api.remove('dashboard_shortcut', id)
  },

  /* ---- 番茄钟 ---- */
  async listFocus(date = dateKey()) {
    const rows = await fetchAll('focus_session', {
      filter: { session_date: date },
      order_by: 'id:desc',
    })
    return rows.map((r) => ({
      id: r.id,
      plan_min: num(r.plan_min) ?? 25,
      actual_min: num(r.actual_min) ?? 0,
      outcome: r.outcome ?? '',
      note: r.note ?? '',
      started_at: r.started_at,
    }))
  },
  recordFocus({ plan_min = 25, actual_min = 0, outcome = '已完成', note = '', date = dateKey() }) {
    return api.create('focus_session', {
      session_date: date,
      started_at: timeKey(),
      ended_at: timeKey(),
      plan_min,
      actual_min,
      outcome,
      note,
    })
  },

  /* ---- 资讯 ---- */
  /**
   * @param {object} o
   * @param {string[]} [o.categories] 命中的 category 集合
   * @param {string}   [o.tag]        tags 模糊匹配
   * @param {number}   [o.limit]      最多返回条数
   */
  async listNews({ categories, tag, limit = 24 } = {}) {
    const params = { page_size: 200, order_by: 'news_date:desc,id:desc' }
    if (tag) params.filter = { tags__like: tag }
    else if (categories?.length === 1) params.filter = { category: categories[0] }
    const rows = await fetchAll('news_item', params)
    const hit = categories?.length
      ? rows.filter((r) => categories.includes(r.category))
      : rows
    return hit.slice(0, limit).map((r) => ({
      id: r.id,
      category: r.category ?? '',
      title: r.title ?? '',
      summary: r.summary ?? '',
      source: r.source ?? '',
      news_date: r.news_date ?? '',
      tags: r.tags ?? '',
    }))
  },

  /* ---- 客户信息更新 ---- */
  async listCustomerUpdates({ date, customerId, limit = 30 } = {}) {
    const params = { page_size: 200, order_by: 'news_date:desc,id:desc' }
    const filter = {}
    if (date) filter.news_date = date
    if (customerId) filter.customer_id = customerId
    if (Object.keys(filter).length) params.filter = filter
    const rows = await fetchAll('customer_news', params)
    return rows.slice(0, limit).map((r) => ({
      id: r.id,
      customer_id: num(r.customer_id),
      title: r.news_title ?? '',
      summary: r.summary ?? '',
      source: r.source ?? '',
      news_date: r.news_date ?? '',
      our_relation: r.our_relation ?? '',
      is_new: num(r.is_new) === 1,
      batch_time: r.batch_time ?? '',
    }))
  },
  /** 某个客户的历史更新（用于从「今日更新」跳转） */
  async listCustomerUpdateHistory(customerId, limit = 60) {
    return this.listCustomerUpdates({ customerId, limit })
  },

  /* ---- 合作 / 竞争方动向 ---- */
  async listEntityMoves({ entityType, limit = 40 } = {}) {
    const params = { page_size: 200, order_by: 'happen_time:desc,id:desc' }
    if (entityType) params.filter = { entity_type: entityType }
    const rows = await fetchAll('entity_history', params)
    return rows.slice(0, limit).map((r) => ({
      id: r.id,
      entity_type: r.entity_type ?? '',
      entity_id: num(r.entity_id),
      happen_time: r.happen_time ?? '',
      project_id: num(r.project_id),
      customer_id: num(r.customer_id),
      role: r.role ?? '',
      their_liaison: r.their_liaison ?? '',
      quote_discount: r.quote_discount ?? '',
      result: r.result ?? '',
      review: r.review ?? '',
    }))
  },

  /* ---- 竞争对抗（策略与动作） ---- */
  async listConfrontations({ entityId, projectName, limit = 40 } = {}) {
    const params = { page_size: 200, order_by: 'happen_time:desc,id:desc' }
    const filter = {}
    if (entityId) filter.entity_id = entityId
    if (projectName) filter.project_name = projectName
    if (Object.keys(filter).length) params.filter = filter
    const rows = await fetchAll('entsales_confrontation', params)
    return rows.slice(0, limit).map((r) => ({
      id: r.id,
      entity_type: r.entity_type ?? '',
      entity_id: num(r.entity_id),
      customer_name: r.customer_name ?? '',
      project_name: r.project_name ?? '',
      happen_time: r.happen_time ?? '',
      their_mode: r.their_mode ?? '',
      their_quote: r.their_quote ?? '',
      our_quote: r.our_quote ?? '',
      result: r.result ?? '',
      key_reason: r.key_reason ?? '',
      review: r.review ?? '',
    }))
  },

  /* ---- 信息更新批次 ---- */
  async latestUpdateLog() {
    const rows = await fetchAll('dashboard_update_log', {
      order_by: 'update_date:desc,id:desc',
      page_size: 1,
    })
    return rows[0] ?? null
  },
  saveUpdateLog({ operator = '我', coverage = '' } = {}) {
    const now = new Date()
    const p = (n) => String(n).padStart(2, '0')
    return api.create('dashboard_update_log', {
      update_date: dateKey(now),
      update_time: `${p(now.getHours())}:${p(now.getMinutes())}`,
      last_update_time: timeKey(now),
      operator,
      coverage,
    })
  },
}
