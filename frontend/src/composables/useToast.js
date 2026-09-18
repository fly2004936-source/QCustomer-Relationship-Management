/**
 * 轻提示（Toast）
 * 全局单例：任何组件 import 后调用 push()，由 App.vue 统一渲染。
 * 写入成功 / 失败都给反馈，避免"点了没反应"的错觉。
 */

import { ref } from 'vue'

const items = ref([])
let seq = 0

/**
 * @param {string} message 文案
 * @param {'ok'|'err'|''} [kind] 语气
 * @param {number} [ms] 停留时长
 */
function push(message, kind = '', ms = 2600) {
  const id = ++seq
  items.value = [...items.value, { id, message, kind }]
  setTimeout(() => {
    items.value = items.value.filter((t) => t.id !== id)
  }, ms)
  return id
}

export function useToast() {
  return {
    items,
    push,
    ok: (m) => push(m, 'ok'),
    err: (m) => push(m, 'err', 4200),
  }
}
