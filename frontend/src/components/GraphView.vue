<template>
  <!--
    组织网络图
    两级导航：
      客户级 —— 选客户 → 展示该客户的人员组织架构网络；右侧可收起侧栏列出该客户的项目
                （项目名称 + ID + C74111 分档配色，25 分一档：绿 / 蓝 / 黄 / 红）
      项目级 —— 点项目进入作战视图；客户节点仍在，点它即可「返回上一步」
  -->
  <div class="app-inner">
    <div class="graph-bar">
      <select v-model="customerId" class="select graph-select" @change="onCustomerChange">
        <option value="">选择客户…</option>
        <option v-for="c in customers" :key="c.id" :value="c.id">
          {{ c.name }}（{{ c.category || '—' }}）
        </option>
      </select>

      <!-- 面包屑：客户 > 项目 -->
      <div class="row" style="gap: 6px; flex-wrap: wrap">
        <button
          class="btn btn--sm"
          :class="level === 'project' ? 'btn--ghost' : 'btn--soft'"
          @click="backToCustomer"
        >
          <AppIcon name="building" :size="13" />
          {{ customerName || '未选客户' }}
        </button>
        <template v-if="level === 'project'">
          <AppIcon name="chevronRight" :size="13" />
          <span class="tag tag--ac">{{ currentProject?.name }}</span>
        </template>
      </div>

      <div class="row" style="gap: 6px; margin-left: auto">
        <button v-if="level === 'project'" class="btn btn--sm" @click="backToCustomer">
          <AppIcon name="chevronLeft" :size="13" />
          返回上一步
        </button>
        <button class="btn btn--sm btn--ghost" title="全部展开" @click="graphRef?.expandAll()">
          <AppIcon name="expand" :size="13" />
        </button>
        <button class="btn btn--sm btn--ghost" title="全部收起" @click="graphRef?.collapseAll()">
          <AppIcon name="collapse" :size="13" />
        </button>
        <button class="btn btn--sm btn--ghost" title="适配视图" @click="graphRef?.fit()">
          <AppIcon name="fit" :size="13" />
        </button>
        <button
          class="btn btn--sm"
          :class="sideOpen ? 'btn--soft' : 'btn--ghost'"
          :title="sideOpen ? '收起侧边栏' : '展开侧边栏'"
          @click="toggleSide"
        >
          <AppIcon name="panel" :size="13" />
          {{ sideOpen ? '收起侧栏' : '展开侧栏' }}
        </button>
      </div>
    </div>

    <div v-if="loading" class="banner" style="margin: 10px 0 0">正在从数据库加载…</div>
    <div v-else-if="error" class="banner banner--err" style="margin: 10px 0 0">{{ error }}</div>
    <div v-else-if="!customerId" class="banner" style="margin: 10px 0 0">
      <AppIcon name="search" :size="14" />
      先在上方选择一个客户，网络图会展示该客户的人员组织架构。
    </div>

    <div class="graph-body" style="margin-top: 12px">
      <div class="graph-canvas">
        <OrgGraph
          ref="graphRef"
          :nodes="nodes"
          :height="graphHeight"
          @select="onNodeSelect"
        />

        <!-- 客户信息：没有更合适的位置，就用可移动浮窗 -->
        <DraggablePanel
          v-if="custPanel"
          :title="custPanel.title"
          :subtitle="custPanel.sub"
          :start-x="14"
          :start-y="14"
          @close="custPanel = null"
        >
          <dl class="kv">
            <template v-for="(v, k) in custPanel.pairs" :key="k">
              <dt>{{ k }}</dt>
              <dd>{{ v }}</dd>
            </template>
          </dl>
          <div class="hairline"></div>
          <div class="row" style="gap: 8px; flex-wrap: wrap">
            <button
              class="btn btn--sm btn--soft"
              @click="emit('edit-table', { table: 'customer', id: custPanel.id })"
            >
              <AppIcon name="edit" :size="13" />
              编辑客户
            </button>
            <span class="card-sub">可拖动标题栏移动 · 双击标题栏复位</span>
          </div>
        </DraggablePanel>

        <!-- 项目信息 -->
        <DraggablePanel
          v-if="projPanel"
          :title="projPanel.title"
          :subtitle="projPanel.sub"
          wide
          :start-x="40"
          :start-y="40"
          @close="projPanel = null"
        >
          <dl class="kv">
            <template v-for="(v, k) in projPanel.pairs" :key="k">
              <dt>{{ k }}</dt>
              <dd>{{ v }}</dd>
            </template>
          </dl>
          <div class="hairline"></div>
          <div class="row" style="gap: 8px; flex-wrap: wrap">
            <button class="btn btn--sm btn--primary" @click="enterProject(projPanel.projectId)">
              <AppIcon name="network" :size="13" />
              进入项目作战视图
            </button>
            <button
              class="btn btn--sm btn--ghost"
              @click="emit('edit-table', { table: 'project', id: projPanel.projectId })"
            >
              <AppIcon name="edit" :size="13" />
              编辑项目
            </button>
          </div>
        </DraggablePanel>

        <!-- 节点详情（人员 / 实体 / 组织 / 诊断） -->
        <DraggablePanel
          v-if="nodePanel"
          :title="nodePanel.title"
          :subtitle="nodePanel.sub"
          :start-x="380"
          :start-y="14"
          @close="nodePanel = null"
        >
          <div class="wrap" style="margin-bottom: 10px">
            <span v-for="t in nodePanel.tags" :key="t.text" class="tag" :class="t.cls">{{ t.text }}</span>
          </div>

          <dl class="kv">
            <template v-for="(v, k) in nodePanel.pairs" :key="k">
              <dt>{{ k }}</dt>
              <dd>{{ v }}</dd>
            </template>
          </dl>

          <!-- C74111 节点：直接给分档明细 + 操作入口 -->
          <template v-if="nodePanel.diagRows?.length">
            <div class="sect">7C + 4P + Key</div>
            <div v-for="row in nodePanel.diagRows" :key="row.code" class="c74-edit" style="grid-template-columns: 40px minmax(0, 1fr) 62px">
              <span class="c74-code">{{ row.code }}</span>
              <span class="c74-edit-name">{{ row.name }}</span>
              <span class="c74-edit-max">{{ row.score }}/{{ row.max }}</span>
            </div>
            <div class="hairline"></div>
            <button class="btn btn--sm btn--primary" @click="openC74111()">
              <AppIcon name="target" :size="13" />
              操作 C74111 模块
            </button>
          </template>

          <div v-if="nodePanel.edit" class="hairline"></div>
          <div v-if="nodePanel.edit" class="row" style="gap: 8px; flex-wrap: wrap">
            <button class="btn btn--sm btn--soft" @click="emit('edit-table', nodePanel.edit)">
              <AppIcon name="edit" :size="13" />
              跳转编辑页面
            </button>
            <span class="card-sub">{{ nodePanel.edit.table }} #{{ nodePanel.edit.id }}</span>
          </div>
        </DraggablePanel>

        <!-- C74111 操作面板 -->
        <C74111Panel
          v-if="c74Open && currentProject"
          :project-id="currentProject.id"
          :project-name="currentProject.name"
          :row="c74111Row"
          @close="c74Open = false"
          @saved="reloadC74111"
        />
      </div>

      <!-- 可收起侧边栏 -->
      <aside class="graph-side" :class="{ 'graph-side--off': !sideOpen }">
        <template v-if="level === 'customer'">
          <div class="graph-side-head">
            <AppIcon name="briefcase" :size="14" />
            <span class="graph-side-title">该客户的项目</span>
            <span class="tag" style="margin-left: auto">{{ myProjects.length }}</span>
          </div>

          <div class="legend-row" style="margin-bottom: 10px">
            <span v-for="t in TIERS" :key="t.key" class="legend-chip">
              <span class="c74" :class="t.cls">{{ t.range }}</span>{{ t.label }}
            </span>
          </div>

          <button
            v-for="p in myProjects"
            :key="p.id"
            class="pj"
            :title="`${p.name}｜点击查看项目信息`"
            @click="showProject(p)"
          >
            <div class="pj-top">
              <span class="pj-name">{{ p.name }}</span>
              <span class="pj-id">#{{ p.id }}</span>
            </div>
            <div class="card-sub">
              {{ p.level || '—' }} · {{ p.stage }} · {{ fmtMoney(p.budget) }}
            </div>
            <div class="pj-meta">
              <span class="c74" :class="p.tier.cls">
                {{ p.tier.known ? `${p.tier.score} 分 · ${p.tier.label}` : '未诊断' }}
              </span>
              <span v-if="p.tier.known" class="c74-bar" :class="p.tier.cls">
                <i :style="{ width: Math.min(100, p.tier.score) + '%' }"></i>
              </span>
            </div>
          </button>

          <div v-if="!myProjects.length && customerId" class="empty">
            该客户下还没有项目。到「数据管理 → 项目 → 项目主档」新增一条即可。
          </div>
          <div v-if="!customerId" class="empty">未选择客户</div>
        </template>

        <!-- 项目级：概览 -->
        <template v-else>
          <div class="graph-side-head">
            <AppIcon name="target" :size="14" />
            <span class="graph-side-title">{{ currentProject?.name }}</span>
          </div>
          <button class="btn btn--sm btn--ghost" style="width: 100%; margin-bottom: 10px" @click="backToCustomer">
            <AppIcon name="chevronLeft" :size="13" />
            返回客户视图
          </button>
          <button class="btn btn--sm btn--soft" style="width: 100%; margin-bottom: 10px" @click="openProjectPanel">
            <AppIcon name="move" :size="13" />
            项目完整信息（可拖动浮窗）
          </button>

          <dl class="kv">
            <dt>项目编号</dt>
            <dd class="mono">#{{ currentProject?.id }}</dd>
            <dt>所属客户</dt>
            <dd>{{ customerName }}</dd>
            <dt>阶段 / 级别</dt>
            <dd>{{ currentProject?.stage }} · {{ currentProject?.level || '—' }}</dd>
            <dt>相关人员</dt>
            <dd>{{ board?.members.length ?? 0 }} 人</dd>
            <dt>我方派出</dt>
            <dd>{{ board?.staff.length ?? 0 }} 人</dd>
            <dt>合作方</dt>
            <dd>{{ board?.partners.length ?? 0 }} 家</dd>
            <dt>竞争方</dt>
            <dd>{{ board?.competitors.length ?? 0 }} 家</dd>
          </dl>

          <div class="hairline"></div>
          <div class="legend-row">
            <span class="legend-chip">节点色</span>
            <span class="legend-chip"><span class="dot" style="background: var(--n-customer)"></span>客户</span>
            <span class="legend-chip"><span class="dot" style="background: var(--n-project)"></span>项目</span>
            <span class="legend-chip"><span class="dot" style="background: var(--n-partner)"></span>合作方</span>
            <span class="legend-chip"><span class="dot" style="background: var(--n-competitor)"></span>竞争方</span>
            <span class="legend-chip"><span class="dot" style="background: var(--n-qax)"></span>我方</span>
          </div>
          <div class="legend-row" style="margin-top: 8px">
            <span class="legend-chip">ADUR</span>
            <span v-for="a in ADUR_LIST" :key="a.k" class="legend-chip">
              <span class="adur" :class="`adur--${a.k}`">{{ a.k }}</span>
            </span>
          </div>
          <div class="legend-row" style="margin-top: 8px">
            <span class="legend-chip">态度</span>
            <span v-for="a in ATTITUDE_LIST" :key="a.v" class="legend-chip">
              <span class="att" :class="a.cls">{{ a.char }}</span>
            </span>
          </div>
        </template>
      </aside>
    </div>

    <!-- 项目作战板（四条要求） -->
    <ProjectBoard
      v-if="level === 'project' && board"
      :data="board"
      @open-person="openPerson"
      @open-staff="openStaff"
      @open-coop-person="openCoopPerson"
      @open-c74111="openC74111"
      @edit-table="(t) => emit('edit-table', t)"
    />
  </div>
</template>

<script setup>
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import AppIcon from './AppIcon.vue'
import OrgGraph from './OrgGraph.vue'
import DraggablePanel from './DraggablePanel.vue'
import ProjectBoard from './ProjectBoard.vue'
import C74111Panel from './C74111Panel.vue'
import { fetchAll } from '@/api/client'
import {
  ATTITUDE_LIST,
  ADUR_LIST,
  TIER_META,
  attChar,
  entityDetail,
  buildCustomerGraph,
  buildProjectGraph,
  internalPersonDetail,
  projectDetail,
  tierInfo,
} from '@/api/graphData'
import { pickStage } from '@/api/adapters'

const props = defineProps({
  online: { type: Boolean, default: true },
  /** 外部要求聚焦的客户（从工作台「客户信息更新」跳进来时用） */
  focusCustomerId: { type: [Number, String], default: null },
})

const emit = defineEmits(['edit-table', 'focus-entity'])

const TIERS = [TIER_META.a, TIER_META.b, TIER_META.c, TIER_META.d]

/* ---------------- 状态 ---------------- */
const graphRef = ref(null)
const loading = ref(false)
const error = ref('')
const sideOpen = ref(true)

const customers = ref([])
const customerId = ref('')
const level = ref('customer')
const currentProject = ref(null)

// 客户级
const custPersons = ref([])
const myProjects = ref([])
/** project_id → project_c74111 行 */
const c74111Map = ref(new Map())

// 项目级
const members = ref([])
const visits = ref([])
const visitStaff = ref([])
const visitsOfProject = ref([])
const entProjects = ref([])
const ppProjects = ref([])
const cpProjects = ref([])
const entityMoves = ref([])
const confrontations = ref([])
const c74111Row = ref(null)

// 全局（一次加载，反复使用）
const entities = ref([])
const partnerPersons = ref([])
const competitorPersons = ref([])
const orgs = ref([])
const internalPersons = ref([])

// 浮层
const custPanel = ref(null)
const projPanel = ref(null)
const nodePanel = ref(null)
const c74Open = ref(false)

const nodes = ref([])
const board = ref(null)

const windowH = ref(typeof window === 'undefined' ? 900 : window.innerHeight)
const graphHeight = computed(() => Math.max(430, Math.min(760, windowH.value - 330)))

const customerName = computed(
  () => customers.value.find((c) => String(c.id) === String(customerId.value))?.name ?? '',
)
const currentCustomer = computed(
  () => customers.value.find((c) => String(c.id) === String(customerId.value)) ?? null,
)

/* ---------------- 工具 ---------------- */
async function safe(fn, fallback) {
  try {
    return await fn()
  } catch (e) {
    if (e?.kind === 'offline' || e?.kind === 'timeout') error.value = `连不上后端：${e.message}`
    return fallback
  }
}

const fmtMoney = (v) =>
  v === null || v === undefined || v === '' ? '—' : `${Number(v).toLocaleString('zh-CN')} 万元`

/* ---------------- 加载 ---------------- */

async function loadGlobal() {
  const [cs, es, pp, cp, og, ip] = await Promise.all([
    safe(() => fetchAll('customer', { page_size: 200, order_by: 'id:asc' }), []),
    safe(() => fetchAll('coop_entity', { page_size: 200, order_by: 'id:asc' }), []),
    safe(() => fetchAll('partner_person', { page_size: 200, order_by: 'id:asc' }), []),
    safe(() => fetchAll('competitor_person', { page_size: 200, order_by: 'id:asc' }), []),
    safe(() => fetchAll('org', { page_size: 200, order_by: 'id:asc' }), []),
    safe(() => fetchAll('internal_person', { page_size: 200, order_by: 'id:asc' }), []),
  ])
  customers.value = cs
  entities.value = es
  partnerPersons.value = pp
  competitorPersons.value = cp
  orgs.value = og
  internalPersons.value = ip
  // 让画布配色对得上：外部要求聚焦某客户时直接落位
  if (props.focusCustomerId && cs.some((c) => String(c.id) === String(props.focusCustomerId))) {
    customerId.value = props.focusCustomerId
    await selectCustomer()
  }
}

/** 客户级数据 + 构图 */
async function selectCustomer() {
  if (!customerId.value) {
    nodes.value = []
    myProjects.value = []
    board.value = null
    return
  }
  loading.value = true
  error.value = ''
  try {
    const cid = Number(customerId.value)
    const [persons, projects, c74] = await Promise.all([
      safe(() => fetchAll('person', { filter: { customer_id: cid }, order_by: 'id:asc' }), []),
      safe(() => fetchAll('project', { filter: { customer_id: cid }, order_by: 'id:desc' }), []),
      safe(() => fetchAll('project_c74111', { page_size: 200 }), []),
    ])
    custPersons.value = persons
    c74111Map.value = new Map(c74.map((r) => [r.project_id, r]))
    // 档位在这里算一次，模板里就不用对同一张卡反复重算了。
    // 排序：已诊断的排前面、分高的排前面，其次按 id 倒序——
    // 侧栏是「该客户的项目」清单，把最值得投入的放在最上面比按创建顺序更有用。
    myProjects.value = projects
      .map((p) => ({
        ...p,
        stage: pickStage(p),
        customer_name: customerName.value,
        tier: tierInfo(c74111Map.value.get(p.id)?.total_score, c74111Map.value.get(p.id)?.commitment),
      }))
      .sort((a, b) => {
        if (a.tier.known !== b.tier.known) return a.tier.known ? -1 : 1
        if (a.tier.known && b.tier.known && a.tier.score !== b.tier.score) return b.tier.score - a.tier.score
        return b.id - a.id
      })

    const built = buildCustomerGraph({
      customer: currentCustomer.value,
      persons,
      projects,
      c74111Map: c74111Map.value,
    })
    nodes.value = built.nodes
    level.value = 'customer'
    currentProject.value = null
    board.value = null
    nodePanel.value = null
    projPanel.value = null

    // 选中客户即展示客户信息
    showCustomerPanel()
    setTimeout(() => graphRef.value?.fit(), 60)
  } finally {
    loading.value = false
  }
}

/** 项目级数据 + 构图 + 作战板 */
async function enterProject(projectId) {
  const project = myProjects.value.find((p) => p.id === projectId)
  if (!project) return
  loading.value = true
  error.value = ''
  // 收起客户信息浮窗：进项目后画布上客户节点仍在、点它就能回退，
  // 留着一张客户浮窗只会和项目浮窗叠在一起。
  custPanel.value = null
  try {
    const [mem, vs, entRel, ppRel, cpRel, moves, conf] = await Promise.all([
      safe(() => fetchAll('project_member', { filter: { project_id: projectId } }), []),
      safe(() => fetchAll('visit_record', { filter: { project_id: projectId } }), []),
      safe(() => fetchAll('coop_entity_project_rel', { filter: { project_id: projectId } }), []),
      safe(() => fetchAll('partner_person_project_rel', { filter: { project_id: projectId } }), []),
      safe(() => fetchAll('competitor_person_project_rel', { filter: { project_id: projectId } }), []),
      safe(() => fetchAll('entity_history', { page_size: 200, order_by: 'happen_time:desc' }), []),
      safe(() => fetchAll('entsales_confrontation', { page_size: 200, order_by: 'happen_time:desc' }), []),
    ])
    members.value = mem
    visitsOfProject.value = vs
    entProjects.value = entRel
    ppProjects.value = ppRel
    cpProjects.value = cpRel
    entityMoves.value = moves
    confrontations.value = conf

    // 我方派出人员：visit_record.project_id → visit_our_staff.internal_person_id
    const visitIds = vs.map((v) => v.id)
    const staff =
      visitIds.length
        ? await safe(
            () =>
              fetchAll('visit_our_staff', { page_size: 200 }).then((all) =>
                all.filter((r) => visitIds.includes(r.visit_id)),
              ),
            [],
          )
        : []
    visitStaff.value = staff
    const staffIds = [...new Set(staff.map((r) => r.internal_person_id))]
    const staffRows = internalPersons.value.filter((p) => staffIds.includes(p.id))

    c74111Row.value = c74111Map.value.get(projectId) ?? null
    currentProject.value = project

    const built = buildProjectGraph({
      customer: currentCustomer.value,
      project,
      persons: custPersons.value,
      members: mem,
      staff: staffRows,
      orgs: orgs.value,
      entities: entities.value,
      partnerPersons: partnerPersons.value,
      competitorPersons: competitorPersons.value,
      entProjects: entRel,
      ppProjects: ppRel,
      cpProjects: cpRel,
      c74111: c74111Row.value,
    })
    nodes.value = built.nodes
    level.value = 'project'

    const orgById = new Map(orgs.value.map((o) => [o.id, o]))
    board.value = {
      members: mem
        .map((m) => {
          const p = custPersons.value.find((x) => x.id === m.person_id)
          if (!p) return null
          return {
            id: p.id,
            name: p.name,
            title: p.title,
            dept: p.dept,
            attitude: m.attitude ?? '',
            adur: String(m.adur_role ?? '').trim().toUpperCase().slice(0, 1) || '',
            mentor: Number(m.is_mentor) === 1,
          }
        })
        .filter(Boolean),
      staff: staffRows.map((s) => ({
        id: s.id,
        name: s.name,
        title: s.title,
        dept: s.dept,
        orgName: orgById.get(s.org_id)?.name ?? '',
        decisionRole: s.decision_role ?? '',
        attitude: s.attitude ?? '',
        detail: s,
      })),
      partners: attachEvidence(built.sides.partners),
      competitors: attachEvidence(built.sides.competitors),
      fallback: built.fallback,
      c74111Id: c74111Row.value?.id ?? null,
      tier: tierInfo(c74111Row.value?.total_score, c74111Row.value?.commitment),
      strategy: c74111Row.value?.strategy ?? project.strategy ?? '',
    }

    nodePanel.value = null
    setTimeout(() => graphRef.value?.fit(), 60)
  } finally {
    loading.value = false
  }
}

/** 给每个实体挂上「最近动向」与「竞争对抗」 */
function attachEvidence(list) {
  return (list ?? []).map((ent) => ({
    ...ent,
    moves: entityMoves.value.filter((m) => m.entity_id === ent.entity.id).slice(0, 6),
    confrontations: confrontations.value
      .filter((c) => c.entity_id === ent.entity.id)
      .slice(0, 6),
  }))
}

async function reloadC74111() {
  const c74 = await safe(() => fetchAll('project_c74111', { page_size: 200 }), [])
  c74111Map.value = new Map(c74.map((r) => [r.project_id, r]))
  c74111Row.value = c74111Map.value.get(currentProject.value?.id) ?? null
  if (board.value) {
    board.value = {
      ...board.value,
      c74111Id: c74111Row.value?.id ?? null,
      tier: tierInfo(c74111Row.value?.total_score, c74111Row.value?.commitment),
      strategy: c74111Row.value?.strategy ?? currentProject.value?.strategy ?? '',
    }
  }
  // 画布上的独立诊断节点也要同步刷新
  if (currentProject.value) {
    const built = buildProjectGraph({
      customer: currentCustomer.value,
      project: currentProject.value,
      persons: custPersons.value,
      members: members.value,
      staff: internalPersons.value.filter((p) =>
        visitStaff.value.some((r) => r.internal_person_id === p.id),
      ),
      orgs: orgs.value,
      entities: entities.value,
      partnerPersons: partnerPersons.value,
      competitorPersons: competitorPersons.value,
      entProjects: entProjects.value,
      ppProjects: ppProjects.value,
      cpProjects: cpProjects.value,
      c74111: c74111Row.value,
    })
    nodes.value = built.nodes
  }
}

/* ---------------- 交互 ---------------- */

function onCustomerChange() {
  selectCustomer()
}

function backToCustomer() {
  level.value = 'customer'
  currentProject.value = null
  board.value = null
  c74Open.value = false
  selectCustomer()
}

function showCustomerPanel() {
  const c = currentCustomer.value
  if (!c) return
  custPanel.value = {
    id: c.id,
    title: c.name,
    sub: `${c.category || '—'} · ${c.level || '—'}`,
    pairs: {
      客户编码: c.crm_code ?? '—',
      客户类别: c.category ?? '—',
      客户级别: c.level ?? '—',
      行业: c.industry ?? '—',
      战区: c.army_region ?? '—',
      人员规模: `${custPersons.value.length} 人`,
      在跟项目: `${myProjects.value.length} 个`,
      最高趋赢力: `${myProjects.value.reduce(
        (m, p) => Math.max(m, Number(c74111Map.value.get(p.id)?.total_score) || 0),
        0,
      )} 分`,
      备注: c.remark ?? '—',
    },
  }
}

/**
 * 造一份「项目完整信息浮窗」的描述对象。
 *
 * 抽出来是因为有三个入口都要它：点侧栏项目卡、点画布上的项目节点、点侧栏底部那个
 * 显式按钮。三个入口对「要不要现在弹」的判断各不相同，但内容永远一致，所以只在这里
 * 组装一次，避免三份字段慢慢漂移。
 */
function projectPanelFor(p) {
  if (!p) return null
  const c74 = c74111Map.value.get(p.id)
  const t = tierInfo(c74?.total_score, c74?.commitment)
  return {
    projectId: p.id,
    title: p.name,
    sub: `#${p.id} · ${t.known ? `${t.label} ${t.score} 分` : '未诊断'}`,
    pairs: projectDetail({ ...p, customer_name: customerName.value }, c74),
  }
}

/** 侧栏里的「项目完整信息」按钮：显式打开可拖动浮窗 */
function openProjectPanel() {
  projPanel.value = projectPanelFor(currentProject.value)
}

/**
 * 收起 / 展开本视图的右侧栏。
 *
 * 收起时右侧栏就没了，项目级下的项目编号 / 所属客户 / 各方人数会一起消失，所以这里
 * 顺手把项目信息落到可移动浮窗上——「合适的位置」没有了就退回浮窗，跟点项目卡时
 * 的判断是同一个规则。展开时把浮窗收掉，避免同一份信息出现两遍。
 */
function toggleSide() {
  sideOpen.value = !sideOpen.value
  if (sideOpen.value) projPanel.value = null
  else if (level.value === 'project') projPanel.value = projectPanelFor(currentProject.value)
}

/**
 * 点侧栏里的项目卡。
 *
 * 需求里这两句其实是一件事的两种表述：
 *   「点击项目应该在合适位置显示项目信息，若找不到合适的位置以可移动弹窗的形式展现」
 *   「点击项目后同样显示客户节点，点击节点返回上一步」
 * 所以这里**同时**做两件事：进入项目作战视图（画布换成项目网络图，客户节点仍在，
 * 点它即可回退），并给出项目信息。
 *
 * 「合适的位置」怎么判断：右侧栏在项目级本来就列了项目编号 / 所属客户 / 阶段级别 /
 * 各方人数，那就是合适的位置，这时不该再叠一个浮窗把画布盖住（实测浮窗正好压住
 * 节点行）。只有侧栏被收起时，才退回可移动浮窗。
 */
function showProject(p) {
  projPanel.value = sideOpen.value ? null : projectPanelFor(p)
  if (currentProject.value?.id !== p.id) enterProject(p.id)
}

function onNodeSelect(n) {
  // 点客户节点 = 返回上一步；点项目节点 = 进入项目
  if (n.kind === 'customer') {
    if (level.value === 'project') backToCustomer()
    showCustomerPanel()
    return
  }
  if (n.kind === 'project') {
    if (n.rowId === currentProject.value?.id) return
    if (level.value === 'customer') enterProject(n.rowId)
    else {
      // 项目级里点了别的项目：先回到客户级再进入，避免状态错乱
      level.value = 'customer'
      enterProject(n.rowId)
    }
    // 侧栏展开时右侧栏就是项目信息的「合适的位置」，不弹浮窗；
    // 侧栏被收起时没有位置可放，退回可移动浮窗（否则点了节点像什么都没发生）。
    if (!sideOpen.value) {
      const target = myProjects.value.find((p) => p.id === n.rowId)
      projPanel.value = projectPanelFor(target)
    }
    return
  }
  if (n.kind === 'c74111') {
    openC74111()
    return
  }
  openNode(n)
}

function openNode(n) {
  const table = n.table
  const id = n.rowId
  const isPerson = n.kind === 'person'
  nodePanel.value = {
    title: n.label,
    sub: n.sub || '',
    pairs: n.detail ?? {},
    tags: buildTags(n),
    diagRows: n.kind === 'c74111' ? [...(n.c74111?.clears ?? []), ...(n.c74111?.priorities ?? []), ...(n.c74111?.key ?? [])] : [],
    edit: table && id ? { table, id } : null,
  }
}

function buildTags(n) {
  const out = []
  if (n.source === 'qax') out.push({ text: '我方（奇安信）', cls: 'tag--ac' })
  else if (n.source === 'partner') out.push({ text: '合作方', cls: 'tag--info' })
  else if (n.source === 'competitor') out.push({ text: '竞争方', cls: 'tag--warn' })
  else if (n.source === 'customer') out.push({ text: '客户方', cls: '' })
  if (n.adur) out.push({ text: `ADUR ${n.adur}`, cls: 'tag--ac' })
  if (n.attitude) out.push({ text: `态度 ${attChar(n.attitude)}`, cls: '' })
  if (n.teamRole) out.push({ text: `我方角色 ${n.teamRole}`, cls: 'tag--ac' })
  if (n.mentor) out.push({ text: '导师', cls: 'tag--warn' })
  if (n.kind === 'c74111') out.push({ text: '独立诊断节点', cls: 'tag--ac' })
  if (n.tier && TIER_META[n.tier]) out.push({ text: TIER_META[n.tier].label, cls: `tag--${TIER_META[n.tier].tone}` })
  return out
}

function openPerson(m) {
  const p = custPersons.value.find((x) => x.id === m.id)
  nodePanel.value = {
    title: m.name,
    sub: [m.title, m.dept].filter(Boolean).join(' · '),
    pairs: p
      ? {
          姓名: p.name ?? '—',
          职务: p.title ?? '—',
          部门: p.dept ?? '—',
          职级: p.rank ?? '—',
          '对项目态度': m.attitude ? attChar(m.attitude) : '未记录',
          'ADUR 角色': m.adur || '未记录',
          决策模式: p.decision_mode ?? '—',
          职业背景: p.career_bg ?? '—',
          可信度: p.credibility ?? '—',
          联系方式: p.contact ?? '—',
          备注: p.remark ?? '—',
        }
      : { 姓名: m.name },
    tags: [
      { text: '客户方', cls: '' },
      ...(m.adur ? [{ text: `ADUR ${m.adur}`, cls: 'tag--ac' }] : []),
      ...(m.attitude ? [{ text: `态度 ${attChar(m.attitude)}`, cls: '' }] : []),
    ],
    diagRows: [],
    edit: { table: 'person', id: m.id },
  }
}

function openStaff(s) {
  // 用 internalPersonDetail 生成中文标签，别把 internal_person 的原始列名
  // （age / birth / career_bg …）直接铺在面板上
  nodePanel.value = {
    title: s.name,
    sub: [s.title, s.dept, s.orgName].filter(Boolean).join(' · '),
    pairs: internalPersonDetail(s.detail ?? {}, {
      所属组织: s.orgName || '',
      本项目派出: '是',
      本项目角色: s.decisionRole || '',
    }),
    tags: [{ text: '我方（奇安信）派出', cls: 'tag--ac' }],
    diagRows: [],
    edit: { table: 'internal_person', id: s.id },
  }
}

function openCoopPerson({ person, type, entity }) {
  nodePanel.value = {
    title: person.name,
    sub: [person.title, person.role].filter(Boolean).join(' · '),
    pairs: { ...(person.detail ?? {}), 所属实体: entity?.name ?? '—' },
    tags: [
      { text: type === 'partner' ? '合作方' : '竞争方', cls: type === 'partner' ? 'tag--info' : 'tag--warn' },
      ...(person.attitude ? [{ text: `态度 ${attChar(person.attitude)}`, cls: '' }] : []),
    ],
    diagRows: [],
    edit: { table: type === 'partner' ? 'partner_person' : 'competitor_person', id: person.personId },
  }
}

function openC74111() {
  if (!currentProject.value) return
  c74Open.value = true
}

/* 实体节点详情里补上动向，方便一眼看全 */
function openEntityFromNode(n) {
  const moves = entityMoves.value.filter((m) => m.entity_id === n.rowId).slice(0, 5)
  openNode({
    ...n,
    detail: {
      ...(n.detail ?? {}),
      ...Object.fromEntries(moves.map((m, i) => [`动向 ${i + 1}`, `${m.happen_time ?? ''} ${m.role ?? ''} ${m.result ?? ''}`])),
    },
  })
}

watch(
  () => props.focusCustomerId,
  async (v) => {
    if (!v) return
    if (!customers.value.length) return
    customerId.value = v
    await selectCustomer()
  },
)

function onResize() {
  windowH.value = window.innerHeight
}

onMounted(async () => {
  window.addEventListener('resize', onResize)
  await loadGlobal()
  // 默认选中「主客户」而不是第一行——customer 表里第一行很可能是「关联客户」
  // （上级集团之类），它下面既没有人员也没有项目，进来会是一张空图。
  if (!customerId.value && customers.value.length) {
    const main = customers.value.find((c) => String(c.category ?? '').includes('主客户'))
    customerId.value = (main ?? customers.value[0]).id
    await selectCustomer()
  }
})
onBeforeUnmount(() => window.removeEventListener('resize', onResize))

defineExpose({ enterProject, backToCustomer, openEntityFromNode })
</script>
