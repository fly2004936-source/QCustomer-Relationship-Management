<template>
  <div class="sf-app mini" :data-style="styleKey">
    <div class="mini-in">
      <!-- 顶栏 -->
      <div class="mini-top">
        <span class="mini-logo"></span>
        <span class="mini-tab"></span>
        <span class="mini-tab mini-tab--short"></span>
        <span class="mini-cta"></span>
      </div>

      <!-- 标题 -->
      <div class="mini-head">
        <span class="mini-title"></span>
        <span class="mini-sub"></span>
      </div>

      <!-- KPI -->
      <div class="mini-kpis">
        <div v-for="i in 3" :key="i" class="mini-card">
          <span class="mini-kpi-label"></span>
          <span class="mini-kpi-num"></span>
          <span class="mini-bar"><i :style="{ width: [72, 46, 88][i - 1] + '%' }"></i></span>
        </div>
      </div>

      <!-- 网络母题 -->
      <div class="mini-graph">
        <svg viewBox="0 0 300 108" class="mini-svg" aria-hidden="true">
          <path
            v-for="(e, i) in MOTIF_EDGES"
            :key="i"
            :d="e"
            fill="none"
            :style="{ stroke: 'var(--edge)' }"
            stroke-width="1.1"
          />
          <circle
            v-for="(n, i) in MOTIF_NODES"
            :key="'n' + i"
            :cx="n.x"
            :cy="n.y"
            :r="n.r"
            :style="{ fill: n.c, stroke: 'var(--bd-hover)' }"
            stroke-width="1.1"
          />
        </svg>
      </div>

      <!-- 表格行 -->
      <div class="mini-rows">
        <div v-for="i in 3" :key="i" class="mini-row">
          <span class="mini-cell" :style="{ width: [58, 44, 66][i - 1] + '%' }"></span>
          <span class="mini-pill"></span>
          <span class="mini-cell mini-cell--short"></span>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
defineProps({ styleKey: { type: String, required: true } })

const MOTIF_NODES = [
  { x: 150, y: 16, r: 11, c: 'var(--n-customer)' },
  { x: 150, y: 56, r: 9, c: 'var(--n-project)' },
  { x: 78, y: 56, r: 8, c: 'var(--n-partner)' },
  { x: 222, y: 56, r: 8, c: 'var(--n-competitor)' },
  { x: 48, y: 92, r: 6, c: 'var(--adur-a)' },
  { x: 106, y: 92, r: 6, c: 'var(--adur-u)' },
  { x: 150, y: 92, r: 7, c: 'var(--n-qax)' },
  { x: 196, y: 92, r: 6, c: 'var(--adur-r)' },
  { x: 250, y: 92, r: 6, c: 'var(--adur-d)' },
]
const MOTIF_EDGES = [
  'M150 27 L150 47',
  'M150 47 C150 51, 78 48, 78 48',
  'M150 47 C150 51, 222 48, 222 48',
  'M78 64 C78 78, 48 80, 48 86',
  'M78 64 C78 78, 106 80, 106 86',
  'M150 65 L150 85',
  'M222 64 C222 78, 196 80, 196 86',
  'M222 64 C222 78, 250 80, 250 86',
]
</script>

<style scoped>
.mini {
  height: 100%;
  overflow: hidden;
}
.mini-in {
  display: flex;
  flex-direction: column;
  gap: 10px;
  padding: 14px;
  height: 100%;
}
.mini-top {
  display: flex;
  align-items: center;
  gap: 7px;
}
.mini-logo {
  width: 16px;
  height: 16px;
  border-radius: var(--r-chip, 4px);
  background: var(--ac);
  background-image: var(--ac-image, none);
  flex-shrink: 0;
}
.mini-tab,
.mini-tab--short {
  height: 7px;
  width: 40px;
  border-radius: var(--r-pill, 4px);
  background: var(--tx-3);
  opacity: 0.5;
}
.mini-tab--short {
  width: 26px;
}
.mini-cta {
  margin-left: auto;
  width: 54px;
  height: 18px;
  border-radius: var(--r-btn, 4px);
  background: var(--ac);
  background-image: var(--ac-image, none);
  border: 1px solid var(--bd);
  box-shadow: var(--sh-btn);
}
.mini-head {
  display: flex;
  flex-direction: column;
  gap: 6px;
  margin-top: 2px;
}
.mini-title {
  height: 16px;
  width: 66%;
  border-radius: var(--r-chip, 4px);
  background: var(--tx-1);
  opacity: 0.85;
}
.mini-sub {
  height: 7px;
  width: 88%;
  border-radius: var(--r-pill, 4px);
  background: var(--tx-3);
  opacity: 0.45;
}
.mini-kpis {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 8px;
}
.mini-card {
  display: flex;
  flex-direction: column;
  gap: 6px;
  padding: 9px;
  border-radius: var(--r-card, 6px);
  border: 1px solid var(--bd);
  background: var(--bg-surface);
  background-image: var(--card-bg-image, none);
  box-shadow: var(--sh-card);
}
.mini-kpi-label {
  height: 6px;
  width: 62%;
  border-radius: var(--r-pill, 4px);
  background: var(--tx-3);
  opacity: 0.5;
}
.mini-kpi-num {
  height: 13px;
  width: 48%;
  border-radius: var(--r-chip, 4px);
  background: var(--tx-1);
  opacity: 0.9;
}
.mini-bar {
  display: block;
  height: 5px;
  border-radius: var(--r-pill, 3px);
  background: var(--bg-elevated);
  overflow: hidden;
}
.mini-bar > i {
  display: block;
  height: 100%;
  background: var(--ac);
  background-image: var(--ac-image, none);
}
.mini-graph {
  flex: 1;
  min-height: 0;
  border-radius: var(--r-card, 6px);
  border: 1px solid var(--bd);
  background: var(--og-bg, var(--bg-surface-2));
  padding: 4px;
  display: flex;
  align-items: center;
}
.mini-svg {
  width: 100%;
  height: 100%;
  display: block;
}
.mini-rows {
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.mini-row {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 8px;
  border-radius: var(--r-input, 4px);
  border: 1px solid var(--bd);
  background: var(--bg-surface-2);
}
.mini-cell {
  height: 6px;
  border-radius: var(--r-pill, 4px);
  background: var(--tx-2);
  opacity: 0.5;
}
.mini-cell--short {
  width: 16%;
  margin-left: auto;
}
.mini-pill {
  width: 26px;
  height: 10px;
  border-radius: var(--r-pill, 4px);
  border: 1px solid var(--bd);
  background: var(--ac-soft);
  flex-shrink: 0;
}
</style>
