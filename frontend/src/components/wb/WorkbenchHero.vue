<template>
  <!-- 工作台顶部：问候 + 时间 + 今天第几周 + 一句话目标 + 今日进度（需求 1 与 6） -->
  <section class="card wb-hero">
    <div>
      <h2 class="wb-greet">
        {{ greeting }}<template v-if="name">，{{ name }}</template>
        <button v-else class="wb-name-add" title="设置你的称呼" @click="editName">写上你的名字</button>
      </h2>
      <div class="wb-date">
        <span class="row" style="gap: 5px">
          <AppIcon name="calendar" :size="13" />
          {{ dateText }}
        </span>
        <span class="tag">第 {{ week }} 周</span>
        <span class="tag">{{ weekday }}</span>
        <button class="btn btn--sm btn--ghost" title="修改称呼" @click="editName">
          <AppIcon name="edit" :size="12" />
        </button>
      </div>

      <!-- 一句话目标：改完即存，回车或失焦都触发 -->
      <div class="wb-goal">
        <AppIcon name="target" :size="14" />
        <input
          v-model="goalDraft"
          placeholder="今天的一句话目标（回车保存）"
          maxlength="120"
          @keyup.enter="commitGoal"
          @blur="commitGoal"
        />
        <span v-if="goalSaved" class="tag tag--ok">已保存</span>
        <span v-else-if="goalDirty" class="tag">未保存</span>
      </div>
    </div>

    <div class="wb-clock">
      <div class="wb-clock-time">
        {{ hhmm }}<span class="wb-clock-sec">:{{ ss }}</span>
      </div>
      <div class="wb-date" style="justify-content: flex-end; margin-top: 6px">
        <span>{{ tone === 'dark' ? '暗色主题' : '亮色主题' }} · {{ styleName }}</span>
      </div>

      <!-- 今日进度：完成待办 + 累计专注 -->
      <div class="wb-progress">
        <div class="wb-progress-top">
          <span>今日完成</span>
          <b>{{ done }}/{{ total }}</b>
          <span class="muted">·</span>
          <span>专注</span>
          <b>{{ focusMin }}</b>
          <span>分钟</span>
          <span style="margin-left: auto">{{ percent }}%</span>
        </div>
        <div class="bar">
          <i :style="{ width: percent + '%' }"></i>
        </div>
      </div>
    </div>
  </section>
</template>

<script setup>
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import AppIcon from '../AppIcon.vue'
import { isoWeek, WEEKDAY_CN } from '@/api/date'
const props = defineProps({
  goal: { type: String, default: '' },
  done: { type: Number, default: 0 },
  total: { type: Number, default: 0 },
  focusMin: { type: Number, default: 0 },
  tone: { type: String, default: 'dark' },
  styleName: { type: String, default: '' },
})

const emit = defineEmits(['save-goal'])

/* ---------------- 时钟 ---------------- */
const now = ref(new Date())
let timer = null

onMounted(() => {
  timer = setInterval(() => (now.value = new Date()), 1000)
})
onBeforeUnmount(() => clearInterval(timer))

const p2 = (n) => String(n).padStart(2, '0')
const hhmm = computed(() => `${p2(now.value.getHours())}:${p2(now.value.getMinutes())}`)
const ss = computed(() => p2(now.value.getSeconds()))

const dateText = computed(() => {
  const d = now.value
  return `${d.getFullYear()} 年 ${d.getMonth() + 1} 月 ${d.getDate()} 日`
})
const weekday = computed(() => WEEKDAY_CN[now.value.getDay()])
const week = computed(() => isoWeek(now.value))

const greeting = computed(() => {
  const h = now.value.getHours()
  if (h < 5) return '夜深了'
  if (h < 11) return '早上好'
  if (h < 13) return '中午好'
  if (h < 18) return '下午好'
  return '晚上好'
})

/* ---------------- 称呼 ---------------- */
// 存 localStorage 而不是数据库：这是「界面偏好」，不是业务数据。
// 没设过时留空，界面上显示一个「写上你的名字」的入口——
// 比硬塞一个「我」要好：'夜深了，我' 读起来很怪。
const LS_NAME = 'bcrm.uname'
const name = ref('')
onMounted(() => {
  try {
    const v = localStorage.getItem(LS_NAME)
    if (v) name.value = v
  } catch {
    /* localStorage 不可用时保持空 */
  }
})
function editName() {
  const v = window.prompt('怎么称呼你？（会记在这台电脑上）', name.value)
  if (v === null) return
  name.value = v.trim()
  try {
    if (name.value) localStorage.setItem(LS_NAME, name.value)
    else localStorage.removeItem(LS_NAME)
  } catch {
    /* 忽略 */
  }
}

/* ---------------- 一句话目标 ---------------- */
const goalDraft = ref(props.goal)
const goalDirty = ref(false)
const goalSaved = ref(false)
let savedTimer = null

watch(
  () => props.goal,
  (v) => {
    // 外部（例如切日期）更新时同步，但不覆盖正在编辑的内容
    if (!goalDirty.value) goalDraft.value = v ?? ''
  },
)

watch(goalDraft, () => {
  goalDirty.value = goalDraft.value !== (props.goal ?? '')
})

let debounce = null
/** 输入停顿 800ms 自动保存，避免每个字符都打一次接口 */
watch(goalDraft, () => {
  clearTimeout(debounce)
  if (!goalDirty.value) return
  debounce = setTimeout(commitGoal, 800)
})

function commitGoal() {
  if (!goalDirty.value) return
  emit('save-goal', goalDraft.value)
  goalDirty.value = false
  goalSaved.value = true
  clearTimeout(savedTimer)
  savedTimer = setTimeout(() => (goalSaved.value = false), 1800)
}

onBeforeUnmount(() => {
  clearTimeout(debounce)
  clearTimeout(savedTimer)
})

/* ---------------- 进度 ---------------- */
const percent = computed(() => {
  if (!props.total) return 0
  return Math.round((props.done / props.total) * 100)
})
</script>
