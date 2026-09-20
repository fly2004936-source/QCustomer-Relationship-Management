<template>
  <!-- C74111 独立节点的操作面板：可直接改分并保存回 project_c74111 -->
  <DraggablePanel
    :title="`C74111 诊断 · ${projectName}`"
    :subtitle="`${total} 分 · ${tier.label} · ${tier.hint}`"
    wide
    :start-x="26"
    :start-y="26"
    @close="emit('close')"
  >
    <template #tools>
      <span v-if="savedAt" class="card-sub">{{ savedAt }}</span>
    </template>

    <div v-if="error" class="banner banner--err" style="margin-bottom: 12px">{{ error }}</div>

    <!-- 总分与分档 -->
    <div class="row" style="gap: 12px; flex-wrap: wrap; align-items: center; margin-bottom: 12px">
      <div class="c74-node-ring" :class="tier.cls">
        <span class="c74-node-score">{{ total }}</span>
        <span class="c74-node-unit">/ 100</span>
      </div>
      <div style="min-width: 140px">
        <div class="row" style="gap: 6px">
          <span class="c74" :class="tier.cls">{{ tier.label }}</span>
          <span class="tag">{{ tier.range }} 分</span>
        </div>
        <div class="card-sub" style="margin-top: 6px">趋赢力 {{ total }}% · {{ tier.hint }}</div>
      </div>
      <div style="margin-left: auto; min-width: 170px">
        <div class="label">可否承诺</div>
        <select v-model="draft.commitment" class="select" style="min-height: 32px">
          <option value="">（不填）</option>
          <option v-for="v in COMMITMENTS" :key="v" :value="v">{{ v }}</option>
        </select>
      </div>
      <div style="min-width: 170px">
        <div class="label">竞争态势</div>
        <select v-model="draft.competition" class="select" style="min-height: 32px">
          <option value="">（不填）</option>
          <option v-for="v in COMPETITIONS" :key="v" :value="v">{{ v }}</option>
        </select>
      </div>
    </div>

    <div class="sect">7 Clears · 合计 40 分</div>
    <div v-for="(row, i) in clears" :key="row.code" class="c74-edit">
      <span class="c74-code">{{ row.code }}</span>
      <span class="c74-edit-name" :title="row.name">{{ row.name }}</span>
      <input v-model.number="draft[`c${i + 1}_score`]" type="number" min="0" :max="row.max" step="1" />
      <span class="c74-edit-max">{{ row.max }}</span>
    </div>

    <div class="sect">4 Priorities · 合计 40 分</div>
    <div v-for="(row, i) in priorities" :key="row.code" class="c74-edit">
      <span class="c74-code">{{ row.code }}</span>
      <span class="c74-edit-name" :title="row.name">{{ row.name }}</span>
      <input v-model.number="draft[`p${i + 1}_score`]" type="number" min="0" :max="row.max" step="1" />
      <button
        class="c74-star"
        :class="{ 'is-on': !!draft[`p${i + 1}_star`] }"
        :title="draft[`p${i + 1}_star`] ? '已打满（☆）' : '标记打满（☆）'"
        @click="draft[`p${i + 1}_star`] = draft[`p${i + 1}_star`] ? 0 : 1"
      >
        ☆
      </button>
    </div>

    <div class="sect">1 Key · 20 分</div>
    <div class="c74-edit">
      <span class="c74-code">Key</span>
      <span class="c74-edit-name">与最高决策人密谋策划</span>
      <input v-model.number="draft.key_score" type="number" min="0" max="20" step="1" />
      <button
        class="c74-star"
        :class="{ 'is-on': !!draft.key_star }"
        title="标记打满（☆）"
        @click="draft.key_star = draft.key_star ? 0 : 1"
      >
        ☆
      </button>
    </div>

    <div class="sect">策略与行动计划</div>
    <div class="field">
      <label class="label">竞争策略</label>
      <textarea v-model="draft.strategy" class="textarea" placeholder="本项目采取 / 拟采取的竞争策略"></textarea>
    </div>
    <div class="field" style="margin-top: 10px">
      <label class="label">行动计划（开门七件事）</label>
      <textarea v-model="draft.action_plan" class="textarea" placeholder="下一步具体动作"></textarea>
    </div>

    <div class="hairline"></div>
    <div class="row" style="gap: 8px; flex-wrap: wrap">
      <button class="btn btn--primary" :disabled="saving" @click="save">
        <AppIcon name="save" :size="14" />
        {{ saving ? '保存中…' : '保存诊断' }}
      </button>
      <button class="btn btn--ghost" @click="restore">还原</button>
      <span class="card-sub" style="margin-left: auto">总分 = 7C + 4P + Key，自动重算</span>
    </div>
  </DraggablePanel>
</template>

<script setup>
import { computed, reactive, ref, watch } from 'vue'
import AppIcon from './AppIcon.vue'
import DraggablePanel from './DraggablePanel.vue'
import { api } from '@/api/client'
import { TIER_META, tierOf } from '@/api/graphData'
import { useToast } from '@/composables/useToast'

const props = defineProps({
  projectId: { type: Number, required: true },
  projectName: { type: String, default: '' },
  /** project_c74111 行；为 null 表示本项目还没有诊断，保存时会自动新建 */
  row: { type: Object, default: null },
})

const emit = defineEmits(['close', 'saved'])

const toast = useToast()

const COMMITMENTS = ['可承诺', '可争取', '可参与', '不可承诺']
const COMPETITIONS = ['绝对优势', '相对优势', '相对劣势', '绝对劣势']

const clears = [
  { code: 'C1', name: '客户组织结构图', max: 10 },
  { code: 'C2', name: '拍板人燃眉之急', max: 5 },
  { code: 'C3', name: '立项六要素', max: 5 },
  { code: 'C4', name: '采购周期', max: 5 },
  { code: 'C5', name: '合作方三问', max: 5 },
  { code: 'C6', name: '竞争方三问', max: 5 },
  { code: 'C7', name: '独特客户价值', max: 5 },
]
const priorities = [
  { code: 'P1', name: '项目组多数人支持', max: 5 },
  { code: 'P2', name: '合作伙伴代理商支持', max: 5 },
  { code: 'P3', name: '与拍板人密谋策划', max: 20 },
  { code: 'P4', name: '关键人密谋策划', max: 10 },
]

const rowId = ref(props.row?.id ?? null)
const saving = ref(false)
const error = ref('')
const savedAt = ref('')

function fromRow(r) {
  const o = { commitment: '', competition: '', strategy: '', action_plan: '' }
  for (let i = 1; i <= 7; i += 1) o[`c${i}_score`] = Number(r?.[`c${i}_score`]) || 0
  for (let i = 1; i <= 4; i += 1) {
    o[`p${i}_score`] = Number(r?.[`p${i}_score`]) || 0
    o[`p${i}_star`] = Number(r?.[`p${i}_star`]) || 0
  }
  o.key_score = Number(r?.key_score) || 0
  o.key_star = Number(r?.key_star) || 0
  o.commitment = r?.commitment ?? ''
  o.competition = r?.competition ?? ''
  o.strategy = r?.strategy ?? ''
  o.action_plan = r?.action_plan ?? ''
  return o
}

const draft = reactive(fromRow(props.row))

watch(
  () => props.row,
  (r) => {
    rowId.value = r?.id ?? null
    Object.assign(draft, fromRow(r))
  },
)

/** 总分实时重算，口径与落库一致：7C(40) + 4P(40) + Key(20) = 100 */
const total = computed(() => {
  let s = 0
  for (let i = 1; i <= 7; i += 1) s += Number(draft[`c${i}_score`]) || 0
  for (let i = 1; i <= 4; i += 1) s += Number(draft[`p${i}_score`]) || 0
  s += Number(draft.key_score) || 0
  return Math.max(0, Math.min(100, Math.round(s)))
})

const tier = computed(() => TIER_META[tierOf(total.value)])

function payload() {
  const p = { project_id: props.projectId }
  for (let i = 1; i <= 7; i += 1) {
    p[`c${i}_score`] = draft[`c${i}_score`] ?? null
    // 备注列原样带回，避免 PATCH 把已有备注清空
    if (props.row) p[`c${i}_note`] = props.row[`c${i}_note`] ?? null
  }
  for (let i = 1; i <= 4; i += 1) {
    p[`p${i}_score`] = draft[`p${i}_score`] ?? null
    if (props.row) p[`p${i}_flags`] = props.row[`p${i}_flags`] ?? null
  }
  p.p3_star = draft.p3_star ? 1 : 0
  p.p4_star = draft.p4_star ? 1 : 0
  p.key_score = draft.key_score ?? null
  p.key_star = draft.key_star ? 1 : 0
  p.total_score = total.value
  p.win_rate = total.value
  // 分档结论与分数保持同步：用户手填的承诺/态势优先，没填就按分数档给默认值
  p.competition = draft.competition || null
  p.commitment = draft.commitment || null
  p.strategy = draft.strategy || null
  p.action_plan = draft.action_plan || null
  return p
}

async function save() {
  saving.value = true
  error.value = ''
  try {
    if (rowId.value) {
      await api.patch('project_c74111', rowId.value, payload())
    } else {
      const r = await api.create('project_c74111', payload())
      rowId.value = r?.id ?? null
    }
    savedAt.value = `已保存 ${new Date().toLocaleTimeString('zh-CN')}`
    toast.ok('C74111 已保存')
    emit('saved')
  } catch (e) {
    error.value = `保存失败：${e.message}`
  } finally {
    saving.value = false
  }
}

function restore() {
  Object.assign(draft, fromRow(props.row))
  error.value = ''
}
</script>
