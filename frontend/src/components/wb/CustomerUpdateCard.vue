<template>
  <!-- 客户今日信息更新 + 可跳转历史（需求 8） -->
  <section class="card wb-col-6">
    <div class="card-head">
      <div class="card-title">
        <AppIcon name="building" :size="15" />
        <span>客户信息更新</span>
      </div>
      <div class="card-actions">
        <div class="seg">
          <button class="seg-btn" :class="{ 'is-on': tab === 'today' }" @click="tab = 'today'">今天</button>
          <button class="seg-btn" :class="{ 'is-on': tab === 'history' }" @click="goHistory">历史</button>
        </div>
        <button class="icon-btn" style="width: 28px; height: 28px" title="刷新" @click="emit('refresh')">
          <AppIcon name="refresh" :size="13" />
        </button>
      </div>
    </div>

    <!-- 历史模式：先选客户再看该客户的全部更新 -->
    <div v-if="tab === 'history'" class="row" style="gap: 6px; margin-bottom: 10px">
      <select v-model="pick" class="select" style="min-height: 32px" @change="emit('load-history', pick)">
        <option value="">全部客户</option>
        <option v-for="c in customers" :key="c.id" :value="c.id">{{ c.name }}</option>
      </select>
      <span class="card-sub">{{ rows.length }} 条</span>
    </div>

    <div class="feed" style="max-height: 292px">
      <template v-for="grp in grouped" :key="grp.day">
        <div v-if="tab === 'history'" class="feed-day">{{ grp.label }} · {{ grp.day }}</div>
        <button
          v-for="r in grp.list"
          :key="r.id"
          class="feed-item"
          :title="r.summary || r.title"
          @click="emit('open-customer', r.customer_id)"
        >
          <div class="feed-top">
            <span class="tag tag--ac">{{ nameOf(r.customer_id) }}</span>
            <span v-if="r.is_new" class="tag tag--ok">增量</span>
            <span v-else class="tag">存量</span>
            <span v-if="r.our_relation" class="card-sub" style="margin-left: auto">{{ r.our_relation }}</span>
          </div>
          <div class="feed-title">{{ r.title }}</div>
          <div v-if="r.summary" class="feed-sum">{{ r.summary }}</div>
          <div class="feed-foot">
            <span>{{ r.news_date }}</span>
            <span v-if="r.source">{{ r.source }}</span>
          </div>
        </button>
      </template>

      <div v-if="!rows.length" class="empty">
        {{ tab === 'today' ? '今天暂无客户信息更新' : '该客户暂无历史更新' }}
      </div>
    </div>
  </section>
</template>

<script setup>
import { computed, ref } from 'vue'
import AppIcon from '../AppIcon.vue'
import { relativeDay } from '@/api/date'

const props = defineProps({
  /** 今天的更新 */
  today: { type: Array, default: () => [] },
  /** 历史模式下的行（由父组件按客户拉取） */
  history: { type: Array, default: () => [] },
  customers: { type: Array, default: () => [] },
  loading: { type: Boolean, default: false },
})

const emit = defineEmits(['refresh', 'open-customer', 'load-history'])

const tab = ref('today')
const pick = ref('')

const byId = computed(() => new Map(props.customers.map((c) => [c.id, c.name])))
const nameOf = (id) => byId.value.get(id) ?? (id ? `客户 #${id}` : '未知客户')

const rows = computed(() => (tab.value === 'today' ? props.today : props.history))

const grouped = computed(() => {
  const map = new Map()
  for (const r of rows.value) {
    const day = String(r.news_date ?? '').slice(0, 10) || '未标注日期'
    if (!map.has(day)) map.set(day, [])
    map.get(day).push(r)
  }
  return [...map.entries()]
    .sort((a, b) => (a[0] < b[0] ? 1 : -1))
    .map(([day, list]) => ({ day, label: day === '未标注日期' ? day : relativeDay(day), list }))
})

function goHistory() {
  tab.value = 'history'
  emit('load-history', pick.value)
}
</script>
