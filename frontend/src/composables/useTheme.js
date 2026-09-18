/**
 * 风格 / 明暗状态
 *
 * 设计说明（重要）：
 * 本项目有 **11 套风格**，每套风格自带明暗基调（7 套暗色 + 4 套亮色），
 * 「明暗」不是可以独立于风格切换的一个属性——把暗色风格强行反相成亮色，
 * 就得为 11 套各手写一份亮色令牌，且极易做出难看的结果。
 *
 * 所以「一键切换明暗」的实现是：跳到**另一种基调里你上次用过的风格**
 * （默认暗色 → Linear，亮色 → 柔雾靛）。一次点击完成，且每套风格都保持原设计水准。
 *
 * 风格选择记在 localStorage（这是界面偏好，不是业务数据，所以没有进库）。
 */

import { computed, ref, watch } from 'vue'
import { STYLES, STYLE_MAP } from '@/styles/registry'

const LS_KEY = 'bcrm.style'

/** 各基调的默认落点 */
const TONE_DEFAULT = { dark: 'linear', light: 'soft-ui' }

function readSaved() {
  try {
    const k = localStorage.getItem(LS_KEY)
    return k && STYLE_MAP[k] ? k : null
  } catch {
    return null
  }
}

export function useTheme() {
  const activeKey = ref(readSaved() ?? STYLES[1].key)
  const menuOpen = ref(false)
  const toneFilter = ref('all') // all | dark | light —— 同时作为下拉里的筛选

  /** 记录各基调最后用过的风格，供「切明暗」跳回 */
  const lastOf = ref({
    dark: STYLE_MAP[activeKey.value]?.tone === 'dark' ? activeKey.value : TONE_DEFAULT.dark,
    light: STYLE_MAP[activeKey.value]?.tone === 'light' ? activeKey.value : TONE_DEFAULT.light,
  })

  const meta = computed(() => STYLE_MAP[activeKey.value] ?? STYLES[0])
  const tone = computed(() => meta.value.tone)

  /** 下拉里展示的风格（受明暗筛选影响） */
  const visibleStyles = computed(() =>
    toneFilter.value === 'all' ? STYLES : STYLES.filter((s) => s.tone === toneFilter.value),
  )

  function setStyle(key) {
    if (!STYLE_MAP[key]) return
    activeKey.value = key
    const t = STYLE_MAP[key].tone
    lastOf.value = { ...lastOf.value, [t]: key }
  }

  /** 一键切明暗：跳到另一基调上次用过的风格 */
  function toggleTone() {
    const target = tone.value === 'dark' ? 'light' : 'dark'
    setStyle(lastOf.value[target] ?? TONE_DEFAULT[target])
  }

  /** 顺序切换（下拉里的「上一套 / 下一套」，遵循当前筛选） */
  function step(delta) {
    const list = visibleStyles.value
    if (!list.length) return
    const i = list.findIndex((s) => s.key === activeKey.value)
    const next = list[((i < 0 ? 0 : i) + delta + list.length) % list.length]
    setStyle(next.key)
  }

  function cycleToneFilter() {
    toneFilter.value =
      toneFilter.value === 'all' ? 'dark' : toneFilter.value === 'dark' ? 'light' : 'all'
    // 当前风格被筛掉时，落到筛选结果里的第一套
    if (!visibleStyles.value.some((s) => s.key === activeKey.value) && visibleStyles.value[0]) {
      setStyle(visibleStyles.value[0].key)
    }
  }

  /** 直接指定明暗筛选（下拉里的三个按钮用），当前风格被筛掉时自动落到第一套 */
  function setFilter(k) {
    if (!['all', 'dark', 'light'].includes(k)) return
    toneFilter.value = k
    if (!visibleStyles.value.some((s) => s.key === activeKey.value) && visibleStyles.value[0]) {
      setStyle(visibleStyles.value[0].key)
    }
  }

  watch(
    activeKey,
    (k) => {
      try {
        localStorage.setItem(LS_KEY, k)
      } catch {
        /* 隐私模式下 localStorage 可能不可用，忽略即可 */
      }
    },
    { immediate: true },
  )

  /** 键盘 ← / → 切换风格（输入框内不触发） */
  function onKey(e) {
    const tag = (e.target?.tagName ?? '').toLowerCase()
    if (tag === 'input' || tag === 'textarea' || tag === 'select') return
    if (e.key === 'ArrowRight') step(1)
    if (e.key === 'ArrowLeft') step(-1)
  }

  return {
    activeKey,
    meta,
    tone,
    menuOpen,
    toneFilter,
    visibleStyles,
    setStyle,
    toggleTone,
    step,
    cycleToneFilter,
    setFilter,
    onKey,
  }
}
