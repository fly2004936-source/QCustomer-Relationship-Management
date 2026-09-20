<template>
  <!-- 专注计时：番茄钟，可开始 / 暂停 / 重置，完成后落库（需求 5） -->
  <section class="card wb-col-4">
    <div class="card-head">
      <div class="card-title">
        <AppIcon name="clock" :size="15" />
        <span>专注计时</span>
      </div>
      <div class="card-actions">
        <span class="tag" :class="doneToday ? 'tag--ok' : ''">今日 {{ doneToday }} 个</span>
        <span v-if="totalToday" class="card-sub">{{ totalToday }} 分钟</span>
      </div>
    </div>

    <div class="pomo">
      <div class="pomo-ring">
        <svg viewBox="0 0 100 100">
          <circle class="pomo-track" cx="50" cy="50" r="44" />
          <circle
            class="pomo-fill"
            cx="50"
            cy="50"
            r="44"
            :stroke-dasharray="C"
            :stroke-dashoffset="dashOffset"
          />
        </svg>
        <div class="pomo-center">
          <div class="pomo-time">{{ mmss }}</div>
          <div class="pomo-state">{{ stateText }}</div>
        </div>
      </div>

      <div class="pomo-presets">
        <button
          v-for="m in PRESETS"
          :key="m"
          class="pomo-preset"
          :class="{ 'is-on': planMin === m }"
          :disabled="running"
          @click="setPreset(m)"
        >
          {{ m }} 分
        </button>
      </div>

      <div class="pomo-ctrl">
        <button class="btn btn--primary btn--sm" :disabled="finished" @click="toggle">
          <AppIcon :name="running ? 'pause' : 'play'" :size="13" />
          {{ running ? '暂停' : elapsed > 0 ? '继续' : '开始' }}
        </button>
        <button class="btn btn--sm btn--ghost" @click="reset">
          <AppIcon name="refresh" :size="13" />
          重置
        </button>
      </div>

      <div class="card-sub" style="text-align: center">
        {{ running ? '保持专注，别切窗口' : finished ? '本轮已完成，已记入数据库' : '选一个时长开始' }}
      </div>
    </div>
  </section>
</template>

<script setup>
import { computed, onBeforeUnmount, ref, watch } from 'vue'
import AppIcon from '../AppIcon.vue'

const props = defineProps({
  /** 今天已完成的番茄数 */
  doneToday: { type: Number, default: 0 },
  /** 今天累计专注分钟 */
  totalToday: { type: Number, default: 0 },
})

const emit = defineEmits(['record'])

const PRESETS = [15, 25, 45]
const C = 2 * Math.PI * 44

const planMin = ref(25)
const elapsed = ref(0) // 已专注秒数
const running = ref(false)

let timer = null

const totalSec = computed(() => planMin.value * 60)
const remain = computed(() => Math.max(0, totalSec.value - elapsed.value))
const finished = computed(() => elapsed.value >= totalSec.value)

const mmss = computed(() => {
  const s = remain.value
  const p2 = (n) => String(n).padStart(2, '0')
  return `${p2(Math.floor(s / 60))}:${p2(s % 60)}`
})

const stateText = computed(() => {
  if (finished.value) return '已完成'
  if (running.value) return '专注中'
  return elapsed.value > 0 ? '已暂停' : '待开始'
})

const dashOffset = computed(() => C * (1 - elapsed.value / totalSec.value))

function tick() {
  if (!running.value) return
  elapsed.value += 1
  if (elapsed.value >= totalSec.value) {
    elapsed.value = totalSec.value
    stop()
    // 自然走完 → 记为「已完成」
    emit('record', { plan_min: planMin.value, actual_min: Math.round(elapsed.value / 60), outcome: '已完成' })
  }
}

function start() {
  if (finished.value) return
  running.value = true
  clearInterval(timer)
  timer = setInterval(tick, 1000)
}
function stop() {
  running.value = false
  clearInterval(timer)
  timer = null
}
function toggle() {
  running.value ? stop() : start()
}

/** 重置：已进行满 1 分钟就记一条「已中断」，否则直接丢弃，避免噪音记录 */
function reset() {
  const sec = elapsed.value
  stop()
  if (sec >= 60 && sec < totalSec.value) {
    emit('record', { plan_min: planMin.value, actual_min: Math.round(sec / 60), outcome: '已中断' })
  }
  elapsed.value = 0
}

function setPreset(m) {
  planMin.value = m
  elapsed.value = 0
  stop()
}

/** 换时长时把进度清零，避免出现 25 分钟已走一半再切 15 分钟直接超时的怪状态 */
watch(planMin, () => {
  if (!running.value) elapsed.value = 0
})

onBeforeUnmount(() => stop())
</script>
