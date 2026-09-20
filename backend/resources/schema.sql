-- =====================================================================
-- 客户管理系统 SQLite 建库脚本（DDL）
-- 依据：客户管理系统数据库说明表（SQLite）
-- 技术栈：SQLite 3（建议开启 JSON1）
-- 说明：本文件仅供审阅 / 执行；日期用 TEXT(ISO8601)，金额用 REAL(万元)，
--       布尔用 INTEGER(0/1)，多值用 TEXT 存 JSON。
-- 关系要点：
--   人员 -> 公司/客户 多对一；内部人员 -> 奇安信组织（不同关联）；
--   项目 -> 公司/客户 多对一；合作/竞争方 <-> 项目 多对多。
-- =====================================================================

-- 建表阶段先关闭外键强制，避免 DROP/建表顺序问题；建表完成后再开启（见文末）。
PRAGMA foreign_keys = OFF;

-- ============================ A. 客户域 ============================

-- A1 客户/公司主表
DROP TABLE IF EXISTS customer;
CREATE TABLE customer (
  id                 INTEGER PRIMARY KEY AUTOINCREMENT,
  crm_code           TEXT NOT NULL,
  name               TEXT NOT NULL,
  category           TEXT CHECK (category IN ('主客户','关联客户')),
  level              TEXT,
  army_region        TEXT,
  industry           TEXT,
  account_manager    TEXT,
  created_date       TEXT,
  updated_date       TEXT,
  data_source        TEXT,
  func               TEXT,
  core_business      TEXT,
  serve_object       TEXT,
  scale              TEXT,
  staff_scale        TEXT,
  output_3y          TEXT,
  dev_plan           TEXT,
  policy_impact      TEXT,
  biz_trend          TEXT,
  demand_factors     TEXT,
  influence          TEXT,
  superior_unit      TEXT,
  co_unit            TEXT,
  support_unit       TEXT,
  key_scenes         TEXT,          -- JSON 数组
  app_systems        TEXT,
  network_env        TEXT,
  security_status    TEXT,
  tags               TEXT,
  remark             TEXT,
  UNIQUE (crm_code)
);

-- A2 关联客户关系
DROP TABLE IF EXISTS customer_relation;
CREATE TABLE customer_relation (
  id                   INTEGER PRIMARY KEY AUTOINCREMENT,
  customer_id          INTEGER NOT NULL,
  related_customer_id  INTEGER,
  related_name         TEXT,
  related_crm_code     TEXT,
  relation_type        TEXT CHECK (relation_type IN ('上级','下属','平级','合作')),
  main_purchase        TEXT,
  remark               TEXT,
  FOREIGN KEY (customer_id) REFERENCES customer(id) ON DELETE CASCADE,
  FOREIGN KEY (related_customer_id) REFERENCES customer(id) ON DELETE SET NULL
);

-- A3 企业画像（1:1）
DROP TABLE IF EXISTS customer_profile;
CREATE TABLE customer_profile (
  id                    INTEGER PRIMARY KEY AUTOINCREMENT,
  customer_id           INTEGER NOT NULL UNIQUE,
  strength_scale        TEXT,
  equity_structure      TEXT,
  lifecycle             TEXT,
  niche                 TEXT,
  strategy_pain         TEXT,
  tech_decision         TEXT,
  purchase_feature      TEXT,
  supplier_structure    TEXT,
  performance_relation  TEXT,
  coop_barrier          TEXT,
  swot                  TEXT,
  remark                TEXT,
  FOREIGN KEY (customer_id) REFERENCES customer(id) ON DELETE CASCADE
);

-- A4 与客户高层互动
DROP TABLE IF EXISTS customer_exec_meeting;
CREATE TABLE customer_exec_meeting (
  id                 INTEGER PRIMARY KEY AUTOINCREMENT,
  customer_id        INTEGER NOT NULL,
  meet_time          TEXT,
  place              TEXT,
  client_role        TEXT,
  our_staff          TEXT,
  topic              TEXT,
  bu_leader_involved INTEGER CHECK (bu_leader_involved IN (0,1)),
  todo               TEXT,
  remark             TEXT,
  FOREIGN KEY (customer_id) REFERENCES customer(id) ON DELETE CASCADE
);

-- A5 客户预算（年度）
DROP TABLE IF EXISTS customer_budget;
CREATE TABLE customer_budget (
  id                    INTEGER PRIMARY KEY AUTOINCREMENT,
  customer_id           INTEGER NOT NULL,
  year                  INTEGER,
  it_budget             REAL,
  sec_budget            REAL,
  sec_budget_exec       REAL,
  key_areas             TEXT,
  our_occupancy         TEXT,
  top5_competitor_share TEXT,
  phenomenon_ratio      REAL,
  continuous_ratio      REAL,
  maintenance_ratio     REAL,
  remark                TEXT,
  FOREIGN KEY (customer_id) REFERENCES customer(id) ON DELETE CASCADE
);

-- A6 年度预算构成明细
DROP TABLE IF EXISTS customer_budget_detail;
CREATE TABLE customer_budget_detail (
  id            INTEGER PRIMARY KEY AUTOINCREMENT,
  customer_id   INTEGER NOT NULL,
  year          INTEGER,
  name          TEXT,
  plan_content  TEXT,
  executor      TEXT,
  dept          TEXT,
  plan_time     TEXT,
  amount        REAL,
  remark        TEXT,
  FOREIGN KEY (customer_id) REFERENCES customer(id) ON DELETE CASCADE
);

-- A7 营销数据 / 我司销售分析（年度）
DROP TABLE IF EXISTS customer_sales_data;
CREATE TABLE customer_sales_data (
  id                  INTEGER PRIMARY KEY AUTOINCREMENT,
  customer_id         INTEGER NOT NULL,
  year                INTEGER,
  product_amount      REAL,
  product_ratio       REAL,
  service_amount      REAL,
  service_ratio       REAL,
  total_amount        REAL,
  sec_budget          REAL,
  est_gross           REAL,
  opportunity_count   INTEGER,
  win_count           INTEGER,
  win_rate            REAL,
  contract_amount     REAL,
  received_amount     REAL,
  receivable_balance  REAL,
  overdue_amount      REAL,
  remark              TEXT,
  FOREIGN KEY (customer_id) REFERENCES customer(id) ON DELETE CASCADE
);

-- A8 标讯信息（年度）
DROP TABLE IF EXISTS customer_tender;
CREATE TABLE customer_tender (
  id                INTEGER PRIMARY KEY AUTOINCREMENT,
  customer_id       INTEGER NOT NULL,
  year              INTEGER,
  relate_type       TEXT,
  tender_amount     REAL,
  sec_amount        REAL,
  proj_count_a      INTEGER,
  should_join_b     INTEGER,
  should_join_rate  REAL,
  bid_c             INTEGER,
  bid_direct_c1     INTEGER,
  bid_auth_c2       INTEGER,
  win_d             INTEGER,
  win_direct_d1     INTEGER,
  win_auth_d2       INTEGER,
  actual_bid_rate   REAL,
  win_rate          REAL,
  direct_win_rate   REAL,
  auth_win_rate     REAL,
  remark            TEXT,
  FOREIGN KEY (customer_id) REFERENCES customer(id) ON DELETE CASCADE
);

-- A9 过往安全产品/服务（我司+友商）
DROP TABLE IF EXISTS customer_purchase;
CREATE TABLE customer_purchase (
  id             INTEGER PRIMARY KEY AUTOINCREMENT,
  customer_id    INTEGER NOT NULL,
  product_type   TEXT,
  brand          TEXT,
  is_ours        INTEGER CHECK (is_ours IN (0,1)),
  purchase_time  TEXT,
  amount         REAL,
  purchase_type  TEXT CHECK (purchase_type IN ('框架','续采','创新','定制','标品')),
  dept           TEXT,
  satisfaction   TEXT,
  remark         TEXT,
  FOREIGN KEY (customer_id) REFERENCES customer(id) ON DELETE CASCADE
);

-- A10 国外产品使用（国产替代线索）
DROP TABLE IF EXISTS customer_foreign_product;
CREATE TABLE customer_foreign_product (
  id                  INTEGER PRIMARY KEY AUTOINCREMENT,
  customer_id         INTEGER NOT NULL,
  army_region         TEXT,
  branch              TEXT,
  account_manager     TEXT,
  product_name        TEXT,
  qty                 TEXT,
  vendor              TEXT,
  client_replace_plan TEXT,
  our_replace_plan    TEXT,
  remark              TEXT,
  FOREIGN KEY (customer_id) REFERENCES customer(id) ON DELETE CASCADE
);

-- A11 客户需求
DROP TABLE IF EXISTS customer_need;
CREATE TABLE customer_need (
  id           INTEGER PRIMARY KEY AUTOINCREMENT,
  customer_id  INTEGER NOT NULL,
  need_desc    TEXT,
  dept_scope   TEXT,   -- JSON 数组
  remark       TEXT,
  FOREIGN KEY (customer_id) REFERENCES customer(id) ON DELETE CASCADE
);

-- A12 线索/商机
DROP TABLE IF EXISTS customer_opportunity;
CREATE TABLE customer_opportunity (
  id              INTEGER PRIMARY KEY AUTOINCREMENT,
  customer_id     INTEGER NOT NULL,
  info_type       TEXT CHECK (info_type IN ('线索','商机')),
  name            TEXT,
  opp_type        TEXT CHECK (opp_type IN ('框架','续采','创新','定制','标品')),
  client_budget   REAL,
  exp_order_time  TEXT,
  prediction      TEXT CHECK (prediction IN ('可争取','可参与','可承诺')),
  est_contract    REAL,
  est_gross       REAL,
  project_mode    TEXT,
  scope           TEXT,
  products        TEXT,
  services        TEXT,
  tech_support    TEXT,
  remark          TEXT,
  FOREIGN KEY (customer_id) REFERENCES customer(id) ON DELETE CASCADE
);

-- A13 竞争情况（整体 + 分类/产品排名）
DROP TABLE IF EXISTS customer_competition;
CREATE TABLE customer_competition (
  id              INTEGER PRIMARY KEY AUTOINCREMENT,
  customer_id     INTEGER NOT NULL,
  competitor_name TEXT,
  value_prop      TEXT,
  advantage       TEXT,
  weakness        TEXT,
  rank            INTEGER,
  product_dim     TEXT,   -- JSON：产品 -> 排名
  remark          TEXT,
  FOREIGN KEY (customer_id) REFERENCES customer(id) ON DELETE CASCADE
);

-- A14 任务目标
DROP TABLE IF EXISTS customer_target;
CREATE TABLE customer_target (
  id             INTEGER PRIMARY KEY AUTOINCREMENT,
  customer_id    INTEGER NOT NULL,
  year           INTEGER,
  target_type    TEXT,
  target_amount  REAL,
  total_amount   REAL,
  remark         TEXT,
  FOREIGN KEY (customer_id) REFERENCES customer(id) ON DELETE CASCADE
);

-- A15 业绩仪表盘
DROP TABLE IF EXISTS customer_perf_dashboard;
CREATE TABLE customer_perf_dashboard (
  id            INTEGER PRIMARY KEY AUTOINCREMENT,
  customer_id   INTEGER NOT NULL,
  year          INTEGER,
  metric        TEXT CHECK (metric IN ('订单额','订单毛利','毛利率')),
  q1_budget     REAL, q1_forecast REAL,
  q2_budget     REAL, q2_forecast REAL,
  q3_budget     REAL, q3_forecast REAL,
  q4_budget     REAL, q4_forecast REAL,
  full_budget   REAL, full_forecast REAL,
  FOREIGN KEY (customer_id) REFERENCES customer(id) ON DELETE CASCADE
);

-- A16 专属团队
DROP TABLE IF EXISTS customer_team;
CREATE TABLE customer_team (
  id           INTEGER PRIMARY KEY AUTOINCREMENT,
  customer_id  INTEGER NOT NULL,
  scope        TEXT CHECK (scope IN ('客户组织','项目组织')),
  role         TEXT,
  person_name  TEXT,
  dept_source  TEXT,
  requirement  TEXT,
  project_id   INTEGER,
  sr           TEXT,
  fr           TEXT,
  cdr          TEXT,
  remark       TEXT,
  FOREIGN KEY (customer_id) REFERENCES customer(id) ON DELETE CASCADE,
  FOREIGN KEY (project_id)  REFERENCES project(id) ON DELETE SET NULL
);

-- A17 行动计划（五+一工程）
DROP TABLE IF EXISTS customer_action_plan;
CREATE TABLE customer_action_plan (
  id               INTEGER PRIMARY KEY AUTOINCREMENT,
  customer_id      INTEGER NOT NULL,
  engine_type      TEXT CHECK (engine_type IN ('关系提升','安全改造','质量提升','服务改善','共创创新','其他')),
  item             TEXT,
  current_problem  TEXT,
  driver           TEXT,
  strategy         TEXT,
  smart_plan       TEXT,
  resource         TEXT,
  plan_time        TEXT,
  plan_goal        TEXT,
  owner            TEXT,
  remark           TEXT,
  FOREIGN KEY (customer_id) REFERENCES customer(id) ON DELETE CASCADE
);

-- ============================ B. 人员域 ============================

-- B1 客户人员（多对一挂客户）
DROP TABLE IF EXISTS person;
CREATE TABLE person (
  id             INTEGER PRIMARY KEY AUTOINCREMENT,
  customer_id    INTEGER NOT NULL,
  name           TEXT NOT NULL,
  gender         TEXT,
  age            INTEGER,
  birth          TEXT,
  zodiac         TEXT,
  native_place   TEXT,
  residence      TEXT,
  education      TEXT,
  marital        TEXT,
  family         TEXT,
  political      TEXT,
  health         TEXT,
  title          TEXT,
  dept           TEXT,
  rank           TEXT,
  org_path       TEXT,
  parent_id      INTEGER,
  decision_mode  TEXT CHECK (decision_mode IN ('诸侯制','集权制')),
  contact        TEXT,
  career_bg      TEXT CHECK (career_bg IN ('技术','销售','财务')),
  role_scope     TEXT,
  source         TEXT,
  credibility    TEXT CHECK (credibility IN ('高','中','低')),
  info_date      TEXT,
  tags           TEXT,
  remark         TEXT,
  FOREIGN KEY (customer_id) REFERENCES customer(id) ON DELETE CASCADE,
  FOREIGN KEY (parent_id)   REFERENCES person(id) ON DELETE SET NULL
);

-- B2 合作方人员（多对一挂合作方实体 coop_entity）
DROP TABLE IF EXISTS partner_person;
CREATE TABLE partner_person (
  id             INTEGER PRIMARY KEY AUTOINCREMENT,
  coop_entity_id INTEGER NOT NULL,
  name           TEXT NOT NULL,
  gender         TEXT,
  age            INTEGER,
  birth          TEXT,
  native_place   TEXT,
  residence      TEXT,
  education      TEXT,
  marital        TEXT,
  family         TEXT,
  political      TEXT,
  contact        TEXT,
  title          TEXT,
  dept           TEXT,
  rank           TEXT,
  entry_time     TEXT,
  role_in_company TEXT,
  decision_power TEXT CHECK (decision_power IN ('拍板','影响','执行')),
  responsibility TEXT,
  resources      TEXT,
  parent_id      INTEGER,
  our_liaison    TEXT,
  common_customer TEXT,
  attitude       TEXT CHECK (attitude IN ('X','-','=','+','⭐')),
  value_to_us    TEXT,
  intel_ability  TEXT CHECK (intel_ability IN ('强','中','弱')),
  private_depth  TEXT,
  source         TEXT,
  credibility    TEXT CHECK (credibility IN ('高','中','低')),
  info_date      TEXT,
  remark         TEXT,
  FOREIGN KEY (coop_entity_id) REFERENCES coop_entity(id) ON DELETE CASCADE,
  FOREIGN KEY (parent_id)  REFERENCES partner_person(id) ON DELETE SET NULL
);

-- B3 竞争方人员（多对一挂竞争方实体 coop_entity）
DROP TABLE IF EXISTS competitor_person;
CREATE TABLE competitor_person (
  id             INTEGER PRIMARY KEY AUTOINCREMENT,
  coop_entity_id INTEGER NOT NULL,
  name           TEXT NOT NULL,
  gender         TEXT,
  age            INTEGER,
  birth          TEXT,
  native_place   TEXT,
  residence      TEXT,
  education      TEXT,
  marital        TEXT,
  family         TEXT,
  political      TEXT,
  contact        TEXT,
  title          TEXT,
  dept           TEXT,
  rank           TEXT,
  entry_time     TEXT,
  role_in_company TEXT,
  decision_power TEXT CHECK (decision_power IN ('拍板','影响','执行')),
  responsibility TEXT,
  resources      TEXT,
  parent_id      INTEGER,
  our_liaison    TEXT,
  common_customer TEXT,
  attitude       TEXT CHECK (attitude IN ('X','-','=','+','⭐')),
  value_to_us    TEXT,
  intel_ability  TEXT CHECK (intel_ability IN ('强','中','弱')),
  private_depth  TEXT,
  source         TEXT,
  credibility    TEXT CHECK (credibility IN ('高','中','低')),
  info_date      TEXT,
  remark         TEXT,
  FOREIGN KEY (coop_entity_id) REFERENCES coop_entity(id) ON DELETE CASCADE,
  FOREIGN KEY (parent_id)     REFERENCES competitor_person(id) ON DELETE SET NULL
);

-- B4 奇安信组织（自关联树）
DROP TABLE IF EXISTS org;
CREATE TABLE org (
  id        INTEGER PRIMARY KEY AUTOINCREMENT,
  name      TEXT NOT NULL,
  parent_id INTEGER,
  org_type  TEXT,
  level     INTEGER,
  remark    TEXT,
  FOREIGN KEY (parent_id) REFERENCES org(id) ON DELETE SET NULL
);

-- B5 奇安信内部人员（不同关联：挂 org，不挂客户）
DROP TABLE IF EXISTS internal_person;
CREATE TABLE internal_person (
  id               INTEGER PRIMARY KEY AUTOINCREMENT,
  org_id           INTEGER NOT NULL,
  name             TEXT NOT NULL,
  gender           TEXT,
  age              INTEGER,
  birth            TEXT,
  zodiac           TEXT,
  native_place     TEXT,
  residence        TEXT,
  marital          TEXT,
  family           TEXT,
  political        TEXT,
  health           TEXT,
  title            TEXT,
  dept             TEXT,
  rank             TEXT,
  entry_time       TEXT,
  career_bg        TEXT,
  management_scope TEXT,
  parent_id        INTEGER,
  decision_role    TEXT CHECK (decision_role IN ('拍板','协调','支撑','执行')),
  support_ability  TEXT,
  attitude         TEXT CHECK (attitude IN ('X','-','=','+','⭐')),
  contact          TEXT,
  source           TEXT,
  credibility      TEXT CHECK (credibility IN ('高','中','低')),
  info_date        TEXT,
  tags             TEXT,
  remark           TEXT,
  FOREIGN KEY (org_id)    REFERENCES org(id) ON DELETE CASCADE,
  FOREIGN KEY (parent_id) REFERENCES internal_person(id) ON DELETE SET NULL
);

-- B6 通用 FORM 画像（多态 1:1）
DROP TABLE IF EXISTS person_form;
CREATE TABLE person_form (
  id             INTEGER PRIMARY KEY AUTOINCREMENT,
  person_type    TEXT NOT NULL CHECK (person_type IN ('customer','partner','competitor','internal')),
  person_id      INTEGER NOT NULL,
  family         TEXT,
  occupation     TEXT,
  recreation     TEXT,
  motivation     TEXT,
  career_bg      TEXT,
  kpi_pressure   TEXT,
  personality    TEXT,
  concern        TEXT,
  supplementary  TEXT,
  remark         TEXT,
  UNIQUE (person_type, person_id)
);

-- B7 通用 DISC 评分（多态 1:1）
DROP TABLE IF EXISTS person_disc;
CREATE TABLE person_disc (
  id                INTEGER PRIMARY KEY AUTOINCREMENT,
  person_type       TEXT NOT NULL CHECK (person_type IN ('customer','partner','competitor','internal')),
  person_id         INTEGER NOT NULL,
  score_d           INTEGER,
  score_i           INTEGER,
  score_s           INTEGER,
  score_c           INTEGER,
  dominant          TEXT,
  behavior_evidence TEXT,
  note              TEXT,
  UNIQUE (person_type, person_id)
);

-- B8 燃眉之急 BI（多态 1:N）
DROP TABLE IF EXISTS person_burning_issue;
CREATE TABLE person_burning_issue (
  id            INTEGER PRIMARY KEY AUTOINCREMENT,
  person_type   TEXT NOT NULL CHECK (person_type IN ('customer','partner','competitor','internal')),
  person_id     INTEGER NOT NULL,
  project_id    INTEGER,
  content       TEXT,
  is_personal   INTEGER CHECK (is_personal IN (0,1)),
  is_immediate  INTEGER CHECK (is_immediate IN (0,1)),
  is_certain    INTEGER CHECK (is_certain IN (0,1)),
  our_solution  TEXT,
  verify_method TEXT,
  remark        TEXT,
  FOREIGN KEY (project_id) REFERENCES project(id) ON DELETE SET NULL
);

-- B9 态度变化记录（多态）
DROP TABLE IF EXISTS person_attitude_log;
CREATE TABLE person_attitude_log (
  id             INTEGER PRIMARY KEY AUTOINCREMENT,
  person_type    TEXT NOT NULL CHECK (person_type IN ('customer','partner','competitor','internal')),
  person_id      INTEGER NOT NULL,
  project_id     INTEGER,
  eval_date      TEXT,
  attitude       TEXT CHECK (attitude IN ('X','-','=','+','⭐')),
  evidence       TEXT,
  target_attitude TEXT,
  upgrade_path   TEXT,
  remark         TEXT,
  FOREIGN KEY (project_id) REFERENCES project(id) ON DELETE SET NULL
);

-- B10 "+/⭐"与可转化性评估（多态）
DROP TABLE IF EXISTS person_assessment;
CREATE TABLE person_assessment (
  id              INTEGER PRIMARY KEY AUTOINCREMENT,
  person_type     TEXT NOT NULL CHECK (person_type IN ('customer','partner','competitor','internal')),
  person_id       INTEGER NOT NULL,
  project_id      INTEGER,
  plus_features   TEXT,   -- JSON：满足的 + 特征
  star_features   TEXT,   -- JSON：满足的 ⭐ 特征
  rating          TEXT,
  evidence        TEXT,
  transformability TEXT,
  intel_value     TEXT,
  risk            TEXT,
  eval_date       TEXT,
  evaluator       TEXT,
  remark          TEXT,
  FOREIGN KEY (project_id) REFERENCES project(id) ON DELETE SET NULL
);

-- B11 情报台账（多态）
DROP TABLE IF EXISTS person_intel;
CREATE TABLE person_intel (
  id            INTEGER PRIMARY KEY AUTOINCREMENT,
  person_type   TEXT NOT NULL CHECK (person_type IN ('customer','partner','competitor','internal')),
  person_id     INTEGER NOT NULL,
  project_id    INTEGER,
  intel_date    TEXT,
  content       TEXT,
  source        TEXT,
  credibility   TEXT CHECK (credibility IN ('高','中','低')),
  application   TEXT,
  verify_result TEXT CHECK (verify_result IN ('属实','存疑','证伪')),
  remark        TEXT,
  FOREIGN KEY (project_id) REFERENCES project(id) ON DELETE SET NULL
);

-- ============================ C. 项目域 ============================

-- C1 项目（多对一挂客户）
DROP TABLE IF EXISTS project;
CREATE TABLE project (
  id                INTEGER PRIMARY KEY AUTOINCREMENT,
  customer_id       INTEGER NOT NULL,
  name              TEXT NOT NULL,
  project_type      TEXT,
  level             TEXT CHECK (level IN ('P1','P2','P3','P4')),
  info_type         TEXT CHECK (info_type IN ('线索','商机','项目')),
  sub_type          TEXT CHECK (sub_type IN ('框架','续采','创新','定制','标品')),
  project_mode      TEXT CHECK (project_mode IN ('直签','渠道','项目合作')),
  tender_mode       TEXT CHECK (tender_mode IN ('公开招标','单一来源采购','询价','邀请招标','竞争性谈判','磋商')),
  background        TEXT,
  time_phase        TEXT,
  purchase_mode     TEXT,
  purchase_cycle    TEXT CHECK (purchase_cycle IN ('前期','中期','后期')),
  budget            REAL,
  sec_budget        REAL,
  exp_budget        REAL,
  act_budget        REAL,
  amount            REAL,
  c74111_amount     REAL,
  gross_margin      REAL,
  exp_order_time    TEXT,
  act_order_time    TEXT,
  otd_phase         TEXT,
  final_accept_time TEXT,
  close_time        TEXT,
  risk_level        TEXT,
  risk              TEXT,
  next_plan         TEXT,
  market_status     TEXT,
  tech_status       TEXT,
  scope             TEXT,
  products          TEXT,
  services          TEXT,
  tech_support      TEXT,
  ucv               TEXT,
  win_score         TEXT,
  strategy          TEXT,
  competition       TEXT CHECK (competition IN ('绝对优势','相对优势','相对劣势','绝对劣势')),
  commitment        TEXT CHECK (commitment IN ('可承诺','可争取','可参与','不可承诺')),
  swot              TEXT,
  other_note        TEXT,
  created_at        TEXT DEFAULT (datetime('now','localtime')),
  updated_at        TEXT DEFAULT (datetime('now','localtime')),
  FOREIGN KEY (customer_id) REFERENCES customer(id) ON DELETE CASCADE
);

-- C2 客户人员 <-> 项目（多对多，ADUR/态度随项目）
DROP TABLE IF EXISTS project_member;
CREATE TABLE project_member (
  id          INTEGER PRIMARY KEY AUTOINCREMENT,
  project_id  INTEGER NOT NULL,
  person_id   INTEGER NOT NULL,
  adur_role   TEXT,
  attitude    TEXT CHECK (attitude IN ('X','-','=','+','⭐')),
  p3p4_flags  TEXT,   -- JSON：六条判定
  is_mentor   INTEGER DEFAULT 0 CHECK (is_mentor IN (0,1)),
  remark      TEXT,
  UNIQUE (project_id, person_id),
  FOREIGN KEY (project_id) REFERENCES project(id) ON DELETE CASCADE,
  FOREIGN KEY (person_id)  REFERENCES person(id) ON DELETE CASCADE
);

-- C3 C74111 诊断（项目 1:1）
DROP TABLE IF EXISTS project_c74111;
CREATE TABLE project_c74111 (
  id           INTEGER PRIMARY KEY AUTOINCREMENT,
  project_id   INTEGER NOT NULL UNIQUE,
  c1_score INTEGER, c1_note TEXT,
  c2_score INTEGER, c2_note TEXT,
  c3_score INTEGER, c3_note TEXT,
  c4_score INTEGER, c4_note TEXT,
  c5_score INTEGER, c5_note TEXT,
  c6_score INTEGER, c6_note TEXT,
  c7_score INTEGER, c7_note TEXT,
  p1_score INTEGER, p1_flags TEXT,
  p2_score INTEGER, p2_flags TEXT,
  p3_score INTEGER, p3_star INTEGER CHECK (p3_star IN (0,1)),
  p4_score INTEGER, p4_star INTEGER CHECK (p4_star IN (0,1)),
  key_score INTEGER, key_star INTEGER CHECK (key_star IN (0,1)),
  clear_score  INTEGER,
  priority_score INTEGER,
  key_total    INTEGER,
  total_score  INTEGER,
  win_rate     REAL,
  competition  TEXT CHECK (competition IN ('绝对优势','相对优势','相对劣势','绝对劣势')),
  commitment   TEXT CHECK (commitment IN ('可承诺','可争取','可参与','不可承诺')),
  strategy     TEXT,
  action_plan  TEXT,
  updated_at   TEXT DEFAULT (datetime('now','localtime')),
  FOREIGN KEY (project_id) REFERENCES project(id) ON DELETE CASCADE
);

-- C4 项目行动计划（开门七件事）
DROP TABLE IF EXISTS project_action_plan;
CREATE TABLE project_action_plan (
  id           INTEGER PRIMARY KEY AUTOINCREMENT,
  project_id   INTEGER NOT NULL,
  scene        TEXT,
  sales_action TEXT,
  goal_eval    TEXT,
  method       TEXT,
  summary      TEXT,
  FOREIGN KEY (project_id) REFERENCES project(id) ON DELETE CASCADE
);

-- ===================== D. 合作 / 竞争方域 =====================

-- D1 合作/竞争方实体（统一表，用 entity_type 区分）
DROP TABLE IF EXISTS coop_entity;
CREATE TABLE coop_entity (
  id                    INTEGER PRIMARY KEY AUTOINCREMENT,
  entity_type           TEXT NOT NULL CHECK (entity_type IN ('partner','competitor','both')),
  name                  TEXT NOT NULL,
  company_type          TEXT CHECK (company_type IN ('代理商','集成商','原厂','渠道','厂商','其他')),
  headquarters          TEXT,
  equity_bg             TEXT,
  listing               TEXT,
  overall_relation      TEXT,
  competition_situation TEXT,
  exclusive_promise     INTEGER DEFAULT 0 CHECK (exclusive_promise IN (0,1)),
  info_source           TEXT,
  credibility           TEXT CHECK (credibility IN ('高','中','低')),
  q_company             TEXT,   -- 三问：公司和人
  q_insider             TEXT,   -- 三问：内线
  q_strategy            TEXT,   -- 三问：策略
  product_matrix        TEXT,   -- JSON
  market_share          TEXT,   -- JSON
  channel_strategy      TEXT,
  agent_system          TEXT,
  price_band            TEXT,
  business_tactics      TEXT,
  ecosystem             TEXT,
  value_prop            TEXT,   -- 竞争方：价值主张
  product_rank          TEXT,   -- 竞争方：产品维度排名
  advantage             TEXT,
  weakness              TEXT,
  our_advantage         TEXT,
  our_weakness          TEXT,
  fab_talk              TEXT,   -- JSON
  remark                TEXT,
  created_at            TEXT DEFAULT (datetime('now','localtime')),
  updated_at            TEXT DEFAULT (datetime('now','localtime'))
);

-- D2 合作/竞争方 <-> 项目（实体级 多对多）
DROP TABLE IF EXISTS coop_entity_project_rel;
CREATE TABLE coop_entity_project_rel (
  id                INTEGER PRIMARY KEY AUTOINCREMENT,
  coop_entity_id    INTEGER NOT NULL,
  project_id        INTEGER NOT NULL,
  our_liaison       TEXT,
  their_liaison     TEXT,
  relation_nature   TEXT CHECK (relation_nature IN ('合作','竞争','竞合')),
  relation_status   TEXT CHECK (relation_status IN ('热','温','冷')),
  diff_points       TEXT,
  their_tactic      TEXT,
  our_strategy      TEXT,
  result            TEXT,
  remark            TEXT,
  UNIQUE (coop_entity_id, project_id),
  FOREIGN KEY (coop_entity_id) REFERENCES coop_entity(id) ON DELETE CASCADE,
  FOREIGN KEY (project_id)     REFERENCES project(id) ON DELETE CASCADE
);

-- D3 合作方人员 <-> 项目（人员级 多对多）
DROP TABLE IF EXISTS partner_person_project_rel;
CREATE TABLE partner_person_project_rel (
  id                INTEGER PRIMARY KEY AUTOINCREMENT,
  partner_person_id INTEGER NOT NULL,
  project_id        INTEGER NOT NULL,
  role              TEXT,
  attitude          TEXT CHECK (attitude IN ('X','-','=','+','⭐')),
  our_liaison       TEXT,
  their_liaison     TEXT,
  relation_nature   TEXT CHECK (relation_nature IN ('合作','竞争','竞合')),
  relation_status   TEXT CHECK (relation_status IN ('热','温','冷')),
  diff_points       TEXT,
  their_tactic      TEXT,
  our_strategy      TEXT,
  result            TEXT,
  remark            TEXT,
  UNIQUE (partner_person_id, project_id),
  FOREIGN KEY (partner_person_id) REFERENCES partner_person(id) ON DELETE CASCADE,
  FOREIGN KEY (project_id)        REFERENCES project(id) ON DELETE CASCADE
);

-- D4 竞争方人员 <-> 项目（人员级 多对多）
DROP TABLE IF EXISTS competitor_person_project_rel;
CREATE TABLE competitor_person_project_rel (
  id                   INTEGER PRIMARY KEY AUTOINCREMENT,
  competitor_person_id INTEGER NOT NULL,
  project_id           INTEGER NOT NULL,
  role                 TEXT,
  attitude             TEXT CHECK (attitude IN ('X','-','=','+','⭐')),
  our_liaison          TEXT,
  their_liaison        TEXT,
  relation_nature      TEXT CHECK (relation_nature IN ('合作','竞争','竞合')),
  relation_status      TEXT CHECK (relation_status IN ('热','温','冷')),
  diff_points          TEXT,
  their_tactic         TEXT,
  our_strategy         TEXT,
  result               TEXT,
  remark               TEXT,
  UNIQUE (competitor_person_id, project_id),
  FOREIGN KEY (competitor_person_id) REFERENCES competitor_person(id) ON DELETE CASCADE,
  FOREIGN KEY (project_id)           REFERENCES project(id) ON DELETE CASCADE
);

-- D5 合作/竞争历史台账（多态）
DROP TABLE IF EXISTS entity_history;
CREATE TABLE entity_history (
  id             INTEGER PRIMARY KEY AUTOINCREMENT,
  entity_type    TEXT NOT NULL CHECK (entity_type IN ('partner','competitor')),
  entity_id      INTEGER NOT NULL,
  happen_time    TEXT,
  customer_id    INTEGER,
  project_id     INTEGER,
  role           TEXT,
  their_liaison  TEXT,
  quote_discount TEXT,
  result         TEXT,
  review         TEXT,
  FOREIGN KEY (customer_id) REFERENCES customer(id) ON DELETE SET NULL,
  FOREIGN KEY (project_id)  REFERENCES project(id) ON DELETE SET NULL
);

-- ======================= E. 销售分析域 =======================
-- 全部通过 (entity_type, entity_id) 多态指向 partner / competitor

-- E1 年度销售业绩
DROP TABLE IF EXISTS entsales_perf;
CREATE TABLE entsales_perf (
  id            INTEGER PRIMARY KEY AUTOINCREMENT,
  entity_type   TEXT NOT NULL CHECK (entity_type IN ('partner','competitor')),
  entity_id     INTEGER NOT NULL,
  year          INTEGER,
  revenue       REAL,
  core_revenue  REAL,
  net_profit    REAL,
  gross_margin  REAL,
  order_amount  REAL,
  rd_invest     REAL,
  rd_ratio      REAL,
  staff         INTEGER,
  sales_staff   INTEGER,
  health        TEXT,
  growth        TEXT,
  source        TEXT,
  remark        TEXT
);

-- E2 分产品收入结构
DROP TABLE IF EXISTS entsales_product;
CREATE TABLE entsales_product (
  id           INTEGER PRIMARY KEY AUTOINCREMENT,
  entity_type  TEXT NOT NULL CHECK (entity_type IN ('partner','competitor')),
  entity_id    INTEGER NOT NULL,
  product_line TEXT,
  revenue      REAL,
  ratio        REAL,
  our_product  TEXT,
  our_revenue  REAL,
  gap          TEXT,
  remark       TEXT
);

-- E3 细分市场份额
DROP TABLE IF EXISTS entsales_market;
CREATE TABLE entsales_market (
  id           INTEGER PRIMARY KEY AUTOINCREMENT,
  entity_type  TEXT NOT NULL CHECK (entity_type IN ('partner','competitor')),
  entity_id    INTEGER NOT NULL,
  year         INTEGER,
  segment      TEXT,
  their_share  REAL,
  their_rank   TEXT,
  our_share    REAL,
  our_rank     TEXT,
  source       TEXT
);

-- E4 分行业 / 区域分布
DROP TABLE IF EXISTS entsales_distribution;
CREATE TABLE entsales_distribution (
  id           INTEGER PRIMARY KEY AUTOINCREMENT,
  entity_type  TEXT NOT NULL CHECK (entity_type IN ('partner','competitor')),
  entity_id    INTEGER NOT NULL,
  year         INTEGER,
  dim_type     TEXT CHECK (dim_type IN ('行业','区域')),
  dim_value    TEXT,
  revenue      REAL,
  ratio        REAL,
  their_rank   TEXT,
  our_revenue  REAL,
  our_ratio    REAL,
  our_position TEXT,
  remark       TEXT
);

-- E5 渠道与代理
DROP TABLE IF EXISTS entsales_channel;
CREATE TABLE entsales_channel (
  id             INTEGER PRIMARY KEY AUTOINCREMENT,
  entity_type    TEXT NOT NULL CHECK (entity_type IN ('partner','competitor')),
  entity_id      INTEGER NOT NULL,
  dim            TEXT,
  content        TEXT,
  our_comparison TEXT
);

-- E6 价格与商务策略
DROP TABLE IF EXISTS entsales_pricing;
CREATE TABLE entsales_pricing (
  id             INTEGER PRIMARY KEY AUTOINCREMENT,
  entity_type    TEXT NOT NULL CHECK (entity_type IN ('partner','competitor')),
  entity_id      INTEGER NOT NULL,
  dim            TEXT,
  their_practice TEXT,
  our_position   TEXT,
  response       TEXT
);

-- E7 客户级竞争份额
DROP TABLE IF EXISTS entsales_customer_share;
CREATE TABLE entsales_customer_share (
  id             INTEGER PRIMARY KEY AUTOINCREMENT,
  entity_type    TEXT NOT NULL CHECK (entity_type IN ('partner','competitor')),
  entity_id      INTEGER NOT NULL,
  customer_name  TEXT,
  year           INTEGER,
  budget         REAL,
  their_amount   REAL,
  their_share    REAL,
  our_amount     REAL,
  our_share      REAL,
  their_rank     TEXT,
  trend          TEXT,
  key_products   TEXT,
  win_loss       TEXT,
  remark         TEXT
);

-- E8 标讯结构对比
DROP TABLE IF EXISTS entsales_tender;
CREATE TABLE entsales_tender (
  id               INTEGER PRIMARY KEY AUTOINCREMENT,
  entity_type      TEXT NOT NULL CHECK (entity_type IN ('partner','competitor')),
  entity_id        INTEGER NOT NULL,
  customer_name    TEXT,
  year             INTEGER,
  tender_amount    REAL,
  proj_count_a     INTEGER,
  should_join_b    INTEGER,
  should_join_rate REAL,
  bid_c            INTEGER,
  win_d            INTEGER,
  actual_bid_rate  REAL,
  win_rate         REAL,
  their_win        TEXT,
  remark           TEXT
);

-- E9 项目级正面交锋
DROP TABLE IF EXISTS entsales_confrontation;
CREATE TABLE entsales_confrontation (
  id            INTEGER PRIMARY KEY AUTOINCREMENT,
  entity_type   TEXT NOT NULL CHECK (entity_type IN ('partner','competitor')),
  entity_id     INTEGER NOT NULL,
  customer_name TEXT,
  project_name  TEXT,
  happen_time   TEXT,
  their_mode    TEXT,
  their_quote   TEXT,
  our_quote     TEXT,
  result        TEXT,
  key_reason    TEXT,
  review        TEXT
);

-- E10 销售打法与套路
DROP TABLE IF EXISTS entsales_tactic;
CREATE TABLE entsales_tactic (
  id             INTEGER PRIMARY KEY AUTOINCREMENT,
  entity_type    TEXT NOT NULL CHECK (entity_type IN ('partner','competitor')),
  entity_id      INTEGER NOT NULL,
  dim            TEXT,
  their_practice TEXT,
  our_signal     TEXT,
  our_response   TEXT
);

-- E11 与我司能力对比
DROP TABLE IF EXISTS entsales_benchmark;
CREATE TABLE entsales_benchmark (
  id           INTEGER PRIMARY KEY AUTOINCREMENT,
  entity_type  TEXT NOT NULL CHECK (entity_type IN ('partner','competitor')),
  entity_id    INTEGER NOT NULL,
  dim          TEXT,
  theirs       TEXT,
  ours         TEXT,
  leader       TEXT,
  gap          TEXT
);

-- E12 应对策略/行动计划
DROP TABLE IF EXISTS entsales_plan;
CREATE TABLE entsales_plan (
  id           INTEGER PRIMARY KEY AUTOINCREMENT,
  entity_type  TEXT NOT NULL CHECK (entity_type IN ('partner','competitor')),
  entity_id    INTEGER NOT NULL,
  goal         TEXT,
  strategy     TEXT,
  smart_plan   TEXT,
  resource     TEXT,
  owner        TEXT,
  plan_time    TEXT,
  target       TEXT,
  remark       TEXT
);

-- E13 信息来源与更新
DROP TABLE IF EXISTS entsales_source;
CREATE TABLE entsales_source (
  id           INTEGER PRIMARY KEY AUTOINCREMENT,
  entity_type  TEXT NOT NULL CHECK (entity_type IN ('partner','competitor')),
  entity_id    INTEGER NOT NULL,
  source       TEXT,
  purpose      TEXT,
  frequency    TEXT,
  owner        TEXT
);

-- ======================= F. 拜访行动域 =======================

-- F1 拜访记录
DROP TABLE IF EXISTS visit_record;
CREATE TABLE visit_record (
  id                  INTEGER PRIMARY KEY AUTOINCREMENT,
  customer_id         INTEGER NOT NULL,
  project_id          INTEGER,
  visit_date          TEXT,
  visit_time          TEXT,
  place               TEXT,
  method              TEXT CHECK (method IN ('面访','电话','视频')),
  our_participants    TEXT,
  client_participants TEXT,
  opportunity         TEXT,
  recorder            TEXT,
  purpose             TEXT,
  goal_smart          TEXT,
  agenda              TEXT,
  process_record      TEXT,
  needs_pain          TEXT,
  competition_intel   TEXT,
  client_feedback     TEXT,
  client_commit       TEXT,
  our_commit          TEXT,
  result              TEXT,
  short_todo          TEXT,
  mid_follow          TEXT,
  next_visit          TEXT,
  resource_need       TEXT,
  review_good         TEXT,
  review_bad          TEXT,
  key_harvest         TEXT,
  next_strategy       TEXT,
  attachment          TEXT,
  remark              TEXT,
  created_at          TEXT DEFAULT (datetime('now','localtime')),
  FOREIGN KEY (customer_id) REFERENCES customer(id) ON DELETE CASCADE,
  FOREIGN KEY (project_id)  REFERENCES project(id) ON DELETE SET NULL
);

-- F2 我方参与人（拜访 -> 奇安信内部人员，多对多）
DROP TABLE IF EXISTS visit_our_staff;
CREATE TABLE visit_our_staff (
  id                INTEGER PRIMARY KEY AUTOINCREMENT,
  visit_id          INTEGER NOT NULL,
  internal_person_id INTEGER NOT NULL,
  role              TEXT,
  remark            TEXT,
  UNIQUE (visit_id, internal_person_id),
  FOREIGN KEY (visit_id)           REFERENCES visit_record(id) ON DELETE CASCADE,
  FOREIGN KEY (internal_person_id) REFERENCES internal_person(id) ON DELETE CASCADE
);

-- F3 拜访对象（客户人员，1:N）
DROP TABLE IF EXISTS visit_object;
CREATE TABLE visit_object (
  id            INTEGER PRIMARY KEY AUTOINCREMENT,
  visit_id      INTEGER NOT NULL,
  person_id     INTEGER,
  adur_role     TEXT,
  attitude      TEXT CHECK (attitude IN ('X','-','=','+','⭐')),
  disc          TEXT,
  burning_issue TEXT,
  remark        TEXT,
  FOREIGN KEY (visit_id)  REFERENCES visit_record(id) ON DELETE CASCADE,
  FOREIGN KEY (person_id) REFERENCES person(id) ON DELETE SET NULL
);

-- F4 拜访行动项（1:N）
DROP TABLE IF EXISTS visit_action_item;
CREATE TABLE visit_action_item (
  id              INTEGER PRIMARY KEY AUTOINCREMENT,
  visit_id        INTEGER NOT NULL,
  seq             INTEGER,
  action          TEXT,
  owner           TEXT,
  deliverable     TEXT,
  due_time        TEXT,
  priority        TEXT CHECK (priority IN ('高','中','低')),
  status          TEXT,
  background      TEXT,
  method          TEXT,
  expected_effect TEXT,
  risk            TEXT,
  FOREIGN KEY (visit_id) REFERENCES visit_record(id) ON DELETE CASCADE
);

-- ============ G. Dashboard / 动态域（来源：网络安全dashboard最新动态模板） ============

-- G1 行业 / AI 安全动态（每日新闻）
DROP TABLE IF EXISTS news_item;
CREATE TABLE news_item (
  id         INTEGER PRIMARY KEY AUTOINCREMENT,
  category   TEXT CHECK (category IN ('行业与市场','政策法规','攻防事件/漏洞','AI安全')),
  title      TEXT,
  summary    TEXT,
  source     TEXT,
  news_date  TEXT,
  tags       TEXT,
  created_at TEXT DEFAULT (datetime('now','localtime'))
);

-- G2 客户动态（存量 + 增量）
DROP TABLE IF EXISTS customer_news;
CREATE TABLE customer_news (
  id            INTEGER PRIMARY KEY AUTOINCREMENT,
  customer_id   INTEGER NOT NULL,
  news_title    TEXT,
  summary       TEXT,
  source        TEXT,
  news_date     TEXT,
  our_relation  TEXT,
  is_new        INTEGER DEFAULT 0 CHECK (is_new IN (0,1)),  -- 0=存量, 1=增量
  batch_time    TEXT,                                        -- 本次/上次更新批次时间
  created_at    TEXT DEFAULT (datetime('now','localtime')),
  FOREIGN KEY (customer_id) REFERENCES customer(id) ON DELETE CASCADE
);

-- G3 本期信息 / 更新日志
DROP TABLE IF EXISTS dashboard_update_log;
CREATE TABLE dashboard_update_log (
  id               INTEGER PRIMARY KEY AUTOINCREMENT,
  update_date      TEXT,
  update_time      TEXT,
  last_update_time TEXT,
  operator         TEXT,
  coverage         TEXT
);

-- G4 用户每日行动计划
DROP TABLE IF EXISTS daily_plan;
CREATE TABLE daily_plan (
  id             INTEGER PRIMARY KEY AUTOINCREMENT,
  plan_date      TEXT,
  one_line_goal  TEXT,
  next_action    TEXT,
  created_at     TEXT DEFAULT (datetime('now','localtime'))
);

-- G5 今日待办
DROP TABLE IF EXISTS daily_todo;
CREATE TABLE daily_todo (
  id          INTEGER PRIMARY KEY AUTOINCREMENT,
  plan_date   TEXT,
  item        TEXT,
  customer_id INTEGER,
  project_id  INTEGER,
  priority    TEXT CHECK (priority IN ('高','中','低')),
  status      TEXT,
  created_at  TEXT DEFAULT (datetime('now','localtime')),
  FOREIGN KEY (customer_id) REFERENCES customer(id) ON DELETE SET NULL,
  FOREIGN KEY (project_id)  REFERENCES project(id) ON DELETE SET NULL
);

-- G6 灵感便签
DROP TABLE IF EXISTS dashboard_note;
CREATE TABLE dashboard_note (
  id         INTEGER PRIMARY KEY AUTOINCREMENT,
  content    TEXT,
  created_at TEXT DEFAULT (datetime('now','localtime')),
  updated_at TEXT DEFAULT (datetime('now','localtime'))
);

-- G7 快捷入口
DROP TABLE IF EXISTS dashboard_shortcut;
CREATE TABLE dashboard_shortcut (
  id         INTEGER PRIMARY KEY AUTOINCREMENT,
  title      TEXT,
  url        TEXT,
  icon       TEXT,
  sort_order INTEGER,
  created_at TEXT DEFAULT (datetime('now','localtime'))
);

-- G8 专注计时记录（番茄钟）
DROP TABLE IF EXISTS focus_session;
CREATE TABLE focus_session (
  id           INTEGER PRIMARY KEY AUTOINCREMENT,
  session_date TEXT,                                        -- 归属日期 YYYY-MM-DD，按天聚合用
  started_at   TEXT,                                        -- 开始时间 ISO8601 本地时间
  ended_at     TEXT,                                        -- 结束时间
  plan_min     INTEGER DEFAULT 25,                          -- 计划时长（分钟）
  actual_min   INTEGER DEFAULT 0,                           -- 实际专注时长（分钟）
  outcome      TEXT CHECK (outcome IN ('已完成','已中断')),  -- 自然走完 / 中途放弃
  task_id      INTEGER,                                     -- 可选：本次专注对应哪条今日待办
  note         TEXT,
  created_at   TEXT DEFAULT (datetime('now','localtime')),
  FOREIGN KEY (task_id) REFERENCES daily_todo(id) ON DELETE SET NULL
);

-- ======================= 索引 =======================
CREATE INDEX idx_customer_relation_cid ON customer_relation(customer_id);
CREATE INDEX idx_project_customer      ON project(customer_id);
CREATE INDEX idx_person_customer       ON person(customer_id);
CREATE INDEX idx_person_parent         ON person(parent_id);
CREATE INDEX idx_partner_person_eid     ON partner_person(coop_entity_id);
CREATE INDEX idx_competitor_person_eid  ON competitor_person(coop_entity_id);
CREATE INDEX idx_internal_person_org   ON internal_person(org_id);
CREATE INDEX idx_org_parent            ON org(parent_id);
CREATE INDEX idx_project_member_proj   ON project_member(project_id);
CREATE INDEX idx_project_member_person ON project_member(person_id);
CREATE INDEX idx_cepr_entity           ON coop_entity_project_rel(coop_entity_id);
CREATE INDEX idx_cepr_project          ON coop_entity_project_rel(project_id);
CREATE INDEX idx_pppr_person           ON partner_person_project_rel(partner_person_id);
CREATE INDEX idx_pppr_project          ON partner_person_project_rel(project_id);
CREATE INDEX idx_cppr_person           ON competitor_person_project_rel(competitor_person_id);
CREATE INDEX idx_cppr_project          ON competitor_person_project_rel(project_id);
CREATE INDEX idx_form_person           ON person_form(person_type, person_id);
CREATE INDEX idx_disc_person           ON person_disc(person_type, person_id);
CREATE INDEX idx_bi_person             ON person_burning_issue(person_type, person_id);
CREATE INDEX idx_attitude_person       ON person_attitude_log(person_type, person_id);
CREATE INDEX idx_assessment_person     ON person_assessment(person_type, person_id);
CREATE INDEX idx_intel_person          ON person_intel(person_type, person_id);
CREATE INDEX idx_ehist_entity          ON entity_history(entity_type, entity_id);
CREATE INDEX idx_eperf_entity          ON entsales_perf(entity_type, entity_id);
CREATE INDEX idx_eshare_entity         ON entsales_customer_share(entity_type, entity_id);
CREATE INDEX idx_visit_customer        ON visit_record(customer_id);
CREATE INDEX idx_visit_staff_visit      ON visit_our_staff(visit_id);
CREATE INDEX idx_visit_staff_person     ON visit_our_staff(internal_person_id);
CREATE INDEX idx_cnews_customer         ON customer_news(customer_id);
CREATE INDEX idx_news_date              ON news_item(news_date);
CREATE INDEX idx_daily_todo_date        ON daily_todo(plan_date);
CREATE INDEX idx_focus_date             ON focus_session(session_date);

-- ======================= 初始数据示例（可选） =======================
-- 奇安信组织根节点
INSERT INTO org (name, parent_id, org_type, level)
VALUES ('奇安信集团', NULL, '集团', 0);
-- 组织层级示例（按用户口径）：
--   目前已明确的仅为【营销体系】架构：
--   奇安信集团
--     └─ 集团营销中心
--          └─ 特区 / 大区
--               └─ 银行集团军
--                    └─ 军团（31/32…）→ 处 / 组
--   其他部门（人力、行政、产研服等）架构尚未明确，org_type 保持自由文本，暂不固定枚举。
-- TODO: 后续补充人力/行政/产研服等部门的 org 层级后，再决定是否固定 org_type 取值。

-- 建表完成，开启外键强制（之后 DML 生效）
PRAGMA foreign_keys = ON;

-- =====================================================================
-- 说明：
-- 1) 多态表（person_form/person_disc/... 与 entsales_*）使用
--    (person_type/person_id) 或 (entity_type/entity_id) 逻辑关联，
--    SQLite 不支持多态外键，故未建 FOREIGN KEY，由应用层保证一致性。
--    其中 entity_type 取值 partner/competitor，entity_id 指向 coop_entity.id。
-- 2) 金额单位统一为「万元」；日期为 ISO8601 文本。
-- 3) 合作方与竞争方**实体合并为 coop_entity 一张表**（entity_type 区分）；
--    partner_person 与 competitor_person 仍为不同人员集合，保持分表。
-- 4) 合作/竞争方与项目为**两层多对多**：
--    - 实体级：coop_entity_project_rel
--    - 人员级：partner_person_project_rel / competitor_person_project_rel
-- 5) project.customer_id 仅指向「主客户」（category='主客户'），由应用层保证。
-- 6) 已确认决策见《客户管理系统数据库说明表（SQLite）》第六节。
-- =====================================================================
