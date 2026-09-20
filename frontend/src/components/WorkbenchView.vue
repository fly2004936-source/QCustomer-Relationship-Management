<template>
  <!--
    工作台（数字桌面）
    顶部：问候 / 时间 / 第几周 / 一句话目标 / 今日进度（需求 1、6）
    中部三列：待办 · 便签+番茄钟 · 快捷入口（需求 2、3、4、5）
    信息区：网络安全资讯 · 客户信息更新 · AI 动向 · 竞品动态 · 金融科技（需求 7~11）
    所有读写都走后端真实表，关掉再打开数据还在。
  -->
  <div class="app-inner">
    <div v-if="offline" class="banner banner--warn">
      <AppIcon name="bell" :size="14" />
      连不上后端（{{ errorText }}），工作台暂时只读。请确认 bcrm-backend 已启动。
    </div>

    <WorkbenchHero
      :goal="plan.one_line_goal"
      :done="doneCount"
      :total="todos.length"
      :focus-min="focusTotalMin"
      :tone="tone"
      :style-name="styleName"
      @save-goal="saveGoal"
    />

    <!-- 中部：三列（宽屏）→ 单列（窄屏） -->
    <div class="wb-grid" style="margin-top: 14px">
      <TodoCard :todos="todos" :loading="loading" @add="addTodo" @toggle="toggleTodo" @remove="removeTodo" />
      <NoteCard
        :notes="notes"
        @create="createNote"
        @save="saveNote"
        @remove="removeNote"
      />
      <div class="wb-col-4" style="display: flex; flex-direction: column; gap: 14px">
        <FocusCard :done-today="focusDone" :total-today="focusTotalMin" @record="recordFocus" />
        <ShortcutCard :shortcuts="shortcuts" @add="addShortcut" @remove="removeShortcut" />
      </div>
    </div>

    <div class="wb-section-note">每日情报</div>

    <!-- 信息区 -->
    <div class="wb-grid">
      <NewsCard
        title="网络安全行业资讯"
        icon="shield"
        :items="newsSec"
        :loading="loading"
        empty-text="暂无网络安全资讯"
        @refresh="loadAll"
        @open="openNews"
      />
      <CustomerUpdateCard
        :today="customerToday"
        :history="customerHistory"
        :customers="customers"
        @refresh="loadCustomerUpdates"
        @load-history="loadCustomerHistory"
        @open-customer="emit('focus-customer', $event)"
      />
      <NewsCard
        title="AI 行业动向"
        icon="spark"
        :items="newsAi"
        :loading="loading"
        empty-text="暂无 AI 资讯"
        @refresh="loadAll"
        @open="openNews"
      />
      <MovesCard
        :moves="competitorMoves"
        :confrontations="confrontations"
        :entities="entities"
        @refresh="loadAll"
        @open-entity="emit('focus-entity', $event)"
      />
      <NewsCard
        title="金融 / 科技圈资讯"
        icon="trend"
        :items="newsFin"
        :loading="loading"
        empty-text="暂无金融科技资讯"
        @refresh="loadAll"
        @open="openNews"
      />
      <section class="card wb-col-4">
        <div class="card-head">
          <div class="card-title">
            <AppIcon name="database" :size="15" />
            <span>数据落库情况</span>
          </div>
        </div>
        <dl class="kv">
          <dt>今日待办</dt>
          <dd>{{ todos.length }} 条（完成 {{ doneCount }}）</dd>
          <dt>便签</dt>
          <dd>{{ notes.length }} 条</dd>
          <dt>快捷入口</dt>
          <dd>{{ shortcuts.length }} 个</dd>
          <dt>番茄钟</dt>
          <dd>{{ focusDone }} 个 / {{ focusTotalMin }} 分钟</dd>
          <dt>资讯</dt>
          <dd>{{ newsSec.length + newsAi.length + newsFin.length }} 条</dd>
          <dt>客户更新</dt>
          <dd>今日 {{ customerToday.length }} 条</dd>
          <dt>竞品动态</dt>
          <dd>{{ competitorMoves.length }} 条</dd>
        </dl>
        <div class="hairline"></div>
        <div class="row" style="gap: 8px; flex-wrap: wrap">
          <button class="btn btn--sm btn--ghost" @click="loadAll">
            <AppIcon name="refresh" :size="13" />
            全部刷新
          </button>
          <span class="card-sub">{{ updatedAt ? `更新于 ${updatedAt}` : '' }}</span>
        </div>
      </section>
    </div>

    <!-- 资讯详情 -->
    <div v-if="newsDetail" class="modal-mask" @click.self="newsDetail = null">
      <div class="modal" style="width: min(620px, 100%)">
        <div class="modal-head">
          <div>
            <div class="modal-title">{{ newsDetail.title }}</div>
            <div class="modal-sub">
              {{ newsDetail.category }} · {{ newsDetail.news_date }}
              <template v-if="newsDetail.source"> · {{ newsDetail.source }}</template>
            </div>
          </div>
          <div class="card-actions">
            <button class="icon-btn" style="width: 28px; height: 28px" @click="newsDetail = null">
              <AppIcon name="close" :size="14" />
            </button>
          </div>
        </div>
        <div class="modal-body">
          <p class="sf-p" style="max-width: none">{{ newsDetail.summary || '（无摘要）' }}</p>
          <div v-if="newsDetail.tags" class="wrap" style="margin-top: 12px">
            <span v-for="t in String(newsDetail.tags).split(',')" :key="t" class="tag">{{ t }}</span>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { computed, onMounted, ref } from 'vue'
import AppIcon from './AppIcon.vue'
import WorkbenchHero from './wb/WorkbenchHero.vue'
import TodoCard from './wb/TodoCard.vue'
import NoteCard from './wb/NoteCard.vue'
import ShortcutCard from './wb/ShortcutCard.vue'
import FocusCard from './wb/FocusCard.vue'
import NewsCard from './wb/NewsCard.vue'
import CustomerUpdateCard from './wb/CustomerUpdateCard.vue'
import MovesCard from './wb/MovesCard.vue'
import { workbenchApi } from '@/api/workbench'
import { ensureWorkbenchSeed } from '@/api/seed'
import { fetchAll } from '@/api/client'
import { dateKey, minuteKey } from '@/api/date'
import { useToast } from '@/composables/useToast'

const props = defineProps({
  online: { type: Boolean, default: true },
  tone: { type: String, default: 'dark' },
  styleName: { type: String, default: '' },
})

const emit = defineEmits(['focus-customer', 'focus-entity'])

const toast = useToast()

/* ---------------- 状态 ---------------- */
const loading = ref(false)
const errorText = ref('')
const updatedAt = ref('')

const plan = ref({ one_line_goal: '', next_action: '' })
const todos = ref([])
const notes = ref([])
const shortcuts = ref([])
const focusRows = ref([])
const allNews = ref([])
const customerToday = ref([])
const customerHistory = ref([])
const customers = ref([])
const competitorMoves = ref([])
const confrontations = ref([])
const entities = ref([])
const newsDetail = ref(null)

const offline = computed(() => !props.online)

const doneCount = computed(() => todos.value.filter((t) => t.done).length)
const focusTotalMin = computed(() => focusRows.value.reduce((s, r) => s + (r.actual_min || 0), 0))
const focusDone = computed(() => focusRows.value.filter((r) => r.outcome === '已完成').length)

/** 资讯分桶：一条只进一个卡片，优先级 金融科技 > AI > 网络安全 */
const newsBuckets = computed(() => {
  const sec = []
  const ai = []
  const fin = []
  for (const n of allNews.value) {
    const tags = String(n.tags ?? '')
    if (tags.includes('金融科技')) fin.push(n)
    else if (tags.includes('AI动向') || n.category === 'AI安全') ai.push(n)
    else sec.push(n)
  }
  return { sec, ai, fin }
})
const newsSec = computed(() => newsBuckets.value.sec)
const newsAi = computed(() => newsBuckets.value.ai)
const newsFin = computed(() => newsBuckets.value.fin)

/* ---------------- 加载 ---------------- */
/** 读操作统一包一层：失败不抛，返回兜底值，避免一个接口挂了整页白屏 */
async function safe(fn, fallback) {
  try {
    return await fn()
  } catch (e) {
    if (e?.kind === 'offline' || e?.kind === 'timeout') errorText.value = e.message
    return fallback
  }
}

async function loadAll() {
  loading.value = true
  const today = dateKey()
  const [p, td, nt, sc, fc, nw, cu, cs, mv, cf, en] = await Promise.all([
    safe(() => workbenchApi.loadPlan(today), null),
    safe(() => workbenchApi.listTodos(today), []),
    safe(() => workbenchApi.listNotes(), []),
    safe(() => workbenchApi.listShortcuts(), []),
    safe(() => workbenchApi.listFocus(today), []),
    safe(() => workbenchApi.listNews({ limit: 200 }), []),
    safe(() => workbenchApi.listCustomerUpdates({ date: today, limit: 200 }), []),
    safe(() => fetchAll('customer', { page_size: 200, order_by: 'id:asc' }), []),
    safe(() => workbenchApi.listEntityMoves({ entityType: 'competitor', limit: 12 }), []),
    safe(() => workbenchApi.listConfrontations({ limit: 8 }), []),
    safe(() => fetchAll('coop_entity', { page_size: 200, order_by: 'id:asc' }), []),
  ])

  plan.value = p ?? { one_line_goal: '', next_action: '' }
  todos.value = td
  notes.value = nt
  shortcuts.value = sc
  focusRows.value = fc
  allNews.value = nw
  customerToday.value = cu
  customers.value = cs.map((c) => ({ id: c.id, name: c.name, category: c.category }))
  competitorMoves.value = mv
  confrontations.value = cf
  entities.value = en.map((e) => ({ id: e.id, name: e.name, entity_type: e.entity_type }))

  updatedAt.value = minuteKey()
  loading.value = false
}

async function loadCustomerUpdates() {
  customerToday.value = await safe(() => workbenchApi.listCustomerUpdates({ date: dateKey(), limit: 200 }), [])
}

async function loadCustomerHistory(customerId) {
  customerHistory.value = await safe(
    () =>
      customerId
        ? workbenchApi.listCustomerUpdateHistory(customerId, 200)
        : workbenchApi.listCustomerUpdates({ limit: 200 }),
    [],
  )
}

/* ---------------- 待办 ---------------- */
async function addTodo({ item, priority }) {
  if (offline.value) return toast.err('后端离线，无法保存')
  try {
    const r = await workbenchApi.addTodo({ item, priority })
    todos.value = [...todos.value, { id: r.id, item, done: false, priority }]
    toast.ok('已添加待办')
  } catch (e) {
    toast.err(`添加失败：${e.message}`)
  }
}

async function toggleTodo(t) {
  const next = !t.done
  // 乐观更新：先改 UI，失败再回滚，点击手感不打折
  todos.value = todos.value.map((x) => (x.id === t.id ? { ...x, done: next } : x))
  try {
    await workbenchApi.setTodoDone(t.id, next)
  } catch (e) {
    todos.value = todos.value.map((x) => (x.id === t.id ? { ...x, done: !next } : x))
    toast.err(`更新失败：${e.message}`)
  }
}

async function removeTodo(t) {
  todos.value = todos.value.filter((x) => x.id !== t.id)
  try {
    await workbenchApi.removeTodo(t.id)
    toast.ok('已删除')
  } catch (e) {
    toast.err(`删除失败：${e.message}`)
    loadAll()
  }
}

/* ---------------- 目标 ---------------- */
async function saveGoal(text) {
  if (offline.value) return
  try {
    const row = await workbenchApi.savePlan({
      id: plan.value.id,
      plan_date: dateKey(),
      one_line_goal: text,
      next_action: plan.value.next_action,
    })
    // 首次保存时后端才分配 id，记下来避免下次又插入一行
    plan.value = { ...plan.value, id: plan.value.id ?? row?.id, one_line_goal: text }
  } catch (e) {
    toast.err(`目标保存失败：${e.message}`)
  }
}

/* ---------------- 便签 ---------------- */
async function createNote() {
  if (offline.value) return toast.err('后端离线，无法新建便签')
  try {
    const r = await workbenchApi.createNote('')
    const row = { id: r.id, content: '', updated_at: null }
    notes.value = [...notes.value, row]
    toast.ok('已新建便签')
  } catch (e) {
    toast.err(`新建失败：${e.message}`)
  }
}

async function saveNote({ id, content }) {
  notes.value = notes.value.map((n) => (n.id === id ? { ...n, content, updated_at: minuteKey() } : n))
  try {
    await workbenchApi.saveNote(id, content)
  } catch (e) {
    toast.err(`保存失败：${e.message}`)
  }
}

async function removeNote(n) {
  notes.value = notes.value.filter((x) => x.id !== n.id)
  try {
    await workbenchApi.removeNote(n.id)
    toast.ok('已删除便签')
  } catch (e) {
    toast.err(`删除失败：${e.message}`)
    loadAll()
  }
}

/* ---------------- 快捷入口 ---------------- */
async function addShortcut({ title, url, sort_order }) {
  try {
    const r = await workbenchApi.addShortcut({ title, url, sort_order })
    shortcuts.value = [...shortcuts.value, { id: r.id, title, url, icon: '', sort_order }]
    toast.ok('已添加快捷入口')
  } catch (e) {
    toast.err(`添加失败：${e.message}`)
  }
}

async function removeShortcut(s) {
  shortcuts.value = shortcuts.value.filter((x) => x.id !== s.id)
  try {
    await workbenchApi.removeShortcut(s.id)
  } catch (e) {
    toast.err(`删除失败：${e.message}`)
    loadAll()
  }
}

/* ---------------- 番茄钟 ---------------- */
async function recordFocus({ plan_min, actual_min, outcome }) {
  try {
    const r = await workbenchApi.recordFocus({ plan_min, actual_min, outcome })
    focusRows.value = [
      { id: r.id, plan_min, actual_min, outcome, started_at: null, note: '' },
      ...focusRows.value,
    ]
    toast.ok(outcome === '已完成' ? `完成一个 ${actual_min} 分钟番茄` : `已记录中断（${actual_min} 分钟）`)
  } catch (e) {
    toast.err(`记录失败：${e.message}`)
  }
}

function openNews(n) {
  newsDetail.value = n
}

/* ---------------- 生命周期 ---------------- */
onMounted(async () => {
  // 空库时补一次种子内容，让资讯 / 客户更新 / 竞品动态卡片有东西可看
  await safe(() => ensureWorkbenchSeed(), null)
  await loadAll()
})

defineExpose({ loadAll })
</script>
