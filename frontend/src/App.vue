<template>
  <!--
    应用外壳
    ┌──────────────────────────────────────────────────────────────┐
    │ 左侧栏（可收起 236px ⇄ 60px）  │ 顶栏（单单元格风格下拉）      │
    │                                ├──────────────────────────────┤
    │ 1. 工作台                      │ 工作台 / 组织网络图 /         │
    │ 2. 组织网络图                  │ 数据分析 / 数据管理 /         │
    │ 3. 数据分析驾驶舱              │ 风格总览 / 单页演示           │
    │ 4. 所有表的增删改查            │                              │
    └──────────────────────────────────────────────────────────────┘

    data-style 挂在 .app 上，11 套风格的所有语义令牌都从这里注入，
    所以顶栏、侧边栏、六个视图会被整体重绘（C74111 四色除外，见 app.css）。
  -->
  <div class="app" :data-style="activeKey">
    <SideBar
      :collapsed="collapsed"
      :view="view"
      :active-table="activeTable"
      :counts="counts"
      :backend="backend"
      :meta="meta"
      @navigate="go"
      @open-table="openTable"
      @toggle-collapse="collapsed = !collapsed"
      @expand-and-crud="expandAndCrud"
    />

    <div class="app-body">
      <!-- ============ 顶栏 ============ -->
      <header class="topbar">
        <button
          class="icon-btn"
          :title="collapsed ? '展开侧边栏' : '收起侧边栏'"
          @click="collapsed = !collapsed"
        >
          <AppIcon name="menu" :size="16" />
        </button>

        <div class="topbar-lead">
          <span class="topbar-title">{{ title }}</span>
          <span class="topbar-crumb">{{ crumb }}</span>
        </div>

        <div class="topbar-right">
          <span
            class="dot"
            :class="online ? 'dot--online' : 'dot--offline'"
            :title="
              online
                ? `后端在线 · ${backend.dbPath} · ${backend.tableCount} 张表`
                : `后端离线${backend.message ? '：' + backend.message : ''}`
            "
          ></span>

          <button class="icon-btn" title="重新探测后端并刷新行数" @click="refresh">
            <AppIcon name="refresh" :size="15" />
          </button>

          <!-- 明暗一键切换：跳到另一基调上次用过的风格 -->
          <button
            class="icon-btn"
            :title="tone === 'dark' ? '切换到亮色风格' : '切换到暗色风格'"
            @click="toggleTone"
          >
            <AppIcon :name="tone === 'dark' ? 'sun' : 'moon'" :size="16" />
          </button>

          <!-- 11 套风格压缩成一个单元格，点击才展开 -->
          <StyleMenu
            :active-key="activeKey"
            :meta="meta"
            :tone="tone"
            :tone-filter="toneFilter"
            :visible="visibleStyles"
            :total="STYLES.length"
            :open="menuOpen"
            @toggle="menuOpen = !menuOpen"
            @close="menuOpen = false"
            @select="pickStyle"
            @step="step"
            @toggle-tone="toggleTone"
            @set-filter="setFilter"
            @open-gallery="openGallery"
          />
        </div>
      </header>

      <!-- ============ 主区 ============ -->
      <main class="app-main" @click="closeSideIfOverlay">
        <!-- 1. 工作台 -->
        <WorkbenchView
          v-if="view === 'workbench'"
          :online="online"
          :tone="tone"
          :style-name="meta.name"
          @focus-customer="focusCustomer"
          @focus-entity="focusEntity"
        />

        <!-- 2. 组织网络图 -->
        <GraphView
          v-else-if="view === 'graph'"
          :online="online"
          :focus-customer-id="graphCustomerId"
          @edit-table="openRow"
        />

        <!-- 3. 数据分析驾驶舱 -->
        <AnalyticsView v-else-if="view === 'analytics'" :online="online" />

        <!-- 4. 所有表的增删改查 -->
        <CrudView
          v-else-if="view === 'crud'"
          :key="activeTable + ':' + crudNonce"
          :table="activeTable"
          :online="online"
          :focus-id="crudFocusId"
          :create-new="crudCreate"
          @focus-opened="onCrudFocusHandled"
        />

        <!-- 风格总览（从风格下拉进入，用于对比 11 套风格） -->
        <template v-else-if="view === 'gallery'">
          <div class="page-head">
            <div class="page-head-main">
              <h2 class="sf-h2" style="font-size: 18px">风格总览</h2>
              <div class="card-sub" style="margin-top: 4px">
                共 {{ STYLES.length }} 套风格：前 8 套严格对齐《设计方案.md》的方案一 ~ 方案八，后 3 套为本次新增。
                点击任意卡片进入单页演示。
              </div>
            </div>
            <div class="page-head-actions">
              <button class="btn btn--ghost" @click="go(prevView)">
                <AppIcon name="chevronLeft" :size="14" />
                返回
              </button>
            </div>
          </div>

          <div class="banner" style="margin-bottom: 14px">
            <AppIcon name="panel" :size="14" />
            当前风格 <b>{{ meta.name }}</b>（{{ meta.idx }}/{{ STYLES.length }}），基调
            {{ tone === 'dark' ? '暗色' : '亮色' }}。键盘 ← → 可直接切换。
          </div>

          <div class="gallery">
            <article
              v-for="s in STYLES"
              :key="s.key"
              class="gcard"
              :class="{ 'is-on': s.key === activeKey }"
              @click="openDemo(s.key)"
            >
              <div class="gcard-head">
                <span
                  class="stylecell-sw"
                  :style="{ background: s.swatch.bg, boxShadow: `0 0 0 1px ${s.swatch.accent}` }"
                ></span>
                <span class="gcard-title">{{ s.name }}</span>
                <span class="gcard-tag">{{ s.idx }} · {{ s.tone === 'dark' ? 'dark' : 'light' }}</span>
              </div>
              <div class="gcard-body">
                <MiniPreview :style-key="s.key" />
              </div>
              <div class="gcard-foot">
                <span>{{ s.note }}</span>
                <span class="gcard-go">{{ s.add ? '新增风格 · 进入演示' : s.source + ' · 进入演示' }}</span>
              </div>
            </article>
          </div>
        </template>

        <!-- 单页演示（整页渲染真实业务界面，用于细致对比风格） -->
        <template v-else>
          <div class="page-head">
            <div class="page-head-main">
              <h2 class="sf-h2" style="font-size: 18px">{{ meta.name }} · 单页演示</h2>
              <div class="stage-meta" style="margin: 6px 0 0">
                <span>编号 <code>{{ meta.idx }} / {{ STYLES.length }}</code></span>
                <span>来源 <code>{{ meta.source }}</code></span>
                <span>基调 <code>{{ tone === 'dark' ? '暗色' : '亮色' }}</code></span>
                <span>{{ meta.notes }}</span>
              </div>
            </div>
            <div class="page-head-actions">
              <button class="btn btn--ghost" @click="step(-1)">
                <AppIcon name="chevronLeft" :size="14" />
                上一套
              </button>
              <button class="btn btn--ghost" @click="step(1)">
                下一套
                <AppIcon name="chevronRight" :size="14" />
              </button>
              <button class="btn" @click="openGallery">
                <AppIcon name="grid" :size="14" />
                风格总览
              </button>
            </div>
          </div>

          <div v-if="isLive" class="banner banner--ok" style="margin-bottom: 12px">
            实时数据已接入 · {{ liveSummary }} · 库 <code>{{ backend.dbPath }}</code>
          </div>
          <div v-else-if="dataError" class="banner banner--err" style="margin-bottom: 12px">
            实时数据连接失败：{{ dataError.message }}
            <button class="btn btn--sm btn--ghost" style="margin-left: 8px" @click="loadLiveData">
              重试
            </button>
          </div>

          <div class="stage-frame">
            <DemoPage :key="activeKey" :style-key="activeKey" :dataset="dataset" />
          </div>
        </template>
      </main>
    </div>

    <!-- ============ 轻提示 ============ -->
    <!--
      ⚠️ 这里必须用 toastItems（一个顶层 ref），不能写成 toasts.items。
      模板只对「setup 作用域的顶层绑定」自动解包 ref；toasts.items 是
      普通对象里的嵌套属性，Vue 不会解包，v-for 拿到的是 RefImpl 实例本身，
      于是会去遍历它内部的 5 个可枚举属性（_value/_rawValue/dep/__v_isRef/__v_isShallow），
      在右下角渲染出 5 个没有文字的空气泡 —— 而且永远不消失。
    -->
    <div class="toasts">
      <div
        v-for="t in toastItems"
        :key="t.id"
        class="toast"
        :class="t.kind ? `toast--${t.kind}` : ''"
      >
        {{ t.message }}
      </div>
    </div>
  </div>
</template>

<script setup>
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'

import AppIcon from './components/AppIcon.vue'
import SideBar from './components/SideBar.vue'
import StyleMenu from './components/StyleMenu.vue'
import WorkbenchView from './components/WorkbenchView.vue'
import GraphView from './components/GraphView.vue'
import AnalyticsView from './components/AnalyticsView.vue'
import CrudView from './components/CrudView.vue'
import DemoPage from './components/DemoPage.vue'
import MiniPreview from './components/MiniPreview.vue'

import { STYLES } from '@/styles/registry'
import { useTheme } from '@/composables/useTheme'
import { useToast } from '@/composables/useToast'
import { useDataSource } from '@/composables/useDataSource'
import { api } from '@/api/client'
import { tableCategory, tableLabel } from '@/nav/dataTree'

/* ---------------------------------------------------------------------------
   风格 / 明暗
   ------------------------------------------------------------------------ */
const {
  activeKey,
  meta,
  tone,
  menuOpen,
  toneFilter,
  visibleStyles,
  setStyle,
  toggleTone,
  step,
  setFilter,
  onKey,
} = useTheme()
const toasts = useToast()
// 单独提成顶层 ref：模板只解包顶层绑定，嵌套的 toasts.items 会被当成 ref 对象遍历（见模板注释）
const toastItems = toasts.items

/* ---------------------------------------------------------------------------
   后端连接状态 + 各表行数
   ------------------------------------------------------------------------ */
const {
  error: dataError,
  dataset,
  isLive,
  backend,
  tableCounts,
  probe,
  loadLive,
} = useDataSource()

const online = computed(() => backend.value.state === 'online')

/** 10 张主表的行数摘要，用来一眼区分「接上了但没数据」和「没接上」 */
const liveSummary = computed(() => {
  const c = tableCounts.value
  if (!c) return ''
  return [
    ['customer', c.customer],
    ['project', c.project],
    ['person', c.person],
    ['coop_entity', c.coop_entity],
    ['internal_person', c.internal_person],
  ]
    .filter(([, n]) => n != null)
    .map(([t, n]) => `${t} ${n}`)
    .join(' · ')
})

/**
 * 侧边栏用的行数。
 * 后端 `/meta/health` 已经一次性返回每张表的 rows，所以这里是 **1 个请求** 而不是 62 个——
 * 之前按表调 `/count` 的写法会让单连接后端排一长串队，纯属浪费。
 */
const counts = ref(null)

async function loadCounts() {
  if (!online.value) {
    counts.value = null
    return
  }
  try {
    const h = await api.health()
    const out = {}
    for (const t of h?.tables ?? []) {
      if (t?.table) out[t.table] = Number(t.rows) || 0
    }
    counts.value = Object.keys(out).length ? out : null
  } catch {
    counts.value = null
  }
}

/* ---------------------------------------------------------------------------
   视图路由
   workbench 工作台 / graph 组织网络图 / analytics 数据分析 / crud 数据管理 / gallery 风格总览 / demo 单页演示
   ------------------------------------------------------------------------ */
const view = ref('workbench')
const prevView = ref('workbench')
const collapsed = ref(false)
const activeTable = ref('customer')
/** 网络图要聚焦的客户（工作台点「客户更新」跳过来时用） */
const graphCustomerId = ref(null)
/** 数据管理页要直接打开的那一行 */
const crudFocusId = ref(null)
/** 数据管理页要直接打开新建表单 */
const crudCreate = ref(false)
/**
 * 让「同一张表、不同入口」也能重新挂载 CrudView。
 * 不加这个计数器的话，用户从空态按钮进来只改 crudCreate，而 key 没变，
 * 组件不会重新挂载，弹窗就不会出现。
 */
const crudNonce = ref(0)

function onCrudFocusHandled() {
  crudFocusId.value = null
  crudCreate.value = false
}

const VIEW_TITLE = {
  workbench: '工作台',
  graph: '组织网络图',
  analytics: '数据分析',
  crud: '数据管理',
  gallery: '风格总览',
  demo: '单页演示',
}

const title = computed(() => VIEW_TITLE[view.value] ?? 'BCRM')

const crumb = computed(() => {
  if (view.value === 'crud') {
    return activeTable.value
      ? `${tableCategory(activeTable.value)} / ${tableLabel(activeTable.value)} · ${activeTable.value}`
      : '请从左侧选择一张表'
  }
  if (view.value === 'graph') return '客户人员组织架构 · 项目作战视图'
  if (view.value === 'analytics') return '分档 / 排行 / 矩阵 / 落库热度 · 全库聚合'
  if (view.value === 'workbench') return '今天也要赢一场'
  if (view.value === 'gallery') return `${STYLES.length} 套风格`
  return `${meta.value.name} · ${meta.value.idx}/${STYLES.length}`
})

function go(next) {
  if (!next || next === view.value) return
  prevView.value = view.value
  view.value = next
  closeSideIfOverlay()
}

/** 点风格名 / 下拉里选一套 */
function pickStyle(key) {
  setStyle(key)
  menuOpen.value = false
}

function openGallery() {
  menuOpen.value = false
  prevView.value = view.value === 'gallery' ? 'workbench' : view.value
  view.value = 'gallery'
}

function openDemo(key) {
  if (key) setStyle(key)
  view.value = 'demo'
}

/* ---------------------------------------------------------------------------
   跨视图跳转
   ------------------------------------------------------------------------ */

/** 侧边栏点表名：切到数据管理并定位到那张表 */
function openTable(name) {
  crudFocusId.value = null
  crudCreate.value = false
  crudNonce.value += 1
  activeTable.value = name
  if (view.value !== 'crud') {
    prevView.value = view.value
    view.value = 'crud'
  }
  closeSideIfOverlay()
}

/** 收起状态下点大类图标：先展开侧栏，再落到该大类的第一张表 */
function expandAndCrud() {
  collapsed.value = false
  openTable(activeTable.value || 'customer')
}

/**
 * 从别的视图点「跳转编辑页面 / 去登记」：
 *   { table, id }                → 打开那一行的编辑弹窗
 *   { table, mode: 'create' }    → 直接打开新建表单
 */
function openRow({ table, id, mode }) {
  if (!table) return
  prevView.value = view.value
  activeTable.value = table
  crudFocusId.value = mode === 'create' ? null : (id ?? null)
  crudCreate.value = mode === 'create'
  crudNonce.value += 1
  view.value = 'crud'
}

/** 工作台点某客户：跳到网络图并聚焦它 */
function focusCustomer(payload) {
  const id = payload?.id ?? payload?.customer_id ?? payload
  graphCustomerId.value = id ?? null
  prevView.value = view.value
  view.value = 'graph'
}

/** 工作台点某竞品：直接打开它的档案（合作竞争方实体表） */
function focusEntity(payload) {
  const id = payload?.id ?? payload?.entity_id ?? payload
  if (id == null) return
  openRow({ table: 'coop_entity', id })
}

/* ---------------------------------------------------------------------------
   刷新
   ------------------------------------------------------------------------ */async function refresh() {
  const ok = await probe()
  counts.value = null
  if (ok) {
    await loadCounts()
    toasts.ok('后端已连接')
  } else {
    toasts.err('连不上后端，请确认 bcrm-backend 已启动')
  }
}

async function loadLiveData() {
  const ok = await loadLive()
  if (ok) toasts.ok('实时数据已接入')
}

/* ---------------------------------------------------------------------------
   生命周期
   ------------------------------------------------------------------------ */
const onResize = () => {
  // 窗口很窄时自动收起侧栏，把宽度让给网络图/表格
  if (window.innerWidth < 1100) collapsed.value = true
}

/**
 * 窄屏（< 900px）下侧栏是浮层抽屉，点主区任意位置就收起它，
 * 否则用户点完导航还得再去按一次汉堡按钮。
 */
function closeSideIfOverlay() {
  if (!collapsed.value && window.innerWidth < 900) collapsed.value = true
}

onMounted(async () => {
  window.addEventListener('keydown', onKey)
  window.addEventListener('resize', onResize)
  onResize()

  const ok = await probe()
  if (ok) {
    // 行数不是首屏关键路径，稍后一点再拉，先让界面出来
    setTimeout(loadCounts, 260)
  }
})

onBeforeUnmount(() => {
  window.removeEventListener('keydown', onKey)
  window.removeEventListener('resize', onResize)
})
</script>
