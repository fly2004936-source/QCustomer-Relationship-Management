/**
 * 数据源：mock / 实时 双通道
 *
 * 设计要点：
 *  · 两条通道产出**完全相同的形状**，组件层无需关心数据来自哪里
 *  · 切到实时遇到任何错误（连不上 / 超时 / 后端报错）都自动回落到 mock 并保留错误信息，
 *    保证 11 套风格演示在任何环境都能看，不会因为空库或后端没启动而白屏
 *  · 记录每张表的实际行数，便于一眼区分「接上了但没数据」和「没接上」
 */

import { computed, ref, shallowRef } from 'vue'
import * as mock from '@/data/mock'
import { api, getBaseUrl, setBaseUrl } from '@/api/client'
import { buildDataset } from '@/api/adapters'

/** mock 通道：从静态数据派生出与实时通道一致的形状 */
function deriveMock() {
  const nodes = mock.GRAPH_NODES
  const personCount = nodes.filter((n) => n.kind === 'person').length
  const diag = nodes.find((n) => n.kind === 'c74111')
  const diagScore = diag ? parseInt(String(diag.sub).match(/\d+/)?.[0] ?? '0', 10) : 0

  return {
    kpis: mock.KPIS,
    projects: mock.PROJECTS,
    graphNodes: nodes,
    heroStats: [
      { k: '决策链', v: personCount, u: '人' },
      { k: '节点', v: nodes.length, u: '个' },
      { k: 'C74111', v: diagScore, u: '分' },
    ],
    personOptions: nodes.filter((n) => n.kind === 'person').map((n) => n.label),
  }
}

export const MOCK_DATASET = deriveMock()

/** 实时通道要拉的表：key → [表名, 查询参数] */
const TABLES = [
  ['customers', 'customer'],
  ['projects', 'project'],
  ['persons', 'person'],
  ['members', 'project_member'],
  ['coopEntities', 'coop_entity'],
  ['partnerPersons', 'partner_person'],
  ['competitorPersons', 'competitor_person'],
  ['internalPersons', 'internal_person'],
  ['orgs', 'org'],
  ['c74111Rows', 'project_c74111'],
]

export function useDataSource() {
  const mode = ref('mock') // mock | live
  const status = ref('idle') // idle | loading | ok | error
  const error = ref(null)
  const baseUrl = ref(getBaseUrl())
  const liveData = shallowRef(null)
  const tableCounts = ref(null)
  const backend = ref({ state: 'unknown', dbPath: '', tableCount: 0, message: '' })

  const dataset = computed(() => (mode.value === 'live' && liveData.value ? liveData.value : MOCK_DATASET))
  const isLive = computed(() => mode.value === 'live' && !!liveData.value)
  const loading = computed(() => status.value === 'loading')

  /** 探测后端是否在线（不改变数据源） */
  async function probe() {
    try {
      const h = await api.health()
      backend.value = {
        state: 'online',
        dbPath: h?.db_path ?? '',
        tableCount: h?.table_count ?? 0,
        message: '',
      }
      return true
    } catch (e) {
      backend.value = { state: 'offline', dbPath: '', tableCount: 0, message: e.message }
      return false
    }
  }

  /** 拉取全量真实数据并装配成 dataset */
  async function loadLive() {
    status.value = 'loading'
    error.value = null
    try {
      const results = await Promise.all(TABLES.map(([, table]) => api.list(table, { page_size: 200 })))
      const raw = {}
      const counts = {}
      results.forEach((d, i) => {
        const [key, table] = TABLES[i]
        const items = Array.isArray(d) ? d : (d?.items ?? [])
        raw[key] = items
        counts[table] = items.length
      })
      tableCounts.value = counts

      const c74111Map = new Map((raw.c74111Rows || []).map((r) => [r.project_id, r]))
      liveData.value = buildDataset({
        customers: raw.customers,
        projects: raw.projects,
        persons: raw.persons,
        members: raw.members,
        coopEntities: raw.coopEntities,
        partnerPersons: raw.partnerPersons,
        competitorPersons: raw.competitorPersons,
        internalPersons: raw.internalPersons,
        orgs: raw.orgs,
        c74111Map,
      })

      mode.value = 'live'
      status.value = 'ok'
      return true
    } catch (e) {
      // 失败即回落，宁可显示演示数据也不要白屏
      status.value = 'error'
      error.value = e
      mode.value = 'mock'
      return false
    }
  }

  async function setMode(next) {
    if (next === 'live') {
      if (!backend.value.state || backend.value.state === 'unknown') await probe()
      return loadLive()
    }
    mode.value = 'mock'
    status.value = 'idle'
    error.value = null
    return true
  }

  function setBase(next) {
    setBaseUrl(next)
    baseUrl.value = getBaseUrl()
  }

  return {
    mode,
    status,
    error,
    loading,
    dataset,
    isLive,
    backend,
    tableCounts,
    baseUrl,
    probe,
    loadLive,
    setMode,
    setBase,
  }
}
