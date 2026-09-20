<template>
  <!--
    环形图（手绘 SVG，不引图表库）
    · 段与段之间留固定弧度空隙，避免相邻同色看不清边界
    · 中心可放主数字（数值 / 占比）
    · items 全为 0 时退化为一条空轨道，不留白屏
  -->
  <div class="dn" :style="{ width: px, height: px }">
    <svg :width="px" :height="px" :viewBox="`0 0 ${size} ${size}`" role="img">
      <g :transform="`rotate(-90 ${c} ${c})`">
        <circle :cx="c" :cy="c" :r="r" fill="none" :stroke="track" :stroke-width="thickness" />
        <circle
          v-for="(s, i) in segments"
          :key="i"
          class="dn-seg"
          :cx="c"
          :cy="c"
          :r="r"
          fill="none"
          :stroke="s.color"
          :stroke-width="thickness"
          :stroke-dasharray="`${s.len} ${circ - s.len}`"
          :stroke-dashoffset="-s.offset"
          stroke-linecap="butt"
        />
      </g>
    </svg>
    <div class="dn-center">
      <div class="dn-value">{{ centerValue }}</div>
      <div v-if="centerSub" class="dn-sub">{{ centerSub }}</div>
    </div>
  </div>
</template>

<script setup>
import { computed } from 'vue'

const props = defineProps({
  items: { type: Array, default: () => [] }, // [{ label, value, color }]
  size: { type: Number, default: 168 },
  thickness: { type: Number, default: 14 },
  centerValue: { type: [String, Number], default: '' },
  centerSub: { type: String, default: '' },
})

const px = computed(() => `${props.size}px`)
const c = computed(() => props.size / 2)
const r = computed(() => (props.size - props.thickness) / 2 - 1)
const circ = computed(() => 2 * Math.PI * r.value)
const track = 'var(--bg-elevated)'

const segments = computed(() => {
  const rows = props.items.filter((it) => Number(it.value) > 0)
  const total = rows.reduce((s, it) => s + Number(it.value), 0)
  if (!total) return []
  // 每段扣掉 2 个单位的空隙（数据点极少时不留，否则细段会消失）
  const gap = rows.length > 1 ? 2 : 0
  let acc = 0
  return rows.map((it) => {
    const len = Math.max((Number(it.value) / total) * circ.value - gap, 0.6)
    const seg = { color: it.color || 'var(--ac)', len, offset: acc }
    acc += (Number(it.value) / total) * circ.value
    return seg
  })
})
</script>

<style scoped>
.dn {
  position: relative;
  flex-shrink: 0;
}
.dn-seg {
  transition: stroke-dasharray var(--dur-slow, 700ms) var(--ease, ease);
}
.dn-center {
  position: absolute;
  inset: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 2px;
  pointer-events: none;
}
.dn-value {
  font-family: var(--ff-mono);
  font-variant-numeric: tabular-nums;
  font-size: 26px;
  font-weight: 700;
  line-height: 1;
  color: var(--tx-1);
}
.dn-sub {
  font-size: 10.5px;
  letter-spacing: 0.04em;
  color: var(--tx-3);
}
</style>
