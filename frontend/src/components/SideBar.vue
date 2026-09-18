<template>
  <!-- 左侧可收起侧边栏：工作台 / 组织网络图 / 数据分析 / 数据管理（客户·项目·合作方·竞争方…） -->
  <aside class="side" :class="{ 'side--mini': collapsed }">
    <div class="side-inner">
      <div v-if="!collapsed" class="side-brand">
        <span class="side-logo" aria-hidden="true"></span>
        <div class="side-brand-text">
          <div class="side-brand-name">BCRM 客户管理</div>
          <div class="side-brand-sub">{{ meta.name }}</div>
        </div>
      </div>
      <div v-else class="side-brand" style="justify-content: center; padding-bottom: 8px">
        <span class="side-logo" aria-hidden="true"></span>
      </div>

      <!-- 主功能 -->
      <button
        class="side-item"
        :class="{ 'is-on': view === 'workbench' }"
        :title="collapsed ? '工作台' : ''"
        @click="emit('navigate', 'workbench')"
      >
        <AppIcon name="home" :size="16" />
        <span v-if="!collapsed" class="side-item-text">工作台</span>
      </button>

      <button
        class="side-item"
        :class="{ 'is-on': view === 'graph' }"
        :title="collapsed ? '组织网络图' : ''"
        @click="emit('navigate', 'graph')"
      >
        <AppIcon name="network" :size="16" />
        <span v-if="!collapsed" class="side-item-text">组织网络图</span>
      </button>

      <button
        class="side-item"
        :class="{ 'is-on': view === 'analytics' }"
        :title="collapsed ? '数据分析' : ''"
        @click="emit('navigate', 'analytics')"
      >
        <AppIcon name="analytics" :size="16" />
        <span v-if="!collapsed" class="side-item-text">数据分析</span>
      </button>

      <!-- 数据管理 -->
      <div v-if="!collapsed" class="side-group-label">数据管理 · 增删改查</div>
      <button
        v-else
        class="side-item"
        :class="{ 'is-on': view === 'crud' }"
        title="数据管理"
        @click="emit('expand-and-crud')"
      >
        <AppIcon name="database" :size="16" />
      </button>

      <template v-for="cat in NAV_TREE" :key="cat.key">
        <!-- 收起态：只显示大类图标 -->
        <button
          v-if="collapsed"
          class="side-item"
          :class="{ 'is-on': view === 'crud' && activeCategory === cat.key }"
          :title="cat.label"
          @click="emit('expand-and-crud')"
        >
          <AppIcon :name="cat.icon" :size="16" />
        </button>

        <!-- 展开态：手风琴 -->
        <template v-else>
          <button
            class="side-item"
            :class="{ 'is-on': view === 'crud' && activeCategory === cat.key && !isOpen(cat.key) }"
            @click="toggleCat(cat.key)"
          >
            <AppIcon :name="cat.icon" :size="15" />
            <span class="side-item-text">{{ cat.label }}</span>
            <AppIcon
              class="side-item-caret"
              :class="{ 'is-open': isOpen(cat.key) }"
              name="chevronRight"
              :size="12"
            />
          </button>

          <div v-if="isOpen(cat.key)" class="side-sublist">
            <template v-for="g in cat.groups" :key="g.label">
              <div class="side-sub-group">{{ g.label }}</div>
              <button
                v-for="it in g.items"
                :key="it.name"
                class="side-sub"
                :class="{ 'is-on': view === 'crud' && activeTable === it.name }"
                :title="`${it.label} · ${it.name}`"
                @click="openTable(it.name)"
              >
                <AppIcon :name="g.label === '人员' ? 'users' : 'table'" :size="12" />
                <span style="overflow: hidden; text-overflow: ellipsis">{{ it.label }}</span>
                <span v-if="counts && counts[it.name] != null" class="side-sub-count">{{ counts[it.name] }}</span>
              </button>
            </template>
          </div>
        </template>
      </template>

      <div v-if="!collapsed" class="side-foot">
        <div class="row" style="gap: 6px">
          <span class="dot" :class="backend.state === 'online' ? 'dot--online' : 'dot--offline'"></span>
          <span>{{ backend.state === 'online' ? '后端在线' : '后端离线' }}</span>
        </div>
        <div style="margin-top: 4px">
          {{ backend.tableCount || '—' }} 张表
          <template v-if="backend.dbPath">
            <br />
            <span class="mono" style="word-break: break-all">{{ backend.dbPath }}</span>
          </template>
        </div>
        <div style="margin-top: 4px">共 {{ ALL_TABLES.length }} 张表已归入导航</div>
      </div>
    </div>
  </aside>
</template>

<script setup>
import { computed, ref } from 'vue'
import AppIcon from './AppIcon.vue'
import { NAV_TREE, ALL_TABLES } from '@/nav/dataTree'

const props = defineProps({
  collapsed: { type: Boolean, default: false },
  view: { type: String, default: 'workbench' },
  activeTable: { type: String, default: '' },
  counts: { type: Object, default: null },
  backend: { type: Object, default: () => ({ state: 'unknown', dbPath: '', tableCount: 0 }) },
  meta: { type: Object, default: () => ({ name: '' }) },
})

const emit = defineEmits(['navigate', 'open-table', 'toggle-collapse', 'expand-and-crud'])

/** 打开的手风琴大类 */
const openCats = ref(new Set(['customer']))
const isOpen = (k) => openCats.value.has(k)

function toggleCat(k) {
  const next = new Set(openCats.value)
  if (next.has(k)) next.delete(k)
  else next.add(k)
  openCats.value = next
}

/** 高亮用：当前表所属的大类 */
const activeCategory = computed(() => {
  if (!props.activeTable) return ''
  const cat = NAV_TREE.find((c) => c.groups.some((g) => g.items.some((i) => i.name === props.activeTable)))
  return cat?.key ?? ''
})

/** 打开某张大类的表时自动展开它 */
function openTable(name) {
  const cat = NAV_TREE.find((c) => c.groups.some((g) => g.items.some((i) => i.name === name)))
  if (cat && !isOpen(cat.key)) {
    const next = new Set(openCats.value)
    next.add(cat.key)
    openCats.value = next
  }
  emit('open-table', name)
}

defineExpose({ openTable })
</script>
