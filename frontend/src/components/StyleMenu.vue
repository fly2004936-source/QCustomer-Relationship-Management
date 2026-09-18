<template>
  <!-- 顶栏风格切换：整个 11 套风格压缩成一个单元格，点击才展开 -->
  <div ref="root" class="stylebox">
    <button
      class="stylecell"
      :class="{ 'is-open': open }"
      :aria-expanded="open"
      aria-haspopup="listbox"
      :title="`当前风格：${meta.name}｜${meta.notes}`"
      @click="emit('toggle')"
    >
      <span
        class="stylecell-sw"
        :style="{ background: meta.swatch.bg, boxShadow: `0 0 0 1px ${meta.swatch.accent}` }"
      ></span>
      <span class="stylecell-name">{{ meta.name }}</span>
      <span class="stylecell-idx">{{ meta.idx }}/{{ total }}</span>
      <AppIcon :name="open ? 'arrowup' : 'arrowdown'" :size="13" />
    </button>

    <div v-if="open" class="styledrop" role="listbox" aria-label="选择风格">
      <div class="styledrop-head">
        <span class="styledrop-title">界面风格</span>
        <span class="styledrop-note">{{ visible.length }} / {{ total }} 套</span>
      </div>

      <!-- 明暗筛选 + 顺序切换 -->
      <div class="styledrop-tools">
        <div class="seg" role="group" aria-label="明暗筛选">
          <button
            v-for="f in FILTERS"
            :key="f.key"
            class="seg-btn"
            :class="{ 'is-on': toneFilter === f.key }"
            @click="emit('set-filter', f.key)"
          >
            {{ f.label }}
          </button>
        </div>
        <button class="btn btn--sm btn--ghost" title="上一套（←）" @click="emit('step', -1)">
          <AppIcon name="chevronLeft" :size="13" />
        </button>
        <button class="btn btn--sm btn--ghost" title="下一套（→）" @click="emit('step', 1)">
          <AppIcon name="chevronRight" :size="13" />
        </button>
      </div>

      <div class="styledrop-grid">
        <button
          v-for="s in visible"
          :key="s.key"
          class="sopt"
          :class="{ 'is-on': s.key === activeKey }"
          role="option"
          :aria-selected="s.key === activeKey"
          :title="`${s.name}｜${s.notes}`"
          @click="emit('select', s.key)"
        >
          <span
            class="stylecell-sw"
            :style="{ background: s.swatch.bg, boxShadow: `0 0 0 1px ${s.swatch.accent}` }"
          ></span>
          <span class="sopt-body">
            <span class="sopt-name">{{ s.idx }} · {{ s.name }}</span>
            <span class="sopt-note">{{ s.note }}</span>
          </span>
          <span class="sopt-tone">{{ s.tone === 'dark' ? '暗' : '亮' }}</span>
        </button>
        <div v-if="!visible.length" class="empty" style="grid-column: 1 / -1">该明暗下暂无风格</div>
      </div>

      <div class="styledrop-foot">
        <button class="btn btn--sm btn--soft" @click="emit('toggle-tone')">
          <AppIcon :name="tone === 'dark' ? 'sun' : 'moon'" :size="13" />
          切换到{{ tone === 'dark' ? '亮色' : '暗色' }}
        </button>
        <button class="btn btn--sm btn--ghost" @click="emit('open-gallery')">
          <AppIcon name="grid" :size="13" />
          风格总览
        </button>
        <span class="card-sub" style="margin-left: auto">← → 可切换</span>
      </div>
    </div>
  </div>
</template>

<script setup>
import { onBeforeUnmount, onMounted, ref } from 'vue'
import AppIcon from './AppIcon.vue'

const props = defineProps({
  activeKey: { type: String, required: true },
  meta: { type: Object, required: true },
  tone: { type: String, default: 'dark' },
  toneFilter: { type: String, default: 'all' },
  visible: { type: Array, default: () => [] },
  total: { type: Number, default: 11 },
  open: { type: Boolean, default: false },
})

const emit = defineEmits(['toggle', 'select', 'step', 'toggle-tone', 'set-filter', 'open-gallery', 'close'])

const FILTERS = [
  { key: 'all', label: '全部' },
  { key: 'dark', label: '暗色' },
  { key: 'light', label: '亮色' },
]

const root = ref(null)

function onDocDown(e) {
  if (!props.open) return
  if (root.value && !root.value.contains(e.target)) emit('close')
}
function onEsc(e) {
  if (e.key === 'Escape' && props.open) emit('close')
}

onMounted(() => {
  document.addEventListener('pointerdown', onDocDown)
  document.addEventListener('keydown', onEsc)
})
onBeforeUnmount(() => {
  document.removeEventListener('pointerdown', onDocDown)
  document.removeEventListener('keydown', onEsc)
})
</script>
