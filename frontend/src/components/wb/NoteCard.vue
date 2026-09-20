<template>
  <!-- 记灵感：随手写的便签区，输入停顿后自动保存（需求 3） -->
  <section class="card wb-col-4">
    <div class="card-head">
      <div class="card-title">
        <AppIcon name="note" :size="15" />
        <span>灵感便签</span>
      </div>
      <div class="card-actions">
        <span v-if="saved" class="tag tag--ok">已保存</span>
        <button class="icon-btn" style="width: 28px; height: 28px" title="新建便签" @click="create">
          <AppIcon name="plus" :size="14" />
        </button>
        <button
          class="icon-btn"
          style="width: 28px; height: 28px"
          title="删除当前便签"
          :disabled="!active"
          @click="remove"
        >
          <AppIcon name="trash" :size="14" />
        </button>
      </div>
    </div>

    <div v-if="notes.length > 1" class="note-tabs">
      <button
        v-for="n in notes"
        :key="n.id"
        class="note-tab"
        :class="{ 'is-on': n.id === activeId }"
        :title="tabTitle(n)"
        @click="select(n.id)"
      >
        {{ tabTitle(n) }}
      </button>
    </div>

    <textarea
      v-model="draft"
      class="note-area"
      placeholder="随手记点什么，停下来就会自动保存…"
      @input="onInput"
      @blur="flush"
    ></textarea>

    <div class="note-foot">
      <span>{{ draft.length }} 字</span>
      <span v-if="active?.updated_at">· 上次保存 {{ active.updated_at }}</span>
      <span style="margin-left: auto">{{ notes.length }} 条便签</span>
    </div>
  </section>
</template>

<script setup>
import { computed, onBeforeUnmount, ref, watch } from 'vue'
import AppIcon from '../AppIcon.vue'

const props = defineProps({
  notes: { type: Array, default: () => [] },
  loading: { type: Boolean, default: false },
})

const emit = defineEmits(['create', 'save', 'remove', 'select'])

const activeId = ref(null)
const draft = ref('')
const saved = ref(false)
let debounce = null
let savedTimer = null

const active = computed(() => props.notes.find((n) => n.id === activeId.value) ?? null)

/** 自动选中：优先上次选中的，其次第一条 */
watch(
  () => props.notes,
  (list) => {
    if (!list.length) {
      activeId.value = null
      draft.value = ''
      return
    }
    if (!list.some((n) => n.id === activeId.value)) {
      activeId.value = list[0].id
      draft.value = list[0].content ?? ''
    }
  },
  { immediate: true, deep: false },
)

watch(activeId, (id) => {
  const n = props.notes.find((x) => x.id === id)
  draft.value = n?.content ?? ''
})

function tabTitle(n) {
  const first = String(n.content ?? '').split('\n')[0].trim()
  if (!first) return `便签 ${n.id}`
  return first.length > 10 ? `${first.slice(0, 10)}…` : first
}

function select(id) {
  if (id === activeId.value) return
  flush()
  activeId.value = id
}

function onInput() {
  clearTimeout(debounce)
  debounce = setTimeout(flush, 700)
}

/** 立即保存当前草稿（自动保存与切页/失焦共用） */
function flush() {
  clearTimeout(debounce)
  const n = active.value
  if (!n) return
  if ((n.content ?? '') === draft.value) return
  emit('save', { id: n.id, content: draft.value })
  saved.value = true
  clearTimeout(savedTimer)
  savedTimer = setTimeout(() => (saved.value = false), 1600)
}

function create() {
  flush()
  emit('create')
}

function remove() {
  if (active.value) emit('remove', active.value)
}

onBeforeUnmount(() => {
  clearTimeout(debounce)
  clearTimeout(savedTimer)
})

/** 供父组件在离开页面前调用，保证不丢字 */
defineExpose({ flush })
</script>
