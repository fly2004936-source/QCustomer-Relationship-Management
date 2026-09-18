<template>
  <!--
    通用数据管理页
    完全由后端元数据驱动：显示哪些列、用什么控件编辑、外键怎么选，全部读 /meta/tables/{table}，
    所以以后 DDL 加了字段，这里不用改代码就能编辑。
  -->
  <div class="app-inner">
    <div class="page-head">
      <div class="page-head-main">
        <div class="row" style="gap: 8px">
          <h2 class="sf-h2" style="font-size: 18px">{{ label }}</h2>
          <span class="tag tag--ac mono">{{ table }}</span>
          <span v-if="category" class="tag">{{ category }}</span>
        </div>
        <div class="card-sub" style="margin-top: 4px">
          {{ meta ? `${meta.columns.length} 列 · 主键 ${meta.primaryKey.join(', ')}` : '加载元数据…' }}
        </div>
      </div>
      <div class="page-head-actions">
        <button class="btn" :disabled="!canWrite" @click="openCreate">
          <AppIcon name="plus" :size="14" />
          新建行
        </button>
        <button class="btn btn--ghost" @click="reload">
          <AppIcon name="refresh" :size="14" />
          刷新
        </button>
      </div>
    </div>

    <div v-if="!online" class="banner banner--warn">
      <AppIcon name="bell" :size="14" />
      后端离线，数据管理页需要真实数据库连接。
    </div>

    <div class="crud-bar">
      <input
        v-model="q"
        class="input"
        placeholder="关键字搜索（对全表文本列模糊匹配）"
        @keyup.enter="search"
      />
      <button class="btn btn--sm" @click="search">
        <AppIcon name="search" :size="13" />
        搜索
      </button>
      <button v-if="q" class="btn btn--sm btn--ghost" @click="clearSearch">清除</button>

      <button
        class="btn btn--sm btn--ghost"
        :class="{ 'is-on': panelOpen }"
        :title="`按列筛选（${meta?.columns.length ?? 0} 列全部可筛）`"
        @click="panelOpen = !panelOpen"
      >
        <AppIcon name="filter" :size="13" />
        条件筛选
        <span v-if="appliedFilters.length" class="tag tag--ac">{{ appliedFilters.length }}</span>
      </button>

      <label class="switch" style="margin-left: auto">
        <input v-model="wide" type="checkbox" />
        <span>紧凑列宽</span>
      </label>
    </div>

    <!--
      条件筛选器
      目标是「所有字段都能单独筛、也能联合筛」：
        · 列下拉列出该表**全部列**（不止表格里显示的那 12 列），按列类型给操作符
        · 列头那个小漏斗点一下就为这一列加一条条件
        · 多条件之间可切换「全部满足 AND / 任一满足 OR」
        · 底部预览真实请求串，避免"界面看着筛了、实际没筛"
    -->
    <div v-if="panelOpen" class="filter-panel">
      <div class="filter-panel-head">
        <span class="card-sub">
          语法 <code class="mono">filter[列名__操作符]=值</code>；共
          <b class="mono">{{ meta?.columns.length ?? 0 }}</b> 列可选，
          <b class="mono">{{ (meta?.columns ?? []).filter((c) => c.references).length }}</b> 个外键，
          <b class="mono">{{ (meta?.columns ?? []).filter((c) => c.json).length }}</b> 个 JSON 列
        </span>
        <div class="seg">
          <button class="seg-btn" :class="{ 'is-on': filterLogic === 'and' }" @click="filterLogic = 'and'">
            全部满足 AND
          </button>
          <button class="seg-btn" :class="{ 'is-on': filterLogic === 'or' }" @click="filterLogic = 'or'">
            任一满足 OR
          </button>
        </div>
      </div>

      <div v-for="(f, i) in filters" :key="i" class="filter-row">
        <select v-model="f.column" class="select filter-col" @change="onColumnChange(f)">
          <option v-for="c in meta?.columns ?? []" :key="c.name" :value="c.name">
            {{ c.name }}<template v-if="c.primaryKey"> *</template
            ><template v-else-if="c.references"> ↗{{ c.references.table }}</template
            ><template v-else-if="c.json"> {}</template>
          </option>
        </select>

        <select v-model="f.op" class="select filter-op">
          <option v-for="op in operatorsFor(colOf(f.column))" :key="op" :value="op">
            {{ opLabel(op) }} · {{ op }}
          </option>
        </select>

        <select
          v-if="!VALUELESS_OPS.has(f.op) && inputKindOf(colOf(f.column)) === 'enum'"
          v-model="f.value"
          class="select filter-val"
        >
          <option value="">选择枚举值…</option>
          <option v-for="v in colOf(f.column).enum" :key="v" :value="v">{{ v }}</option>
        </select>

        <select
          v-else-if="!VALUELESS_OPS.has(f.op) && inputKindOf(colOf(f.column)) === 'boolean'"
          v-model="f.value"
          class="select filter-val"
        >
          <option value="1">是 (1)</option>
          <option value="0">否 (0)</option>
        </select>

        <select
          v-else-if="!VALUELESS_OPS.has(f.op) && inputKindOf(colOf(f.column)) === 'fk'"
          v-model="f.value"
          class="select filter-val"
        >
          <option value="">选择外键行…</option>
          <option v-for="o in fkOptions[f.column] ?? []" :key="o.id" :value="o.id">
            {{ o.id }} · {{ o.label }}
          </option>
        </select>

        <input
          v-else-if="!VALUELESS_OPS.has(f.op)"
          v-model="f.value"
          class="input filter-val"
          :type="inputKindOf(colOf(f.column)) === 'date' ? 'date' : 'text'"
          :placeholder="placeholderOf(f)"
          @keyup.enter="applyFilters"
        />

        <span v-else class="filter-novalue mono">{{ f.op }}（无需填值）</span>

        <button class="icon-btn" style="width: 26px; height: 26px" title="移除该条件" @click="filters.splice(i, 1)">
          <AppIcon name="close" :size="13" />
        </button>
      </div>

      <div class="filter-actions">
        <button class="btn btn--sm btn--ghost" @click="addFilter()">
          <AppIcon name="plus" :size="13" />
          添加条件
        </button>
        <button class="btn btn--sm btn--primary" @click="applyFilters">
          <AppIcon name="check" :size="13" />
          应用筛选
        </button>
        <button v-if="appliedFilters.length" class="btn btn--sm btn--ghost" @click="resetFilters">清空全部</button>
        <button
          class="btn btn--sm btn--ghost"
          :disabled="!rows.length"
          title="取当前页里非空字段最多的那一行，为这张表的每一列各生成一条等值条件（一次覆盖全部字段）"
          @click="fillAllFromFirstRow"
        >
          全列等值（取最全的一行）
        </button>
      </div>

      <div v-if="appliedFilters.length" class="filter-preview">
        <span class="card-sub">实际请求</span>
        <code class="mono">GET /{{ table }}{{ appliedQueryString }}</code>
      </div>
    </div>

    <!-- 已生效条件的缩略显示 -->
    <div v-if="appliedFilters.length" class="filter-chips">
      <span class="card-sub">{{ appliedLogic === 'or' ? '任一满足' : '全部满足' }}</span>
      <span v-for="(f, i) in appliedFilters" :key="i" class="chip mono">
        {{ f.column }}<template v-if="f.op && f.op !== 'eq'">__{{ f.op }}</template> =
        {{ VALUELESS_OPS.has(f.op) ? f.op : f.value }}
      </span>
      <button class="btn btn--sm btn--ghost" style="margin-left: auto" @click="resetFilters">清空条件</button>
    </div>

    <div class="crud-meta">
      <span>共 <b class="mono">{{ total }}</b> 行</span>
      <span>第 {{ page }} / {{ Math.max(pageCount, 1) }} 页</span>
      <span v-if="error" style="color: var(--bad-fg, var(--bad))">{{ error }}</span>
    </div>

    <div class="table-wrap">
      <table class="dtable" :class="{ 'is-wide': wide }">
        <thead>
          <tr>
            <th v-for="c in visibleColumns" :key="c.name" :title="`${c.name} · ${c.sqlType}${c.references ? ' → ' + c.references.table : ''}`">
              <span class="th-inner">
                {{ c.name }}
                <span v-if="c.primaryKey" style="color: var(--ac)">*</span>
                <button
                  class="th-filter"
                  :class="{ 'is-on': isFilteredColumn(c.name) }"
                  :title="`筛选 ${c.name}`"
                  @click.stop="filterByColumn(c.name)"
                >
                  <AppIcon name="filter" :size="10" />
                </button>
              </span>
            </th>
            <th class="dtable-actions" style="width: 1%">操作</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="row in rows" :key="pkOf(row)" @dblclick="openEdit(row)">
            <td
              v-for="c in visibleColumns"
              :key="c.name"
              :class="{ 'is-pk': c.primaryKey, 'is-null': display(c, row).null, 'is-json': display(c, row).json }"
              :title="display(c, row).title || display(c, row).text"
            >
              {{ fkLabel(c, row) ?? display(c, row).text }}
            </td>
            <td class="dtable-actions">
              <button class="icon-btn" style="width: 26px; height: 26px" title="编辑" @click="openEdit(row)">
                <AppIcon name="edit" :size="13" />
              </button>
              <button
                class="icon-btn"
                style="width: 26px; height: 26px"
                title="删除"
                @click="removeRow(row)"
              >
                <AppIcon name="trash" :size="13" />
              </button>
            </td>
          </tr>
          <tr v-if="!rows.length && !loading">
            <td :colspan="visibleColumns.length + 1">
              <div class="empty">
                {{ appliedFilters.length || q ? '没有匹配的行，试试放宽筛选条件' : '这张表还没有数据' }}
              </div>
            </td>
          </tr>
        </tbody>
      </table>
    </div>

    <div class="pager">
      <button class="btn btn--sm btn--ghost" :disabled="page <= 1" @click="go(page - 1)">上一页</button>
      <button class="btn btn--sm btn--ghost" :disabled="page >= pageCount" @click="go(page + 1)">下一页</button>
      <div class="pager-mid">
        <span>每页</span>
        <select v-model.number="pageSize" class="select" style="width: 78px; min-height: 30px" @change="go(1)">
          <option :value="20">20</option>
          <option :value="50">50</option>
          <option :value="100">100</option>
          <option :value="200">200</option>
        </select>
      </div>
    </div>

    <!-- 编辑 / 新建 -->
    <div v-if="editing" class="modal-mask" @click.self="editing = null">
      <div class="modal">
        <div class="modal-head">
          <div>
            <div class="modal-title">{{ isNew ? `新建 ${label}` : `编辑 ${label} #${pkOf(editing.row)}` }}</div>
            <div class="modal-sub mono">{{ table }}</div>
          </div>
          <div class="card-actions">
            <button class="icon-btn" style="width: 28px; height: 28px" @click="editing = null">
              <AppIcon name="close" :size="14" />
            </button>
          </div>
        </div>

        <div class="modal-body">
          <div v-if="formError" class="banner banner--err" style="margin-bottom: 12px">{{ formError }}</div>
          <div class="form-grid">
            <div
              v-for="c in meta.columns"
              :key="c.name"
              class="field"
              :class="{ 'field--full': c.json || controlOf(c) === 'text' }"
            >
              <div class="field-head">
                <label class="label" :for="`f-${c.name}`">{{ c.name }}</label>
                <span v-if="c.notNull" class="field-flag">必填</span>
                <span v-if="c.enum" class="field-flag">枚举</span>
                <span v-if="c.json" class="field-flag">JSON</span>
                <span v-if="c.references" class="field-flag">{{ c.references.table }}</span>
              </div>

              <!-- 自增主键只读 -->
              <input
                v-if="controlOf(c) === 'readonly'"
                :id="`f-${c.name}`"
                class="input"
                :value="form[c.name] ?? '（新建时自动生成）'"
                disabled
              />

              <!-- 布尔 -->
              <label v-else-if="controlOf(c) === 'boolean'" class="switch">
                <input v-model="form[c.name]" type="checkbox" />
                <span>{{ form[c.name] ? '是' : '否' }}</span>
              </label>

              <!-- 枚举 -->
              <select v-else-if="controlOf(c) === 'enum'" :id="`f-${c.name}`" v-model="form[c.name]" class="select">
                <option value="">（空）</option>
                <option v-for="v in c.enum" :key="v" :value="v">{{ v }}</option>
              </select>

              <!-- 外键 -->
              <select v-else-if="controlOf(c) === 'fk'" :id="`f-${c.name}`" v-model="form[c.name]" class="select">
                <option value="">（空）</option>
                <option v-for="o in fkOptions[c.name] ?? []" :key="o.id" :value="o.id">
                  {{ o.id }} · {{ o.label }}
                </option>
              </select>

              <!-- JSON / 长文本 -->
              <textarea
                v-else-if="controlOf(c) === 'json'"
                :id="`f-${c.name}`"
                v-model="form[c.name]"
                class="textarea mono"
                spellcheck="false"
                :placeholder="c.json ? 'JSON，如 [&quot;值1&quot;,&quot;值2&quot;]；也可直接写 值1,值2 自动转换' : ''"
              ></textarea>

              <!-- 数字 -->
              <input
                v-else-if="controlOf(c) === 'number'"
                :id="`f-${c.name}`"
                v-model="form[c.name]"
                class="input"
                type="number"
                step="any"
              />

              <input v-else :id="`f-${c.name}`" v-model="form[c.name]" class="input" />
            </div>
          </div>
        </div>

        <div class="modal-foot">
          <button class="btn btn--primary" :disabled="saving" @click="save">
            <AppIcon name="save" :size="14" />
            {{ saving ? '保存中…' : '保存' }}
          </button>
          <button class="btn btn--ghost" @click="editing = null">取消</button>
          <span class="card-sub" style="margin-left: auto">只提交改动的字段（PATCH）</span>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { computed, onMounted, ref, watch } from 'vue'
import AppIcon from './AppIcon.vue'
import { api, toQueryString } from '@/api/client'
import {
  VALUELESS_OPS,
  RANGE_OPS,
  MULTI_OPS,
  clearMetaCache,
  controlOf,
  displayValue,
  inputKindOf,
  loadTableMeta,
  opLabel,
  operatorsFor,
  toForm,
  toSubmit,
} from '@/api/meta'
import { tableLabel, tableCategory } from '@/nav/dataTree'
import { useToast } from '@/composables/useToast'

const props = defineProps({
  table: { type: String, required: true },
  online: { type: Boolean, default: true },
  /** 从网络图「跳转编辑页面」过来时，直接打开这一行的编辑弹窗 */
  focusId: { type: [Number, String], default: null },
  /** 从空态按钮（如「去登记一次拜访」）过来时，直接打开新建表单 */
  createNew: { type: Boolean, default: false },
})

/** 处理完 focusId / createNew 后通知上层清空，否则每次回到本页都会自动弹窗 */
const emit = defineEmits(['focus-opened'])

const toast = useToast()

const meta = ref(null)
const rows = ref([])
const total = ref(0)
const page = ref(1)
const pageSize = ref(20)
const pageCount = ref(1)
const q = ref('')
const appliedQ = ref('')
const loading = ref(false)
const error = ref('')
const wide = ref(false)
const editing = ref(null)
const form = ref({})
const formError = ref('')
const saving = ref(false)
const fkOptions = ref({})

/* ---------------- 条件筛选器状态 ---------------- */

/** 面板是否展开 */
const panelOpen = ref(false)
/** 编辑中的条件（可增删，点「应用筛选」才生效） */
const filters = ref([])
/** 条件之间的逻辑：and = 全部满足，or = 任一满足 */
const filterLogic = ref('and')
/** 已生效的条件快照 —— 改 filters 不会影响当前结果，点「应用」才切过去 */
const appliedFilters = ref([])
const appliedLogic = ref('and')

const label = computed(() => tableLabel(props.table))
const category = computed(() => tableCategory(props.table))
const online = computed(() => props.online)
const canWrite = computed(() => props.online && !!meta.value)
const isNew = computed(() => editing.value?.mode === 'create')

/** 列很多时（如 customer 有十几列）只显示前 12 列，避免横向滚动过长 */
const visibleColumns = computed(() => {
  if (!meta.value) return []
  const cols = meta.value.columns
  if (cols.length <= 12) return cols
  const pk = cols.filter((c) => c.primaryKey)
  const rest = cols.filter((c) => !c.primaryKey)
  const keep = [...pk, ...rest.filter((c) => !c.json).slice(0, 11)]
  return keep
})

function pkOf(row) {
  if (!meta.value) return row?.id
  return meta.value.primaryKey.map((k) => row?.[k]).join('/')
}

const display = (c, row) => displayValue(c, row?.[c.name])

/** 外键显示成可读文字，而不是光秃秃的数字 */
function fkLabel(c, row) {
  if (!c.references) return null
  const v = row?.[c.name]
  if (v === null || v === undefined) return null
  const o = (fkOptions.value[c.name] ?? []).find((x) => String(x.id) === String(v))
  return o ? `${o.label}` : null
}

/* ---------------- 加载 ---------------- */

async function loadMeta() {
  meta.value = null
  try {
    meta.value = await loadTableMeta(props.table)
    await loadFkOptions()
  } catch (e) {
    error.value = `元数据加载失败：${e.message}`
  }
}

/** 为所有外键列拉一次候选值，用于表格显示和表单下拉 */
async function loadFkOptions() {
  const out = {}
  const cols = meta.value?.fkColumns ?? []
  await Promise.all(
    cols.map(async (c) => {
      try {
        const d = await api.list(c.references.table, { page_size: 200, order_by: 'id:asc' })
        const list = Array.isArray(d) ? d : (d?.items ?? [])
        out[c.name] = list.map((r) => ({ id: r.id, label: labelOfRow(r) }))
      } catch {
        out[c.name] = []
      }
    }),
  )
  fkOptions.value = out
}

/** 挑一个最能代表这一行的列做标签 */
function labelOfRow(r) {
  for (const k of ['name', 'title', 'item', 'news_title', 'crm_code', 'code', 'plan_date']) {
    if (r[k]) return String(r[k]).slice(0, 40)
  }
  const firstText = Object.entries(r).find(
    ([k, v]) => k !== 'id' && typeof v === 'string' && v.length && v.length < 60,
  )
  return firstText ? firstText[1] : `#${r.id}`
}

async function loadRows() {
  if (!props.online) return
  loading.value = true
  error.value = ''
  try {
    const params = buildParams(appliedFilters.value, appliedLogic.value)
    params.page = page.value
    const d = await api.list(props.table, params)
    if (Array.isArray(d)) {
      rows.value = d
      total.value = d.length
      pageCount.value = 1
    } else {
      rows.value = d?.items ?? []
      total.value = d?.total ?? rows.value.length
      pageCount.value = d?.page_count ?? 1
    }
  } catch (e) {
    error.value = e.message
    rows.value = []
    total.value = 0
  } finally {
    loading.value = false
  }
}

function reload() {
  clearMetaCache(props.table)
  loadMeta().then(loadRows)
}

function go(p) {
  page.value = Math.min(Math.max(1, p), Math.max(pageCount.value, 1))
  loadRows()
}

function search() {
  appliedQ.value = q.value.trim()
  page.value = 1
  loadRows()
}

function clearSearch() {
  q.value = ''
  appliedQ.value = ''
  page.value = 1
  loadRows()
}

/* ---------------- 条件筛选 ---------------- */

const colOf = (name) => meta.value?.columns.find((c) => c.name === name) ?? null
const needsValue = (op) => !VALUELESS_OPS.has(op)

/** 选一个适合做筛选条件的默认列：优先普通文本/数字列，避开主键与 JSON */
function defaultFilterColumn() {
  const cols = meta.value?.columns ?? []
  return (
    cols.find((c) => !c.primaryKey && !c.json && !c.references && !c.enum) ??
    cols.find((c) => !c.primaryKey && !c.json) ??
    cols[0] ??
    null
  )
}

/**
 * 新增一条条件。
 * 操作符按列类型收窄（比如 JSON 列只给 json_* 系列），
 * 切换列时若原操作符不适用会自动换成 eq —— 否则会拼出后端不接受的组合。
 */
function addFilter(columnName = '', op = '') {
  const col = (columnName && colOf(columnName)) || defaultFilterColumn()
  if (!col) return
  const ops = operatorsFor(col)
  filters.value.push({
    column: col.name,
    op: op && ops.includes(op) ? op : ops.includes('eq') ? 'eq' : ops[0],
    value: '',
  })
}

function onColumnChange(f) {
  const ops = operatorsFor(colOf(f.column))
  if (!ops.includes(f.op)) f.op = ops.includes('eq') ? 'eq' : ops[0]
  f.value = ''
}

/** 点列头小漏斗：为这一列加一条条件并展开面板 */
function filterByColumn(name) {
  panelOpen.value = true
  if (!filters.value.some((f) => f.column === name)) addFilter(name)
}

const isFilteredColumn = (name) => appliedFilters.value.some((f) => f.column === name)

function placeholderOf(f) {
  if (RANGE_OPS.has(f.op)) return '起,止（英文逗号分隔）'
  if (MULTI_OPS.has(f.op)) return '多个值用英文逗号分隔'
  if (f.op === 'json_len_gte') return '最小长度，如 1'
  if (f.op === 'json_eq') return '整段 JSON，如 ["A","B"]'
  if (f.op && f.op.startsWith('json_')) return '裸元素，如 VF（不要加引号）'
  return inputKindOf(colOf(f.column)) === 'date' ? '2026-01-01' : '输入值…'
}

/** 无需填值的操作符（null / notnull）后端要一个占位值，统一给 1 */
function normalizeFilterValue(f) {
  if (!needsValue(f.op)) return '1'
  return String(f.value ?? '').trim()
}

/**
 * 把条件数组编译成后端查询参数。
 *
 * 后端是「**组内 OR、组间 AND**」（API 文档 6.4），所以两种逻辑要映射到不同的语法形态：
 *   · 全部满足 → `and[0][a]=1&and[1][b]=2`：每个条件各自成组，组间 AND。
 *     这里特意用 and[i] 而不是平铺的 `filter[a]=1&filter[b]=2`，
 *     因为平铺时**同名参数会被聚合成 IN 语义**——同一列加两条就会从 AND 静默变成 OR。
 *     分组成 and[i] 后每个条件一个组，同列重复也不会合并。
 *   · 任一满足 → `or[0][a]=1&or[0][b]=2`：所有条件必须塞进**同一个组**才是 OR。
 *     写成 or[0]/or[1] 会变成组间 AND，语义完全相反 —— 这是最容易写错的一处。
 */
function buildParams(list, logic) {
  const params = { page: 1, page_size: pageSize.value, order_by: 'id:desc' }
  if (appliedQ.value) params.q = appliedQ.value

  const usable = list.filter((f) => f.column && colOf(f.column) && (!needsValue(f.op) || normalizeFilterValue(f) !== ''))
  if (!usable.length) return params

  const kv = (f) => [f.op && f.op !== 'eq' ? `${f.column}__${f.op}` : f.column, normalizeFilterValue(f)]

  if (logic === 'or') {
    const group = {}
    for (const f of usable) {
      const [k, v] = kv(f)
      // 同列同操作符出现两次时收集成多值，后端会走 IN 语义，仍然等价于 OR
      if (k in group) group[k] = [].concat(group[k], v)
      else group[k] = v
    }
    params.or = [group]
  } else {
    params.and = usable.map((f) => {
      const [k, v] = kv(f)
      return { [k]: v }
    })
  }
  return params
}

function applyFilters() {
  appliedFilters.value = filters.value.map((f) => ({ ...f }))
  appliedLogic.value = filterLogic.value
  page.value = 1
  loadRows()
}

function resetFilters() {
  filters.value = []
  appliedFilters.value = []
  page.value = 1
  loadRows()
}

/**
 * 为这张表**每一列**各生成一条等值条件，值取自当前页里"数据最全"的那一行。
 *
 * 为什么取最全的一行而不是第 1 行：第 1 行往往只填了必填字段，
 * 用它只能生成两三条条件，起不到"覆盖全部字段"的作用。
 * 取非空字段最多的那一行，一次就能把绝大部分列都带进查询。
 *
 * JSON 列要用 json_contains（传整段 JSON 去 eq 语义不同），这里自动换掉。
 */
function fillAllFromFirstRow() {
  const list = rows.value
  if (!list.length) return
  const score = (r) => meta.value.columns.filter((c) => r[c.name] !== null && r[c.name] !== undefined && r[c.name] !== '').length
  const row = list.reduce((best, r) => (score(r) > score(best) ? r : best), list[0])

  const out = []
  for (const c of meta.value.columns) {
    const v = row[c.name]
    if (v === null || v === undefined || v === '') continue
    if (c.json) {
      const first = Array.isArray(v) ? v[0] : null
      if (first === null) continue
      out.push({ column: c.name, op: 'json_contains', value: String(first) })
    } else if (c.boolean) {
      out.push({ column: c.name, op: 'eq', value: String(Number(v)) })
    } else {
      out.push({ column: c.name, op: 'eq', value: String(v) })
    }
  }
  filters.value = out
  filterLogic.value = 'and'
  toast.ok(`已按第 ${list.indexOf(row) + 1} 行（${out.length} 个非空字段）生成等值条件，点「应用筛选」验证`)
}

/** 把实际发出的请求串显示出来，避免"界面看着筛了、其实没筛" */
const appliedQueryString = computed(() => toQueryString(buildParams(appliedFilters.value, appliedLogic.value)))

/* ---------------- 编辑 ---------------- */

function blankForm() {
  const f = {}
  for (const c of meta.value.columns) {
    f[c.name] = c.boolean ? false : ''
  }
  return f
}

function openCreate() {
  if (!meta.value) return
  form.value = blankForm()
  formError.value = ''
  editing.value = { mode: 'create', row: {} }
}

function openEdit(row) {
  const f = {}
  for (const c of meta.value.columns) f[c.name] = toForm(c, row)
  form.value = f
  formError.value = ''
  editing.value = { mode: 'edit', row }
}

/**
 * 按主键单独取一行再打开弹窗。
 * 不走「在已加载的当前页里找」是因为目标记录可能在别的页/被搜索条件排除，
 * 那样用户会看到"点了没反应"。
 */
async function openFocus(id) {
  if (id === null || id === undefined || id === '' || !meta.value) return
  try {
    const row = await api.byId(props.table, id)
    if (row) openEdit(row)
    else toast.err(`没找到 ${props.table} #${id}`)
  } catch (e) {
    toast.err(`打不开 #${id}：${e.message}`)
  } finally {
    emit('focus-opened')
  }
}

async function save() {
  saving.value = true
  formError.value = ''
  try {
    const payload = {}
    for (const c of meta.value.columns) {
      // 自增主键不提交
      if (c.primaryKey && (c.autoIncrement || isNew.value)) continue
      payload[c.name] = toSubmit(c, form.value[c.name])
    }
    if (isNew.value) {
      const r = await api.create(props.table, payload)
      toast.ok(`已新建 #${r?.id ?? ''}`)
    } else {
      const id = meta.value.primaryKey.map((k) => editing.value.row[k]).join('/')
      await api.patch(props.table, id, payload)
      toast.ok('已保存')
    }
    editing.value = null
    await loadRows()
  } catch (e) {
    // 后端会返回具体原因（枚举非法、外键不存在、JSON 格式错误等），直接展示
    formError.value = e.message
  } finally {
    saving.value = false
  }
}

async function removeRow(row) {
  const id = meta.value.primaryKey.map((k) => row[k]).join('/')
  if (!window.confirm(`确认删除 ${props.table} #${id}？此操作不可撤销。`)) return
  try {
    await api.remove(props.table, id)
    toast.ok(`已删除 #${id}`)
    await loadRows()
  } catch (e) {
    toast.err(`删除失败：${e.message}`)
  }
}

/* ---------------- 生命周期 ---------------- */

watch(
  () => props.table,
  async () => {
    page.value = 1
    q.value = ''
    appliedQ.value = ''
    // 换表必须清筛选条件：列的形态完全不同，留着上一张表的列名会直接 400
    filters.value = []
    appliedFilters.value = []
    filterLogic.value = 'and'
    appliedLogic.value = 'and'
    await loadMeta()
    await loadRows()
  },
)

watch(
  () => props.online,
  (v) => {
    if (v) {
      loadMeta().then(loadRows)
    }
  },
)

watch(
  () => props.focusId,
  (v) => {
    if (v !== null && v !== undefined && v !== '') openFocus(v)
  },
)

/** 从空态按钮进来时直接开新建表单（「去登记一次拜访」之类） */
function openCreateNew() {
  if (!props.createNew || !meta.value) return
  openCreate()
  emit('focus-opened')
}

onMounted(async () => {
  await loadMeta()
  await loadRows()
  if (props.focusId !== null && props.focusId !== undefined && props.focusId !== '') {
    await openFocus(props.focusId)
  } else {
    openCreateNew()
  }
})
</script>
