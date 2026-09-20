<template>
  <!-- 可移动浮层面板：拖标题栏移动，双击标题栏复位，位置约束在父容器内 -->
  <section
    ref="el"
    class="panel"
    :class="{ 'panel--wide': wide }"
    :style="{ left: pos.x + 'px', top: pos.y + 'px' }"
    role="dialog"
    :aria-label="title"
  >
    <header class="panel-head" @pointerdown="onDown" @dblclick="reset">
      <AppIcon class="panel-grip" name="move" :size="14" />
      <div class="panel-title">
        <div class="panel-title-main">{{ title }}</div>
        <div v-if="subtitle" class="panel-title-sub">{{ subtitle }}</div>
      </div>
      <div class="panel-tools">
        <slot name="tools" />
        <button class="icon-btn" style="width: 26px; height: 26px" title="复位位置" @click="reset">
          <AppIcon name="fit" :size="13" />
        </button>
        <button class="icon-btn" style="width: 26px; height: 26px" title="关闭" @click="emit('close')">
          <AppIcon name="close" :size="13" />
        </button>
      </div>
    </header>
    <div class="panel-body">
      <slot />
    </div>
  </section>
</template>

<script setup>
import { onBeforeUnmount, onMounted, ref } from 'vue'
import AppIcon from './AppIcon.vue'

const props = defineProps({
  title: { type: String, default: '' },
  subtitle: { type: String, default: '' },
  wide: { type: Boolean, default: false },
  startX: { type: Number, default: 18 },
  startY: { type: Number, default: 18 },
})

const emit = defineEmits(['close'])

const el = ref(null)
const pos = ref({ x: props.startX, y: props.startY })

let dragging = false
let origin = { x: 0, y: 0, px: 0, py: 0 }

/** 把位置夹在父容器可视范围内，保证面板不会被拖出屏幕 */
function clamp(p) {
  const box = el.value
  const parent = box?.parentElement
  if (!box || !parent) return p
  const maxX = Math.max(0, parent.clientWidth - box.offsetWidth - 6)
  const maxY = Math.max(0, parent.clientHeight - box.offsetHeight - 6)
  return {
    x: Math.min(Math.max(0, p.x), maxX),
    y: Math.min(Math.max(0, p.y), maxY),
  }
}

function onDown(e) {
  if (e.button !== undefined && e.button !== 0) return
  // 点在工具按钮上时不启动拖拽，否则会吞掉点击
  if (e.target.closest('button')) return
  dragging = true
  origin = { x: e.clientX, y: e.clientY, px: pos.value.x, py: pos.value.y }
  window.addEventListener('pointermove', onMove)
  window.addEventListener('pointerup', onUp)
  window.addEventListener('pointercancel', onUp)
}

function onMove(e) {
  if (!dragging) return
  pos.value = clamp({
    x: origin.px + (e.clientX - origin.x),
    y: origin.py + (e.clientY - origin.y),
  })
}

function onUp() {
  dragging = false
  window.removeEventListener('pointermove', onMove)
  window.removeEventListener('pointerup', onUp)
  window.removeEventListener('pointercancel', onUp)
}

function reset() {
  pos.value = clamp({ x: props.startX, y: props.startY })
}

function onResize() {
  pos.value = clamp(pos.value)
}

onMounted(() => {
  pos.value = clamp({ x: props.startX, y: props.startY })
  window.addEventListener('resize', onResize)
})
onBeforeUnmount(() => {
  onUp()
  window.removeEventListener('resize', onResize)
})

defineExpose({ reset })
</script>
