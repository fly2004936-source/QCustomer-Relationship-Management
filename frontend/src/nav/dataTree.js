/**
 * 「数据管理」导航树
 *
 * 按用户要求：所有表按 **客户 / 项目 / 合作方 / 竞争方** 分类，每个类下面都有「人员」。
 * 62 张业务表在这里**每张恰好出现一次**（可在导航里核对，不会漏表也不会重复）。
 *
 * 另外补了两个必要的分组（属于"实用优先"的改动，已在交付说明里标注原因）：
 *   · 我方组织 —— org / internal_person 属奇安信自己，既不是合作方也不是竞争方，
 *     硬塞进四个分类里都会造成语义错误；
 *   · 人员画像 —— person_form / person_disc 等 6 张多态表用 person_type 区分对象，
 *     同时服务于客户方、合作方、竞争方、我方，放到任一分类下都会误导。
 */

/** 生成一条表项：name = 表名（= 接口路径），label = 中文名 */
const t = (name, label) => ({ name, label })

export const NAV_TREE = [
  {
    key: 'customer',
    label: '客户',
    icon: 'building',
    groups: [
      {
        label: '客户档案',
        items: [t('customer', '客户主档'), t('customer_profile', '客户画像'), t('customer_relation', '客户关系网')],
      },
      {
        label: '客户经营',
        items: [
          t('customer_budget', '客户预算'),
          t('customer_budget_detail', '预算明细'),
          t('customer_sales_data', '销售数据'),
          t('customer_perf_dashboard', '业绩看板'),
          t('customer_target', '客户指标'),
        ],
      },
      {
        label: '商机洞察',
        items: [
          t('customer_need', '客户需求'),
          t('customer_opportunity', '商机'),
          t('customer_purchase', '采购情况'),
          t('customer_tender', '招标信息'),
          t('customer_foreign_product', '友商产品'),
          t('customer_competition', '竞争态势'),
        ],
      },
      {
        label: '客户活动',
        items: [
          t('customer_exec_meeting', '高层会议'),
          t('customer_team', '客户团队'),
          t('customer_action_plan', '客户行动计划'),
          t('customer_news', '客户动态'),
        ],
      },
      { label: '人员', items: [t('person', '客户方人员')] },
    ],
  },
  {
    key: 'project',
    label: '项目',
    icon: 'briefcase',
    groups: [
      {
        label: '项目',
        items: [t('project', '项目主档'), t('project_c74111', 'C74111 诊断'), t('project_action_plan', '开门七件事')],
      },
      { label: '人员', items: [t('project_member', '项目成员')] },
      {
        label: '拜访行动',
        items: [
          t('visit_record', '拜访记录'),
          t('visit_our_staff', '拜访我方人员'),
          t('visit_object', '拜访对象'),
          t('visit_action_item', '行动项'),
        ],
      },
    ],
  },
  {
    key: 'partner',
    label: '合作方',
    icon: 'link',
    groups: [
      {
        label: '合作方',
        items: [t('coop_entity', '合作/竞争方实体'), t('coop_entity_project_rel', '实体 ↔ 项目')],
      },
      { label: '人员', items: [t('partner_person', '合作方人员'), t('partner_person_project_rel', '人员 ↔ 项目')] },
    ],
  },
  {
    key: 'competitor',
    label: '竞争方',
    icon: 'crosshair',
    groups: [
      { label: '竞争方', items: [t('entity_history', '实体动向'), t('entsales_confrontation', '竞争对抗')] },
      {
        label: '销售分析（按实体，含合作方）',
        items: [
          t('entsales_perf', '经营绩效'),
          t('entsales_product', '产品矩阵'),
          t('entsales_market', '市场占有'),
          t('entsales_distribution', '渠道分布'),
          t('entsales_channel', '渠道'),
          t('entsales_pricing', '定价'),
          t('entsales_customer_share', '客户份额'),
          t('entsales_tender', '投标记录'),
          t('entsales_tactic', '策略战术'),
          t('entsales_benchmark', '对标'),
          t('entsales_plan', '计划'),
          t('entsales_source', '信息来源'),
        ],
      },
      { label: '人员', items: [t('competitor_person', '竞争方人员'), t('competitor_person_project_rel', '人员 ↔ 项目')] },
    ],
  },
  {
    key: 'ours',
    label: '我方组织',
    icon: 'shield',
    groups: [
      { label: '组织', items: [t('org', '奇安信组织树')] },
      { label: '人员', items: [t('internal_person', '我方人员')] },
    ],
  },
  {
    key: 'persona',
    label: '人员画像',
    icon: 'users',
    groups: [
      {
        label: '多态表（person_type + person_id）',
        items: [
          t('person_form', 'FORM 测评'),
          t('person_disc', 'DISC 测评'),
          t('person_burning_issue', '燃眉之急'),
          t('person_attitude_log', '态度日志'),
          t('person_assessment', '人员评估'),
          t('person_intel', '人员情报'),
        ],
      },
    ],
  },
  {
    key: 'workbench',
    label: '工作台数据',
    icon: 'grid',
    groups: [
      { label: '计划待办', items: [t('daily_plan', '每日计划'), t('daily_todo', '今日待办')] },
      { label: '便签快捷', items: [t('dashboard_note', '灵感便签'), t('dashboard_shortcut', '快捷入口')] },
      { label: '专注计时', items: [t('focus_session', '番茄钟记录')] },
      { label: '资讯', items: [t('news_item', '行业资讯'), t('dashboard_update_log', '更新日志')] },
    ],
  },
]

/** 表名 → 中文名，其它地方（面包屑等）复用 */
export const TABLE_LABEL = Object.fromEntries(
  NAV_TREE.flatMap((c) => c.groups.flatMap((g) => g.items.map((i) => [i.name, i.label]))),
)

/** 表名 → 所属大类，用于面包屑与高亮 */
export const TABLE_CATEGORY = Object.fromEntries(
  NAV_TREE.flatMap((c) => c.groups.flatMap((g) => g.items.map((i) => [i.name, c.label]))),
)

/** 全部表名（去重校验用） */
export const ALL_TABLES = NAV_TREE.flatMap((c) => c.groups.flatMap((g) => g.items.map((i) => i.name)))

export const tableLabel = (name) => TABLE_LABEL[name] ?? name
export const tableCategory = (name) => TABLE_CATEGORY[name] ?? ''
