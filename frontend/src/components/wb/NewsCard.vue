<template>
  <!-- 资讯卡片：网络安全 / AI / 金融科技 三处复用（需求 7、10、11） -->
  <section class="card wb-col-4 card--hover">
    <div class="card-head">
      <div class="card-title">
        <AppIcon :name="icon" :size="15" />
        <span>{{ title }}</span>
      </div>
      <div class="card-actions">
        <span class="card-sub">{{ items.length }} 条</span>
        <button class="icon-btn" style="width: 28px; height: 28px" title="刷新" :disabled="loading" @click="emit('refresh')">
          <AppIcon name="refresh" :size="13" />
        </button>
      </div>
    </div>

    <div class="feed">
      <template v-for="grp in grouped" :key="grp.day">
        <div class="feed-day">{{ grp.label }}</div>
        <button
          v-for="n in grp.list"
          :key="n.id"
          class="feed-item"
          :title="n.summary || n.title"
          @click="emit('open', n)"
        >
          <div class="feed-top">
            <span class="tag" :class="catCls(n.category)">{{ catLabel(n.category) }}</span>
            <span v-if="n.source" class="card-sub" style="margin-left: auto">{{ n.source }}</span>
          </div>
          <div class="feed-title">{{ n.title }}</div>
          <div v-if="n.summary" class="feed-sum">{{ n.summary }}</div>
        </button>
      </template>

      <div v-if="!items.length" class="empty">{{ loading ? '加载中…' : emptyText }}</div>
    </div>
  </section>
</template>

<script setup>
import { computed } from 'vue'
import AppIcon from '../AppIcon.vue'
import { relativeDay } from '@/api/date'
import { CATEGORY_LABEL } from '@/api/workbench'

const props = defineProps({
  title: { type: String, required: true },
  icon: { type: String, default: 'news' },
  items: { type: Array, default: () => [] },
  loading: { type: Boolean, default: false },
  emptyText: { type: String, default: '暂无资讯' },
})

const emit = defineEmits(['refresh', 'open'])

const catLabel = (c) => CATEGORY_LABEL[c] ?? c ?? '资讯'
function catCls(c) {
  if (c === 'AI安全') return 'tag--ac'
  if (c === '攻防事件/漏洞') return 'tag--bad'
  if (c === '政策法规') return 'tag--warn'
  return 'tag--info'
}

/** 按日期分组，最近的在最上面 */
const grouped = computed(() => {
  const map = new Map()
  for (const n of props.items) {
    const day = String(n.news_date ?? '').slice(0, 10) || '未标注日期'
    if (!map.has(day)) map.set(day, [])
    map.get(day).push(n)
  }
  return [...map.entries()].map(([day, list]) => ({
    day,
    label: day === '未标注日期' ? day : relativeDay(day),
    list,
  }))
})
</script>
