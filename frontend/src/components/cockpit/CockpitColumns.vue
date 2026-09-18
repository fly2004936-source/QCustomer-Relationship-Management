<template>
  <!--
    纵向柱状图（手绘 SVG）
    用于「项目阶段 / 拜访月度趋势」这类等距分类对比。
    柱高按 max 归一，底部留 18px 给分类标签，顶部留 16px 给数值。
  -->
  <div class="cc">
    <svg :width="'100%'" :height="height" :viewBox="`0 0 ${w} ${height}`" preserveAspectRatio="none" aria-hidden="true">
      <line :x1="pad" :y1="baseY" :x2="w - pad" :y2="baseY" :stroke="'var(--hairline)'" stroke-width="1" />
      <g v-for="(it, i) in items" :key="i">
        <rect
          :x="xOf(i)"
          :y="yOf(it.value)"
          :width="barW"
          :height="Math.max(2, baseY - yOf(it.value))"
          :fill="it.color || 'var(--ac)'"
          rx="2"
        />
        <text :x="xOf(i) + barW / 2" :y="yOf(it.value) - 5" class="cc-num" text-anchor="middle">
          {{ fmt(it.value) }}
        </text>
      </g>
    </svg>
    <div class="cc-labels">
      <span v-for="(it, i) in items" :key="i" class="cc-label" :title="it.label">{{ it.label }}</span>
    </div>
  </div>
</template>

<script setup>
import { computed } from 'vue'

const props = defineProps({
  items: { type: Array, default: () => [] }, // [{ label, value, color? }]
  height: { type: Number, default: 132 },
  unit: { type: String, default: '' },
})

const w = 320
const pad = 6
const topPad = 18
const baseY = computed(() => props.height - 6)

const max = computed(() => Math.max(1, ...props.items.map((it) => Number(it.value) || 0)))
const slot = computed(() => (w - pad * 2) / Math.max(1, props.items.length))
const barW = computed(() => Math.max(6, Math.min(38, slot.value * 0.52)))

const xOf = (i) => pad + slot.value * i + (slot.value - barW.value) / 2
const yOf = (v) => {
  const ratio = (Number(v) || 0) / max.value
  return baseY.value - ratio * (baseY.value - topPad)
}

const fmt = (v) => String(Number(v) || 0)
</script>

<style scoped>
.cc {
  width: 100%;
}
.cc-num {
  font-family: var(--ff-mono);
  font-size: 9px;
  fill: var(--tx-2);
}
.cc-labels {
  display: flex;
  margin-top: 4px;
}
.cc-label {
  flex: 1;
  text-align: center;
  font-size: 10.5px;
  color: var(--tx-3);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
</style>
