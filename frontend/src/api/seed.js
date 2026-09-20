/**
 * 工作台初始内容
 *
 * 为什么需要它：工作台的 11 个功能里，有 4 个（资讯 / 客户更新 / 竞品动态 / 快捷入口）
 * 依赖数据库里已有内容。空库时这些卡片会全空，看不出效果也验证不了布局，
 * 所以这里在**表为空时**补一批种子数据，并且：
 *   · 资讯 source 一律写 "演示数据"，界面上能一眼看出不是真实抓取
 *   · 客户更新 / 竞品动态会去读真实的 customer / coop_entity id，
 *     保证外键指向存在的行，不产生脏数据
 *   · 只补一次：已有数据就完全不动
 *
 * 依赖真实 id 的部分（客户更新、竞品动态）在 ensureSeed 里延迟执行。
 */

import { api, fetchAll } from './client'
import { NEWS_CATEGORY, NEWS_TAG, dateKey } from './workbench'

const SRC = '演示数据'

/** 资讯：标题写成「行业议题」而非伪造的具体新闻事件，避免被误当成真实报道 */
const NEWS = [
  // —— 网络安全（每日网络安全行业新闻）——
  { c: NEWS_CATEGORY.incident, t: '漏洞治理从"打补丁"转向"资产暴露面收敛"的实践路径', s: '围绕资产测绘、优先级排序与闭环验证，讨论如何把有限人力投到真正可达的攻击路径上。', tag: NEWS_TAG.sec },
  { c: NEWS_CATEGORY.incident, t: '勒索攻击的新趋势：从加密数据转向窃取与勒索并举', s: '数据窃取后再加密的双重勒索已成主流，备份可用性与响应演练成为关键防线。', tag: NEWS_TAG.sec },
  { c: NEWS_CATEGORY.policy, t: '关键信息基础设施安全保护要求落地要点梳理', s: '围绕责任分工、检测评估频次、事件报告时限，梳理运营者的合规动作清单。', tag: NEWS_TAG.sec },
  { c: NEWS_CATEGORY.industry, t: '安全运营中心（SOC）建设：自建、共建与托管三种模式的取舍', s: '从人力成本、告警质量、响应时效三个维度比较模式差异与适用场景。', tag: NEWS_TAG.sec },
  { c: NEWS_CATEGORY.incident, t: '供应链安全：第三方组件引入环节的风险管控清单', s: '覆盖准入评估、SBOM 管理、漏洞跟踪与退出机制四个阶段。', tag: NEWS_TAG.sec },
  { c: NEWS_CATEGORY.industry, t: '零信任架构在大型央国企落地：常见误区与分阶段实施建议', s: '强调"先身份、再设备、后应用"的推进顺序，避免一次性改造带来的业务中断。', tag: NEWS_TAG.sec },

  // —— AI 动向 ——
  { c: NEWS_CATEGORY.ai, t: '大模型安全评测：从内容合规到智能体权限管控', s: '随着智能体具备工具调用能力，权限边界与操作审计成为新的评测重点。', tag: NEWS_TAG.ai },
  { c: NEWS_CATEGORY.ai, t: '企业级 AI 落地的数据治理前置条件', s: '数据分级分类、脱敏与留痕是模型可用性与合规性的共同前提。', tag: NEWS_TAG.ai },
  { c: NEWS_CATEGORY.ai, t: '提示注入（Prompt Injection）防护的工程化思路', s: '从输入隔离、工具白名单、输出校验三层构建纵深防御。', tag: NEWS_TAG.ai },
  { c: NEWS_CATEGORY.ai, t: 'AI 编码助手在研发流程中的引入位置与评审要点', s: '讨论代码生成占比上升后，评审、测试与责任归属如何调整。', tag: NEWS_TAG.ai },

  // —— 金融 / 科技圈 ——
  { c: NEWS_CATEGORY.industry, t: '数据要素市场建设对行业数字化投入的带动效应', s: '数据确权、流通与定价机制逐步清晰，带动底层基础设施与安全投入。', tag: NEWS_TAG.fintech },
  { c: NEWS_CATEGORY.industry, t: '算力基础设施投资节奏与安全配套的匹配问题', s: '算力扩张同时，网络分区、算力调度安全与能耗管理需要同步规划。', tag: NEWS_TAG.fintech },
  { c: NEWS_CATEGORY.policy, t: '数据出境合规评估流程中的常见补正事项', s: '梳理申报材料高频问题，减少因材料不全导致的往复。', tag: NEWS_TAG.fintech },
  { c: NEWS_CATEGORY.industry, t: '科技企业研发投入结构变化：安全与合规支出占比上升', s: '合规成本正在从"项目性支出"转为"经常性支出"。', tag: NEWS_TAG.fintech },
]

/** 默认快捷入口（真实可访问的站点，不留死链） */
const SHORTCUTS = [
  { title: '中国政府采购网', url: 'http://www.ccgp.gov.cn', icon: '', sort_order: 10 },
  { title: '安全客', url: 'https://www.anquanke.com', icon: '', sort_order: 20 },
  { title: 'CVE 漏洞库', url: 'https://www.cve.org', icon: '', sort_order: 30 },
  { title: 'GitHub', url: 'https://github.com', icon: '', sort_order: 40 },
  { title: '腾讯文档', url: 'https://docs.qq.com', icon: '', sort_order: 50 },
  { title: 'SQLite 文档', url: 'https://www.sqlite.org/docs.html', icon: '', sort_order: 60 },
]

/** 客户信息更新的素材，按客户顺序循环取用 */
const CUSTOMER_FEED = [
  { t: '年度安全服务框架完成新一轮续签', s: '框架协议周期与金额保持稳定，服务范围新增攻防演练支持。', rel: '存量续签' },
  { t: '安全运营中心二期立项进入预算评审', s: '信息化部门已完成需求汇总，进入预算评审环节。', rel: '在建项目' },
  { t: '上级集团下发网络安全专项检查通知', s: '要求下属单位在规定时间内完成自查并上报整改台账。', rel: '合规驱动' },
  { t: '信息中心组织架构调整，新增数据安全岗', s: '数据安全职责从运维组独立，决策链出现新的关键人。', rel: '组织变化' },
  { t: '启动国产化替代试点，范围覆盖终端与网络设备', s: '试点先行，后续视效果决定推广节奏。', rel: '机会点' },
  { t: '年度护网行动总结会确认下一年度投入方向', s: '确认在检测响应与威胁情报方向继续加大投入。', rel: '预算线索' },
]

/** 竞品动向素材 */
const MOVE_FEED = [
  { role: '技术交流', quote: '—', result: '推进中', review: '以产品演示切入，强调与现有平台的兼容性。' },
  { role: '方案汇报', quote: '让价 8%', result: '未定', review: '用整体方案报价施压，聚焦交付周期承诺。' },
  { role: '高层拜访', quote: '—', result: '推进中', review: '通过上级集团层面推动，试图影响立项节奏。' },
  { role: '投标', quote: '让价 12%', result: '我方领先', review: '价格明显低于常规区间，需关注其交付能力与后续服务。' },
]

async function isEmpty(table, params = {}) {
  try {
    const rows = await fetchAll(table, { page_size: 1, ...params })
    return rows.length === 0
  } catch {
    return false
  }
}

/**
 * 空表补种。任何一步失败都不抛出——工作台应该照常可用，
 * 只是对应卡片显示为空。
 * @returns {Promise<{seeded: string[], skipped: string[], failed: string[]}>}
 */
export async function ensureWorkbenchSeed() {
  const seeded = []
  const skipped = []
  const failed = []

  const run = async (name, fn) => {
    try {
      const ok = await fn()
      ;(ok ? seeded : skipped).push(name)
    } catch {
      failed.push(name)
    }
  }

  /* 快捷入口 */
  await run('dashboard_shortcut', async () => {
    if (!(await isEmpty('dashboard_shortcut'))) return false
    for (const s of SHORTCUTS) await api.create('dashboard_shortcut', s)
    return true
  })

  /* 资讯 */
  await run('news_item', async () => {
    if (!(await isEmpty('news_item'))) return false
    const base = new Date()
    let i = 0
    for (const n of NEWS) {
      // 让条目散布在最近 5 天，列表看起来有先后
      const d = new Date(base)
      d.setDate(base.getDate() - Math.floor(i / 3))
      await api.create('news_item', {
        category: n.c,
        title: n.t,
        summary: n.s,
        source: SRC,
        news_date: dateKey(d),
        tags: n.tag,
      })
      i += 1
    }
    return true
  })

  /* 客户信息更新：必须绑定真实 customer.id */
  await run('customer_news', async () => {
    if (!(await isEmpty('customer_news'))) return false
    const customers = await fetchAll('customer', { page_size: 50, order_by: 'id:asc' })
    if (!customers.length) return false
    const base = new Date()
    let i = 0
    for (const c of customers) {
      for (let k = 0; k < 2; k += 1) {
        const f = CUSTOMER_FEED[i % CUSTOMER_FEED.length]
        const d = new Date(base)
        d.setDate(base.getDate() - k)
        await api.create('customer_news', {
          customer_id: c.id,
          news_title: f.t,
          summary: f.s,
          source: SRC,
          news_date: dateKey(d),
          our_relation: f.rel,
          is_new: k === 0 ? 1 : 0,
          batch_time: dateKey(base),
        })
        i += 1
      }
    }
    return true
  })

  /* 竞品动向：必须绑定真实 coop_entity.id（entity_type='competitor'） */
  await run('entity_history', async () => {
    if (!(await isEmpty('entity_history'))) return false
    const ents = await fetchAll('coop_entity', { page_size: 50 })
    const competitors = ents.filter((e) => e.entity_type === 'competitor' || e.entity_type === 'both')
    if (!competitors.length) return false
    const base = new Date()
    let i = 0
    for (const e of competitors) {
      for (let k = 0; k < 2; k += 1) {
        const m = MOVE_FEED[i % MOVE_FEED.length]
        const d = new Date(base)
        d.setDate(base.getDate() - k * 3)
        await api.create('entity_history', {
          entity_type: 'competitor',
          entity_id: e.id,
          happen_time: dateKey(d),
          role: m.role,
          quote_discount: m.quote === '—' ? null : m.quote,
          result: m.result,
          review: m.review,
        })
        i += 1
      }
    }
    return true
  })

  /* 竞争对抗记录（竞争策略与动作） */
  await run('entsales_confrontation', async () => {
    if (!(await isEmpty('entsales_confrontation'))) return false
    const ents = await fetchAll('coop_entity', { page_size: 50 })
    const competitors = ents.filter((e) => e.entity_type === 'competitor' || e.entity_type === 'both')
    if (!competitors.length) return false
    const projects = await fetchAll('project', { page_size: 50 })
    const base = new Date()
    let i = 0
    for (const e of competitors) {
      const p = projects[i % Math.max(projects.length, 1)]
      const m = MOVE_FEED[(i + 2) % MOVE_FEED.length]
      const d = new Date(base)
      d.setDate(base.getDate() - i * 2)
      await api.create('entsales_confrontation', {
        entity_type: 'competitor',
        entity_id: e.id,
        customer_name: p?.customer_name ?? null,
        project_name: p?.name ?? null,
        happen_time: dateKey(d),
        their_mode: m.role,
        their_quote: m.quote === '—' ? null : m.quote,
        our_quote: null,
        result: m.result,
        key_reason: '以产品能力与本地服务响应速度形成差异',
        review: m.review,
      })
      i += 1
    }
    return true
  })

  /* 信息更新批次 */
  await run('dashboard_update_log', async () => {
    if (!(await isEmpty('dashboard_update_log'))) return false
    const p = (n) => String(n).padStart(2, '0')
    const now = new Date()
    await api.create('dashboard_update_log', {
      update_date: dateKey(now),
      update_time: `${p(now.getHours())}:${p(now.getMinutes())}`,
      last_update_time: `${dateKey(now)} ${p(now.getHours())}:${p(now.getMinutes())}:00`,
      operator: '系统初始化',
      coverage: '演示内容已就绪',
    })
    return true
  })

  return { seeded, skipped, failed }
}
