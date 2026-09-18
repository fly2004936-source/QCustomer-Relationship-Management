<template>
  <!-- 记事：快速加待办、点勾完成、可删（需求 2） -->
  <section class="card wb-col-4">
    <div class="card-head">
      <div class="card-title">
        <AppIcon name="check" :size="15" />
        <span>今日待办</span>
      </div>
      <div class="card-actions">
        <span class="tag">{{ doneCount }}/{{ todos.length }}</span>
      </div>
    </div>

    <div class="todo-add">
      <input
        v-model="draft"
        class="input"
        placeholder="加一条待办，回车确认"
        maxlength="120"
        @keyup.enter="submit"
      />
      <select v-model="priority" class="select" style="width: 82px; flex-shrink: 0" title="优先级">
        <option value="">优先级</option>
        <option value="高">高</option>
        <option value="中">中</option>
        <option value="低">低</option>
      </select>
      <button class="icon-btn" title="添加" :disabled="!draft.trim()" @click="submit">
        <AppIcon name="plus" :size="15" />
      </button>
    </div>

    <div class="todo-list">
      <div v-for="t in todos" :key="t.id" class="todo" :class="{ 'is-done': t.done }">
        <button class="todo-box" :title="t.done ? '标记未完成' : '标记完成'" @click="emit('toggle', t)">
          <AppIcon v-if="t.done" name="check" :size="11" :stroke-width="3" />
        </button>
        <span class="todo-text">{{ t.item }}</span>
        <span class="todo-meta">
          <span v-if="t.priority" class="tag" :class="priCls(t.priority)">{{ t.priority }}</span>
          <button class="todo-del" title="删除" @click="emit('remove', t)">
            <AppIcon name="trash" :size="13" />
          </button>
        </span>
      </div>

      <div v-if="!todos.length" class="empty">
        {{ loading ? '加载中…' : '今天还没有待办，加一条试试' }}
      </div>
    </div>
  </section>
</template>

<script setup>
import { computed, ref } from 'vue'
import AppIcon from '../AppIcon.vue'

const props = defineProps({
  todos: { type: Array, default: () => [] },
  loading: { type: Boolean, default: false },
})

const emit = defineEmits(['add', 'toggle', 'remove'])

const draft = ref('')
const priority = ref('')

const doneCount = computed(() => props.todos.filter((t) => t.done).length)

function priCls(p) {
  if (p === '高') return 'tag--bad'
  if (p === '中') return 'tag--warn'
  return 'tag--info'
}

function submit() {
  const item = draft.value.trim()
  if (!item) return
  emit('add', { item, priority: priority.value || null })
  draft.value = ''
  // 优先级保留上次选择，连续录入多条同优先级的待办更省事
}
</script>
