<template>
  <!-- 竞争对手最新动态（需求 9） -->
  <section class="card wb-col-6">
    <div class="card-head">
      <div class="card-title">
        <AppIcon name="crosshair" :size="15" />
        <span>竞争对手最新动态</span>
      </div>
      <div class="card-actions">
        <span class="card-sub">{{ moves.length }} 条</span>
        <button class="icon-btn" style="width: 28px; height: 28px" title="刷新" @click="emit('refresh')">
          <AppIcon name="refresh" :size="13" />
        </button>
      </div>
    </div>

    <div class="moves" style="max-height: 292px; overflow-y: auto">
      <button
        v-for="m in moves"
        :key="m.id"
        class="move"
        style="cursor: pointer; text-align: left; width: 100%; font: inherit; color: inherit"
        :title="m.review"
        @click="emit('open-entity', m.entity_id)"
      >
        <span class="move-date">{{ String(m.happen_time ?? '').slice(5, 10).replace('-', '/') }}</span>
        <span class="move-body">
          <span class="row" style="gap: 6px; flex-wrap: wrap">
            <span class="tag tag--info">{{ nameOf(m.entity_id) }}</span>
            <span v-if="m.role" class="tag">{{ m.role }}</span>
            <span v-if="m.quote_discount" class="tag tag--warn">{{ m.quote_discount }}</span>
            <span v-if="m.result" class="tag" :class="resultCls(m.result)">{{ m.result }}</span>
          </span>
          <span v-if="m.review" class="move-desc">{{ m.review }}</span>
        </span>
      </button>

      <div v-if="!moves.length" class="empty">暂无竞品动态</div>
    </div>

    <!-- 竞争策略与动作：来自 entsales_confrontation -->
    <template v-if="confrontations.length">
      <div class="sect">竞争策略与动作</div>
      <div class="moves" style="max-height: 200px; overflow-y: auto">
        <div v-for="c in confrontations" :key="`c${c.id}`" class="move">
          <span class="move-date">{{ String(c.happen_time ?? '').slice(5, 10).replace('-', '/') }}</span>
          <span class="move-body">
            <span class="row" style="gap: 6px; flex-wrap: wrap">
              <span v-if="c.their_mode" class="tag tag--bad">{{ c.their_mode }}</span>
              <span v-if="c.their_quote" class="tag">对手报价 {{ c.their_quote }}</span>
              <span v-if="c.result" class="tag" :class="resultCls(c.result)">{{ c.result }}</span>
            </span>
            <span v-if="c.key_reason" class="move-desc">关键理由：{{ c.key_reason }}</span>
            <span v-if="c.review" class="move-desc">{{ c.review }}</span>
          </span>
        </div>
      </div>
    </template>
  </section>
</template>

<script setup>
import { computed } from 'vue'
import AppIcon from '../AppIcon.vue'

const props = defineProps({
  moves: { type: Array, default: () => [] },
  confrontations: { type: Array, default: () => [] },
  entities: { type: Array, default: () => [] },
})

const emit = defineEmits(['refresh', 'open-entity'])

const byId = computed(() => new Map(props.entities.map((e) => [e.id, e.name])))
const nameOf = (id) => byId.value.get(id) ?? (id ? `实体 #${id}` : '未知实体')

function resultCls(r) {
  const s = String(r)
  if (s.includes('领先') || s.includes('成功')) return 'tag--ok'
  if (s.includes('失败') || s.includes('落后')) return 'tag--bad'
  if (s.includes('推进')) return 'tag--info'
  return ''
}
</script>
