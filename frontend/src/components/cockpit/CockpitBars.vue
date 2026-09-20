<template>
  <!--
    横向条形榜（手绘，CSS 宽度过渡）
    用于「预算排行 / 库表落库热度 / 战区分布」这类需要看标签的排行。
    · 条形长度按 max 归一；给了 color 就用它，否则用强调色
    · 第三列可显示附加文本（如金额、级别）
  -->
  <div class="cb">
    <div v-for="(it, i) in items" :key="i" class="cb-row">
      <div class="cb-label" :title="it.label">{{ it.label }}</div>
      <div class="cb-track">
        <span
          class="cb-fill"
          :style="{ width: pct(it.value) + '%', background: it.color || 'var(--ac)' }"
        ></span>
      </div>
      <div class="cb-value mono">{{ fmt(it.value) }}<em v-if="unit">{{ unit }}</em></div>
      <div v-if="subKey" class="cb-sub">{{ it[subKey] }}</div>
    </div>
    <div v-if="!items.length" class="empty">暂无数据</div>
  </div>
</template>

<script setup>
import { computed } from 'vue'

const props = defineProps({
  items: { type: Array, default: () => [] }, // [{ label, value, color?, ... }]
  unit: { type: String, default: '' },
  subKey: { type: String, default: '' },
  decimals: { type: Number, default: 0 },
})

/** 归一基准：取最大值，避免某一条爆表把其它条压成一条线 */
const max = computed(() => Math.max(1, ...props.items.map((it) => Math.abs(Number(it.value) || 0))))

const pct = (v) => {
  const n = Math.abs(Number(v) || 0)
  return Math.max(n > 0 ? 2 : 0, Math.round((n / max.value) * 100))
}

const fmt = (v) => {
  const n = Number(v) || 0
  return props.decimals > 0 ? n.toFixed(props.decimals) : Math.round(n).toLocaleString('zh-CN')
}
</script>

<style scoped>
.cb {
  display: flex;
  flex-direction: column;
  gap: 9px;
}
.cb-row {
  display: grid;
  grid-template-columns: minmax(72px, 26%) 1fr auto;
  align-items: center;
  gap: 10px;
}
.cb-label {
  font-size: 11.5px;
  color: var(--tx-2);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.cb-track {
  height: 8px;
  border-radius: var(--r-pill, 999px);
  background: var(--bg-elevated);
  overflow: hidden;
}
.cb-fill {
  display: block;
  height: 100%;
  border-radius: inherit;
  min-width: 2px;
  transition: width var(--dur-slow, 700ms) var(--ease, ease);
}
.cb-value {
  font-size: 11.5px;
  font-variant-numeric: tabular-nums;
  color: var(--tx-1);
  min-width: 46px;
  text-align: right;
}
.cb-value em {
  font-style: normal;
  font-size: 10px;
  color: var(--tx-3);
  margin-left: 2px;
}
.cb-sub {
  grid-column: 3;
  font-size: 10.5px;
  color: var(--tx-3);
}
</style>
