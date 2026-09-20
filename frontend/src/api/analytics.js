/**
 * 数据分析驾驶舱 —— 数据装配与聚合
 *
 * 设计原则与 adapters.js 一致：**只在这一层碰后端列名**，组件层拿到的是一套
 * 与数据源无关的展示模型；mock 与 live 两条通道产出**完全相同的形状**。
 *
 * 聚合全部基于真实库结构（列名以 backend/resources/schema.sql 为准）：
 *   · 客户域      customer（category / industry / army_region）
 *   · 项目域      project（budget / sec_budget / level / info_type / competition）
 *                 + project_c74111（7 Clears + 4 Priorities + 1 Key）
 *                 + project_member（ADUR / attitude，客户方人员的态度只存在这里）
 *   · 人员域      person / internal_person / partner_person / competitor_person
 *   · 合作竞争方  coop_entity（entity_type）/ entsales_perf（经营绩效）
 *   · 拜访行动    visit_record（method / visit_date）
 *   · 客户经营    customer_budget（it_budget / sec_budget）
 *
 * ⚠️ 口径提醒（踩过的坑）：
 *   · `person` 表**没有 attitude 列**，客户方人员态度必须取 `project_member.attitude`；
 *   · C74111 分档边界是「>75 才是绝对优势」，75 归相对优势；没有诊断记录是**第五档「未诊断」**，
 *     不能并进「绝对劣势」，否则「没评过」会被显示成「0 分」。
 */

import { api, fetchAll } from './client'
import { pickStage } from './adapters'
import * as mock from '@/data/mock'

/* ---------------------------------------------------------------------------
   常量
   --------------------------------------------------------------------------- */

/** C74111 四档 + 未诊断。颜色沿用 app.css 里固定不随主题变化的 --c74-* 四色 */
export const C74_BANDS = [
  { key: 'a', label: '绝对优势', range: '> 75', color: 'var(--c74-a)', hint: '可承诺' },
  { key: 'b', label: '相对优势', range: '50 – 75', color: 'var(--c74-b)', hint: '可争取' },
  { key: 'c', label: '相对劣势', range: '25 – 50', color: 'var(--c74-c)', hint: '可参与' },
  { key: 'd', label: '绝对劣势', range: '< 25', color: 'var(--c74-d)', hint: '不可承诺' },
  { key: 'none', label: '未诊断', range: '无记录', color: 'var(--tx-2)', hint: '待评分' },
]

export const BAND_MAP = Object.fromEntries(C74_BANDS.map((b) => [b.key, b]))

/**
 * 按总分定档。边界归属严格照《06-数据库架构解析》：
 *   > 75 → a    50–75（含 75） → b    25–50（含 25/50） → c    < 25 → d    无记录 → none
 */
export function bandOf(score) {
  const n = Number(score)
  if (score === null || score === undefined || score === '' || !Number.isFinite(n)) return 'none'
  if (n > 75) return 'a'
  if (n >= 50) return 'b'
  if (n >= 25) return 'c'
  return 'd'
}

/** 人员态度（枚举 X / - / = / + / ⭐），按「支持度」从高到低排列 */
export const ATTITUDE_META = [
  { v: '⭐', label: '铁杆支持', color: 'var(--ac)' },
  { v: '+', label: '支持', color: 'var(--ok)' },
  { v: '=', label: '中立', color: 'var(--tx-3)' },
  { v: '-', label: '不支持', color: 'var(--warn)' },
  { v: 'X', label: '反对', color: 'var(--bad)' },
]

const ATT_ORDER = ATTITUDE_META.map((a) => a.v)

/** C74111 三大板块：7 Clears 40 + 4 Priorities 40 + 1 Key 20 */
const C74_BLOCKS = [
  { code: 'C1', name: '客户组织结构图', max: 10, group: 'clear', field: 'c1_score' },
  { code: 'C2', name: '拍板人燃眉之急', max: 5, group: 'clear', field: 'c2_score' },
  { code: 'C3', name: '立项六要素', max: 5, group: 'clear', field: 'c3_score' },
  { code: 'C4', name: '采购周期', max: 5, group: 'clear', field: 'c4_score' },
  { code: 'C5', name: '合作方三问', max: 5, group: 'clear', field: 'c5_score' },
  { code: 'C6', name: '竞争方三问', max: 5, group: 'clear', field: 'c6_score' },
  { code: 'C7', name: '独特客户价值', max: 5, group: 'clear', field: 'c7_score' },
  { code: 'P1', name: '项目组多数人支持', max: 5, group: 'priority', field: 'p1_score' },
  { code: 'P2', name: '合作伙伴代理商支持', max: 5, group: 'priority', field: 'p2_score' },
  { code: 'P3', name: '与拍板人密谋策划', max: 20, group: 'priority', field: 'p3_score' },
  { code: 'P4', name: '关键人密谋策划', max: 10, group: 'priority', field: 'p4_score' },
  { code: 'Key', name: '与最高决策人密谋策划', max: 20, group: 'key', field: 'key_score' },
]

/** 驾驶舱要拉的表（一次并发拉完，失败即整体回落到 mock） */
const TABLES = [
  'customer',
  'project',
  'project_c74111',
  'project_member',
  'coop_entity',
  'entsales_perf',
  'visit_record',
  'customer_budget',
  'person',
  'internal_person',
  'partner_person',
  'competitor_person',
]

/* ---------------------------------------------------------------------------
   工具
   --------------------------------------------------------------------------- */

const num = (v, fb = 0) => {
  const n = Number(v)
  return Number.isFinite(n) ? n : fb
}

const str = (v, fb = '') => {
  const s = String(v ?? '').trim()
  return s || fb
}

/** 按 key 计数，返回 [key, count] 降序数组 */
function tally(list, pick) {
  const m = new Map()
  for (const it of list) {
    const k = pick(it)
    if (!k) continue
    m.set(k, (m.get(k) || 0) + 1)
  }
  return m
}

const sumBy = (list, pick) => list.reduce((s, it) => s + num(pick(it)), 0)

/** 数值千分位；驾驶舱里金额一律「万元」 */
const fmtWan = (v) => Math.round(num(v)).toLocaleString('zh-CN')

function monthOf(dateStr) {
  const m = String(dateStr ?? '').match(/^(\d{4})-(\d{2})/)
  return m ? `${m[1]}-${m[2]}` : ''
}

/* ---------------------------------------------------------------------------
   核心聚合
   --------------------------------------------------------------------------- */

export function buildAnalytics(raw = {}) {
  const customers = raw.customers || []
  const projects = raw.projects || []
  const c74111 = raw.c74111 || []
  const members = raw.members || []
  const coopEntities = raw.coopEntities || []
  const entsalesPerf = raw.entsalesPerf || []
  const visits = raw.visits || []
  const budgets = raw.customerBudgets || []
  const persons = raw.persons || []
  const internalPersons = raw.internalPersons || []
  const partnerPersons = raw.partnerPersons || []
  const competitorPersons = raw.competitorPersons || []
  const tableHeat = raw.tableHeat || []

  const custName = new Map(customers.map((c) => [c.id, str(c.name, `客户 #${c.id}`)]))
  const entName = new Map(coopEntities.map((e) => [e.id, str(e.name, `实体 #${e.id}`)]))
  const diagByProject = new Map(c74111.map((r) => [r.project_id, r]))

  /* --- 项目视图：合并诊断得分 --- */
  const projView = projects.map((p) => {
    const d = diagByProject.get(p.id)
    const score = d ? num(d.total_score) : null
    const band = bandOf(score)
    return {
      id: p.id,
      name: str(p.name, `项目 #${p.id}`),
      customer: custName.get(p.customer_id) || '—',
      customerId: p.customer_id ?? null,
      stage: pickStage(p),
      level: str(p.level, 'P3'),
      budget: num(p.budget),
      secBudget: num(p.sec_budget),
      // win_score 是 TEXT，可能是 "62" 或空
      winScore: num(p.win_score, NaN),
      competition: str(p.competition),
      commitment: str(p.commitment),
      score,
      winRate: d ? num(d.win_rate, num(d.total_score)) : null,
      band,
    }
  })

  /* --- KPI 条 --- */
  const totalBudget = sumBy(projView, (p) => p.budget)
  const secTotal = sumBy(projView, (p) => p.secBudget)
  const diagnosed = projView.filter((p) => p.band !== 'none')
  const avgScore = diagnosed.length ? sumBy(diagnosed, (p) => p.score) / diagnosed.length : 0
  const avgWinRate = diagnosed.length ? sumBy(diagnosed, (p) => p.winRate) / diagnosed.length : 0
  const personTotal = persons.length + internalPersons.length + partnerPersons.length + competitorPersons.length
  const mainCustomers = customers.filter((c) => c.category === '主客户').length

  const kpis = [
    {
      key: 'customers',
      name: '客户主体',
      value: String(customers.length),
      unit: '家',
      foot: `主客户 ${mainCustomers} · 关联 ${customers.length - mainCustomers}`,
      icon: 'building',
      tone: 'ac',
    },
    {
      key: 'projects',
      name: '在跟项目',
      value: String(projView.length),
      unit: '个',
      foot: `P1 ${projView.filter((p) => p.level === 'P1').length} · P2 ${projView.filter((p) => p.level === 'P2').length}`,
      icon: 'briefcase',
      tone: 'info',
    },
    {
      key: 'budget',
      name: '预计合同额',
      value: fmtWan(totalBudget),
      unit: '万元',
      foot: `安全预算 ${fmtWan(secTotal)} 万元 · 占比 ${totalBudget > 0 ? Math.round((secTotal / totalBudget) * 100) : 0}%`,
      icon: 'coin',
      tone: 'ok',
    },
    {
      key: 'winrate',
      name: '平均趋赢力',
      value: avgWinRate.toFixed(1),
      unit: '%',
      foot: `C74111 平均 ${avgScore.toFixed(1)} 分 / 100`,
      icon: 'target',
      tone: avgWinRate >= 50 ? 'ok' : 'warn',
    },
    {
      key: 'persons',
      name: '建档人员',
      value: String(personTotal),
      unit: '人',
      foot: `客户 ${persons.length} · 我方 ${internalPersons.length}`,
      icon: 'users',
      tone: 'ac2',
    },
    {
      key: 'visits',
      name: '拜访记录',
      value: String(visits.length),
      unit: '次',
      foot: `覆盖客户 ${tally(visits, (v) => v.customer_id).size} 家`,
      icon: 'flag',
      tone: 'info',
    },
  ]

  /* --- C74111：四档分布 --- */
  const bandCounts = Object.fromEntries(C74_BANDS.map((b) => [b.key, 0]))
  for (const p of projView) bandCounts[p.band] += 1
  const c74111Dist = {
    total: projView.length,
    diagnosed: diagnosed.length,
    avgScore,
    avgWinRate,
    bands: C74_BANDS.map((b) => ({
      ...b,
      count: bandCounts[b.key],
      pct: projView.length ? Math.round((bandCounts[b.key] / projView.length) * 100) : 0,
    })),
    // 7 Clears + 4 Priorities + 1 Key 的各项均分（只对已诊断项目求平均）
    blocks: C74_BLOCKS.map((b) => {
      const avg = diagnosed.length ? sumBy(diagnosed, (p) => num(diagByProject.get(p.id)?.[b.field])) / diagnosed.length : 0
      return { ...b, avg, pct: b.max ? Math.round((avg / b.max) * 100) : 0 }
    }),
    // 风险榜：已诊断但分数最低的几个项目
    risk: [...diagnosed].sort((x, y) => x.score - y.score).slice(0, 5),
  }

  /* --- 预算排行（Top 8） --- */
  const budgetRank = [...projView].sort((a, b) => b.budget - a.budget).slice(0, 8)

  /* --- 项目阶段漏斗 --- */
  const stageOrder = ['线索', '商机', '项目']
  const stageMap = tally(projView, (p) => p.stage)
  const stages = stageOrder.map((label) => {
    const rows = projView.filter((p) => p.stage === label)
    return { label, count: stageMap.get(label) || 0, budget: sumBy(rows, (p) => p.budget) }
  })

  /* --- 项目级别分布 --- */
  const levels = ['P1', 'P2', 'P3', 'P4'].map((label) => ({
    label,
    count: projView.filter((p) => p.level === label).length,
    budget: sumBy(
      projView.filter((p) => p.level === label),
      (p) => p.budget,
    ),
  }))

  /* --- 客户行业 / 战区分布 --- */
  const industry = [...tally(customers, (c) => str(c.industry, '未分类')).entries()]
    .map(([label, count]) => ({ label, count }))
    .sort((a, b) => b.count - a.count)
    .slice(0, 8)

  const regions = [...tally(customers, (c) => str(c.army_region, '未划分')).entries()]
    .map(([label, count]) => ({ label, count }))
    .sort((a, b) => b.count - a.count)
    .slice(0, 8)

  /* --- 竞争态势矩阵：预算 × 趋赢力 --- */
  const matrix = projView
    .filter((p) => p.winRate != null)
    .map((p) => ({
      name: p.name,
      customer: p.customer,
      x: p.budget,
      y: p.winRate,
      band: p.band,
      level: p.level,
    }))

  /* --- 人员态度：客户方取 project_member，其余三张人员表自带 attitude --- */
  const attitudeRows = [
    ...members.map((m) => m.attitude),
    ...internalPersons.map((p) => p.attitude),
    ...partnerPersons.map((p) => p.attitude),
    ...competitorPersons.map((p) => p.attitude),
  ].filter(Boolean)
  const attMap = tally(attitudeRows, (v) => v)
  const attitude = ATTITUDE_META.map((a) => ({
    ...a,
    count: attMap.get(a.v) || 0,
  })).sort((x, y) => ATT_ORDER.indexOf(x.v) - ATT_ORDER.indexOf(y.v))
  const attTotal = attitude.reduce((s, a) => s + a.count, 0)
  const supportRate = attTotal ? Math.round(((attMap.get('⭐') || 0) + (attMap.get('+') || 0)) / attTotal * 100) : 0
  const opposeRate = attTotal ? Math.round(((attMap.get('X') || 0) + (attMap.get('-') || 0)) / attTotal * 100) : 0

  /* --- 四类人员 / 实体类型 --- */
  const personTypes = [
    { label: '客户方', count: persons.length, color: 'var(--n-customer)' },
    { label: '我方（奇安信）', count: internalPersons.length, color: 'var(--n-qax)' },
    { label: '合作方', count: partnerPersons.length, color: 'var(--n-partner)' },
    { label: '竞争方', count: competitorPersons.length, color: 'var(--n-competitor)' },
  ]

  const entMap = tally(coopEntities, (e) => e.entity_type)
  const entityTypes = [
    { label: '合作方', count: entMap.get('partner') || 0, color: 'var(--n-partner)' },
    { label: '竞争方', count: entMap.get('competitor') || 0, color: 'var(--n-competitor)' },
    { label: '竞合（both）', count: entMap.get('both') || 0, color: 'var(--n-op)' },
  ]

  /* --- 竞品经营绩效（最新一年，按营收降序） --- */
  const perfByEntity = new Map()
  for (const r of entsalesPerf) {
    const key = `${r.entity_type}#${r.entity_id}`
    const prev = perfByEntity.get(key)
    if (!prev || num(r.year) >= num(prev.year)) perfByEntity.set(key, r)
  }
  const entityPerf = [...perfByEntity.values()]
    .map((r) => ({
      name: entName.get(r.entity_id) || `实体 #${r.entity_id}`,
      year: num(r.year),
      revenue: num(r.revenue),
      netProfit: num(r.net_profit),
      grossMargin: num(r.gross_margin),
      rdRatio: num(r.rd_ratio),
      growth: num(r.growth, NaN),
      type: r.entity_type,
    }))
    .sort((a, b) => Math.abs(b.revenue) - Math.abs(a.revenue))
    .slice(0, 8)

  /* --- 拜访行动 --- */
  const methodMap = tally(visits, (v) => str(v.method, '未标注'))
  const visitMethods = ['面访', '电话', '视频'].map((label) => ({
    label,
    count: methodMap.get(label) || 0,
    color: label === '面访' ? 'var(--ac)' : label === '电话' ? 'var(--info)' : 'var(--ok)',
  }))
  const monthMap = tally(visits, (v) => monthOf(v.visit_date))
  const visitMonths = [...monthMap.entries()].sort((a, b) => a[0].localeCompare(b[0])).slice(-6)
    .map(([label, count]) => ({ label, count }))
  const custVisit = tally(visits, (v) => v.customer_id)
  const visitTopCustomers = [...custVisit.entries()]
    .map(([id, count]) => ({ label: custName.get(id) || `客户 #${id}`, count }))
    .sort((a, b) => b.count - a.count)
    .slice(0, 5)

  /* --- 客户预算（最新年度汇总） --- */
  const budgetByCust = new Map()
  for (const b of budgets) {
    const prev = budgetByCust.get(b.customer_id)
    if (!prev || num(b.year) >= num(prev.year)) budgetByCust.set(b.customer_id, b)
  }
  const budgetRows = [...budgetByCust.values()]
  const budgetTotals = {
    it: sumBy(budgetRows, (b) => b.it_budget),
    sec: sumBy(budgetRows, (b) => b.sec_budget),
    secExec: sumBy(budgetRows, (b) => b.sec_budget_exec),
    years: [...new Set(budgetRows.map((b) => b.year).filter(Boolean))].sort(),
  }
  const budgetExecRate = budgetTotals.sec > 0 ? Math.round((budgetTotals.secExec / budgetTotals.sec) * 100) : 0

  /* --- 库表落库热度 --- */
  const heat = [...tableHeat]
    .filter((t) => t.rows > 0)
    .sort((a, b) => b.rows - a.rows)
    .slice(0, 12)

  return {
    source: raw.source || 'live',
    updatedAt: raw.updatedAt || '',
    kpis,
    c74111: c74111Dist,
    budgetRank,
    stages,
    levels,
    industry,
    regions,
    matrix,
    attitude,
    attitudeSummary: { total: attTotal, supportRate, opposeRate },
    personTypes,
    entityTypes,
    entityPerf,
    visits: {
      total: visits.length,
      methods: visitMethods,
      months: visitMonths,
      topCustomers: visitTopCustomers,
    },
    budget: { totals: budgetTotals, execRate: budgetExecRate, customers: budgetRows.length },
    tableHeat: heat,
    db: {
      tableCount: raw.tableCount ?? 0,
      rowTotal: tableHeat.reduce((s, t) => s + num(t.rows), 0),
      withData: tableHeat.filter((t) => t.rows > 0).length,
    },
    totals: {
      customers: customers.length,
      mainCustomers,
      projects: projView.length,
      persons: personTotal,
      entities: coopEntities.length,
      visits: visits.length,
      budgets: budgets.length,
      totalBudget,
      secTotal,
    },
  }
}

/* ---------------------------------------------------------------------------
   实时通道
   --------------------------------------------------------------------------- */

/**
 * 从后端拉全量并聚合成驾驶舱模型。
 * 任何一张表失败都直接抛出，由调用方回落到 mock —— 驾驶舱最忌讳「部分图有数据、
 * 部分图是空的」却又不告诉用户为什么。
 */
export async function loadAnalytics() {
  const results = await Promise.all(TABLES.map((t) => fetchAll(t, { page_size: 200 })))
  const byTable = Object.fromEntries(TABLES.map((t, i) => [t, results[i]]))

  // 每张表的行数（侧边栏与库表热度都用它）。health 是一次性返回，比 62 次 count 便宜得多。
  let tableHeat = []
  let tableCount = 0
  try {
    const h = await api.health()
    tableCount = h?.table_count ?? 0
    tableHeat = (h?.tables ?? []).map((t) => ({ table: t.table, rows: num(t.rows) }))
  } catch {
    tableHeat = []
  }

  const now = new Date()
  const pad = (n) => String(n).padStart(2, '0')

  return buildAnalytics({
    source: 'live',
    updatedAt: `${now.getFullYear()}-${pad(now.getMonth() + 1)}-${pad(now.getDate())} ${pad(now.getHours())}:${pad(now.getMinutes())}`,
    customers: byTable.customer,
    projects: byTable.project,
    c74111: byTable.project_c74111,
    members: byTable.project_member,
    coopEntities: byTable.coop_entity,
    entsalesPerf: byTable.entsales_perf,
    visits: byTable.visit_record,
    customerBudgets: byTable.customer_budget,
    persons: byTable.person,
    internalPersons: byTable.internal_person,
    partnerPersons: byTable.partner_person,
    competitorPersons: byTable.competitor_person,
    tableHeat,
    tableCount,
  })
}

/* ---------------------------------------------------------------------------
   mock 通道：与实时形状完全一致，保证离线 / 空库也能看到完整驾驶舱
   --------------------------------------------------------------------------- */

function deriveMock() {
  const projects = mockProjectRows()
  const c74111 = projects.map((p, i) => mockDiag(p, i))

  const customers = [
    { id: 1, name: '国家能源集团江苏电力有限公司', category: '主客户', industry: '电力 / 能源', army_region: '华东战区' },
    { id: 2, name: '华能江苏能源开发有限公司', category: '主客户', industry: '电力 / 能源', army_region: '华东战区' },
    { id: 3, name: '江苏交通控股有限公司', category: '主客户', industry: '交通 / 基建', army_region: '华东战区' },
    { id: 4, name: '南京钢铁集团', category: '主客户', industry: '钢铁 / 制造', army_region: '华东战区' },
    { id: 5, name: '苏州工业园区管委会', category: '主客户', industry: '政务', army_region: '华东战区' },
    { id: 6, name: '国家能源集团', category: '关联客户', industry: '电力 / 能源', army_region: '总部' },
    { id: 7, name: '奇安信集团', category: '关联客户', industry: '网络安全', army_region: '总部' },
  ]

  const coopEntities = [
    { id: 1, name: '中科软科技股份有限公司', entity_type: 'partner' },
    { id: 2, name: '华云信息技术有限公司', entity_type: 'partner' },
    { id: 3, name: '启明星辰信息技术集团', entity_type: 'competitor' },
    { id: 4, name: '深信服科技股份有限公司', entity_type: 'competitor' },
    { id: 5, name: '亚信安全科技股份有限公司', entity_type: 'both' },
  ]

  const entsalesPerf = [
    { entity_type: 'competitor', entity_id: 3, year: 2025, revenue: 438000, net_profit: 52000, gross_margin: 61, rd_ratio: 22, growth: 8.4 },
    { entity_type: 'competitor', entity_id: 4, year: 2025, revenue: 762000, net_profit: 38000, gross_margin: 55, rd_ratio: 26, growth: 3.1 },
    { entity_type: 'competitor', entity_id: 5, year: 2025, revenue: 312000, net_profit: 21000, gross_margin: 52, rd_ratio: 19, growth: -2.2 },
    { entity_type: 'partner', entity_id: 1, year: 2025, revenue: 685000, net_profit: 74000, gross_margin: 33, rd_ratio: 9, growth: 12.6 },
    { entity_type: 'partner', entity_id: 2, year: 2025, revenue: 240000, net_profit: 26000, gross_margin: 41, rd_ratio: 14, growth: 6.8 },
  ]

  const visits = Array.from({ length: 46 }, (_, i) => ({
    id: i + 1,
    customer_id: (i % 5) + 1,
    method: ['面访', '面访', '电话', '视频'][i % 4],
    visit_date: `2026-0${(i % 6) + 4}-${String((i % 27) + 1).padStart(2, '0')}`,
  }))

  const customerBudgets = [
    { customer_id: 1, year: 2026, it_budget: 12800, sec_budget: 4200, sec_budget_exec: 3100 },
    { customer_id: 2, year: 2026, it_budget: 7600, sec_budget: 2400, sec_budget_exec: 1900 },
    { customer_id: 3, year: 2026, it_budget: 9200, sec_budget: 2800, sec_budget_exec: 1500 },
    { customer_id: 4, year: 2026, it_budget: 5600, sec_budget: 1600, sec_budget_exec: 1200 },
    { customer_id: 5, year: 2026, it_budget: 4300, sec_budget: 1500, sec_budget_exec: 1100 },
  ]

  const persons = Array.from({ length: 38 }, (_, i) => ({ id: i + 1, name: `客户人员${i + 1}` }))
  const internalPersons = Array.from({ length: 24 }, (_, i) => ({
    id: 100 + i,
    name: `我方成员${i + 1}`,
    attitude: ['+', '+', '=', '+', '-', '⭐'][i % 6],
  }))
  const partnerPersons = Array.from({ length: 11 }, (_, i) => ({
    id: 200 + i,
    name: `合作方人员${i + 1}`,
    attitude: ['+', '=', '+', '-'][i % 4],
  }))
  const competitorPersons = Array.from({ length: 9 }, (_, i) => ({
    id: 300 + i,
    name: `竞争方人员${i + 1}`,
    attitude: ['X', '-', '=', '-'][i % 4],
  }))

  const members = Array.from({ length: 38 }, (_, i) => ({
    id: i + 1,
    person_id: i + 1,
    attitude: ['+', '+', '=', '-', '+', '⭐', '=', 'X'][i % 8],
    adur_role: ['A', 'D', 'U', 'R'][i % 4],
  }))

  const tableHeat = [
    ['news_item', 128], ['customer', 42], ['visit_record', 46], ['person', 38],
    ['project_member', 38], ['daily_todo', 36], ['internal_person', 24],
    ['project', 12], ['coop_entity', 5], ['customer_budget', 5],
    ['entsales_perf', 5], ['project_c74111', 9], ['dashboard_note', 8],
    ['focus_session', 22], ['customer_news', 17], ['customer_opportunity', 11],
  ].map(([table, rows]) => ({ table, rows }))

  return {
    source: 'mock',
    updatedAt: '演示数据',
    customers,
    projects,
    c74111,
    members,
    coopEntities,
    entsalesPerf,
    visits,
    customerBudgets,
    persons,
    internalPersons,
    partnerPersons,
    competitorPersons,
    tableHeat,
    tableCount: 62,
  }
}

/** 把 mock.js 的 5 个演示项目转成后端 project 行的形状 */
function mockProjectRows() {
  return mock.PROJECTS.map((p, i) => ({
    id: p.id,
    customer_id: (i % 5) + 1,
    name: p.name,
    level: p.level,
    info_type: p.stage,
    budget: p.budget,
    sec_budget: p.secBudget,
    win_score: String(p.winRate),
    competition: p.competition,
    commitment: ['可争取', '可承诺', '可参与', '可争取', '不可承诺'][i] || '可争取',
  }))
}

/** 与 mock 项目的 competition 对齐造一份诊断行，保证分档与态势自洽 */
function mockDiag(project, i) {
  const total = Number(project.win_score) || 0
  const c = [8, 4, 3, 4, 4, 3, 4]
  const p = [4, 3, Math.round(total * 0.2), Math.round(total * 0.1)]
  const key = Math.round(total * 0.1)
  const sum = (arr) => arr.reduce((s, x) => s + Number(x || 0), 0)
  return {
    project_id: project.id,
    c1_score: c[0], c2_score: c[1], c3_score: c[2], c4_score: c[3],
    c5_score: c[4], c6_score: c[5], c7_score: c[6],
    p1_score: p[0], p2_score: p[1], p3_score: p[2], p4_score: p[3],
    key_score: key,
    clear_score: sum(c),
    priority_score: sum(p),
    key_total: key,
    total_score: total,
    win_rate: total,
    competition: project.competition,
    commitment: project.commitment,
  }
}

export const MOCK_ANALYTICS = buildAnalytics(deriveMock())
