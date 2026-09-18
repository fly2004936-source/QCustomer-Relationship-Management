<template>
  <!-- 快捷入口：一排图标，点了新标签页打开，能自己加自己删（需求 4） -->
  <section class="card wb-col-4">
    <div class="card-head">
      <div class="card-title">
        <AppIcon name="layers" :size="15" />
        <span>快捷入口</span>
      </div>
      <div class="card-actions">
        <button
          class="icon-btn"
          style="width: 28px; height: 28px"
          :title="adding ? '取消' : '添加入口'"
          @click="adding = !adding"
        >
          <AppIcon :name="adding ? 'close' : 'plus'" :size="14" />
        </button>
      </div>
    </div>

    <div v-if="adding" class="row" style="gap: 6px; margin-bottom: 10px; flex-wrap: wrap">
      <input v-model="title" class="input" style="flex: 1 1 110px; min-height: 32px" placeholder="名称" maxlength="30" />
      <input
        v-model="url"
        class="input"
        style="flex: 2 1 160px; min-height: 32px"
        placeholder="https://…"
        maxlength="300"
        @keyup.enter="submit"
      />
      <button class="btn btn--sm btn--primary" @click="submit">添加</button>
    </div>

    <div class="sc-grid">
      <a
        v-for="s in shortcuts"
        :key="s.id"
        class="sc"
        :href="s.url"
        target="_blank"
        rel="noopener noreferrer"
        :title="s.url"
      >
        <span class="sc-mark">
          <AppIcon v-if="isIcon(s.icon)" :name="s.icon" :size="17" />
          <template v-else>{{ monogram(s.title) }}</template>
        </span>
        <span class="sc-title">{{ s.title }}</span>
        <button class="sc-del" title="删除" @click.prevent.stop="emit('remove', s)">
          <AppIcon name="close" :size="11" />
        </button>
      </a>

      <button class="sc sc-add" @click="adding = true">
        <span class="sc-mark" style="background: transparent; border: 1px dashed var(--bd)">
          <AppIcon name="plus" :size="16" />
        </span>
        <span class="sc-title">添加</span>
      </button>
    </div>

    <div v-if="!shortcuts.length && !adding" class="empty">还没有快捷入口</div>
  </section>
</template>

<script setup>
import { ref } from 'vue'
import AppIcon from '../AppIcon.vue'

const props = defineProps({
  shortcuts: { type: Array, default: () => [] },
})

const emit = defineEmits(['add', 'remove'])

/** 与 AppIcon 内置集合保持一致，icon 命中就画图标，否则退化成首字标 */
const ICONS = new Set([
  'home', 'network', 'database', 'table', 'building', 'briefcase', 'link', 'crosshair',
  'users', 'grid', 'layers', 'coin', 'target', 'shield', 'clock', 'note', 'news', 'trend',
  'calendar', 'search', 'flag', 'bell', 'spark', 'folder', 'globe',
])

const adding = ref(false)
const title = ref('')
const url = ref('')

const isIcon = (n) => !!n && ICONS.has(n)
const monogram = (s) => (String(s || '?').trim()[0] ?? '?')

function normalizeUrl(u) {
  const s = String(u || '').trim()
  if (!s) return ''
  if (/^https?:\/\//i.test(s)) return s
  return `https://${s}`
}

function submit() {
  const t = title.value.trim()
  const u = normalizeUrl(url.value)
  if (!t || !u) return
  emit('add', { title: t, url: u, sort_order: (props.shortcuts.length + 1) * 10 })
  title.value = ''
  url.value = ''
  adding.value = false
}
</script>
