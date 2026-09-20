<template>
  <!--
    数据分析与可视化驾驶舱
    ─────────────────────────────────────────────────────────────
    位置：个人工作台 / 客户架构网络图 之后的第三个视图。
    数据：全部来自 SQLite 真实表（customer / project / project_c74111 /
          project_member / coop_entity / entsales_perf / visit_record /
          customer_budget / 四张人员表），离线时整体回落演示数据。

    视觉：控制台风格 —— 面板带四角标记与网格底纹、数字走等宽字体、
          图表全部手绘 SVG（沿用本项目「不引图表库」的既有取舍）。
          所有颜色 / 圆角 / 阴影 / 动效都读 themes.css 的语义令牌，
          所以 11 套风格、明暗两种基调都会把这里整体重绘。
  -->
  <div class="app-inner ck">
    <!-- ============ 驾驶舱顶栏 ============ -->
    <div class="ck-bar">
      <div class="ck-bar-main">
        <div class="ck-eyebrow">DATA COCKPIT · BCRM</div>
        <h2 class="ck-title">数据分析与可视化驾驶舱</h2>
        <div class="ck-sub">
          覆盖 <b>{{ a.db.tableCount || 62 }}</b> 张业务表 ·
          有数据 <b>{{ a.db.withData }}</b> 张 ·
          记录 <b>{{ a.db.rowTotal.toLocaleString('zh-CN') }}</b> 条 ·
          客户 <b>{{ a.totals.customers }}</b> 家 /
          项目 <b>{{ a.totals.projects }}</b> 个 /
          人员 <b>{{ a.totals.persons }}</b> 人
        </div>
      </div>

      <div class="ck-bar-side">
        <div class="ck-clock">
          <span class="ck-clock-t mono">{{ clockTime }}</span>
          <span class="ck-clock-d">{{ clockDate }}</span>
        </div>
        <div class="ck-status">
          <span class="ck-led" :class="live ? 'is-live' : 'is-mock'"></span>
          <span>{{ live ? '实时库' : '演示数据' }}</span>
        </div>
        <button class="btn btn--sm" :disabled="loading" @click="loadAll">
          <AppIcon name="refresh" :size="13" />
          {{ loading ? '读取中…' : '刷新' }}
        </button>
      </div>
    </div>

    <div v-if="errorText" class="banner banner--warn" style="margin-bottom: 14px">
      <AppIcon name="bell" :size="14" />
      实时数据读取失败（{{ errorText }}），已回落演示数据。请确认 bcrm-backend 已启动。
    </div>

    <!-- ============ KPI 带 ============ -->
    <div class="ck-kpis">
      <div v-for="k in a.kpis" :key="k.key" class="ck-kpi" :class="`ck-kpi--${k.tone}`">
        <div class="ck-kpi-top">
          <span class="ck-kpi-name">{{ k.name }}</span>
          <AppIcon :name="k.icon" :size="15" />
        </div>
        <div class="ck-kpi-num">
          <span class="mono">{{ k.value }}</span>
          <em>{{ k.unit }}</em>
        </div>
        <div class="ck-kpi-foot">{{ k.foot }}</div>
      </div>
    </div>

    <!-- ============ 图表网格 ============ -->
    <div class="ck-grid">
      <!-- 1. C74111 分档 -->
      <section class="card ck-panel ck-p4">
        <div class="card-head">
          <div class="card-title">
            <AppIcon name="target" :size="15" />
            <span>C74111 竞争态势分档</span>
          </div>
          <div class="card-actions">
            <span class="tag">{{ a.c74111.diagnosed }} / {{ a.c74111.total }} 已诊断</span>
          </div>
        </div>
        <div class="ck-split">
          <CockpitDonut
            :items="bandDonut"
            :size="150"
            :thickness="13"
            :center-value="a.c74111.avgScore.toFixed(0)"
            center-sub="平均分 /100"
          />
          <div class="ck-legend">
            <div v-for="b in a.c74111.bands" :key="b.key" class="ck-legend-row">
              <i :style="{ background: b.color }"></i>
              <span class="ck-legend-label">{{ b.label }}</span>
              <span class="ck-legend-range mono">{{ b.range }}</span>
              <span class="ck-legend-val mono">{{ b.count }}</span>
              <span class="ck-legend-pct mono">{{ b.pct }}%</span>
            </div>
          </div>
        </div>
        <div class="hairline"></div>
        <div class="ck-note">
          平均趋赢力 <b class="mono">{{ a.c74111.avgWinRate.toFixed(1) }}%</b> ·
          分档口径：&gt;75 可承诺 / 50–75 可争取 / 25–50 可参与 / &lt;25 不可承诺 / 无记录为「未诊断」
        </div>
      </section>

      <!-- 2. 项目阶段与级别 -->
      <section class="card ck-panel ck-p4">
        <div class="card-head">
          <div class="card-title">
            <AppIcon name="layers" :size="15" />
            <span>项目阶段漏斗 · 级别分布</span>
          </div>
        </div>
        <div class="ck-block-label">按 info_type / time_phase 归一阶段（个数）</div>
        <CockpitColumns :items="a.stages" :height="128" />

        <div class="ck-block-label" style="margin-top: 12px">
          按 level 分布（金额 万元）
        </div>
        <CockpitBars :items="levelBars" unit="万元" />
      </section>

      <!-- 3. 人员态度 -->
      <section class="card ck-panel ck-p4">
        <div class="card-head">
          <div class="card-title">
            <AppIcon name="users" :size="15" />
            <span>决策链态度分布</span>
          </div>
          <div class="card-actions">
            <span class="tag tag--ok">支持 {{ a.attitudeSummary.supportRate }}%</span>
            <span class="tag tag--bad">反对 {{ a.attitudeSummary.opposeRate }}%</span>
          </div>
        </div>
        <div class="ck-split">
          <CockpitDonut
            :items="a.attitude"
            :size="150"
            :thickness="13"
            :center-value="a.attitudeSummary.total"
            center-sub="建档态度"
          />
          <div class="ck-legend">
            <div v-for="r in a.attitude" :key="r.v" class="ck-legend-row">
              <i :style="{ background: r.color }"></i>
              <span class="ck-legend-label">{{ r.label }}</span>
              <span class="ck-legend-range mono">{{ r.v }}</span>
              <span class="ck-legend-val mono">{{ r.count }}</span>
            </div>
          </div>
        </div>
        <div class="hairline"></div>
        <div class="ck-note">
          客户方人员态度取 <code>project_member.attitude</code>（<code>person</code> 表无此列）；
          合作 / 竞争 / 我方三张人员表自带 <code>attitude</code>。
        </div>
      </section>

      <!-- 4. 预算排行 -->
      <section class="card ck-panel ck-p7">
        <div class="card-head">
          <div class="card-title">
            <AppIcon name="coin" :size="15" />
            <span>项目预算排行 Top 8</span>
          </div>
          <div class="card-actions">
            <span class="tag">合计 {{ a.totals.totalBudget.toLocaleString('zh-CN') }} 万元</span>
          </div>
        </div>
        <CockpitBars :items="budgetBars" unit="万元" />
        <div class="ck-risk" style="margin-top: 12px">
          <div class="ck-block-label">趋赢力风险榜（已诊断项目中分数最低）</div>
          <div class="ck-tbl">
            <div v-for="p in a.c74111.risk" :key="p.id" class="ck-tbl-row">
              <span class="ck-tbl-name" :title="p.name">{{ p.name }}</span>
              <span class="ck-tbl-cust" :title="p.customer">{{ p.customer }}</span>
              <span class="c74" :class="c74Class(p.band)">{{ Math.round(p.score) }} 分</span>
              <span class="ck-tbl-band">{{ bandLabel(p.band) }}</span>
            </div>
            <div v-if="!a.c74111.risk.length" class="empty">暂无已诊断项目</div>
          </div>
        </div>
      </section>

      <!-- 5. 竞争态势矩阵 -->
      <section class="card ck-panel ck-p5">
        <div class="card-head">
          <div class="card-title">
            <AppIcon name="crosshair" :size="15" />
            <span>竞争态势矩阵</span>
          </div>
          <div class="card-actions">
            <span class="tag">{{ a.matrix.length }} 个项目</span>
          </div>
        </div>
        <CockpitScatter :points="a.matrix" />
      </section>

      <!-- 6. C74111 十二项画像 -->
      <section class="card ck-panel ck-p4">
        <div class="card-head">
          <div class="card-title">
            <AppIcon name="spark" :size="15" />
            <span>C74111 得分画像（均分归一）</span>
          </div>
        </div>
        <CockpitRadar :axes="a.c74111.blocks" />
        <div class="ck-mini">
          <span v-for="g in groupScores" :key="g.label" class="ck-mini-item">
            <i :style="{ background: g.color }"></i>
            {{ g.label }} <b class="mono">{{ g.avg.toFixed(1) }}</b>/{{ g.max }}
          </span>
        </div>
      </section>

      <!-- 7. 客户行业 -->
      <section class="card ck-panel ck-p4">
        <div class="card-head">
          <div class="card-title">
            <AppIcon name="building" :size="15" />
            <span>客户行业分布</span>
          </div>
        </div>
        <CockpitBars :items="industryBars" unit="家" />
      </section>

      <!-- 8. 战区分布 -->
      <section class="card ck-panel ck-p4">
        <div class="card-head">
          <div class="card-title">
            <AppIcon name="network" :size="15" />
            <span>战区 / 区域分布</span>
          </div>
        </div>
        <CockpitBars :items="regionBars" unit="家" />
        <div class="hairline"></div>
        <div class="ck-block-label">合作 / 竞争方实体构成（coop_entity.entity_type）</div>
        <CockpitBars :items="entityBars" unit="家" />
      </section>

      <!-- 9. 竞品经营绩效 -->
      <section class="card ck-panel ck-p6">
        <div class="card-head">
          <div class="card-title">
            <AppIcon name="trend" :size="15" />
            <span>合作 / 竞争方经营绩效</span>
          </div>
          <div class="card-actions">
            <span class="tag">entsales_perf · 最新年度</span>
          </div>
        </div>
        <CockpitBars :items="perfBars" unit="万元" />
        <div v-if="a.entityPerf.length" class="ck-perf-foot">
          毛利率最高：<b>{{ maxMargin.name }}</b>
          <span class="mono">{{ maxMargin.grossMargin }}%</span>
          ｜研发投入比最高：<b>{{ maxRd.name }}</b>
          <span class="mono">{{ maxRd.rdRatio }}%</span>
        </div>
      </section>

      <!-- 10. 拜访行动 -->
      <section class="card ck-panel ck-p6">
        <div class="card-head">
          <div class="card-title">
            <AppIcon name="flag" :size="15" />
            <span>拜访行动（visit_record）</span>
          </div>
          <div class="card-actions">
            <span class="tag">共 {{ a.visits.total }} 次</span>
          </div>
        </div>
        <div class="ck-split ck-split--top">
          <CockpitDonut
            :items="a.visits.methods"
            :size="118"
            :thickness="11"
            :center-value="a.visits.total"
            center-sub="拜访次数"
          />
          <div class="ck-legend">
            <div v-for="m in a.visits.methods" :key="m.label" class="ck-legend-row">
              <i :style="{ background: m.color }"></i>
              <span class="ck-legend-label">{{ m.label }}</span>
              <span class="ck-legend-val mono">{{ m.count }}</span>
              <span class="ck-legend-pct mono">{{ pctOf(m.count, a.visits.total) }}%</span>
            </div>
          </div>
        </div>
        <div class="ck-block-label" style="margin-top: 12px">近六个月拜访频次</div>
        <CockpitColumns :items="a.visits.months" :height="118" />
      </section>

      <!-- 11. 库表落库热度 -->
      <section class="card ck-panel ck-p7">
        <div class="card-head">
          <div class="card-title">
            <AppIcon name="database" :size="15" />
            <span>数据落库热度（行数 Top 12）</span>
          </div>
          <div class="card-actions">
            <span class="tag">{{ a.db.withData }} 张表有数据</span>
          </div>
        </div>
        <CockpitBars :items="heatBars" unit="行" />
      </section>

      <!-- 12. 客户预算 -->
      <section class="card ck-panel ck-p5">
        <div class="card-head">
          <div class="card-title">
            <AppIcon name="coin" :size="15" />
            <span>客户安全预算（customer_budget）</span>
          </div>
          <div class="card-actions">
            <span v-if="a.budget.totals.years.length" class="tag">{{ a.budget.totals.years.join(' / ') }}</span>
          </div>
        </div>
        <div class="ck-split">
          <CockpitDonut
            :items="budgetSplit"
            :size="132"
            :thickness="12"
            :center-value="secShare + '%'"
            center-sub="安全预算占比"
          />
          <dl class="kv ck-kv">
            <dt>IT 预算</dt>
            <dd class="mono">{{ a.budget.totals.it.toLocaleString('zh-CN') }} 万元</dd>
            <dt>其中安全</dt>
            <dd class="mono">{{ a.budget.totals.sec.toLocaleString('zh-CN') }} 万元</dd>
            <dt>安全已执行</dt>
            <dd class="mono">{{ a.budget.totals.secExec.toLocaleString('zh-CN') }} 万元</dd>
            <dt>执行率</dt>
            <dd class="mono">{{ a.budget.execRate }}%</dd>
          </dl>
        </div>
        <div class="bar" style="margin-top: 10px">
          <i :style="{ width: a.budget.execRate + '%' }"></i>
        </div>
        <div class="card-sub" style="margin-top: 6px">
          覆盖 {{ a.budget.customers }} 家客户的预算记录 · 已执行 / 预算 = {{ a.budget.execRate }}%
        </div>
      </section>
    </div>

    <div class="ck-foot">
      数据来源：{{ SOURCE_TABLES.join(' · ') }}（共 {{ SOURCE_TABLES.length }} 张表） ·
      图表为手绘 SVG，未引入任何图表库
    </div>
  </div>
</template>

<script setup>
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import AppIcon from './AppIcon.vue'
import CockpitDonut from './cockpit/CockpitDonut.vue'
import CockpitBars from './cockpit/CockpitBars.vue'
import CockpitColumns from './cockpit/CockpitColumns.vue'
import CockpitScatter from './cockpit/CockpitScatter.vue'
import CockpitRadar from './cockpit/CockpitRadar.vue'
import { C74_BANDS, MOCK_ANALYTICS, loadAnalytics } from '@/api/analytics'

const props = defineProps({
  online: { type: Boolean, default: false },
})

const SOURCE_TABLES = [
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

/* ---------------- 数据 ---------------- */
const a = ref(MOCK_ANALYTICS)
const live = ref(false)
const loading = ref(false)
const errorText = ref('')

async function loadAll() {
  if (!props.online) {
    a.value = MOCK_ANALYTICS
    live.value = false
    errorText.value = ''
    return
  }
  loading.value = true
  try {
    a.value = await loadAnalytics()
    live.value = true
    errorText.value = ''
  } catch (e) {
    a.value = MOCK_ANALYTICS
    live.value = false
    errorText.value = e.message
  } finally {
    loading.value = false
  }
}

/* ---------------- 时钟 ---------------- */
const clockTime = ref('')
const clockDate = ref('')
let timer = null
const WEEK = ['周日', '周一', '周二', '周三', '周四', '周五', '周六']

function tickClock() {
  const d = new Date()
  const p = (n) => String(n).padStart(2, '0')
  clockTime.value = `${p(d.getHours())}:${p(d.getMinutes())}:${p(d.getSeconds())}`
  clockDate.value = `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())} ${WEEK[d.getDay()]}`
}

/* ---------------- 展示派生 ---------------- */
const c74Class = (band) => `c74--${band || 'none'}`
const bandLabel = (band) => (C74_BANDS.find((b) => b.key === band) || C74_BANDS[4]).label
const pctOf = (n, total) => (total ? Math.round((Number(n) || 0) / total * 100) : 0)

/** 环形图统一吃 { label, value, color } —— 聚合结果里这些字段叫 count，这里做一次适配 */
const bandDonut = computed(() =>
  a.value.c74111.bands.map((b) => ({ label: b.label, value: b.count, color: b.color })),
)
const attitudeDonut = computed(() =>
  a.value.attitude.map((r) => ({ label: r.label, value: r.count, color: r.color })),
)
const methodDonut = computed(() =>
  a.value.visits.methods.map((m) => ({ label: m.label, value: m.count, color: m.color })),
)
const entityBars = computed(() =>
  a.value.entityTypes.map((e) => ({ label: e.label, value: e.count, color: e.color })),
)

const levelBars = computed(() =>
  a.value.levels
    .filter((l) => l.count > 0 || l.budget > 0)
    .map((l, i) => ({ label: l.label, value: l.budget, color: `var(--n-${['customer', 'qax', 'op', 'clue'][i] || 'person'})` })),
)

const budgetBars = computed(() =>
  a.value.budgetRank.map((p) => ({
    label: p.name,
    value: p.budget,
    color: (C74_BANDS.find((b) => b.key === p.band) || C74_BANDS[4]).color,
  })),
)

const industryBars = computed(() =>
  a.value.industry.map((it, i) => ({
    label: it.label,
    value: it.count,
    color: `var(--n-${['customer', 'qax', 'partner', 'competitor', 'op', 'person', 'clue', 'project'][i] || 'person'})`,
  })),
)

const regionBars = computed(() =>
  a.value.regions.map((it) => ({ label: it.label, value: it.count, color: 'var(--ac)' })),
)

const heatBars = computed(() =>
  a.value.tableHeat.map((t) => ({ label: t.table, value: t.rows, color: 'var(--ac)' })),
)

const perfBars = computed(() =>
  a.value.entityPerf.map((e) => ({
    label: `${e.name}（${e.year}）`,
    value: e.revenue,
    color: e.type === 'competitor' ? 'var(--n-competitor)' : 'var(--n-partner)',
  })),
)

const budgetSplit = computed(() => {
  const { it, sec } = a.value.budget.totals
  return [
    { label: '安全预算', value: sec, color: 'var(--ac)' },
    { label: '其他 IT', value: Math.max(0, it - sec), color: 'var(--tx-3)' },
  ]
})

const secShare = computed(() => {
  const { it, sec } = a.value.budget.totals
  return it > 0 ? Math.round((sec / it) * 100) : 0
})

/** 7 + 4 + 1 三块小计（按满分加权到同一口径展示） */
const groupScores = computed(() => {
  const blocks = a.value.c74111.blocks
  const pick = (g) => blocks.filter((b) => b.group === g)
  const wrap = (label, g, max, color) => {
    const rows = pick(g)
    const avg = rows.reduce((s, r) => s + r.avg, 0)
    return { label, g, avg, max, color }
  }
  return [
    wrap('7 Clears', 'clear', 40, 'var(--c74-b)'),
    wrap('4 Priorities', 'priority', 40, 'var(--c74-c)'),
    wrap('1 Key', 'key', 20, 'var(--c74-a)'),
  ]
})

const maxMargin = computed(() => {
  const rows = a.value.entityPerf
  if (!rows.length) return { name: '—', grossMargin: 0 }
  return rows.reduce((best, r) => (r.grossMargin > best.grossMargin ? r : best), rows[0])
})
const maxRd = computed(() => {
  const rows = a.value.entityPerf
  if (!rows.length) return { name: '—', rdRatio: 0 }
  return rows.reduce((best, r) => (r.rdRatio > best.rdRatio ? r : best), rows[0])
})

/* ---------------- 生命周期 ---------------- */
watch(() => props.online, loadAll)

onMounted(() => {
  tickClock()
  timer = setInterval(tickClock, 1000)
  loadAll()
})

onBeforeUnmount(() => {
  if (timer) clearInterval(timer)
})

defineExpose({ loadAll })
</script>

<style scoped>
/* ---------- 顶栏 ---------- */
.ck-bar {
  display: flex;
  align-items: flex-end;
  gap: 18px;
  flex-wrap: wrap;
  padding: 14px 16px;
  border: 1px solid var(--bd);
  border-radius: var(--r-card);
  background: var(--bg-surface);
  background-image: var(--card-bg-image, none);
  box-shadow: var(--sh-card);
  position: relative;
  overflow: hidden;
}
.ck-bar::before {
  /* 控制台的扫描网格底纹 */
  content: '';
  position: absolute;
  inset: 0;
  pointer-events: none;
  background-image: linear-gradient(var(--hairline) 1px, transparent 1px),
    linear-gradient(90deg, var(--hairline) 1px, transparent 1px);
  background-size: 34px 34px;
  opacity: 0.28;
  mask-image: linear-gradient(90deg, #000, transparent 78%);
  -webkit-mask-image: linear-gradient(90deg, #000, transparent 78%);
}
.ck-bar-main {
  position: relative;
  min-width: 0;
}
.ck-eyebrow {
  font-family: var(--ff-mono);
  font-size: 10px;
  letter-spacing: 0.22em;
  color: var(--tx-3);
  text-transform: uppercase;
}
.ck-title {
  margin: 4px 0 0;
  font-family: var(--ff-display);
  font-weight: var(--fw-display, 600);
  letter-spacing: var(--ls-display, 0);
  font-size: 21px;
  color: var(--tx-1);
}
.ck-sub {
  margin-top: 6px;
  font-size: 11.5px;
  color: var(--tx-3);
  line-height: 1.7;
}
.ck-sub b {
  color: var(--tx-1);
  font-family: var(--ff-mono);
  font-variant-numeric: tabular-nums;
}
.ck-bar-side {
  position: relative;
  margin-left: auto;
  display: flex;
  align-items: center;
  gap: 14px;
  flex-wrap: wrap;
}
.ck-clock {
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  line-height: 1.15;
}
.ck-clock-t {
  font-size: 22px;
  font-weight: 700;
  color: var(--tx-1);
  letter-spacing: 0.02em;
}
.ck-clock-d {
  font-size: 10.5px;
  color: var(--tx-3);
}
.ck-status {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  font-size: 11px;
  color: var(--tx-2);
}
.ck-led {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  display: inline-block;
}
.ck-led.is-live {
  background: var(--ok);
  box-shadow: 0 0 0 0 color-mix(in srgb, var(--ok) 60%, transparent);
  animation: ck-pulse 2s var(--ease, ease-out) infinite;
}
.ck-led.is-mock {
  background: var(--warn);
}
@keyframes ck-pulse {
  0% {
    box-shadow: 0 0 0 0 color-mix(in srgb, var(--ok) 55%, transparent);
  }
  70% {
    box-shadow: 0 0 0 7px transparent;
  }
  100% {
    box-shadow: 0 0 0 0 transparent;
  }
}

/* ---------- KPI 带 ---------- */
.ck-kpis {
  display: grid;
  grid-template-columns: repeat(6, minmax(0, 1fr));
  gap: 12px;
  margin-top: 14px;
}
.ck-kpi {
  position: relative;
  padding: 12px 13px 11px;
  border: 1px solid var(--bd);
  border-radius: var(--r-card);
  background: var(--bg-surface);
  background-image: var(--card-bg-image, none);
  box-shadow: var(--sh-card);
  overflow: hidden;
  transition: transform var(--dur) var(--ease), border-color var(--dur) var(--ease);
}
.ck-kpi::before {
  content: '';
  position: absolute;
  left: 0;
  top: 0;
  width: 100%;
  height: 2px;
  background: var(--ck-hue, var(--ac));
  opacity: 0.9;
}
.ck-kpi:hover {
  transform: var(--lift-card, none);
  border-color: var(--bd-hover);
}
.ck-kpi--ac {
  --ck-hue: var(--ac);
}
.ck-kpi--ac2 {
  --ck-hue: var(--ac-2, var(--ac));
}
.ck-kpi--ok {
  --ck-hue: var(--ok);
}
.ck-kpi--warn {
  --ck-hue: var(--warn);
}
.ck-kpi--info {
  --ck-hue: var(--info);
}
.ck-kpi-top {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 6px;
  color: var(--tx-3);
}
.ck-kpi-name {
  font-size: 11px;
  color: var(--tx-2);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.ck-kpi-num {
  display: flex;
  align-items: baseline;
  gap: 4px;
  margin-top: 6px;
}
.ck-kpi-num span {
  font-size: 23px;
  font-weight: 700;
  line-height: 1.1;
  color: var(--tx-1);
  font-variant-numeric: tabular-nums;
}
.ck-kpi-num em {
  font-style: normal;
  font-size: 10.5px;
  color: var(--tx-3);
}
.ck-kpi-foot {
  margin-top: 5px;
  font-size: 10.5px;
  color: var(--tx-3);
  line-height: 1.5;
  min-height: 16px;
}

/* ---------- 网格 ---------- */
.ck-grid {
  display: grid;
  grid-template-columns: repeat(12, minmax(0, 1fr));
  gap: 14px;
  margin-top: 14px;
}
.ck-p4 {
  grid-column: span 4;
}
.ck-p5 {
  grid-column: span 5;
}
.ck-p6 {
  grid-column: span 6;
}
.ck-p7 {
  grid-column: span 7;
}

/* 面板四角标记：驾驶舱味的细节，不影响布局 */
.ck-panel {
  position: relative;
}
.ck-panel::before,
.ck-panel::after {
  content: '';
  position: absolute;
  width: 9px;
  height: 9px;
  border: 1px solid var(--bd-accent);
  pointer-events: none;
  opacity: 0.75;
}
.ck-panel::before {
  left: -1px;
  top: -1px;
  border-right: 0;
  border-bottom: 0;
  border-top-left-radius: var(--r-card);
}
.ck-panel::after {
  right: -1px;
  bottom: -1px;
  border-left: 0;
  border-top: 0;
  border-bottom-right-radius: var(--r-card);
}

.ck-block-label {
  font-size: 10.5px;
  letter-spacing: 0.06em;
  color: var(--tx-3);
  margin-bottom: 8px;
}
.ck-note {
  font-size: 10.5px;
  line-height: 1.7;
  color: var(--tx-3);
}
.ck-note b {
  color: var(--tx-1);
}
.ck-note code,
.ck-foot code {
  font-family: var(--ff-mono);
  font-size: 10px;
  padding: 1px 4px;
  border-radius: 3px;
  background: var(--bg-elevated);
  color: var(--tx-2);
}

.ck-split {
  display: flex;
  align-items: center;
  gap: 14px;
  flex-wrap: wrap;
}
.ck-split--top {
  align-items: flex-start;
}
.ck-legend {
  flex: 1;
  min-width: 150px;
  display: flex;
  flex-direction: column;
  gap: 7px;
}
.ck-legend-row {
  display: grid;
  grid-template-columns: 9px 1fr auto 30px 32px;
  align-items: center;
  gap: 7px;
  font-size: 11px;
  color: var(--tx-2);
}
.ck-legend-row i {
  width: 9px;
  height: 9px;
  border-radius: 2px;
  display: inline-block;
}
.ck-legend-label {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.ck-legend-range {
  font-size: 10px;
  color: var(--tx-3);
}
.ck-legend-val {
  text-align: right;
  font-variant-numeric: tabular-nums;
  color: var(--tx-1);
  font-weight: 600;
}
.ck-legend-pct {
  text-align: right;
  font-size: 10px;
  color: var(--tx-3);
}

/* 风险榜小表 */
.ck-tbl {
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.ck-tbl-row {
  display: grid;
  grid-template-columns: 1.6fr 1fr auto auto;
  align-items: center;
  gap: 8px;
  font-size: 11px;
  padding: 5px 7px;
  border-radius: var(--r-chip, 8px);
  background: var(--bg-surface-2);
  border: 1px solid var(--bd);
}
.ck-tbl-name,
.ck-tbl-cust {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.ck-tbl-name {
  color: var(--tx-1);
}
.ck-tbl-cust {
  color: var(--tx-3);
  font-size: 10.5px;
}
.ck-tbl-band {
  font-size: 10.5px;
  color: var(--tx-3);
}

.ck-mini {
  display: flex;
  gap: 12px;
  flex-wrap: wrap;
  justify-content: center;
  margin-top: 6px;
}
.ck-mini-item {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  font-size: 10.5px;
  color: var(--tx-3);
}
.ck-mini-item i {
  width: 8px;
  height: 8px;
  border-radius: 2px;
  display: inline-block;
}
.ck-mini-item b {
  color: var(--tx-1);
  font-variant-numeric: tabular-nums;
}

.ck-kv {
  flex: 1;
  min-width: 150px;
}
.ck-perf-foot {
  margin-top: 11px;
  font-size: 10.5px;
  color: var(--tx-3);
  line-height: 1.7;
}
.ck-perf-foot b {
  color: var(--tx-2);
}

.ck-foot {
  margin-top: 16px;
  padding-top: 12px;
  border-top: 1px solid var(--hairline);
  font-size: 10.5px;
  color: var(--tx-3);
  line-height: 1.7;
}

/* ---------- 响应式 ---------- */
@media (max-width: 1360px) {
  .ck-kpis {
    grid-template-columns: repeat(3, minmax(0, 1fr));
  }
}
@media (max-width: 1240px) {
  .ck-p4,
  .ck-p5 {
    grid-column: span 6;
  }
  .ck-p7 {
    grid-column: span 12;
  }
}
@media (max-width: 900px) {
  .ck-kpis {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
  .ck-p4,
  .ck-p5,
  .ck-p6,
  .ck-p7 {
    grid-column: span 12;
  }
  .ck-bar-side {
    margin-left: 0;
  }
}
</style>
