<template>
  <div class="sf-app" :data-style="styleKey">
    <div class="sf-inner">
      <!-- ============ Hero ============ -->
      <header class="sf-hero">
        <div class="sf-hero-row">
          <div class="sf-hero-copy">
            <span class="sf-eyebrow">{{ styleMeta.idx }} / {{ styleMeta.source }} · {{ styleMeta.name }}</span>
            <h1 class="sf-h1">大客户关系网络<br />与 C74111 决策链诊断</h1>
            <p class="sf-p sf-hero-sub">
              把客户组织架构、决策链角色（ADUR）、合作方与竞争方、我方团队与项目诊断整合到一张可交互的关系网络里。
              点击任意节点查看档案，点击圆形按钮展开或收起下级。
            </p>
            <div class="sf-hero-actions">
              <button class="sf-btn sf-btn--primary" @click="focusGraph">
                <AppIcon name="shield" :size="15" />打开客户地图
              </button>
              <button class="sf-btn sf-btn--ghost" @click="toast('C74111 诊断已导出（演示）')">
                <AppIcon name="frame" :size="15" />导出诊断
              </button>
              <span v-if="flash" class="sf-tag sf-tag--ok"><AppIcon name="check" :size="12" />{{ flash }}</span>
            </div>
          </div>

          <div class="sf-hero-stats">
            <div v-for="s in heroStats" :key="s.k" class="sf-card" style="min-width: 108px; padding: 12px 14px">
              <div class="sf-small">{{ s.k }}</div>
              <div class="sf-kpi-num" style="font-size: 20px; margin-top: 4px">
                {{ s.v }}<span class="sf-kpi-unit">{{ s.u }}</span>
              </div>
            </div>
          </div>
        </div>
      </header>

      <main class="sf-stack">
        <!-- ============ KPI ============ -->
        <section class="sf-kpis" aria-label="关键指标">
          <article v-for="k in D.kpis" :key="k.name" class="sf-card sf-card--hover sf-kpi">
            <div class="sf-kpi-top">
              <span class="sf-kpi-name">{{ k.name }}</span>
              <span class="sf-icon-btn"><AppIcon :name="k.icon" :size="16" /></span>
            </div>
            <div class="sf-kpi-num">
              {{ k.value }}<span class="sf-kpi-unit">{{ k.unit }}</span>
            </div>
            <div class="sf-kpi-foot">
              <span
                v-if="k.delta"
                class="sf-delta"
                :class="k.trend === 'up' ? 'sf-delta--up' : 'sf-delta--down'"
              >
                <AppIcon :name="k.trend === 'up' ? 'arrowup' : 'arrowdown'" :size="12" />{{ k.delta }}
              </span>
              <span>{{ k.foot }}</span>
            </div>
          </article>
        </section>

        <!-- ============ 关系网络 / 组织架构 ============ -->
        <section ref="graphCard" class="sf-card sf-card--hover" aria-label="客户关系网络与组织架构">
          <div class="sf-card-head">
            <div>
              <h2 class="sf-h3">客户关系网络与组织架构</h2>
              <div class="sf-card-sub" style="margin-top: 4px">
                {{ graphCaption }}
              </div>
            </div>
            <div class="sf-hero-actions" style="margin: 0">
              <button
                v-for="f in filters"
                :key="f.key"
                class="sf-btn"
                :class="f.on ? 'sf-btn--soft' : 'sf-btn--ghost'"
                style="min-height: 34px; padding: 6px 12px; font-size: 12px"
                @click="f.on = !f.on"
              >
                <span class="sf-dot" :style="{ background: f.color, borderRadius: '3px' }"></span>
                {{ f.label }}
              </button>
            </div>
          </div>

          <div class="sf-legend" style="margin-bottom: 10px">
            <span v-for="l in LEGEND" :key="l.label" class="sf-legend-item">
              <span class="sf-dot" :style="{ background: l.color }"></span>{{ l.label }}
            </span>
          </div>

          <OrgGraph :nodes="graphNodes" :height="560" @select="onNodeSelect" />

          <div class="sf-hairline"></div>
          <div class="sf-legend">
            <span class="sf-small">
              业务口径：奇安信为投标主体，属「我方（自有团队）」独立类别，<strong style="color: var(--tx-1)">既不计入合作方、也不计入竞争方</strong>；
              合作方与竞争方来自 <span class="sf-mono">coop_entity</span>（实体合并），人员分表存储。
            </span>
          </div>
        </section>

        <!-- ============ 表格 + 表单 ============ -->
        <div class="sf-split">
          <section class="sf-card sf-card--hover" aria-label="在跟项目">
            <div class="sf-card-head">
              <h2 class="sf-h3">在跟项目</h2>
              <span class="sf-card-sub">点击行可写入右侧拜访记录</span>
            </div>
            <div class="sf-table-wrap">
              <table class="sf-table">
                <caption>金额单位：万元 · 趋赢力按 C74111 满分 100 折算</caption>
                <thead>
                  <tr>
                    <th>项目</th>
                    <th>类型</th>
                    <th>级别</th>
                    <th style="text-align: right">预算</th>
                    <th>趋赢力</th>
                  </tr>
                </thead>
                <tbody>
                  <tr
                    v-for="p in D.projects"
                    :key="p.id"
                    @click="pickProject(p)"
                    :aria-selected="activeProjectId === p.id"
                  >
                    <td>
                      <div class="strong">{{ p.name }}</div>
                      <div class="sf-small" style="margin-top: 2px">{{ p.customer }}</div>
                    </td>
                    <td>
                      <span
                        class="sf-tag"
                        :class="p.stage === '项目' ? 'sf-tag--ac' : p.stage === '商机' ? 'sf-tag--warn' : ''"
                      >
                        {{ p.stage }}
                      </span>
                    </td>
                    <td class="sf-mono">{{ p.level }}</td>
                    <td class="sf-mono" style="text-align: right">{{ p.budget.toLocaleString() }}</td>
                    <td>
                      <div style="display: flex; align-items: center; gap: 8px">
                        <span class="sf-bar" style="flex: 1"><i :style="{ width: p.winRate + '%' }"></i></span>
                        <span class="sf-mono sf-small" style="min-width: 30px">{{ p.winRate }}%</span>
                      </div>
                    </td>
                  </tr>
                </tbody>
              </table>
            </div>
          </section>

          <section class="sf-card sf-card--hover" aria-label="新增拜访记录">
            <div class="sf-card-head">
              <h2 class="sf-h3">新增拜访记录</h2>
              <span class="sf-card-sub">自动关联决策链节点</span>
            </div>

            <form @submit.prevent="submitVisit" style="display: flex; flex-direction: column; gap: 12px">
              <div class="sf-field">
                <label class="sf-label" for="v-person">被访人（来自网络图）</label>
                <select id="v-person" v-model="form.person" class="sf-select">
                  <option v-for="p in personOptions" :key="p" :value="p">{{ p }}</option>
                </select>
              </div>

              <div class="sf-field">
                <label class="sf-label" for="v-project">关联项目</label>
                <select id="v-project" v-model="form.project" class="sf-select">
                  <option v-for="p in D.projects" :key="p.id" :value="p.name">{{ p.name }}</option>
                </select>
              </div>

              <div class="sf-grid-2">
                <div class="sf-field">
                  <label class="sf-label" for="v-date">拜访日期</label>
                  <input id="v-date" v-model="form.date" type="date" class="sf-input" />
                </div>
                <div class="sf-field">
                  <label class="sf-label">对我方态度</label>
                  <div style="display: flex; gap: 6px; flex-wrap: wrap">
                    <button
                      v-for="a in ATTITUDE_OPTIONS"
                      :key="a.value"
                      type="button"
                      class="sf-btn"
                      :class="form.attitude === a.value ? 'sf-btn--primary' : 'sf-btn--ghost'"
                      style="min-height: 38px; padding: 6px 12px; font-size: 12px"
                      @click="form.attitude = a.value"
                    >
                      {{ a.label }}
                    </button>
                  </div>
                </div>
              </div>

              <div class="sf-field">
                <label class="sf-label" for="v-summary">拜访纪要</label>
                <textarea
                  id="v-summary"
                  v-model="form.summary"
                  class="sf-textarea"
                  placeholder="记录对方诉求、燃眉之急与我方承诺…"
                ></textarea>
              </div>

              <div class="sf-form-actions">
                <button class="sf-btn sf-btn--primary" type="submit">
                  <AppIcon name="check" :size="15" />提交归档
                </button>
                <button class="sf-btn sf-btn--ghost" type="button" @click="resetForm">重置</button>
                <span class="sf-small">共 {{ visits }} 条记录</span>
              </div>
            </form>
          </section>
        </div>
      </main>

      <!-- ============ 页脚 ============ -->
      <footer class="sf-footer">
        <div>
          <div class="sf-h3">{{ styleMeta.name }}</div>
          <div class="sf-small" style="margin-top: 4px; max-width: 46ch">{{ styleMeta.notes }}</div>
        </div>
        <div class="sf-legend">
          <span class="sf-tag">{{ styleMeta.tone === 'dark' ? '暗色风格' : '亮色风格' }}</span>
          <span class="sf-tag">{{ styleMeta.source }}</span>
          <span class="sf-tag">{{ styleMeta.idx }} / 11</span>
        </div>
      </footer>
    </div>
  </div>
</template>

<script setup>
import { computed, reactive, ref } from 'vue'
import AppIcon from './AppIcon.vue'
import OrgGraph from './OrgGraph.vue'
import { MOCK_DATASET } from '@/composables/useDataSource'
import { LEGEND, ATTITUDE_OPTIONS, FORM_DEFAULTS } from '@/data/mock'
import { STYLE_MAP } from '@/styles/registry'

const props = defineProps({
  styleKey: { type: String, required: true },
  /** 由外部注入的数据集；不传则回落到内置演示数据 */
  dataset: { type: Object, default: null },
})

const styleMeta = computed(() => STYLE_MAP[props.styleKey] ?? { idx: '00', name: '', notes: '', source: '', tone: 'dark' })

/** 数据来源的唯一入口：组件层不关心它是 mock 还是真实库 */
const D = computed(() => props.dataset ?? MOCK_DATASET)
const heroStats = computed(() => D.value.heroStats)

/* ---------------- 图层筛选 ---------------- */
const filters = reactive([
  { key: 'qax', label: '我方团队', color: 'var(--n-qax)', on: true },
  { key: 'partner', label: '合作方', color: 'var(--n-partner)', on: true },
  { key: 'competitor', label: '竞争方', color: 'var(--n-competitor)', on: true },
  { key: 'diag', label: 'C74111', color: 'var(--n-project)', on: true },
])

const graphNodes = computed(() => {
  const on = Object.fromEntries(filters.map((f) => [f.key, f.on]))
  return D.value.graphNodes.filter((n) => {
    if (n.source === 'qax') return on.qax
    if (n.source === 'partner') return on.partner
    if (n.source === 'competitor') return on.competitor
    if (n.zone === 'diag') return on.diag
    return true
  })
})

/* ---------------- 交互 ---------------- */
const selectedPerson = ref('')
const activeProjectId = ref(null)
const visits = ref(0)
const flash = ref('')
const graphCard = ref(null)

const personOptions = computed(() => D.value.personOptions)

/** 图形卡副标题：跟随数据源，不写死客户名 */
const graphCaption = computed(() => {
  const top = D.value.graphNodes.find((n) => n.zone === 'top')
  const proj = D.value.projects[0]
  const head = [top?.label, proj?.name].filter(Boolean).join(' · ')
  return `${head ? head + ' · ' : ''}共 ${graphNodes.value.length} 个节点`
})

const form = reactive({ ...FORM_DEFAULTS })

function onNodeSelect(node) {
  if (node.kind === 'person') selectedPerson.value = node.label
  if (node.kind === 'person') form.person = node.label
  if (node.kind === 'project') {
    const hit = D.value.projects.find((p) => p.name.includes(node.label.replace(/\s/g, '')))
    if (hit) pickProject(hit)
  }
}

function pickProject(p) {
  activeProjectId.value = p.id
  form.project = p.name
}

function submitVisit() {
  visits.value += 1
  toast(`已归档：${form.person} · ${form.attitude} · ${form.date}`)
}

function resetForm() {
  Object.assign(form, FORM_DEFAULTS)
  activeProjectId.value = null
}

let flashTimer = null
function toast(msg) {
  flash.value = msg
  clearTimeout(flashTimer)
  flashTimer = setTimeout(() => (flash.value = ''), 2600)
}

function focusGraph() {
  graphCard.value?.scrollIntoView({ behavior: 'smooth', block: 'center' })
}
</script>
