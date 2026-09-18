<template>
  <div class="og-wrap" :style="{ '--og-h': height + 'px' }" ref="wrapEl">
    <svg
      ref="svgEl"
      class="og-svg"
      :class="{ 'is-panning': dragging }"
      :viewBox="`0 0 ${vw} ${vh}`"
      @pointerdown="onDown"
      @wheel.prevent="onWheel"
      role="img"
      :aria-label="`客户关系网络与组织架构图，共 ${visibleNodes.length} 个节点`"
    >
      <!-- 背景网格 -->
      <rect x="0" y="0" :width="vw" :height="vh" :style="{ fill: 'transparent' }" />
      <g :transform="`translate(${view.x},${view.y}) scale(${view.k})`">
        <!-- 分区标题 -->
        <g>
          <text
            v-for="z in zoneLabels"
            :key="z.id"
            class="og-group-label"
            :x="z.x"
            :y="z.y"
            text-anchor="middle"
          >
            {{ z.text }}
          </text>
        </g>

        <!-- 连线 -->
        <g>
          <path v-for="e in laidEdges" :key="e.id" class="og-edge" :class="e.cls" :d="e.d" />
        </g>

        <!-- 节点 -->
        <g>
          <g
            v-for="n in laid"
            :key="n.id"
            class="og-node"
            :class="{ 'is-selected': selectedId === n.id }"
            :transform="`translate(${n.x},${n.y})`"
            :data-node-id="n.id"
            :data-kind="n.kind"
            @click="onNodeClick(n)"
          >
            <!-- 客户 / 我方 等锚点的外圈 -->
            <circle
              v-if="n.kind === 'qax' || n.kind === 'customer'"
              :r="radius(n) + 6"
              fill="none"
              :style="{ stroke: fillOf(n), opacity: 0.45 }"
              stroke-width="1.2"
              stroke-dasharray="2 4"
            />
            <!-- 导师环 -->
            <circle
              v-if="n.mentor"
              :r="radius(n) + 4"
              fill="none"
              :style="{ stroke: 'var(--n-mentor)' }"
              stroke-width="1.6"
            />
            <circle
              class="og-node-body"
              :r="radius(n)"
              :style="{ fill: fillOf(n), stroke: strokeOf(n) }"
              :stroke-width="strokeW(n)"
              :stroke-dasharray="n.kind === 'competitor' ? '4 3' : '0'"
            />
            <!-- 中心标记 -->
            <text
              v-if="n.kind === 'c74111'"
              class="og-badge"
              y="4"
              text-anchor="middle"
              :style="{ fill: 'var(--ac-on, #fff)' }"
              font-size="11"
            >
              {{ n.score ?? '—' }}
            </text>
            <text
              v-else-if="n.kind !== 'person'"
              class="og-badge"
              y="4.5"
              text-anchor="middle"
              :style="{ fill: 'var(--og-badge-fg, #fff)' }"
              font-size="10"
            >
              {{ shortMark(n) }}
            </text>

            <!-- 名称 -->
            <text class="og-label" :y="radius(n) + 16" text-anchor="middle">{{ n.label }}</text>
            <text v-if="n.sub" class="og-sub" :y="radius(n) + 30" text-anchor="middle">{{ n.sub }}</text>

            <!-- ADUR / 团队角色 -->
            <text
              v-if="roleMark(n)"
              class="og-badge"
              :y="-radius(n) - 7"
              text-anchor="middle"
              :style="{ fill: roleColor(n) }"
              font-size="10"
            >
              {{ roleMark(n) }}
            </text>

            <!-- 态度（显示成 × − = + ※） -->
            <text
              v-if="n.attitude"
              class="og-badge"
              :x="radius(n) + 8"
              y="4"
              text-anchor="middle"
              :style="{ fill: attitudeColor(n.attitude) }"
              font-size="12"
            >
              {{ attChar(n.attitude) }}
            </text>

            <!-- 展开 / 收起 -->
            <g
              v-if="hasChildren(n.id)"
              class="og-toggle-hit"
              :transform="`translate(0,${radius(n) + 1})`"
              @click.stop="toggle(n.id)"
              role="button"
              :aria-label="collapsed.has(n.id) ? '展开下级' : '收起下级'"
            >
              <circle class="og-toggle-bg" r="8.5" />
              <line class="og-toggle-sign" x1="-3.6" y1="0" x2="3.6" y2="0" />
              <line v-if="collapsed.has(n.id)" class="og-toggle-sign" x1="0" y1="-3.6" x2="0" y2="3.6" />
              <text class="og-count" :y="20" text-anchor="middle">{{ descendantCount(n.id) }}</text>
            </g>
          </g>
        </g>
      </g>
    </svg>

    <!-- 工具条 -->
    <div class="og-toolbar">
      <button class="og-tb" title="放大" @click="zoomBy(1.22)"><AppIcon name="plus" :size="15" /></button>
      <button class="og-tb" title="缩小" @click="zoomBy(0.82)"><AppIcon name="minus" :size="15" /></button>
      <button class="og-tb" title="适配全部" @click="fit"><AppIcon name="fit" :size="15" /></button>
      <button class="og-tb" title="全部展开" @click="expandAll"><AppIcon name="expand" :size="15" /></button>
      <button class="og-tb" title="全部收起" @click="collapseAll"><AppIcon name="collapse" :size="15" /></button>
      <button class="og-tb" title="重置视图" @click="reset"><AppIcon name="refresh" :size="15" /></button>
    </div>

    <!-- 提示 -->
    <div class="og-hint">
      <AppIcon name="search" :size="12" />
      <span>点击节点查看详情 · 点击圆形 +/− 展开收起 · 滚轮缩放 · 拖拽平移</span>
    </div>

    <!-- 详情面板 -->
    <div v-if="selected" class="og-detail">
      <div class="og-detail-head">
        <div>
          <div class="og-detail-title">{{ selected.label }}</div>
          <div class="sf-small" style="margin-top: 2px">{{ sourceText(selected) }}</div>
        </div>
        <button class="og-detail-close" title="关闭" @click="selectedId = null">
          <AppIcon name="close" :size="15" />
        </button>
      </div>

      <dl class="og-kv">
        <template v-for="(v, k) in detailPairs" :key="k">
          <dt>{{ k }}</dt>
          <dd>{{ v }}</dd>
        </template>
      </dl>

      <!-- C74111 明细 -->
      <template v-if="selected.c74111">
        <div class="sf-hairline"></div>
        <div class="og-mini">
          <div v-for="row in diagRows" :key="row.code" class="og-mini-row">
            <span class="sf-mono">{{ row.code }}</span>
            <span class="sf-bar"><i :style="{ width: pct(row.score, row.max) + '%' }"></i></span>
            <span class="sf-mono" style="text-align: right">{{ row.score }}/{{ row.max }}</span>
          </div>
        </div>
      </template>

      <div class="sf-hairline"></div>
      <div style="display: flex; gap: 6px; flex-wrap: wrap">
        <span class="sf-tag sf-tag--ac">{{ zoneText(selected) }}</span>
        <span v-if="selected.attitude" class="sf-tag">态度 {{ selected.attitude }}</span>
        <span v-if="selected.adur" class="sf-tag">ADUR {{ selected.adur }}</span>
        <span v-if="selected.teamRole" class="sf-tag">我方 {{ selected.teamRole }}</span>
        <span v-if="selected.mentor" class="sf-tag sf-tag--warn">导师</span>
      </div>
    </div>

    <div v-if="!visibleNodes.length" class="og-empty">暂无节点，请选择要展示的对象</div>
  </div>
</template>

<script setup>
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import AppIcon from './AppIcon.vue'
import { attChar } from '@/api/graphData'

const props = defineProps({
  nodes: { type: Array, default: () => [] },
  legend: { type: Array, default: () => [] },
  height: { type: Number, default: 520 },
})

const emit = defineEmits(['select'])

/* ---------------- 常量 ---------------- */
const LEAF_GAP = 108
const LEVEL_GAP = 94
const PAD = 48
const ZONE_TOP_Y = 46
const ZONE_ANCHOR_Y = 158
const ZONE_TREE_Y = 268
const BELT_GAP = 110

const wrapEl = ref(null)
const svgEl = ref(null)
const vw = ref(1000)
const vh = ref(520)

/* ---------------- 展开 / 收起 ---------------- */
const collapsed = ref(new Set())
const selectedId = ref(null)

const byId = computed(() => new Map(props.nodes.map((n) => [n.id, n])))
const kidsMap = computed(() => {
  const m = new Map()
  for (const n of props.nodes) {
    if (n.parent && byId.value.has(n.parent) && n.parent !== n.id) {
      if (!m.has(n.parent)) m.set(n.parent, [])
      m.get(n.parent).push(n)
    }
  }
  return m
})
const hasChildren = (id) => (kidsMap.value.get(id)?.length ?? 0) > 0

function descendantCount(id) {
  let total = 0
  const walk = (pid) => {
    for (const c of kidsMap.value.get(pid) ?? []) {
      total += 1
      walk(c.id)
    }
  }
  walk(id)
  return total
}

/** 判断节点是否被任一祖先收起 */
const hiddenSet = computed(() => {
  const hidden = new Set()
  const walk = (id) => {
    for (const c of kidsMap.value.get(id) ?? []) {
      hidden.add(c.id)
      walk(c.id)
    }
  }
  for (const id of collapsed.value) walk(id)
  return hidden
})

const visibleNodes = computed(() => props.nodes.filter((n) => !hiddenSet.value.has(n.id)))

function toggle(id) {
  const next = new Set(collapsed.value)
  if (next.has(id)) next.delete(id)
  else next.add(id)
  collapsed.value = next
}
function expandAll() {
  collapsed.value = new Set()
}
function collapseAll() {
  // 只收起「有上级」的节点，保留锚点/客户层展开，避免整图消失
  const childIds = new Set(props.nodes.filter((n) => n.parent).map((n) => n.id))
  const next = new Set()
  for (const [pid, list] of kidsMap.value) {
    if (list.length && childIds.has(pid)) next.add(pid)
  }
  collapsed.value = next
}
function resetCollapse() {
  collapsed.value = new Set()
}

/* ---------------- 布局 ---------------- */
const laid = ref([])
const laidEdges = ref([])
const zoneLabels = ref([])

/** 对一組节点做「整齐树」布局，返回相对坐标 */
function tidy(list) {
  const ids = new Set(list.map((n) => n.id))
  const kids = new Map()
  const roots = []
  for (const n of list) {
    if (n.parent && ids.has(n.parent) && n.parent !== n.id) {
      if (!kids.has(n.parent)) kids.set(n.parent, [])
      kids.get(n.parent).push(n)
    } else {
      roots.push(n)
    }
  }
  let cursor = 0
  let maxDepth = 0
  const pos = new Map()
  const walk = (n, depth, guard) => {
    if (guard > 40) return
    maxDepth = Math.max(maxDepth, depth)
    const ks = kids.get(n.id) ?? []
    if (!ks.length) {
      pos.set(n.id, { col: cursor++, depth })
      return
    }
    for (const k of ks) walk(k, depth + 1, guard + 1)
    const first = pos.get(ks[0].id).col
    const last = pos.get(ks[ks.length - 1].id).col
    pos.set(n.id, { col: (first + last) / 2, depth })
  }
  for (const r of roots) walk(r, 0, 0)
  const cols = Math.max(cursor, 1)
  return { pos, cols, maxDepth }
}

function kindRadius(kind, zone) {
  if (kind === 'customer') return 28
  if (kind === 'project') return zone === 'anchor' ? 26 : 20
  if (kind === 'partner' || kind === 'competitor') return 24
  if (kind === 'qax') return 22
  if (kind === 'c74111') return 26
  return 16
}
function radius(n) {
  return kindRadius(n.kind, n.zone)
}

function computeLayout() {
  const visible = visibleNodes.value
  const zoneOf = (z) => visible.filter((n) => n.zone === z)

  const top = zoneOf('top')
  const anchor = zoneOf('anchor')
  const diag = zoneOf('diag')
  const center = zoneOf('tree')
  const left = zoneOf('left')
  const right = zoneOf('right')
  const team = zoneOf('team')

  const forests = {}
  const measure = (list) => {
    if (!list.length) return { pos: new Map(), cols: 0, maxDepth: -1 }
    return tidy(list)
  }
  forests.center = measure(center)
  forests.left = measure(left)
  forests.right = measure(right)
  forests.team = measure(team)

  const forestWidth = (f) => (f.cols ? (f.cols - 1) * LEAF_GAP : 0) + 52
  const forestHeight = (f) => (f.maxDepth >= 0 ? f.maxDepth * LEVEL_GAP : 0) + 34

  const centerW = forestWidth(forests.center)
  const anchorW = anchor.length ? (anchor.length - 1) * 220 + 90 : 0
  const topW = top.length ? (top.length - 1) * 220 + 90 : 0
  const diagW = diag.length ? 190 : 0

  const beltW =
    forestWidth(forests.left) + forestWidth(forests.team) + forestWidth(forests.right) +
    (forests.left.cols ? BELT_GAP : 0) +
    (forests.right.cols ? BELT_GAP : 0) +
    (forests.team.cols && (forests.left.cols || forests.right.cols) ? BELT_GAP : 0)

  const totalW = Math.max(centerW, anchorW, topW, beltW, diagW + 240)

  const centerX = (fw) => (totalW - fw) / 2

  const placed = []
  const put = (n, x, y) => placed.push({ ...n, x, y })

  // 1. 客户节点
  top.forEach((n, i) => {
    const w = top.length > 1 ? (i - (top.length - 1) / 2) * 220 : 0
    put(n, totalW / 2 + w, ZONE_TOP_Y)
  })

  // 2. 项目锚点
  const anchorY = top.length ? ZONE_ANCHOR_Y : ZONE_ANCHOR_Y - 60
  anchor.forEach((n, i) => {
    const w = anchor.length > 1 ? (i - (anchor.length - 1) / 2) * 220 : 0
    put(n, totalW / 2 + w, anchorY)
  })

  // 3. C74111 诊断节点（右侧独立）
  diag.forEach((n, i) => {
    put(n, totalW - 84, anchorY + i * 96)
  })

  // 4. 客户方人员层级树
  const treeX = centerX(centerW)
  const treeY = anchorY + 108
  if (forests.center.cols) {
    const f = forests.center
    for (const n of center) {
      const p = f.pos.get(n.id)
      if (p) put(n, treeX + 26 + p.col * LEAF_GAP, treeY + p.depth * LEVEL_GAP)
    }
  }

  // 5. 底部三翼：合作方 | 我方团队 | 竞争方
  const beltY = (forests.center.cols ? treeY + forestHeight(forests.center) : treeY) + 96
  let bx = centerX(beltW)
  const beltStart = bx
  const leftX = forests.left.cols ? bx : null
  const placeForest = (list, f) => {
    if (!f.cols) return
    for (const n of list) {
      const p = f.pos.get(n.id)
      if (p) put(n, bx + 26 + p.col * LEAF_GAP, beltY + p.depth * LEVEL_GAP)
    }
    bx += forestWidth(f) + BELT_GAP
  }
  placeForest(left, forests.left)
  const teamX = forests.team.cols ? bx : null
  placeForest(team, forests.team)
  const rightX = forests.right.cols ? bx : null
  placeForest(right, forests.right)

  // 6. 归一化到正坐标
  let minX = Infinity
  let minY = Infinity
  for (const n of placed) {
    minX = Math.min(minX, n.x - radius(n) - 26)
    minY = Math.min(minY, n.y - radius(n) - 26)
  }
  if (!Number.isFinite(minX)) minX = 0
  if (!Number.isFinite(minY)) minY = 0
  const dx = PAD - minX
  const dy = PAD - minY
  for (const n of placed) {
    n.x += dx
    n.y += dy
  }

  // 7. 连线（父 → 子，仅两端都可见）
  const visIds = new Set(visible.map((n) => n.id))
  const edges = []
  for (const n of placed) {
    if (!n.parent || !visIds.has(n.parent)) continue
    const p = placed.find((x) => x.id === n.parent)
    if (!p) continue
    const d = elbow(p, n)
    edges.push({ id: `${n.parent}->${n.id}`, d, cls: edgeClass(n) })
  }

  // 8. 分区标题（与节点一同位移）
  const shiftX = (x) => x + dx
  const labels = []
  if (top.length) labels.push({ id: 'z-top', x: shiftX(totalW / 2), y: ZONE_TOP_Y - 34 + dy, text: '主 客 户' })
  if (center.length) labels.push({ id: 'z-tree', x: shiftX(totalW / 2), y: treeY - 46 + dy, text: '客户方决策链（组织架构）' })
  if (anchor.length)
    labels.push({ id: 'z-anchor', x: shiftX(totalW / 2), y: anchorY - 46 + dy, text: '项 目 锚 点' })
  if (left.length)
    labels.push({
      id: 'z-left',
      x: shiftX(leftX + forestWidth(forests.left) / 2),
      y: beltY - 42 + dy,
      text: '合 作 方',
    })
  if (team.length)
    labels.push({
      id: 'z-team',
      x: shiftX(teamX + forestWidth(forests.team) / 2),
      y: beltY - 42 + dy,
      text: '我方（奇安信）团队 · 非合作方 / 非竞争方',
    })
  if (right.length)
    labels.push({
      id: 'z-right',
      x: shiftX(rightX + forestWidth(forests.right) / 2),
      y: beltY - 42 + dy,
      text: '竞 争 方',
    })

  laid.value = placed
  laidEdges.value = edges
  zoneLabels.value = labels
  contentBox.value = bounds(placed)
}

function elbow(a, b) {
  const dy = (b.y - a.y) * 0.5
  return `M ${a.x} ${a.y} C ${a.x} ${a.y + dy}, ${b.x} ${b.y - dy}, ${b.x} ${b.y}`
}

function edgeClass(n) {
  if (n.source === 'partner') return 'og-edge--partner'
  if (n.source === 'competitor') return 'og-edge--competitor'
  if (n.source === 'qax') return 'og-edge--qax'
  if (n.kind === 'project') return 'og-edge--project'
  return ''
}

const contentBox = ref({ x: 0, y: 0, w: 1, h: 1 })
function bounds(list) {
  if (!list.length) return { x: 0, y: 0, w: 1, h: 1 }
  let x1 = Infinity
  let y1 = Infinity
  let x2 = -Infinity
  let y2 = -Infinity
  for (const n of list) {
    const r = radius(n) + 34
    x1 = Math.min(x1, n.x - r)
    y1 = Math.min(y1, n.y - r)
    x2 = Math.max(x2, n.x + r + 30)
    y2 = Math.max(y2, n.y + r + 40)
  }
  return { x: x1, y: y1, w: Math.max(x2 - x1, 1), h: Math.max(y2 - y1, 1) }
}

/* ---------------- 视图变换 ---------------- */
const view = ref({ x: 0, y: 0, k: 1 })
function measure() {
  const el = wrapEl.value
  if (!el) return
  vw.value = Math.max(el.clientWidth, 320)
  vh.value = Math.max(el.clientHeight, 260)
}
function fit() {
  measure()
  const b = contentBox.value
  const k = Math.min((vw.value - 48) / b.w, (vh.value - 48) / b.h)
  const kk = Math.min(1.5, Math.max(0.24, k))
  view.value = {
    k: kk,
    x: (vw.value - b.w * kk) / 2 - b.x * kk,
    y: (vh.value - b.h * kk) / 2 - b.y * kk,
  }
}
function zoomBy(f) {
  const next = Math.min(2.4, Math.max(0.2, view.value.k * f))
  const cx = vw.value / 2
  const cy = vh.value / 2
  const ratio = next / view.value.k
  view.value = {
    k: next,
    x: cx - (cx - view.value.x) * ratio,
    y: cy - (cy - view.value.y) * ratio,
  }
}
function onWheel(e) {
  zoomBy(e.deltaY > 0 ? 0.9 : 1.1)
}
function reset() {
  resetCollapse()
  selectedId.value = null
  fit()
}

/* ---------------- 拖拽平移 + 点击判定（本次修复的核心） ----------------
   旧实现把 pan 挂在 svg 的 mousedown 上，任何 1px 位移都会移动图形，
   导致 mouseup 落在空白处，节点的 click 事件不再触发 —— 表现为"点不动节点"。
   现在：记录拖拽累计位移，位移 > 5px 时抑制节点点击，其余情况正常派发。 */
const dragging = ref(false)
let dragMoved = 0
let dragStart = { x: 0, y: 0, vx: 0, vy: 0 }
const CLICK_TOLERANCE = 5

function onDown(e) {
  if (e.button !== undefined && e.button !== 0) return
  dragging.value = true
  dragMoved = 0
  dragStart = { x: e.clientX, y: e.clientY, vx: view.value.x, vy: view.value.y }
  window.addEventListener('pointermove', onMove)
  window.addEventListener('pointerup', onUp)
  window.addEventListener('pointercancel', onUp)
}
function onMove(e) {
  if (!dragging.value) return
  const dx = e.clientX - dragStart.x
  const dy = e.clientY - dragStart.y
  dragMoved = Math.max(dragMoved, Math.hypot(dx, dy))
  view.value = { ...view.value, x: dragStart.vx + dx, y: dragStart.vy + dy }
}
function onUp() {
  dragging.value = false
  window.removeEventListener('pointermove', onMove)
  window.removeEventListener('pointerup', onUp)
  window.removeEventListener('pointercancel', onUp)
}
onBeforeUnmount(() => {
  onUp()
  ro?.disconnect()
})

function onNodeClick(n) {
  if (dragMoved > CLICK_TOLERANCE) return
  selectedId.value = selectedId.value === n.id ? null : n.id
  emit('select', n)
}

/* ---------------- 配色 ---------------- */
function fillOf(n) {
  switch (n.kind) {
    case 'customer':
      return 'var(--n-customer)'
    case 'project':
      if (n.stage === '线索') return 'var(--n-clue)'
      if (n.stage === '商机') return 'var(--n-op)'
      return 'var(--n-project)'
    case 'c74111':
      // 独立诊断节点按分数分档着色：>75 绿 / 50-75 蓝 / 25-50 黄 / <25 红
      return tierFill(n.tier)
    case 'partner':
      return 'var(--n-partner)'
    case 'competitor':
      return 'var(--n-competitor)'
    case 'qax':
      return 'var(--n-qax)'
    default:
      if (n.source === 'qax') return 'var(--team-ar)'
      if (n.source === 'partner') return 'var(--n-partner)'
      if (n.source === 'competitor') return 'var(--n-competitor)'
      return adurColor(n.adur)
  }
}
function adurColor(a) {
  if (a === 'A') return 'var(--adur-a)'
  if (a === 'D') return 'var(--adur-d)'
  if (a === 'U') return 'var(--adur-u)'
  if (a === 'R') return 'var(--adur-r)'
  return 'var(--n-person)'
}
/** C74111 分档色（定义在 app.css 的 .app 作用域上，四个中间调在明暗底上都可读） */
function tierFill(t) {
  if (t === 'a') return 'var(--c74-a)'
  if (t === 'b') return 'var(--c74-b)'
  if (t === 'c') return 'var(--c74-c)'
  if (t === 'd') return 'var(--c74-d)'
  return 'var(--n-project)'
}
function strokeOf(n) {
  if (n.kind === 'person') return 'var(--bd-hover)'
  return 'var(--bd-hover)'
}
function strokeW(n) {
  if (n.kind === 'person') return 1.4
  if (n.kind === 'c74111') return 2
  return 2
}
function roleMark(n) {
  if (n.teamRole) return n.teamRole
  if (n.adur) return n.adur
  return ''
}
function roleColor(n) {
  if (n.teamRole) {
    const map = { AR: '--team-ar', SR: '--team-sr', CSR: '--team-csr', CDR: '--team-cdr', FR: '--team-sr', MGR: '--team-ar' }
    return `var(${map[n.teamRole] ?? '--team-ar'})`
  }
  return adurColor(n.adur)
}
function attitudeColor(a) {
  if (a === '+') return 'var(--ok)'
  if (a === '=') return 'var(--tx-3)'
  if (a === '-') return 'var(--warn)'
  if (a === 'X') return 'var(--bad)'
  if (a === '⭐') return 'var(--ac)'
  return 'var(--tx-3)'
}
function shortMark(n) {
  if (n.kind === 'customer') return '客户'
  if (n.kind === 'project') return n.stage ?? '项目'
  if (n.kind === 'partner') return '合作'
  if (n.kind === 'competitor') return '竞争'
  if (n.kind === 'qax') return '我方'
  return ''
}

/* ---------------- 详情 ---------------- */
const selected = computed(() => props.nodes.find((n) => n.id === selectedId.value) ?? null)
const detailPairs = computed(() => {
  const n = selected.value
  if (!n) return {}
  const out = { ...(n.detail ?? {}) }
  if (n.parent) {
    const p = byId.value.get(n.parent)
    if (p) out['上级 / 归属'] = p.label
  }
  const kids = kidsMap.value.get(n.id) ?? []
  if (kids.length) out['直属下级'] = kids.map((k) => k.label).join('、')
  return out
})
const diagRows = computed(() => {
  const d = selected.value?.c74111
  if (!d) return []
  return [...d.clears, ...d.priorities, ...d.key]
})
function pct(score, max) {
  return Math.round(((score ?? 0) / (max || 1)) * 100)
}
function sourceText(n) {
  if (n.source === 'qax') return '我方（奇安信）自有团队 · 不属于合作方，也不属于竞争方'
  if (n.source === 'partner') return '合作方'
  if (n.source === 'competitor') return '竞争方'
  if (n.kind === 'customer') return '主客户'
  if (n.kind === 'project') return '商机 / 项目'
  if (n.kind === 'c74111') return 'C74111 项目诊断（独立节点）'
  return '客户方人员'
}
function zoneText(n) {
  const map = {
    top: '客户层',
    anchor: '项目锚点',
    tree: '客户方决策链',
    left: '合作方翼',
    right: '竞争方翼',
    team: '我方团队',
    diag: 'C74111 诊断',
  }
  return map[n.zone] ?? '节点'
}

/* ---------------- 生命周期 ---------------- */
let ro = null
function relayout() {
  computeLayout()
  measure()
  fit()
}

watch(
  () => [props.nodes, collapsed.value],
  () => relayout(),
  { deep: true, flush: 'post' },
)

onMounted(() => {
  relayout()
  if (typeof ResizeObserver !== 'undefined' && wrapEl.value) {
    ro = new ResizeObserver(() => {
      measure()
      fit()
    })
    ro.observe(wrapEl.value)
  }
})

defineExpose({ fit, expandAll, collapseAll, reset })
</script>
