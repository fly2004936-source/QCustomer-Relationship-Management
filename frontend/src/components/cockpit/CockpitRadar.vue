<template>
  <!--
    雷达图（手绘 SVG）—— C74111「7 Clears + 4 Priorities + 1 Key」十二项均分画像
    每个轴按 pct（该项均分 / 满分）归一，所以满分 20 的 P3 和满分 5 的 C2 能同屏比较，
    不会被绝对值大小压扁。
  -->
  <div class="cr">
    <svg :viewBox="`0 0 ${S} ${S}`" class="cr-svg" role="img">
      <!-- 同心参考多边形 -->
      <polygon v-for="(lv, i) in levels" :key="'g' + i" :points="polyOf(lv)" class="cr-grid" />
      <!-- 轴辐条 -->
      <line v-for="(a, i) in axes" :key="'s' + i" :x1="cx" :y1="cy" :x2="pt(i, 1).x" :y2="pt(i, 1).y" class="cr-spoke" />
      <!-- 数据面 -->
      <polygon :points="polyOf(1, true)" class="cr-area" />
      <polyline :points="polyOf(1, true)" class="cr-line" />
      <!-- 顶点 -->
      <circle
        v-for="(a, i) in axes"
        :key="'p' + i"
        :cx="pt(i, (a.pct || 0) / 100).x"
        :cy="pt(i, (a.pct || 0) / 100).y"
        r="3"
        class="cr-dot"
      >
        <title>{{ a.code }} {{ a.name }}｜均分 {{ a.avg?.toFixed?.(1) ?? a.avg }} / {{ a.max }}（{{ a.pct }}%）</title>
      </circle>
      <!-- 轴标签 -->
      <text
        v-for="(a, i) in axes"
        :key="'t' + i"
        :x="pt(i, 1.17).x"
        :y="pt(i, 1.17).y + 3"
        class="cr-label"
        :text-anchor="anchorOf(i)"
      >
        {{ a.code }}
      </text>
    </svg>
    <div class="cr-legend">
      <span class="cr-lg"><i class="cr-sw cr-sw--clear"></i>7 Clears</span>
      <span class="cr-lg"><i class="cr-sw cr-sw--pri"></i>4 Priorities</span>
      <span class="cr-lg"><i class="cr-sw cr-sw--key"></i>1 Key</span>
    </div>
  </div>
</template>

<script setup>
import { computed } from 'vue'

const props = defineProps({
  axes: { type: Array, default: () => [] }, // [{ code, name, avg, max, pct, group }]
})

const S = 260
const cx = S / 2
const cy = S / 2 + 2
const R = 92
const levels = [0.25, 0.5, 0.75, 1]

const n = computed(() => Math.max(1, props.axes.length))

/** 从正上方开始逆时针排布 */
function pt(i, ratio) {
  const ang = -Math.PI / 2 + (i / n.value) * Math.PI * 2
  return { x: cx + Math.cos(ang) * R * ratio, y: cy + Math.sin(ang) * R * ratio }
}

function polyOf(ratio, data = false) {
  return props.axes
    .map((a, i) => {
      const rr = data ? Math.max(0.02, Math.min(1, (a.pct || 0) / 100)) : ratio
      const p = pt(i, rr)
      return `${p.x.toFixed(1)},${p.y.toFixed(1)}`
    })
    .join(' ')
}

function anchorOf(i) {
  const p = pt(i, 1.17)
  if (Math.abs(p.x - cx) < 8) return 'middle'
  return p.x > cx ? 'start' : 'end'
}
</script>

<style scoped>
.cr {
  display: flex;
  flex-direction: column;
  align-items: center;
}
.cr-svg {
  width: 100%;
  max-width: 300px;
  height: auto;
}
.cr-grid {
  fill: none;
  stroke: var(--hairline);
  stroke-width: 1;
}
.cr-spoke {
  stroke: var(--hairline);
  stroke-width: 1;
}
.cr-area {
  fill: var(--ac-soft);
  stroke: none;
  transition: all var(--dur-slow, 700ms) var(--ease, ease);
}
.cr-line {
  fill: none;
  stroke: var(--ac);
  stroke-width: 1.6;
  stroke-linejoin: round;
}
.cr-dot {
  fill: var(--ac);
  cursor: help;
}
.cr-label {
  font-family: var(--ff-mono);
  font-size: 9px;
  fill: var(--tx-3);
}
.cr-legend {
  display: flex;
  gap: 12px;
  margin-top: 4px;
  flex-wrap: wrap;
  justify-content: center;
}
.cr-lg {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  font-size: 10.5px;
  color: var(--tx-2);
}
.cr-sw {
  width: 8px;
  height: 8px;
  border-radius: 2px;
  display: inline-block;
}
.cr-sw--clear {
  background: var(--c74-b);
}
.cr-sw--pri {
  background: var(--c74-c);
}
.cr-sw--key {
  background: var(--c74-a);
}
</style>
