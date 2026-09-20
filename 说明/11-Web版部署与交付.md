# 11 · Web 版部署与交付

> 本文讲**第三种交付形态**：不装桌面程序，服务端起一个进程，用户用浏览器访问。
>
> 相对桌面版（Tauri 单文件 exe）的核心差别：**客户端不再创建任何自研可执行文件**。

---

## 一、先说结论

Web 版采用「**单进程内嵌托管**」：后端进程既提供 API，也把前端静态产物一起托管。目标机：

- 不需要装 Node / Rust / 任何运行时
- 不需要装 WebView2
- 只跑 **1 个进程**、只占 **1 个端口**
- 用户侧只用浏览器

代价：**当前 API 没有任何鉴权**，所以默认只监听 `127.0.0.1`。要开放给局域网必须先想清楚网络边界（见第六节）。

---

## 二、交付目录长什么样

```text
Web版/
├── bcrm-backend.exe     后端（含静态托管能力），3.65 MB
├── 启动Web版.bat        一键启动：双击即可
├── web/                 前端构建产物
│   ├── index.html
│   └── assets/
│       ├── index-*.js
│       └── style-*.css
└── data/                首次运行自动创建
    ├── crm.db           数据库（62 张表，自动建库）
    └── web-server.log   服务端日志
```

⚠️ **`bcrm-backend.exe` 必须放在 `web/` 之外**。静态托管会把 `web/` 下的所有文件原样发给浏览器，
如果把 exe 塞进 `web/`，它自己也会被 HTTP 下载走。

---

## 三、一键启动

双击 `启动Web版.bat`。它做这几件事：

1. 检查是否已有 BCRM 服务在跑（有就直接开浏览器，不重复起进程）
2. 定位 `bcrm-backend.exe` 与 `web/index.html`
3. 从 8787 起找一个空闲端口（被占自动顺延，最多试 20 个）
4. 起服务（接口 + 页面同一个进程）
5. 轮询等就绪（最多 40 秒）
6. 自动打开浏览器

**关闭窗口 = 停止服务**（服务与窗口共享同一个控制台）。

### 可选参数

```bat
启动Web版.bat                    默认 127.0.0.1:8787，仅本机可访问
启动Web版.bat 9000               改用 9000 端口
启动Web版.bat 9000 0.0.0.0       监听所有网卡（局域网可访问，先看第六节）
```

---

## 四、命令行用法

Web 版能力来自后端新增的参数，脚本只是包了一层：

```bat
bcrm-backend.exe --web-dir ..\frontend\dist
bcrm-backend.exe --web-dir .\web --port 9000 --db .\data\crm.db
bcrm-backend.exe --web-dir .\web --host 0.0.0.0
bcrm-backend.exe --no-web                      强制纯 API 模式
bcrm-backend.exe --help
```

| 参数 | 说明 |
| --- | --- |
| `--port <端口>` | 监听端口，默认 8787 |
| `--host <地址>` | 监听地址，默认 `127.0.0.1` |
| `--db <文件>` | 指定数据库文件；不给则按 exe 同级 → `%LOCALAPPDATA%\Bcrm` 自动定位 |
| `--web-dir <目录>` | 托管该目录的前端产物（需含 `index.html`） |
| `--no-web` | 强制纯 API，不做静态托管 |
| `--db-check` | 只做数据库自检并打印报告后退出 |

**不给 `--web-dir` 时会自动探测**，按顺序找：

1. exe 同级 `web/`、`dist/`
2. 源码树：`backend/target/release/` → 仓库根 `frontend/dist/`
3. 当前目录下的 `web/`、`frontend/dist/`

全都找不到就退化成**纯 API 服务**（不报错）。

---

## 五、三种交付形态怎么选

| | A · 单进程内嵌托管 | B · 前后端分离 | C · vite preview 脚本 |
| --- | --- | --- | --- |
| 服务端组成 | 1 个进程 | Nginx/IIS + 后端 | node vite + 后端 |
| 端口 | 1 个 | 2 个 | 2 个 |
| 目标机需装 | **无** | Web 服务器 | Rust + Node 工具链 |
| 跨源处理 | 同源，无需处理 | 需自行配置 CORS / 反代 | 靠 vite 反代 |
| 适合 | **交付、演示、内网部署** | 已有 Web 服务器的环境 | 开发联调 |
| 入口 | `启动Web版.bat` | 手工部署 | `启动Web服务.bat` |

形态 A 和 B **共用同一份 API 定义**，只差一个 `--web-dir`：

- 给 `--web-dir` → 形态 A
- 不给（或 `--no-web`）→ 形态 B 的后端

所以从 A 切到 B 不用改代码，把 `web/` 交给 Nginx，后端加 `--no-web` 起起来即可。

---

## 六、⚠️ 安全边界

**当前后端 API 没有任何鉴权。** 任何能访问到该端口的人都能读写全部 62 张表。

- 默认只监听 `127.0.0.1` —— 只有本机能访问，可以接受
- 用 `--host 0.0.0.0` 之前，请务必确认：
  - 这台机器不在能上外网的网络里
  - 或者已经在前置反代上加了访问控制（Basic Auth / IP 白名单 / 公司 SSO）

> ⚠️ 不要把 `0.0.0.0` 直接暴露到公网。

后续要正式对内网提供服务，建议按这个顺序加固：反代加鉴权 → 后端加会话令牌 → 才考虑开放网段。

---

## 七、和终端管控的关系（重要）

Web 版是**换一种交付形态**，不是绕过手段。要分清两件事：

| | 桌面版 | Web 版 |
| --- | --- | --- |
| 谁创建进程 | **每台目标机**，创建自研 exe | **只有服务端一台** |
| 终端管控影响 | 每台机器都要加白 | 只需服务端加白一次 |
| 用户侧 | 需要装、需要加白 | 只用浏览器，无进程创建 |

也就是说：把服务部署到一台**允许运行该程序的机器**（通常是服务器，且服务器一般不受终端管控的「陌生程序」策略约束），
其余机器就只用浏览器访问 —— 这才是 Web 版的真正价值。

如果连服务端那台也被拦，症状是**连 `data\web-server.log` 都不生成**，
按 [10 篇](./10-终端管控拦截：机制与解决方案.md) 的思路处理：把 `Web版/` 整个目录加进信任区（**按目录加，不要按文件**，否则每重编一版都要重加）。

---

## 八、本轮实测证据

环境：Windows · `bcrm-backend.exe` release 构建 · 临时库（非生产数据）

启动日志：

```text
客户管理系统后端已启动
  模式    : Web 版（接口 + 前端页面，同一进程托管）
  界面    : http://127.0.0.1:18788/
  静态产物: ...\Application\Web版\web
  接口    : http://127.0.0.1:18788/api/v1
  数据库  : ...\crm_auto.db
  表白名单: 62 张
```

同端口逐项探测：

| 请求 | 状态 | Content-Type | 字节 |
| --- | --- | --- | --- |
| `GET /` | 200 | text/html | 519 |
| `GET /index.html` | 200 | text/html | 519 |
| `GET /some/deep/link` | 200 | text/html | 519 |
| `GET /assets/index-*.js` | 200 | text/javascript | 308,012 |
| `GET /assets/style-*.css` | 200 | text/css | 111,889 |
| `GET /api/v1/meta/health` | 200 | application/json | 2,504 |
| `GET /api/v1/customer` | 200 | application/json | 94 |

两个值得记下来的坑：

1. **SPA 回落不能用 `not_found_service`。** 它的返回类型是 `ServeDir<SetStatus<F2>>`，
   会把回落响应的状态码**强制改写成 404**。实测确认：`GET /some/deep/link` 拿到的是
   「404 + 519 字节正确 index.html」—— 内容对、状态码错。改用 `ServeDir::fallback` 原样透传 200 才对。

2. **`.bat` 必须是 GBK + CRLF。** 本仓库的 bat 交付物统一 GBK(936) 编码；
   另外 cmd.exe 对 **LF-only** 的 bat 在 `goto` 标签和括号块上解析不可靠，必须用 CRLF。

---

## 九、从源码重建

```bash
# 1. 前端产物
cd frontend && npm install && npm run build

# 2. 后端
cargo build --release --manifest-path backend/Cargo.toml

# 3. 组装
mkdir -p "Application/Web版/web"
cp backend/target/release/bcrm-backend.exe "Application/Web版/"
cp -r frontend/dist/. "Application/Web版/web/"
cp 启动Web版.bat "Application/Web版/"
```

前端**不需要特殊构建参数**：`client.js` 在页面由 http/https 提供时自动走同源相对路径 `/api/v1`，
所以同一份 `dist/` 在三种环境下都能用：

| 打开方式 | API 基址 | 走哪条分支 |
| --- | --- | --- |
| Tauri 桌面壳 | `http://127.0.0.1:<随机端口>/api/v1` | 宿主注入（优先级最高） |
| 浏览器 + Web 版 | `/api/v1`（相对路径） | 同源兜底 |
| 浏览器直接打开 `dist/index.html`（`file://`） | `http://127.0.0.1:8787/api/v1` | 写死兜底 |
