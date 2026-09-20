/**
 * 关系网络 / 组织架构的数据装配
 *
 * 两个入口：
 *   buildCustomerGraph —— 选客户后展示「该客户的人员组织架构网络」
 *   buildProjectGraph  —— 点开项目后的作战视图（人员+态度+ADUR / C74111 / 我方派出 / 合作竞争方）
 *
 * 节点结构沿用 OrgGraph 的契约：
 *   { id, zone, kind, label, sub, parent, source, attitude, adur, teamRole, stage, detail, raw, ... }
 * zone 决定摆放位置：top 客户 / anchor 项目锚点 / tree 人员组织树 / team 我方 / left 合作方 / right 竞争方 / diag 独立诊断节点
 *
 * 关键真实列（踩过坑，别再凭印象写）：
 *   · project_member.person_id → person.id，态度列是 project_member.attitude，ADUR 是自由文本 adur_role
 *   · partner_person / competitor_person 的归属列是 **coop_entity_id**（不是 entity_id）
 *   · 我方「派出人员」没有直接挂在项目上，唯一真实链路是
 *     visit_record.project_id → visit_our_staff.internal_person_id → internal_person
 */

import { pickAttitude, pickStage, normalizeAdur, toDiagNode } from './adapters'

const S = (v, d = '—') => {
  const s = String(v ?? '').trim()
  return s || d
}
const N = (v) => (v === null || v === undefined || v === '' ? null : Number(v))

/** 去掉值为「—」的键，避免详情面板被空字段淹没 */
function compact(obj) {
  const out = {}
  for (const [k, v] of Object.entries(obj)) {
    if (v === null || v === undefined) continue
    const s = String(v).trim()
    if (!s || s === '—') continue
    out[k] = s
  }
  return out
}

/* ---------------------------------------------------------------------------
   C74111 分档
   口径：满分 100（7 Clears 40 + 4 Priorities 40 + Key 20）。
   四档按用户要求从高到低 = 绿 / 蓝 / 黄 / 红，25 分一档。
   ------------------------------------------------------------------------ */

export const TIER_META = {
  a: { key: 'a', cls: 'c74--a', label: '绝对优势', range: '> 75', hint: '可承诺', tone: 'ok' },
  b: { key: 'b', cls: 'c74--b', label: '相对优势', range: '50 – 75', hint: '可争取', tone: 'info' },
  c: { key: 'c', cls: 'c74--c', label: '相对劣势', range: '25 – 50', hint: '可参与', tone: 'warn' },
  d: { key: 'd', cls: 'c74--d', label: '绝对劣势', range: '< 25', hint: '不可承诺', tone: 'bad' },
  /**
   * 第五种「档位」：还没做过 C74111 诊断。
   * 不加这一档的话，没有诊断记录的项目会被当成 0 分，界面显示「0 分 · 绝对劣势 · 不可承诺」——
   * 这是**误导性**的：没评过和评完得 0 分是两件事。
   */
  none: { key: 'none', cls: 'c74--none', label: '未诊断', range: '—', hint: '待评分', tone: '' },
}

export function tierOf(score) {
  const s = Number(score)
  if (!Number.isFinite(s)) return 'd'
  if (s > 75) return 'a'
  if (s >= 50) return 'b'
  if (s >= 25) return 'c'
  return 'd'
}

/** 是否已经有诊断记录（NULL / '' / 非数字都算没有） */
export function isScored(score) {
  if (score === null || score === undefined || score === '') return false
  return Number.isFinite(Number(score))
}

/**
 * 分数 → 展示用的分档信息。
 * 标签优先用库里的 commitment，颜色一定按分数档；
 * 没有诊断记录时返回 `known:false` + 未诊断档，由界面决定怎么呈现。
 */
export function tierInfo(score, commitment = '') {
  if (!isScored(score)) {
    return { ...TIER_META.none, score: null, known: false, commitment: '待评分' }
  }
  const t = TIER_META[tierOf(score)]
  return { ...t, score: Number(score), known: true, commitment: S(commitment, t.hint) }
}

/* ---------------------------------------------------------------------------
   人员详情的通用拼装
   ------------------------------------------------------------------------ */

function customerPersonDetail(p, extra = {}) {
  return compact({
    姓名: S(p.name),
    职务: S(p.title),
    部门: S(p.dept),
    职级: S(p.rank),
    决策模式: S(p.decision_mode),
    职业背景: S(p.career_bg),
    角色范围: S(p.role_scope),
    可信度: S(p.credibility),
    联系方式: S(p.contact),
    组织路径: S(p.org_path),
    备注: S(p.remark),
    ...extra,
  })
}

export function internalPersonDetail(p, extra = {}) {
  return compact({
    姓名: S(p.name),
    职务: S(p.title),
    部门: S(p.dept),
    职级: S(p.rank),
    决策角色: S(p.decision_role),
    管理范围: S(p.management_scope),
    支撑能力: S(p.support_ability),
    职业背景: S(p.career_bg),
    入职时间: S(p.entry_time),
    联系方式: S(p.contact),
    可信度: S(p.credibility),
    备注: S(p.remark),
    ...extra,
  })
}

export function coopPersonDetail(p, extra = {}) {
  return compact({
    姓名: S(p.name),
    职务: S(p.title),
    公司角色: S(p.role_in_company),
    决策力: S(p.decision_power),
    部门: S(p.dept),
    职级: S(p.rank),
    职责: S(p.responsibility),
    可调动资源: S(p.resources),
    我方对接人: S(p.our_liaison),
    共同客户: S(p.common_customer),
    对我方价值: S(p.value_to_us),
    情报能力: S(p.intel_ability),
    私人关系深度: S(p.private_depth),
    联系方式: S(p.contact),
    ...extra,
  })
}

/** 把 JSON 列（可能是字符串也可能是对象）转成可读文本 */
function jsonText(v, fallback = '—') {
  if (v === null || v === undefined || v === '') return fallback
  if (typeof v === 'string') {
    const s = v.trim()
    if (!s) return fallback
    try {
      const o = JSON.parse(s)
      if (Array.isArray(o)) return o.map((x) => (typeof x === 'object' ? JSON.stringify(x) : x)).join('、')
      if (o && typeof o === 'object') return Object.entries(o).map(([k, val]) => `${k}: ${val}`).join('；')
      return String(o)
    } catch {
      return s
    }
  }
  if (Array.isArray(v)) return v.map((x) => (typeof x === 'object' ? JSON.stringify(x) : x)).join('、')
  if (typeof v === 'object') return Object.entries(v).map(([k, val]) => `${k}: ${val}`).join('；')
  return String(v)
}

export function entityDetail(e) {
  return compact({
    名称: S(e.name),
    类型: e.entity_type === 'partner' ? '合作方' : e.entity_type === 'competitor' ? '竞争方' : '合作 + 竞争',
    公司类型: S(e.company_type),
    总部: S(e.headquarters),
    股权背景: S(e.equity_bg),
    上市情况: S(e.listing),
    总体关系: S(e.overall_relation),
    竞争态势: S(e.competition_situation),
    独家承诺: N(e.exclusive_promise) === 1 ? '是' : '否',
    产品矩阵: jsonText(e.product_matrix),
    市场份额: jsonText(e.market_share),
    渠道策略: S(e.channel_strategy),
    代理体系: S(e.agent_system),
    价格带: S(e.price_band),
    业务战术: S(e.business_tactics),
    生态: S(e.ecosystem),
    价值主张: S(e.value_prop),
    产品排名: S(e.product_rank),
    对方优势: S(e.advantage),
    对方劣势: S(e.weakness),
    我方优势: S(e.our_advantage),
    我方劣势: S(e.our_weakness),
    桌面话术: jsonText(e.fab_talk),
    备注: S(e.remark),
  })
}

/* ---------------------------------------------------------------------------
   态度 / ADUR 的展示口径
   库里 attitude 的枚举值含 "⭐"，界面上用户要求用「※」来表示"铁杆支持"，
   "X" 也用全角的「×」更醒目。统一在这里映射，避免各处写法不一致。
   ------------------------------------------------------------------------ */

export const ATTITUDE_CHAR = { X: '×', '-': '−', '=': '=', '+': '+', '⭐': '※' }
export const ATTITUDE_CLS = {
  X: 'att--x',
  '-': 'att--minus',
  '=': 'att--eq',
  '+': 'att--plus',
  '⭐': 'att--star',
}
export const ATTITUDE_LIST = [
  { v: 'X', char: '×', label: '反对', cls: 'att--x' },
  { v: '-', char: '−', label: '不支持', cls: 'att--minus' },
  { v: '=', char: '=', label: '中立', cls: 'att--eq' },
  { v: '+', char: '+', label: '支持', cls: 'att--plus' },
  { v: '⭐', char: '※', label: '铁杆支持', cls: 'att--star' },
]

export const ADUR_LABEL = { A: '决定人', D: '拍板人', U: '使用者', R: '推荐人' }
export const ADUR_LIST = [
  { k: 'A', label: '决定人' },
  { k: 'D', label: '拍板人' },
  { k: 'U', label: '使用者' },
  { k: 'R', label: '推荐人' },
]

export const attChar = (a) => ATTITUDE_CHAR[a] ?? a ?? ''
export const attCls = (a) => ATTITUDE_CLS[a] ?? ''

/* ---------------------------------------------------------------------------
   客户视图：客户 + 该客户的人员组织架构
   ------------------------------------------------------------------------ */

/**
 * @param {object} o
 * @param {object} o.customer
 * @param {object[]} o.persons   该客户下的 person 行
 * @param {object[]} o.projects  该客户的项目（只用于详情里的项目数统计）
 * @param {Map}      o.c74111Map project_id → project_c74111 行
 */
export function buildCustomerGraph({ customer, persons = [], projects = [], c74111Map = new Map() }) {
  if (!customer) return { nodes: [], meta: { kind: 'customer' } }

  const custId = `cust-${customer.id}`
  const ids = new Set(persons.map((p) => p.id))

  const nodes = [
    {
      id: custId,
      zone: 'top',
      kind: 'customer',
      table: 'customer',
      rowId: customer.id,
      label: S(customer.name, `客户 #${customer.id}`),
      sub: [S(customer.category, ''), S(customer.level, '')].filter(Boolean).join(' · '),
      detail: compact({
        客户编码: S(customer.crm_code),
        客户名称: S(customer.name),
        客户类别: S(customer.category),
        客户级别: S(customer.level),
        行业: S(customer.industry),
        战区: S(customer.army_region),
        备注: S(customer.remark),
      }),
      raw: customer,
    },
  ]

  for (const p of persons) {
    const parent = p.parent_id && ids.has(p.parent_id) ? `person-${p.parent_id}` : custId
    nodes.push({
      id: `person-${p.id}`,
      zone: 'tree',
      kind: 'person',
      table: 'person',
      rowId: p.id,
      parent,
      source: 'customer',
      label: S(p.name, `人员 #${p.id}`),
      sub: [S(p.title, ''), S(p.dept, '')].filter(Boolean).join(' · '),
      detail: customerPersonDetail(p),
      raw: p,
    })
  }

  return {
    nodes,
    meta: {
      kind: 'customer',
      customer,
      personCount: persons.length,
      projectCount: projects.length,
      projectMaxScore: projects.reduce((m, p) => {
        const s = Number(c74111Map.get(p.id)?.total_score)
        return Number.isFinite(s) ? Math.max(m, s) : m
      }, 0),
    },
  }
}

/* ---------------------------------------------------------------------------
   项目视图
   ------------------------------------------------------------------------ */

/**
 * @param {object} o
 * @param {object}   o.customer
 * @param {object}   o.project
 * @param {object[]} o.persons            客户方人员（用于挂出真实组织层级）
 * @param {object[]} o.members            project_member 行（含 attitude / adur_role）
 * @param {object[]} o.staff              internal_person 行（本项目派出人员）
 * @param {object[]} o.orgs               org 行（我方组织树）
 * @param {object[]} o.entities           coop_entity 行
 * @param {object[]} o.partnerPersons     partner_person 行
 * @param {object[]} o.competitorPersons  competitor_person 行
 * @param {object[]} o.entProjects        coop_entity_project_rel 行
 * @param {object[]} o.ppProjects         partner_person_project_rel 行
 * @param {object[]} o.cpProjects         competitor_person_project_rel 行
 * @param {object}   o.c74111             project_c74111 行
 */
export function buildProjectGraph({
  customer,
  project,
  persons = [],
  members = [],
  staff = [],
  orgs = [],
  entities = [],
  partnerPersons = [],
  competitorPersons = [],
  entProjects = [],
  ppProjects = [],
  cpProjects = [],
  c74111 = null,
}) {
  if (!project) return { nodes: [], sides: { partners: [], competitors: [] }, meta: {} }

  const nodes = []
  const custId = `cust-${customer?.id}`
  const projId = `proj-${project.id}`

  /* 1. 客户节点 —— 需求要求「点击项目后同样显示客户节点」，点它可返回上一步 */
  if (customer) {
    nodes.push({
      id: custId,
      zone: 'top',
      kind: 'customer',
      table: 'customer',
      rowId: customer.id,
      label: S(customer.name, `客户 #${customer.id}`),
      sub: [S(customer.category, ''), S(customer.level, '')].filter(Boolean).join(' · '),
      detail: compact({
        客户编码: S(customer.crm_code),
        客户名称: S(customer.name),
        客户类别: S(customer.category),
        客户级别: S(customer.level),
        行业: S(customer.industry),
        战区: S(customer.army_region),
      }),
      raw: customer,
    })
  }

  /* 2. 项目锚点 */
  nodes.push({
    id: projId,
    zone: 'anchor',
    kind: 'project',
    table: 'project',
    rowId: project.id,
    parent: customer ? custId : null,
    stage: pickStage(project),
    label: S(project.name, `项目 #${project.id}`),
    sub: `${S(project.level, '—')} · ${pickStage(project)}`,
    detail: projectDetail(project, c74111),
    raw: project,
  })

  /* 3. 项目相关人员 —— 客户方组织架构 + 态度 + ADUR */
  const personById = new Map(persons.map((p) => [p.id, p]))
  const memberByPerson = new Map(members.map((m) => [m.person_id, m]))
  const memberIds = new Set(members.map((m) => m.person_id))
  // 组织树里要出现「相关人员的上级」，否则相关人只能挂到项目上，看不出组织关系
  const includeIds = new Set(memberIds)
  for (const id of memberIds) {
    let cur = personById.get(id)
    let guard = 0
    while (cur?.parent_id && guard < 40) {
      includeIds.add(cur.parent_id)
      cur = personById.get(cur.parent_id)
      guard += 1
    }
  }

  for (const p of persons) {
    if (!includeIds.has(p.id)) continue
    const m = memberByPerson.get(p.id)
    const isMember = !!m
    const parentInTree = p.parent_id && includeIds.has(p.parent_id)
    nodes.push({
      id: `person-${p.id}`,
      zone: 'tree',
      kind: 'person',
      table: 'person',
      rowId: p.id,
      parent: parentInTree ? `person-${p.parent_id}` : projId,
      source: 'customer',
      label: S(p.name, `人员 #${p.id}`),
      sub: [S(p.title, ''), S(p.dept, '')].filter(Boolean).join(' · '),
      // 只有真正挂在项目上的人才带态度 / ADUR，上级只作为组织背景出现
      attitude: isMember ? pickAttitude(m.attitude) : '',
      adur: isMember ? normalizeAdur(m.adur_role) : '',
      memberId: m?.id ?? null,
      mentor: isMember ? N(m.is_mentor) === 1 : false,
      detail: customerPersonDetail(p, {
        '本项目身份': isMember ? '项目相关人' : '上级 / 组织背景',
        '对项目态度': isMember ? S(pickAttitude(m.attitude), '未记录') : '—',
        'ADUR 角色': isMember ? S(m.adur_role) : '—',
        'P3P4 标记': isMember ? jsonText(m.p3p4_flags, '') : '',
        导师: isMember && N(m.is_mentor) === 1 ? '是' : '',
        项目备注: isMember ? S(m.remark, '') : '',
      }),
      raw: p,
    })
  }

  /* 4. 我方（奇安信）派出人员 */
  const orgById = new Map(orgs.map((o) => [o.id, o]))
  const staffIds = new Set(staff.map((s) => s.id))
  if (staff.length) {
    // 只把「派出人员所属组织」及其上级串进来，避免把整棵几千人的组织树画上去
    const needOrgs = new Set()
    for (const s of staff) {
      let cur = orgById.get(s.org_id)
      let guard = 0
      while (cur && guard < 12) {
        needOrgs.add(cur.id)
        cur = orgById.get(cur.parent_id)
        guard += 1
      }
    }
    // 组织节点先从顶层铺下来，保证父存在于子之前
    const sortedOrgs = orgs
      .filter((o) => needOrgs.has(o.id))
      .sort((a, b) => (N(a.level) ?? 0) - (N(b.level) ?? 0))
    for (const o of sortedOrgs) {
      const parentOk = o.parent_id && needOrgs.has(o.parent_id)
      nodes.push({
        id: `org-${o.id}`,
        zone: 'team',
        kind: 'qax',
        table: 'org',
        rowId: o.id,
        parent: parentOk ? `org-${o.parent_id}` : projId,
        source: 'qax',
        label: S(o.name, `组织 #${o.id}`),
        sub: S(o.org_type, '组织'),
        detail: compact({ 组织: S(o.name), 类型: S(o.org_type), 层级: S(o.level), 备注: S(o.remark) }),
        raw: o,
      })
    }
    for (const s of staff) {
      const orgOk = s.org_id && needOrgs.has(s.org_id)
      nodes.push({
        id: `staff-${s.id}`,
        zone: 'team',
        kind: 'person',
        table: 'internal_person',
        rowId: s.id,
        parent: orgOk ? `org-${s.org_id}` : projId,
        source: 'qax',
        label: S(s.name, `我方人员 #${s.id}`),
        sub: [S(s.title, ''), S(s.dept, '')].filter(Boolean).join(' · '),
        attitude: pickAttitude(s.attitude),
        teamRole: s.decision_role === '拍板' ? 'AR' : s.decision_role === '协调' ? 'SR' : s.decision_role === '支撑' ? 'CSR' : 'CDR',
        detail: internalPersonDetail(s, { 本项目派出: '是' }),
        raw: s,
      })
    }
  }

  /* 5. 合作方 / 竞争方 */
  const entById = new Map(entities.map((e) => [e.id, e]))
  const entProjectIds = new Set(entProjects.map((r) => r.coop_entity_id))
  const ppByPerson = new Map(ppProjects.map((r) => [r.partner_person_id, r]))
  const cpByPerson = new Map(cpProjects.map((r) => [r.competitor_person_id, r]))

  function buildSide(type) {
    const personsAll = type === 'partner' ? partnerPersons : competitorPersons
    const relByPerson = type === 'partner' ? ppByPerson : cpByPerson

    // 参与本项目的人员 id（人员级关联表的主键就是人员 id）
    const participantIds = new Set(relByPerson.keys())

    // 关联到本项目的实体：实体级关联 或 至少有一个参与人员
    let list = entities.filter(
      (e) =>
        (e.entity_type === type || e.entity_type === 'both') &&
        (entProjectIds.has(e.id) ||
          personsAll.some((p) => p.coop_entity_id === e.id && participantIds.has(p.id))),
    )
    let fallback = false
    if (!list.length) {
      // 本项目还没有任何关联记录时，退化为展示全部同类实体，并明确标注
      list = entities.filter((e) => e.entity_type === type || e.entity_type === 'both')
      fallback = true
    }

    const sidesOut = []
    for (const e of list) {
      const nodeId = `ent-${type}-${e.id}`
      const kids = []
      for (const p of personsAll) {
        if (p.coop_entity_id !== e.id) continue
        const rel = relByPerson.get(p.id)
        // 实体已按项目筛选，人员仍全部列出，但标注是否参与本项目
        const participates = !!rel
        kids.push({
          id: `${type === 'partner' ? 'pp' : 'cp'}-${p.id}`,
          zone: type === 'partner' ? 'left' : 'right',
          kind: 'person',
          table: type === 'partner' ? 'partner_person' : 'competitor_person',
          rowId: p.id,
          parent: nodeId,
          source: type,
          label: S(p.name, `人员 #${p.id}`),
          sub: [S(p.title, ''), S(p.role_in_company, '')].filter(Boolean).join(' · '),
          attitude: pickAttitude(rel?.attitude ?? p.attitude),
          projectRelId: rel?.id ?? null,
          detail: coopPersonDetail(p, {
            '本项目参与': participates ? '是' : '否',
            '本项目角色': participates ? S(rel.role, '') : '',
            '对本项目态度': participates ? S(pickAttitude(rel.attitude), '未记录') : '—',
            关系性质: participates ? S(rel.relation_nature, '') : '',
            关系温度: participates ? S(rel.relation_status, '') : '',
            对方战术: participates ? S(rel.their_tactic, '') : '',
            我方策略: participates ? S(rel.our_strategy, '') : '',
            分歧点: participates ? S(rel.diff_points, '') : '',
            结果: participates ? S(rel.result, '') : '',
          }),
          raw: p,
        })
      }

      nodes.push({
        id: nodeId,
        zone: type === 'partner' ? 'left' : 'right',
        kind: type,
        table: 'coop_entity',
        rowId: e.id,
        parent: projId,
        source: type,
        label: S(e.name, `实体 #${e.id}`),
        sub: [S(e.company_type, ''), S(e.overall_relation, '')].filter(Boolean).join(' · '),
        detail: entityDetail(e),
        raw: e,
      })
      nodes.push(...kids)

      sidesOut.push({
        entity: e,
        nodeId,
        detail: entityDetail(e),
        participates: !fallback,
        persons: kids.map((k) => ({
          id: k.rowId, // 人员行 id，用于「跳转编辑页面」
          nodeId: k.id,
          name: k.label,
          title: S(k.raw?.title, ''),
          role: S(k.raw?.role_in_company, ''),
          decisionPower: S(k.raw?.decision_power, ''),
          attitude: k.attitude,
          participates: k.detail['本项目参与'] === '是',
          detail: k.detail,
        })),
        projectRel: entProjects.find((r) => r.coop_entity_id === e.id) ?? null,
      })
    }
    return { list: sidesOut, fallback }
  }

  const partnerSide = buildSide('partner')
  const competitorSide = buildSide('competitor')

  /* 6. C74111 —— 作为独立节点挂在网络之外（diag 分区） */
  let diagNode = null
  if (c74111) {
    diagNode = toDiagNode({ id: project.id, competition: project.competition, strategy: project.strategy }, c74111)
    if (diagNode) {
      // 补上分数与分档，供画布直接上色、显示在圆形节点里
      diagNode.score = Number(c74111.total_score) || 0
      diagNode.tier = tierOf(diagNode.score)
      diagNode.commitment = S(c74111.commitment, TIER_META[diagNode.tier].hint)
      nodes.push(diagNode)
    }
  }

  return {
    nodes,
    diagNode,
    sides: { partners: partnerSide.list, competitors: competitorSide.list },
    fallback: { partners: partnerSide.fallback, competitors: competitorSide.fallback },
    meta: {
      kind: 'project',
      customer,
      project,
      memberCount: memberIds.size,
      staffCount: staff.length,
      c74111,
    },
  }
}

/* ---------------------------------------------------------------------------
   项目详情（弹窗 / 面板用）
   ------------------------------------------------------------------------ */

export function projectDetail(p, c74111) {
  const score = Number(c74111?.total_score)
  return compact({
    项目名称: S(p.name),
    项目编号: `#${p.id}`,
    所属客户: S(p.customer_name, ''),
    阶段: pickStage(p),
    级别: S(p.level),
    子类型: S(p.sub_type),
    项目模式: S(p.project_mode),
    招标方式: S(p.tender_mode),
    预算: p.budget !== null && p.budget !== undefined ? `${p.budget} 万元` : '',
    安全预算: p.sec_budget !== null && p.sec_budget !== undefined ? `${p.sec_budget} 万元` : '',
    预计合同额: p.amount !== null && p.amount !== undefined ? `${p.amount} 万元` : '',
    趋赢力: Number.isFinite(score) ? `${score} 分` : S(p.win_score, ''),
    竞争态势: S(c74111?.competition || p.competition),
    可否承诺: S(c74111?.commitment || p.commitment),
    采购周期: S(p.purchase_cycle),
    风险等级: S(p.risk_level),
    风险: S(p.risk),
    背景: S(p.background),
    下一步计划: S(p.next_plan),
    策略: S(c74111?.strategy || p.strategy),
    行动计划: S(c74111?.action_plan, ''),
    SWOT: S(p.swot),
    备注: S(p.other_note),
  })
}

export { S as str, N as num, jsonText, compact }
