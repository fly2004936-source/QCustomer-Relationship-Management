<template>
  <!--
    竞争态势矩阵（散点）
    X 轴 = 项目预算（万元），Y 轴 = 趋赢力（%）。
    背景按 C74111 四档横向分区（<25 / 25–50 / 50–75 / >75），所以任何一点落在
    哪一档、离「可承诺」还差多少，一眼就能看出来 —— 这就是驾驶舱里最该有的那种图。
  -->
  <div class="cs">
    <svg :viewBox="`0 0 ${W} ${H}`" class="cs-svg" role="img">
      <!-- 四档背景带 -->
      <g>
        <rect
          v-for="z in zones"
          :key="z.key"
          :x="M.l"
          :y="yOf(z.to)"
          :width="W - M.l - M.r"
          :height="Math.max(0, yOf(z.from) - yOf(z.to))"
          :fill="z.color"
          opacity="0.09"
        />
      </g>

      <!-- 网格 -->
      <g>
        <line v-for="t in yTicks" :key="'h' + t" :x1="M.l" :y1="yOf(t)" :x2="W - M.r" :y2="yOf(t)" class="cs-grid" />
        <line v-for="t in xTicks" :key="'v' + t.v" :x1="xOf(t.v)" :y1="H - M.b" :x2="xOf(t.v)" :y2="M.t" class="cs-grid" />
      </g>

      <!-- 坐标轴 -->
      <line :x1="M.l" :y1="H - M.b" :x2="W - M.r" :y2="H - M.b" class="cs-axis" />
      <line :x1="M.l" :y1="M.t" :x2="M.l" :y2="H - M.b" class="cs-axis" />

      <!-- 刻度文字 -->
      <g>
        <text v-for="t in yTicks" :key="'yl' + t" :x="M.l - 6" :y="yOf(t) + 3" class="cs-tick" text-anchor="end">
          {{ t }}%
        </text>
        <text v-for="t in xTicks" :key="'xl' + t.v" :x="xOf(t.v)" :y="H - M.b + 13" class="cs-tick" text-anchor="middle">
          {{ t.label }}
        </text>
      </g>

      <!-- 数据点 -->
      <g>
        <circle
          v-for="(p, i) in points"
          :key="i"
          :cx="xOf(p.x)"
          :cy="yOf(p.y)"
          :r="p.level === 'P1' ? 6 : 4.6"
          :fill="bandColor(p.band)"
          :stroke="'var(--bg-surface)'"
          stroke-width="1.4"
          class="cs-dot"
        >
          <title>{{ p.name }}｜{{ p.customer }}｜预算 {{ Math.round(p.x) }} 万元 · 趋赢力 {{ Math.round(p.y) }}%</title>
        </circle>
      </g>

      <text :x="W - M.r" :y="M.t + 4" class="cs-unit" text-anchor="end">趋赢力 % ↑</text>
      <text :x="W - M.r" :y="H - 4" class="cs-unit" text-anchor="end">预算（万元）→</text>
    </svg>

    <div class="cs-legend">
      <span v-for="b in bands" :key="b.key" class="cs-lg">
        <i :style="{ background: b.color }"></i>{{ b.label }}
      </span>
      <span class="cs-hint">圆点越大 = P1 级项目</span>
    </div>
  </div>
</template>

<script setup>
import { computed } from 'vue'
import { C74_BANDS } from '@/api/analytics'

const props = defineProps({
  points: { type: Array, default: () => [] }, // [{ x, y, name, customer, band, level }]
})

const W = 440
const H = 236
const M = { l: 40, r: 12, t: 14, b: 26 }

const bands = C74_BANDS
const bandColor = (k) => (C74_BANDS.find((b) => b.key === k) || C74_BANDS[4]).color

const xMax = computed(() => {
  const m = Math.max(1, ...props.points.map((p) => Number(p.x) || 0))
  // 取整到「好看的」刻度上界
  const step = m > 4000 ? 1000 : m > 800 ? 200 : 100
  return Math.ceil(m / step) * step
})

/** 四档背景带：y 从低到高分别是 d / c / b / a */
const zones = computed(() => [
  { key: 'd', from: 0, to: 25, color: bandColor('d') },
  { key: 'c', from: 25, to: 50, color: bandColor('c') },
  { key: 'b', from: 50, to: 75, color: bandColor('b') },
  { key: 'a', from: 75, to: 100, color: bandColor('a') },
])

const yTicks = [0, 25, 50, 75, 100]
const xTicks = computed(() => {
  const max = xMax.value
  return [0, 0.25, 0.5, 0.75, 1].map((r) => ({ v: max * r, label: Math.round(max * r).toLocaleString('zh-CN') }))
})

const xOf = (v) => M.l + ((Number(v) || 0) / xMax.value) * (W - M.l - M.r)
const yOf = (v) => H - M.b - (Math.min(100, Math.max(0, Number(v) || 0)) / 100) * (H - M.t - M.b)
</script>

<style scoped>
.cs {
  width: 100%;
}
.cs-svg {
  width: 100%;
  height: auto;
  display: block;
}
.cs-grid {
  stroke: var(--hairline);
  stroke-width: 1;
  stroke-dasharray: 3 4;
}
.cs-axis {
  stroke: var(--bd-hover);
  stroke-width: 1.2;
}
.cs-tick {
  font-family: var(--ff-mono);
  font-size: 9px;
  fill: var(--tx-3);
}
.cs-unit {
  font-size: 9.5px;
  fill: var(--tx-3);
}
.cs-dot {
  transition: opacity var(--dur) var(--ease, ease);
  cursor: help;
}
.cs-dot:hover {
  opacity: 0.72;
  stroke: var(--tx-1);
  stroke-width: 2;
}
.cs-legend {
  display: flex;
  flex-wrap: wrap;
  gap: 6px 12px;
  margin-top: 8px;
  align-items: center;
}
.cs-lg {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  font-size: 10.5px;
  color: var(--tx-2);
}
.cs-lg i {
  width: 8px;
  height: 8px;
  border-radius: 2px;
  display: inline-block;
}
.cs-hint {
  margin-left: auto;
  font-size: 10px;
  color: var(--tx-3);
}
</style>
