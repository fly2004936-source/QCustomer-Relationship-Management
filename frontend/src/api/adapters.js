/**
 * 后端行 → 前端展示模型
 *
 * 目标：把后端表结构翻译成 mock.js 里那套模型，让组件层**完全不需要知道**
 * 数据来自 mock 还是真实库。命名差异（sec_budget → secBudget）与外键
 * （project.customer_id → 客户名）都在这一层消化掉。
 *
 * 节点格式（与 OrgGraph 约定一致）：
 *   { id, zone, kind, source, parent, label, sub, adur, attitude, detail, c74111 }
 *   zone:   top | anchor | tree | left | right | team | diag
 *   kind:   customer | project | person | partner | competitor | qax | c74111
 *   source: customer | partner | competitor | qax
 *
 * 列名以实测库结构为准（勿凭印象改）：
 *   · partner_person / competitor_person 的关联列是 **coop_entity_id**（不是 entity_id）
 *   · 我方团队走 **org 组织树**（internal_person.org_id → org.id），不是平铺
 *   · project_member 提供 ADUR（adur_role）与态度（attitude），随项目变化
 */

/* ---------------------------------------------------------------------------
   归一化工具
   --------------------------------------------------------------------------- */

const STAGE_RULES = [
  [/线索/, '线索'],
  [/商机/, '商机'],
  [/项目|实施|交付|签约|中标|投标|招标/, '项目'],
]

/**
 * 后端没有 `project.stage` 列。阶段要按 `info_type`（枚举：线索 / 商机 / 项目）
 * 优先取，它才是枚举化的权威字段；`time_phase` 与 `market_status` 是自由文本，仅作兜底。
 */
export function pickStage(row) {
  for (const raw of [row?.info_type, row?.time_phase, row?.market_status]) {
    const s = String(raw ?? '').trim()
    if (!s) continue
    for (const [re, label] of STAGE_RULES) if (re.test(s)) return label
  }
  return '商机'
}

const ADUR_ALIAS = {
  A: 'A',
  决定人: 'A',
  决策人: 'A',
  D: 'D',
  拍板人: 'D',
  U: 'U',
  使用者: 'U',
  R: 'R',
  推荐人: 'R',
}
const ADUR_LABEL = { A: 'A 决定人', D: 'D 拍板人', U: 'U 使用者', R: 'R 推荐人' }

/**
 * ADUR 可能是 'A' 也可能是 '拍板人'，统一成单字母；无法识别返回空串。
 * 注意：**只用于 `project_member.adur_role`**（自由文本）。
 * `internal_person.decision_role` 的枚举是「拍板/协调/支撑/执行」，
 * `partner_person.decision_power` 是「拍板/影响/执行」，都是另一套语义，不要走这里。
 */
export function normalizeAdur(v) {
  const s = String(v ?? '').trim()
  if (!s) return ''
  if (ADUR_ALIAS[s.toUpperCase()]) return ADUR_ALIAS[s.toUpperCase()]
  for (const [k, val] of Object.entries(ADUR_ALIAS)) if (s.includes(k)) return val
  return ''
}

/** 后端态度枚举：["X","-","=","+","⭐"]（⭐ 门槛极高）；同时兼容早期的 1/0/-1 */
const ATTITUDE_ENUM = ['X', '-', '=', '+', '⭐']
export function pickAttitude(v) {
  const s = String(v ?? '').trim()
  if (!s) return ''
  if (ATTITUDE_ENUM.includes(s)) return s
  if (['1', '支持', '正'].includes(s)) return '+'
  if (['0', '中立', '中'].includes(s)) return '='
  if (['-1', '不支持', '负', '反对'].includes(s)) return '-'
  return ''
}

const num = (v, fallback = 0) => {
  const n = Number(v)
  return Number.isFinite(n) ? n : fallback
}

const str = (v, fallback = '—') => {
  const s = String(v ?? '').trim()
  return s || fallback
}

/* ---------------------------------------------------------------------------
   项目 / KPI
   --------------------------------------------------------------------------- */

export function toProject(row, customerNameById) {
  return {
    id: row.id,
    name: str(row.name, `项目 #${row.id}`),
    customer: customerNameById?.get(row.customer_id) || (row.customer_id ? `客户 #${row.customer_id}` : '—'),
    customerId: row.customer_id ?? null,
    stage: pickStage(row),
    level: str(row.level, 'P3'),
    budget: num(row.budget),
    secBudget: num(row.sec_budget),
    winRate: num(row.win_score),
    competition: str(row.competition, ''),
    commitment: str(row.commitment, ''),
    strategy: str(row.strategy, ''),
    tenderMode: str(row.tender_mode, ''),
    raw: row,
  }
}

export const buildProjects = (rows, nameById) => (rows || []).map((r) => toProject(r, nameById))

/**
 * KPI 卡。
 * 注意：`delta` 语义是「较上期变化」，真实库里没有历史快照，因此留空，
 * 由模板侧 `v-if` 跳过——不要把绝对值塞进 delta 冒充环比。
 */
export function buildKpis({ projects, personCount }) {
  const totalBudget = projects.reduce((s, p) => s + p.budget, 0)
  const secTotal = projects.reduce((s, p) => s + p.secBudget, 0)
  const secRatio = totalBudget > 0 ? Math.round((secTotal / totalBudget) * 100) : 0
  const avgWin = projects.length ? projects.reduce((s, p) => s + p.winRate, 0) / projects.length : 0
  const p1 = projects.filter((p) => p.level === 'P1').length

  return [
    {
      name: '在跟项目',
      value: String(projects.length),
      unit: '个',
      foot: `其中 P1 级 ${p1} 个`,
      delta: '',
      trend: projects.length ? 'up' : 'down',
      icon: 'layers',
    },
    {
      name: '预计合同金额',
      value: totalBudget.toLocaleString('zh-CN'),
      unit: '万元',
      foot: `安全预算 ${secTotal.toLocaleString('zh-CN')} 万元 · 占比 ${secRatio}%`,
      delta: '',
      trend: secRatio > 0 ? 'up' : 'down',
      icon: 'coin',
    },
    {
      name: '平均趋赢力',
      value: avgWin.toFixed(1),
      unit: '%',
      foot: `${projects.length} 个项目加权（C74111 满分 100）`,
      delta: '',
      trend: avgWin >= 50 ? 'up' : 'down',
      icon: 'target',
    },
    {
      name: '决策链建档人数',
      value: String(personCount),
      unit: '人',
      foot: '含客户方与我方自有团队',
      delta: '',
      trend: personCount ? 'up' : 'down',
      icon: 'users',
    },
  ]
}

/* ---------------------------------------------------------------------------
   C74111 诊断
   --------------------------------------------------------------------------- */

const CLEARS = [
  ['C1', '客户组织结构图', 10],
  ['C2', '拍板人燃眉之急', 5],
  ['C3', '立项六要素', 5],
  ['C4', '采购周期', 5],
  ['C5', '合作方三问', 5],
  ['C6', '竞争方三问', 5],
  ['C7', '独特客户价值', 5],
]
const PRIORITIES = [
  ['P1', '项目组多数人支持', 5],
  ['P2', '合作伙伴代理商支持', 5],
  ['P3', '与拍板人密谋策划', 20],
  ['P4', '关键人密谋策划', 10],
]

export function toDiagNode(project, c) {
  if (!c) return null
  const total = num(c.total_score)
  const winRate = num(c.win_rate, total)
  const competition = str(c.competition, '') || project.competition || ''
  const commitment = str(c.commitment, '')

  return {
    id: `diag-${project.id}`,
    zone: 'diag',
    kind: 'c74111',
    label: competition || '未评估',
    sub: `${total} 分 · ${Math.round(winRate)}%`,
    detail: {
      总分: `${total} / 100`,
      趋赢力: `${Math.round(winRate)}%`,
      竞争态势: competition || '—',
      可否承诺: commitment || '—',
      竞争策略: str(c.strategy, '') || project.strategy || '尚未填写',
      行动计划: str(c.action_plan, '尚未填写'),
    },
    c74111: {
      clears: CLEARS.map(([code, name, max], i) => ({
        code,
        name,
        score: num(c[`c${i + 1}_score`]),
        max,
      })),
      priorities: PRIORITIES.map(([code, name, max], i) => ({
        code,
        name,
        score: num(c[`p${i + 1}_score`]),
        max,
        star: i >= 2 ? !!c[`p${i + 1}_star`] : false,
      })),
      key: [
        {
          code: 'Key',
          name: '与最高决策人密谋策划',
          score: num(c.key_score),
          max: 20,
          star: !!c.key_star,
        },
      ],
    },
  }
}

/* ---------------------------------------------------------------------------
   关系网络 / 组织架构
   --------------------------------------------------------------------------- */

const TEAM_FALLBACK_ROOT = 'team-root'

/**
 * 装配 OrgGraph 的节点数组。
 *
 * 分层容错：任何一张表缺失或为空都只影响对应那一翼，不会让整图渲染失败。
 * 这样空库上依然能看到客户层，便于区分「确实没数据」和「接口坏了」。
 */
export function buildGraph({
  customers = [],
  projects = [],
  persons = [],
  members = [],
  coopEntities = [],
  partnerPersons = [],
  competitorPersons = [],
  internalPersons = [],
  orgs = [],
  c74111Map = new Map(),
}) {
  const nodes = []

  /* ADUR / 态度取自 project_member（随项目变化）*/
  const memberByPerson = new Map()
  for (const m of members) if (m?.person_id != null) memberByPerson.set(m.person_id, m)

  /* --- 1. 客户层 --- */
  const mainCustomers = customers.filter((c) => (c.category || '') === '主客户')
  const topCustomers = mainCustomers.length ? mainCustomers : customers
  for (const c of topCustomers) {
    nodes.push({
      id: `cust-${c.id}`,
      zone: 'top',
      kind: 'customer',
      source: 'customer',
      label: str(c.name, `客户 #${c.id}`),
      sub: [str(c.category, '客户'), str(c.industry, '')].filter(Boolean).join(' · '),
      detail: {
        客户名称: str(c.name),
        分类: str(c.category),
        行业: str(c.industry),
        职能: str(c.func),
        上级单位: str(c.superior_unit),
        客户编码: str(c.crm_code),
      },
    })
  }
  const firstTopId = nodes.find((n) => n.zone === 'top')?.id ?? null
  const topIds = new Set(topCustomers.map((c) => `cust-${c.id}`))

  /* --- 2. 项目锚点 --- */
  for (const p of projects) {
    nodes.push({
      id: `proj-${p.id}`,
      zone: 'anchor',
      kind: 'project',
      source: 'customer',
      parent: topIds.has(`cust-${p.customerId}`) ? `cust-${p.customerId}` : firstTopId,
      label: p.name,
      sub: `${p.stage} · ${p.level}`,
      stage: p.stage,
      detail: {
        项目名称: p.name,
        客户: p.customer,
        阶段类型: p.stage,
        项目级别: p.level,
        预算万元: String(p.budget),
        安全预算万元: String(p.secBudget),
        趋赢力: String(p.winRate),
        竞争态势: p.competition || '—',
      },
    })
  }

  /* --- 3. 客户方人员层级树：有 parent_id 就按它接，否则挂所属项目锚点 --- */
  const pIdMap = new Map(persons.map((p) => [p.id, `p-${p.id}`]))
  const anchorOfCustomer = (custId) => {
    const hit = projects.find((pr) => pr.customerId === custId)
    return hit ? `proj-${hit.id}` : projects[0] ? `proj-${projects[0].id}` : firstTopId
  }

  for (const p of persons) {
    const m = memberByPerson.get(p.id)
    const adur = normalizeAdur(m?.adur_role)
    nodes.push({
      id: pIdMap.get(p.id),
      zone: 'tree',
      kind: 'person',
      source: 'customer',
      parent:
        p.parent_id != null && pIdMap.has(p.parent_id)
          ? pIdMap.get(p.parent_id)
          : anchorOfCustomer(p.customer_id),
      label: str(p.name, `人员 #${p.id}`),
      sub: str(p.title, '') || str(p.dept, '—'),
      adur,
      attitude: pickAttitude(m?.attitude),
      mentor: !!m?.is_mentor,
      detail: {
        姓名: str(p.name),
        职务: str(p.title),
        部门: str(p.dept),
        层级: str(p.rank),
        ADUR: ADUR_LABEL[adur] || '未标注',
        态度: pickAttitude(m?.attitude) || '未标注',
        决策模式: str(p.decision_mode),
      },
    })
  }

  /* --- 4. 合作方（左）/ 竞争方（右）；人员用 coop_entity_id 关联，parent_id 建层级 --- */
  const entityNodeId = (side, id) => `ent-${side}-${id}`

  const pushEntity = (e, side) => {
    nodes.push({
      id: entityNodeId(side, e.id),
      zone: side === 'partner' ? 'left' : 'right',
      kind: side,
      source: side,
      label: str(e.name, `${side === 'partner' ? '合作方' : '竞争方'} #${e.id}`),
      sub: str(e.overall_relation, '') || str(e.company_type, '') || (side === 'partner' ? '合作方' : '竞争方'),
      detail: {
        名称: str(e.name),
        实体类型: str(e.entity_type),
        整体关系: str(e.overall_relation),
        总部: str(e.headquarters),
        股权背景: str(e.equity_bg),
        竞争优势: str(e.advantage),
        竞争劣势: str(e.weakness),
        我方优势: str(e.our_advantage),
        市场地位: str(e.market_share),
      },
    })
  }

  const partnerEnts = coopEntities.filter((e) => ['partner', 'both'].includes(e.entity_type))
  const competitorEnts = coopEntities.filter((e) => ['competitor', 'both'].includes(e.entity_type))
  for (const e of partnerEnts) pushEntity(e, 'partner')
  for (const e of competitorEnts) pushEntity(e, 'competitor')

  const pushEntityPersons = (rows, side) => {
    const idMap = new Map(rows.map((r) => [r.id, `ep-${side}-${r.id}`]))
    for (const r of rows) {
      const parentEnt = entityNodeId(side, r.coop_entity_id)
      nodes.push({
        id: idMap.get(r.id),
        zone: side === 'partner' ? 'left' : 'right',
        kind: 'person',
        source: side,
        parent:
          r.parent_id != null && idMap.has(r.parent_id)
            ? idMap.get(r.parent_id)
            : nodes.some((n) => n.id === parentEnt)
              ? parentEnt
              : null,
        label: str(r.name, `${side === 'partner' ? '合作方' : '竞争方'}人员 #${r.id}`),
        sub: str(r.title, '') || str(r.dept, '—'),
        attitude: pickAttitude(r.attitude),
        detail: {
          姓名: str(r.name),
          职务: str(r.title),
          部门: str(r.dept),
          公司内角色: str(r.role_in_company),
          决策力: str(r.decision_power),
          对我方态度: pickAttitude(r.attitude) || '—',
          与我方关系: str(r.our_liaison),
          共同客户: str(r.common_customer),
          情报能力: str(r.intel_ability),
        },
      })
    }
  }
  pushEntityPersons(partnerPersons, 'partner')
  pushEntityPersons(competitorPersons, 'competitor')

  /* --- 5. 我方自有团队（奇安信）：org 组织树 + internal_person --- */
  if (orgs.length || internalPersons.length) {
    const orgIdMap = new Map(orgs.map((o) => [o.id, `org-${o.id}`]))
    const sortedOrgs = [...orgs].sort((a, b) => num(a.level) - num(b.level))
    const rootOrg = sortedOrgs.find((o) => !o.parent_id) || sortedOrgs[0] || null
    const rootId = rootOrg ? orgIdMap.get(rootOrg.id) : TEAM_FALLBACK_ROOT

    if (!rootOrg) {
      nodes.push({
        id: TEAM_FALLBACK_ROOT,
        zone: 'team',
        kind: 'qax',
        source: 'qax',
        label: '奇安信集团',
        sub: '我方团队 · 独立类别',
        detail: {
          名称: '奇安信集团',
          类别: '我方（自有团队）',
          归属: '既不属于合作方，也不属于竞争方',
        },
      })
    } else {
      for (const o of sortedOrgs) {
        nodes.push({
          id: orgIdMap.get(o.id),
          zone: 'team',
          kind: 'qax',
          source: 'qax',
          parent: o.parent_id != null && orgIdMap.has(o.parent_id) ? orgIdMap.get(o.parent_id) : null,
          label: str(o.name, `组织 #${o.id}`),
          sub: [str(o.org_type, ''), o.level != null ? `L${o.level}` : ''].filter(Boolean).join(' · ') || '我方组织',
          detail: {
            组织名称: str(o.name),
            类型: str(o.org_type),
            层级: o.level != null ? `L${o.level}` : '—',
            归属: '我方（自有团队）· 独立类别',
            说明: '奇安信为投标主体，既不属于合作方，也不属于竞争方',
          },
        })
      }
    }

    for (const t of internalPersons) {
      nodes.push({
        id: `team-${t.id}`,
        zone: 'team',
        kind: 'person',
        source: 'qax',
        parent: (t.org_id != null && orgIdMap.has(t.org_id) ? orgIdMap.get(t.org_id) : rootId) ?? null,
        label: str(t.name, `成员 #${t.id}`),
        sub: str(t.title, '') || str(t.dept, '—'),
        // decision_role 是「拍板/协调/支撑/执行」，不是 ADUR，故不设 adur 徽标
        attitude: pickAttitude(t.attitude),
        detail: {
          姓名: str(t.name),
          职务: str(t.title),
          部门: str(t.dept),
          职级: str(t.rank),
          决策角色: str(t.decision_role),
          支持能力: str(t.support_ability),
          管理范围: str(t.management_scope),
          态度: pickAttitude(t.attitude) || '—',
        },
      })
    }
  }

  /* --- 6. C74111 诊断：独立节点，不参与连线 --- */
  for (const p of projects) {
    const d = toDiagNode(p, c74111Map.get(p.id))
    if (d) nodes.push(d)
  }

  return nodes
}

/* ---------------------------------------------------------------------------
   汇总入口
   --------------------------------------------------------------------------- */

export function buildDataset(raw) {
  const nameById = new Map((raw.customers || []).map((c) => [c.id, c.name]))
  const projects = buildProjects(raw.projects, nameById)
  const graphNodes = buildGraph({ ...raw, projects })
  const personCount =
    (raw.persons || []).length +
    (raw.internalPersons || []).length +
    (raw.partnerPersons || []).length +
    (raw.competitorPersons || []).length
  const firstDiag = raw.c74111Map?.get(projects[0]?.id)

  return {
    kpis: buildKpis({ projects, personCount }),
    projects,
    graphNodes,
    heroStats: [
      { k: '决策链', v: personCount, u: '人' },
      { k: '节点', v: graphNodes.length, u: '个' },
      { k: 'C74111', v: num(firstDiag?.total_score), u: '分' },
    ],
    personOptions: [
      ...(raw.persons || []).map((p) => p.name),
      ...(raw.internalPersons || []).map((p) => p.name),
    ].filter(Boolean),
  }
}
