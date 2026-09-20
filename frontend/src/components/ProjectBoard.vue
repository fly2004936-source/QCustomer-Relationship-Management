<template>
  <!--
    项目作战板 —— 项目节点的展开内容，对应需求的四条：
    1. 相关人员组织架构（态度 ×-=+※ + ADUR 决策身份配色）
    2. C74111（在网络之外作为独立节点，这里给入口与完整分档）
    3. 奇安信对本项目的派出人员
    4. 竞争方 / 合作方：参与人员、最近动向、竞争策略与动作
  -->
  <div class="board-grid">
    <!-- ① 相关人员 -->
    <section class="card">
      <div class="card-head">
        <div class="card-title">
          <AppIcon name="users" :size="15" />
          <span>相关人员组织架构</span>
        </div>
        <div class="card-actions">
          <span class="tag">{{ data.members.length }} 人</span>
        </div>
      </div>

      <div class="legend-row" style="margin-bottom: 9px">
        <span class="legend-chip">ADUR</span>
        <span v-for="a in ADUR_LIST" :key="a.k" class="legend-chip">
          <span class="adur" :class="`adur--${a.k}`">{{ a.k }}</span>{{ a.label }}
        </span>
      </div>
      <div class="legend-row" style="margin-bottom: 10px">
        <span class="legend-chip">态度</span>
        <span v-for="a in ATTITUDE_LIST" :key="a.v" class="legend-chip">
          <span class="att" :class="a.cls">{{ a.char }}</span>{{ a.label }}
        </span>
      </div>

      <div class="plist">
        <button
          v-for="m in data.members"
          :key="m.id"
          class="prow"
          :title="`点击查看 ${m.name} 的信息`"
          @click="emit('open-person', m)"
        >
          <span class="adur" :class="m.adur ? `adur--${m.adur}` : 'adur--none'">
            {{ m.adur || '·' }}
          </span>
          <span class="pbody">
            <span class="pname">{{ m.name }}</span>
            <span class="psub">{{ [m.title, m.dept].filter(Boolean).join(' · ') || '—' }}</span>
          </span>
          <span
            v-if="m.attitude"
            class="att"
            :class="attCls(m.attitude)"
            :title="`对项目态度 ${m.attitude}`"
          >
            {{ attChar(m.attitude) }}
          </span>
          <span v-else class="att" title="未记录态度">—</span>
          <span v-if="m.mentor" class="tag tag--warn">导师</span>
        </button>
        <div v-if="!data.members.length" class="empty">
          <div>
            本项目还没有关联人员。把人挂到「项目成员」表上，态度和 ADUR 就会出现在这里。
          </div>
          <button
            class="btn btn--sm btn--soft"
            style="margin-top: 9px"
            @click="emit('edit-table', { table: 'project_member', mode: 'create' })"
          >
            <AppIcon name="plus" :size="13" />
            去添加项目成员
          </button>
        </div>
      </div>
    </section>

    <!-- ② C74111 -->
    <section class="card">
      <div class="card-head">
        <div class="card-title">
          <AppIcon name="target" :size="15" />
          <span>C74111 项目诊断</span>
        </div>
        <div class="card-actions">
          <span class="tag">网络外独立节点</span>
        </div>
      </div>

      <button class="c74-node" :class="tier.cls" @click="emit('open-c74111')">
        <span class="c74-node-ring">
          <span class="c74-node-score">{{ tier.known ? tier.score : '—' }}</span>
          <span class="c74-node-unit">{{ tier.known ? '/ 100' : '未评分' }}</span>
        </span>
        <span class="pbody">
          <span class="row" style="gap: 6px; flex-wrap: wrap">
            <span class="c74" :class="tier.cls">{{ tier.label }}</span>
            <span v-if="tier.known" class="tag">{{ tier.range }} 分</span>
          </span>
          <span class="psub" style="margin-top: 4px">
            <template v-if="tier.known">
              趋赢力 {{ tier.score }}% · {{ tier.commitment }} · 点开可改分与写行动计划
            </template>
            <template v-else>
              这个项目还没做过 C74111 诊断，点开即可逐项评分
            </template>
          </span>
        </span>
        <AppIcon name="chevronRight" :size="15" />
      </button>

      <div v-if="tier.known" class="c74-bar" :class="tier.cls" style="margin-top: 12px">
        <i :style="{ width: Math.min(100, tier.score) + '%' }"></i>
      </div>
      <div class="legend-row" style="margin-top: 10px">
        <span v-for="t in TIERS" :key="t.key" class="legend-chip">
          <span class="c74" :class="t.cls">{{ t.range }}</span>{{ t.label }}
        </span>
      </div>

      <div class="hairline"></div>
      <div class="row" style="gap: 8px; flex-wrap: wrap">
        <button class="btn btn--primary btn--sm" @click="emit('open-c74111')">
          <AppIcon name="edit" :size="13" />
          {{ tier.known ? '操作 C74111 模块' : '开始 C74111 诊断' }}
        </button>
        <button v-if="data.c74111Id" class="btn btn--ghost btn--sm" @click="emit('edit-table', { table: 'project_c74111', id: data.c74111Id })">
          <AppIcon name="table" :size="13" />
          在数据表里编辑
        </button>
      </div>

      <template v-if="data.strategy">
        <div class="sect">竞争策略</div>
        <p class="sf-p" style="max-width: none; font-size: 12.5px">{{ data.strategy }}</p>
      </template>
    </section>

    <!-- ③ 我方派出人员 -->
    <section class="card">
      <div class="card-head">
        <div class="card-title">
          <AppIcon name="shield" :size="15" />
          <span>奇安信派出人员</span>
        </div>
        <div class="card-actions">
          <span class="tag tag--ac">{{ data.staff.length }} 人</span>
        </div>
      </div>

      <div class="plist">
        <button
          v-for="s in data.staff"
          :key="s.id"
          class="prow"
          :title="`点击查看 ${s.name} 的信息`"
          @click="emit('open-staff', s)"
        >
          <span class="pavatar pavatar--src">{{ (s.name || '?')[0] }}</span>
          <span class="pbody">
            <span class="pname">{{ s.name }}</span>
            <span class="psub">{{ [s.title, s.dept, s.orgName].filter(Boolean).join(' · ') || '—' }}</span>
          </span>
          <span v-if="s.decisionRole" class="tag tag--info">{{ s.decisionRole }}</span>
          <span v-if="s.attitude" class="att" :class="attCls(s.attitude)">{{ attChar(s.attitude) }}</span>
        </button>
        <div v-if="!data.staff.length" class="empty">
          <div>
            还没有本项目的人员派出记录。链路是 拜访记录.project_id → 拜访我方人员 → 我方人员，
            先在「项目 → 拜访行动」里登记一次拜访即可出现。
          </div>
          <button
            class="btn btn--sm btn--soft"
            style="margin-top: 9px"
            @click="emit('edit-table', { table: 'visit_record', mode: 'create' })"
          >
            <AppIcon name="plus" :size="13" />
            去登记一次拜访
          </button>
        </div>
      </div>
    </section>

    <!-- ④ 合作方 / 竞争方 -->
    <section v-for="side in SIDES" :key="side.type" class="card board-wide">
      <div class="card-head">
        <div class="card-title">
          <AppIcon :name="side.type === 'partner' ? 'link' : 'crosshair'" :size="15" />
          <span>{{ side.title }}</span>
        </div>
        <div class="card-actions">
          <span class="tag">{{ side.list.length }} 家</span>
          <span v-if="side.fallback" class="tag tag--warn">本项目暂无关联，展示全部</span>
        </div>
      </div>

      <div v-if="!side.list.length" class="empty">暂无{{ side.title }}数据</div>

      <div v-for="ent in side.list" :key="ent.nodeId" class="ent-card">
        <div class="ent-top">
          <AppIcon :name="side.type === 'partner' ? 'link' : 'crosshair'" :size="14" />
          <span class="ent-name">{{ ent.entity.name }}</span>
          <span v-if="ent.entity.company_type" class="tag">{{ ent.entity.company_type }}</span>
          <span v-if="ent.entity.overall_relation" class="tag tag--info">{{ ent.entity.overall_relation }}</span>
          <span v-if="!ent.participates" class="tag tag--warn">未与本项目建立关联</span>
          <button
            class="btn btn--sm btn--ghost"
            style="margin-left: auto"
            @click="emit('edit-table', { table: 'coop_entity', id: ent.entity.id })"
          >
            <AppIcon name="edit" :size="12" />
            编辑实体
          </button>
        </div>

        <dl class="ent-kv">
          <template v-if="ent.detail['竞争态势']">
            <dt>竞争态势</dt>
            <dd>{{ ent.detail['竞争态势'] }}</dd>
          </template>
          <template v-if="ent.detail['我方优势']">
            <dt>我方优势</dt>
            <dd>{{ ent.detail['我方优势'] }}</dd>
          </template>
          <template v-if="ent.detail['对方优势']">
            <dt>对方优势</dt>
            <dd>{{ ent.detail['对方优势'] }}</dd>
          </template>
          <template v-if="ent.detail['业务战术']">
            <dt>对方战术</dt>
            <dd>{{ ent.detail['业务战术'] }}</dd>
          </template>
          <template v-if="ent.projectRel?.our_strategy || ent.projectRel?.their_tactic">
            <dt>本项目策略</dt>
            <dd>
              <template v-if="ent.projectRel.their_tactic">对方：{{ ent.projectRel.their_tactic }}　</template>
              <template v-if="ent.projectRel.our_strategy">我方：{{ ent.projectRel.our_strategy }}</template>
            </dd>
          </template>
        </dl>

        <div class="sect">参与人员（{{ ent.persons.length }}）</div>
        <div class="plist">
          <button
            v-for="p in ent.persons"
            :key="p.nodeId"
            class="prow"
            :title="`点击查看 ${p.name} 的信息`"
            @click="emit('open-coop-person', { person: p, type: side.type, entity: ent.entity })"
          >
            <span class="pavatar pavatar--src">{{ (p.name || '?')[0] }}</span>
            <span class="pbody">
              <span class="pname">{{ p.name }}</span>
              <span class="psub">{{ [p.title, p.role, p.decisionPower].filter(Boolean).join(' · ') || '—' }}</span>
            </span>
            <span v-if="p.participates" class="tag tag--ac">参与本项目</span>
            <span v-else class="tag">未参与</span>
            <span v-if="p.attitude" class="att" :class="attCls(p.attitude)">{{ attChar(p.attitude) }}</span>
          </button>
          <div v-if="!ent.persons.length" class="empty">该{{ side.type === 'partner' ? '合作方' : '竞争方' }}还没有登记人员</div>
        </div>

        <template v-if="ent.moves?.length">
          <div class="sect">最近动向</div>
          <div class="moves">
            <div v-for="m in ent.moves" :key="`m${m.id}`" class="move">
              <span class="move-date">{{ String(m.happen_time ?? '').slice(5, 10).replace('-', '/') }}</span>
              <span class="move-body">
                <span class="row" style="gap: 6px; flex-wrap: wrap">
                  <span v-if="m.role" class="tag">{{ m.role }}</span>
                  <span v-if="m.quote_discount" class="tag tag--warn">{{ m.quote_discount }}</span>
                  <span v-if="m.result" class="tag">{{ m.result }}</span>
                </span>
                <span v-if="m.review" class="move-desc">{{ m.review }}</span>
              </span>
            </div>
          </div>
        </template>

        <template v-if="ent.confrontations?.length">
          <div class="sect">竞争策略与动作</div>
          <div class="moves">
            <div v-for="c in ent.confrontations" :key="`c${c.id}`" class="move">
              <span class="move-date">{{ String(c.happen_time ?? '').slice(5, 10).replace('-', '/') }}</span>
              <span class="move-body">
                <span class="row" style="gap: 6px; flex-wrap: wrap">
                  <span v-if="c.their_mode" class="tag tag--bad">{{ c.their_mode }}</span>
                  <span v-if="c.their_quote" class="tag">对方报价 {{ c.their_quote }}</span>
                  <span v-if="c.our_quote" class="tag">我方报价 {{ c.our_quote }}</span>
                  <span v-if="c.result" class="tag">{{ c.result }}</span>
                </span>
                <span v-if="c.key_reason" class="move-desc">关键理由：{{ c.key_reason }}</span>
                <span v-if="c.review" class="move-desc">{{ c.review }}</span>
              </span>
            </div>
          </div>
        </template>

        <div
          v-if="!ent.moves?.length && !ent.confrontations?.length"
          class="card-sub"
          style="margin-top: 8px"
        >
          暂无动向与对抗记录（数据来自 entity_history / entsales_confrontation）
        </div>
      </div>
    </section>
  </div>
</template>

<script setup>
import { computed } from 'vue'
import AppIcon from './AppIcon.vue'
import { ADUR_LIST, ATTITUDE_LIST, TIER_META, attChar, attCls } from '@/api/graphData'

const props = defineProps({
  data: { type: Object, required: true },
})

const emit = defineEmits(['open-person', 'open-staff', 'open-coop-person', 'open-c74111', 'edit-table'])

const TIERS = [TIER_META.a, TIER_META.b, TIER_META.c, TIER_META.d]

const SIDES = computed(() => [
  {
    type: 'partner',
    title: '合作方参与情况',
    list: props.data.partners ?? [],
    fallback: props.data.fallback?.partners,
  },
  {
    type: 'competitor',
    title: '竞争方参与情况',
    list: props.data.competitors ?? [],
    fallback: props.data.fallback?.competitors,
  },
])

const tier = computed(() => props.data.tier ?? TIER_META.none)
</script>
