# CC Project Manager（Windows）软件设计

2026-09-22 · 依据《CC Project Manager（Windows）需求规格》（同目录）编写 · 状态：**已定稿（v1.0）**，2026-09-22 需求方确认全部决策点按推荐选项执行，技术栈选定 Tauri 2

> 阅读指引：文中标记 **【D-n】** 的地方是曾经的决策点，第 12 节记录了每一项的最终决定。标记 **【假设】** 的是本文基于本机实测做出、但需在实现时再次验证的假设。

---

## 1. 文档目的与范围

本文将需求规格转化为可实施的软件设计：总体架构、技术选型、核心数据模型、各核心模块的接口与算法、UI 结构、工具自身的数据存放、测试策略与分阶段计划。

范围与需求规格第 3～6 节一致：项目列表与操作（含 purge）、记忆迁移与导出/导入、全局视图、横切安全规则。需求第 8 节的非目标本文不设计，但在架构上预留扩展点并逐条说明。

## 2. 本机实测结论（影响设计的事实）

设计前在本机（CC 2.1.278，数据根目录 `%USERPROFILE%\.claude`，未设置 `CLAUDE_CONFIG_DIR`）做了只读核实。以下事实与需求规格的描述存在差异或补充，直接影响设计：

| # | 实测事实 | 对设计的影响 |
| --- | --- | --- |
| F1 | `claude project purge` 存在，选项为 `--all`、`--dry-run`、`-i/--interactive`、`-y/--yes`。dry-run 输出为人类可读文本（`dir:` / `config:` / `filter:` 三类条目 + 提示行 + `Dry run: N item(s) would be deleted.`），**不是 JSON** | 确认框按需求「原样展示」，工具不解析其语义；只用退出码判断成败 |
| F2 | `~/.claude.json` 的 `projects` 键以**正斜杠**路径为 key（如 `C:/Users/alice/Desktop/video`），UNC 路径写作 `//192.168.1.10/share/camera`。文件为 2 空格缩进的 pretty JSON，约 100 KB、2800 行 | 迁移改写 key 时须按此格式生成；改写后保持缩进与键顺序，避免大 diff |
| F3 | 编码函数实测：先把路径统一为正斜杠，再把**每个**不属于 `[A-Za-z0-9-]` 的字符替换为 `-`（`:`、`/`、`.`、`_`、空格、每个中文字符各变一个 `-`）。以此规则匹配本机 38 条配置 vs 18 个数据目录，匹配 17 个 | 这是 `core.discovery` 编码函数的初始实现 |
| F4 | 唯一未匹配的目录 `C--Users-alice-Desktop-work-win_project-food-ordering` 保留了 `_`，与同项目的新目录 `...-win-project-food-ordering` 并存。说明**旧版本 CC 的编码规则保留下划线**，规则已至少变过一次 | 证实需求 2.3 第 5 条自校验的必要性；同时引入「旧编码残留」这一子类【D-2】 |
| F5 | 38 条配置里 **21 条没有对应数据目录**（例如 `C:/Windows/System32`、`C:/Users/alice`）——CC 只要在某目录启动过就会写配置项，但没产生转录就没有数据目录 | 需求只定义了「孤儿」「无主」两种异常态，须新增「仅配置」态【D-3】 |
| F6 | `sessions/` 下每个会话有 `<pid>.json`（含 `pid`、`sessionId`、`cwd`、`procStart`、`status`、`updatedAt`）和 `<pid>.<hash>.key`。存在**僵尸文件**：`10568.json` 记录的 cwd 为 `Y:\imv350`，但 PID 10568 现已被 `StartMenuExperienceHost.exe` 复用 | 运行中判定不能只看文件存在，必须做「PID 存活 + 进程映像名 + 进程启动时间」三重校验（第 6.2 节） |
| F7 | `settings.json` 未设置 `cleanupPeriodDays` | 全局视图显示时须区分「显式设置值」与「默认值 30」 |
| F8 | 转录每行一个 JSON，`type` 有 `user`/`assistant`/`attachment`/`file-history-snapshot`/`ai-title`/`last-prompt` 等多种；`assistant` 行的 `message.usage` 含 `input_tokens`、`output_tokens`、`cache_creation_input_tokens`、`cache_read_input_tokens` 及若干其他键；每行带 `cwd`、`sessionId`、`timestamp` | `core.usage` 只累加四个标准字段，其余键忽略；`cwd` 可用于交叉校验目录归属 |
| F9 | `file-history/<sessionId>/<hash>@vN` 按会话组织；转录中的 `file-history-snapshot` 行给出 session 与快照的关联 | 第一版按需求只做全局统计；session→project 映射的数据源已确认，留作扩展 |
| F10 | `stats-cache.json`（version 5）含 `dailyModelTokens[].tokensByModel`、`modelUsage`、`totalSessions`、`totalMessages`、`lastComputedDate` 等 | 全局 token 直接读此文件；注明 `lastComputedDate` 以说明数据时效 |
| F11 | 数据根目录下还有需求未列出的 `cache/`、`skills/`、`hooks/`、`memory/`（全局记忆）、`telemetry/`、`feedback/`、`CLAUDE.md`（全局用户指令）、`daemon*` 若干文件、`.last-cleanup` 等 | 白名单需扩充【D-5】；未列入分类表的一律按「未知数据」处理 |
| F12 | 本机开发工具：Node 24 / npm 11 / Python 3.11；**没有** Rust、.NET SDK、pwsh 7 | 影响技术选型成本评估【D-1】 |

## 3. 总体架构

### 3.1 分层

```
┌──────────────────────────────────────────────────────────┐
│  UI 层（桌面窗口）                                          │
│  项目列表 │ 项目面板 │ 全局视图 │ 记忆管理 │ 确认对话框      │
├──────────────────────────────────────────────────────────┤
│  应用层（App Shell）                                        │
│  命令路由 · 后台任务调度 · 进度/事件推送 · 工具自身设置       │
├──────────────────────────────────────────────────────────┤
│  核心库 cc_core（无 UI 依赖，可独立测试）                     │
│  discovery │ scan │ usage │ purge │ memory │ guard          │
│  ─────────────── 公共基础 ───────────────                    │
│  paths（数据根解析）· model（数据模型）· cache（工具缓存）    │
│  · backup（工具备份）· proc（进程/CLI 调用）· log            │
├──────────────────────────────────────────────────────────┤
│  外部：文件系统 · ~/.claude.json · claude CLI · Windows API   │
└──────────────────────────────────────────────────────────┘
```

原则：

- **核心库与 UI 严格分离**。核心库不引用任何 UI 框架，所有输入输出是普通数据结构，便于单元测试和替换 UI。
- **所有写操作只有一个入口**：`guard.preflight()`。任何模块想写盘，必须先拿到 preflight 返回的通行令牌（见 6.2）。
- **只读路径与写路径分离**：扫描、统计等只读功能对数据根目录只开只读句柄，且不在数据根目录内创建任何文件；工具自身的缓存、备份、日志全部放在独立目录（第 9 节）。

### 3.2 进程与线程模型

- 单进程桌面应用。UI 线程只做渲染与事件分发。
- 核心库的耗时操作（全量扫描、转录解析、zip 打包）在**后台工作线程**执行，通过事件通道向 UI 推送进度与结果；UI 先渲染上次缓存的结果，后台完成后局部刷新（需求 5.4）。
- 调用 `claude` CLI 时以子进程方式执行，捕获 stdout/stderr 与退出码，设置超时（dry-run 30 s，purge 120 s）；超时视为失败并提示用户手工检查。
- 同一时刻只允许一个写操作在进行（全局互斥），避免迁移与 purge 交错。

### 3.3 关键数据流

```
启动
 ├─ paths.resolve_root()  → 读 CLAUDE_CONFIG_DIR / %USERPROFILE%\.claude
 ├─ discovery.load()      → 读 ~/.claude.json + 枚举 projects/ → 项目清单 + 编码自校验结果
 ├─ purge.detect_cli()    → where claude → CLI 可用性
 ├─ cache.load_last_scan()→ 立即渲染上次结果
 └─ 后台：scan.run() + usage.update() → 推送刷新
写操作（purge / 迁移 / 导入 / 分类删除）
 └─ guard.preflight(op) → 白名单校验 + 运行中会话检测 + 编码自校验状态
      → 通过：备份（如需）→ 执行 → 记录操作日志 → 触发局部重扫
      → 拒绝：返回结构化原因，UI 原文展示
```

## 4. 技术选型 【D-1】

需求第 7 节把选型列为开放问题。**最终选定方案 A：Tauri 2 + Rust 核心库 + Vue 3/TypeScript 界面。** 以下保留三个方案的对比作为决策记录。

| 方案 | 核心库语言 | UI | 产物体积 | 原型速度 | 测试/健壮性 | 本机现状 |
| --- | --- | --- | --- | --- | --- | --- |
| **A. Tauri 2（推荐）** | Rust（`cc_core` crate） | Web 前端（Vue 3 + TypeScript），WebView2 渲染 | 单 exe 约 5～10 MB，Win11 自带 WebView2 | 中：需装 Rust 工具链，编译 1～2 分钟 | 强类型 + serde 防御性解析，`cargo test` 覆盖核心库 | 需安装 rustup |
| B. Python + PySide6 | Python 包 `cc_core` | Qt Widgets | PyInstaller 约 50～80 MB，启动稍慢 | 快：Python 已在本机 | pytest 易写；运行时类型弱，防御性解析靠人工 | 可立即开始 |
| C. Electron + TypeScript | TypeScript 包 `cc_core` | Web 前端 | 150 MB 以上 | 快：Node 已在本机 | vitest；类型尚可 | 可立即开始 |

选 A 的理由：这是一个长期使用的小工具，「分发体验」和「让人放心」比一次性原型速度重要；Rust 的强类型和 serde 对第 10 节防御性解析最有帮助；产物小、启动快。代价是要装 Rust 工具链并接受较长编译时间。

**方案 A 的具体落地**

| 层 | 选型 |
| --- | --- |
| 核心库 | Rust crate `cc_core`（workspace 成员，纯库，无 Tauri 依赖）。关键依赖：`serde`/`serde_json`（`preserve_order` 特性，保证改写 `~/.claude.json` 时键顺序不变）、`walkdir`、`zip`、`windows` crate（进程查询、回收站 `IFileOperation`）、`thiserror`、`tracing` |
| 应用层 | Tauri 2 应用 `src-tauri`，只包含 `#[tauri::command]` 薄封装、后台任务调度（`tokio` 任务 + `tauri::Emitter` 事件推送）、工具自身设置 |
| 界面 | Vue 3 + TypeScript + Vite；UI 组件库用 Naive UI（内置深浅色主题、表格与对话框满足第 7 节需要）；状态管理用 Pinia |
| 工程 | Cargo workspace（`cc_core` + `src-tauri`），前端 `npm`（本机已有 Node 24）；测试 `cargo test`（核心库）+ `vitest`（前端纯逻辑）；打包 `tauri build` 产出便携单 exe（D-12） |
| 工具链前置 | 安装 rustup（stable-x86_64-pc-windows-msvc）与 Visual Studio Build Tools 的 C++ 桌面工作负载；Node 24 已有；WebView2 运行时 Win11 自带 |

第 5～9 节的设计（数据模型、模块接口、算法、UI 结构）与语言无关，接口签名以伪代码给出，实现时映射为 Rust 类型与 Tauri 命令。

## 5. 核心数据模型

```
DataRoot {
  root: Path                       // %USERPROFILE%\.claude 或 CLAUDE_CONFIG_DIR
  config_file: Path                // %USERPROFILE%\.claude.json（注意：在 root 之外）
  source: Enum { EnvVar, Default } // 告知 UI 数据根来源
}

ProjectState: Enum {
  Normal,          // 配置项存在 + 真实路径存在 + 数据目录存在
  ConfigOnly,      // 配置项存在 + 真实路径存在 + 无数据目录      【D-3】
  Orphan,          // 配置项存在 + 真实路径不存在（有无数据目录均算）
  Unreachable,     // 配置项存在 + 真实路径为网络位置且探测超时    【D-4】
  Unowned,         // 数据目录存在 + 无配置项
  LegacyEncoded,   // Unowned 的子类：目录名能被「旧编码规则」匹配到某个配置项  【D-2】
}

Project {
  id: String                       // 稳定标识：优先用配置 key（正斜杠真实路径）；Unowned 用编码目录名
  real_path: Option<Path>          // Unowned 无
  encoded_dir: Option<Path>        // ConfigOnly 无
  state: ProjectState
  running: Option<RunningSession>  // 由 guard 填充
  last_active: Option<Timestamp>   // 转录 .jsonl 最大 mtime（排除 memory/）
  config_hint: Option<ConfigHint>  // 来自 ~/.claude.json 条目：lastSessionId、hasTrustDialogAccepted、lastTotal*Tokens
  size: Option<SizeBreakdown>      // 扫描后填充
  usage: Option<UsageStat>         // 统计后填充
  memory_summary: Option<MemorySummary>  // 记忆文件数、MEMORY.md 行数
}

RunningSession { pid, session_id, cwd, started_at, status, verified: Enum { Alive, Stale, Unknown } }

Category: Enum {
  Transcripts, AutoMemory, FileHistory, PasteCache, Uploads, Debug, Plans, Tasks, SessionEnv,
  HistoryLog, StatsCache, Legacy(name), Protected(name), Unknown(name)
}
CategoryMeta { category, retention: Enum { Auto30d(days), Permanent, MemoryRule, LegacyRemoved, Unknown }, deletable_by_tool: bool, consequence_text }

SizeBreakdown { total_bytes, by_category: Map<Category, { bytes, file_count, oldest_mtime, newest_mtime }> }

UsageStat {
  input, output, cache_creation, cache_read: u64
  message_count, session_count
  scope_note: "现存转录的用量（受清扫策略影响），非历史总量"
  per_file_cache: Map<Path, { mtime, size, partial: UsageStat }>
}

EncodingSelfCheck {
  total_entries, matched_by_current_rule, matched_by_legacy_rule, unmatched: List<String>   // unmatched = 两种规则都无法解释的 projects/ 子目录
  migration_enabled: bool          // = unmatched.is_empty()
}

WriteOp: Enum { Purge(project), Migrate(from, to), DeleteCategory(project, category), ImportMemory(project, zip) }
PreflightResult: Enum { Allowed(token), Denied(reasons: List<DenyReason>) }
```

## 6. 核心模块设计

每个模块按「职责 / 接口 / 算法与规则 / 错误与边界 / 测试要点」描述。

### 6.1 `core.discovery`

**职责**：解析数据根目录；读取 `~/.claude.json`；实现编码函数及旧规则；匹配配置项与数据目录；判定项目状态；启动自校验。

**接口**

```
resolve_root() -> DataRoot
encode_path(real: &str) -> String                 // 当前规则
encode_path_legacy(real: &str) -> String          // 旧规则（保留下划线）
load_projects(root) -> (List<Project>, EncodingSelfCheck)
```

**算法与规则**

1. 路径规范化：把 `\` 替换为 `/`；不改变大小写（配置 key 大小写以文件为准）；除此之外不改动任何字符。
2. 当前编码规则（F3）：对规范化后的字符串逐字符处理，凡不在 `[A-Za-z0-9-]` 内的字符替换为 `-`。
3. 旧编码规则（F4）：同上，但 `_` 保留。
4. 匹配流程：对每个配置 key **同时**计算当前编码与旧编码。当前编码命中 `projects/` 目录 → 归属；旧编码（与当前编码不同时）命中的目录标记 `LegacyEncoded` 并记录其「疑似所属项目」——**无论当前编码是否命中都要做这一步**（实测 F4：同一项目可同时存在新目录与旧编码残留目录，M1 实现时曾因只在当前规则未命中时才试旧规则而误把残留目录判为无主，导致自校验失败）。当前编码未命中且真实路径存在 → `ConfigOnly`。剩余目录 → `Unowned`。真实路径不存在的配置项 → `Orphan`（优先级高于其他状态）。
5. 自校验：判定标准（**2026-09-22 M1 实现时定稿**）：`projects/` 下存在**当前规则与旧规则都无法解释**的目录 → `migration_enabled = false`。只被旧规则解释的目录（旧编码残留）**不**禁用迁移——它恰恰是迁移要修的对象，且其归属已经确定。理由：迁移会按当前规则生成新目录名，若出现两种规则都解释不了的目录，说明官方编码规则可能又变了，规则错一处就可能把数据挪到 CC 找不到的地方。判定结果与未匹配清单在 UI「诊断」页展示。
6. 真实路径存在性检查：对 UNC 路径与网络盘符（本机有 `Y:`、`Z:` 和 `//192.168.1.10/...`）设置 2 s 超时，超时视为 `Unreachable` 而非 `Orphan`，标签显示为「路径不可达」【D-4】。

**错误与边界**

- `~/.claude.json` 缺失或解析失败：项目列表只显示 `Unowned` 目录，所有写操作禁用，UI 顶部横幅提示。
- `projects` 键缺失：视为零条目。
- 配置 key 含非法路径字符：保留原样，不做任何猜测。

**测试要点**：用本机 38 条 key / 18 个目录做回归夹具（脱敏后写入测试数据）；覆盖中文路径、UNC、盘符根、下划线、空格、`.`；自校验在人为注入一个旧规则目录时应正确降级。

### 6.2 `core.guard`

**职责**：所有写操作的统一前置。白名单校验、运行中会话检测、自校验状态门禁、全局写互斥。

**接口**

```
preflight(op: WriteOp) -> PreflightResult
list_running_sessions(root) -> List<RunningSession>
is_project_running(project) -> Option<RunningSession>
```

**白名单（硬性，任何操作触碰即拒绝）**

需求 6.1 列表 + 实测补充【D-5】：

| 路径（相对数据根） | 来源 |
| --- | --- |
| `../.claude.json`（仅 Migrate 可改其 `projects` 键，且必须先备份） | 需求 |
| `.credentials.json`、`settings.json`、`settings.local.json`、`plugins/`、`agent-memory/`、`jobs/`、`daemon/` | 需求 |
| 项目源码目录中的 `.claude/settings.json`、`.claude/settings.local.json` | 需求 |
| `CLAUDE.md`（全局用户指令）、`memory/`（全局记忆）、`skills/`、`hooks/` | 实测补充，均为用户手写资产 |
| `sessions/`、`daemon*`、`*.lock`、`.last-cleanup`、`.last-update-result.json`、`policy-limits*`、`remote-settings.json`、`telemetry/`、`cache/`、`feedback/` | 实测补充：运行时状态，工具不理解其语义，只展示不动 |

实现：白名单以规范化绝对路径前缀集合表示；`DeleteCategory` 与 `ImportMemory` 展开为具体文件清单后逐条比对；`Purge` 与 `Migrate` 只校验其目标目录不落在白名单内（purge 的实际删除由 CLI 负责）。

**运行中会话检测（三重校验，回应 F6）**

1. 枚举 `sessions/*.json`，解析 `pid`、`cwd`、`procStart`、`sessionId`、`updatedAt`。解析失败的文件忽略。
2. 校验 PID 存活：Windows API 打开进程；失败 → `Stale`。
3. 校验映像名：进程主模块文件名须为 `claude.exe`、`node.exe` 或 `bun.exe` 之一；不符 → `Stale`（PID 被复用）。**实测补充（2026-09-22，M1 实现时发现）**：CC 自更新后会把仍在运行的二进制重命名为 `claude.exe.old.<时间戳>`，`QueryFullProcessImageNameW` 返回的就是重命名后的名字。因此比较前先把文件名截到第一个 `.exe`（大小写不敏感）再匹配；启动时间校验仍然生效，误判为 Alive 的风险可忽略。
4. 校验启动时间：`procStart` 与进程实际创建时间（FILETIME）比对，容差 2 s；不符 → `Stale`。步骤 3、4 任一无法获取（权限不足）→ `Unknown`，**按运行中处理**（保守）。
5. `cwd` 规范化后与项目真实路径比较：相等或为其子目录 → 该项目运行中。对 `Migrate`，源路径与目标路径都要查。此外任何 `Migrate` 要求**全局无任何 Alive/Unknown 会话**（需求 4.2 第 1 条：CC 退出时会重写 `~/.claude.json`）。
6. 工具**不删除**僵尸 session 文件（在白名单内），只在诊断页列出。

**测试要点**：伪造 sessions 目录 + 注入进程查询桩，覆盖 Alive/Stale(PID 不存在)/Stale(映像名不符)/Unknown 四种；白名单前缀边界（`plugins` vs `plugins-old`）。

### 6.3 `core.scan`

**职责**：分类空间统计、最近活跃时间、后台扫描与缓存、清扫模拟。

**接口**

```
scan(root, projects, prev: Option<ScanCache>, progress: Sink) -> ScanResult
simulate_cleanup(result, cleanup_days) -> CleanupPreview
```

**分类规则表**（第一版固定在代码中，未来可外置为配置）

| Category | 匹配规则 | 保留策略 | 工具可删 |
| --- | --- | --- | --- |
| Transcripts | `projects/<dir>/**/*.jsonl`、`projects/<dir>/**/subagents/**`、`projects/<dir>/**/tool-results/**`（**实测 2026-09-22**：子代理转录与工具结果嵌套在 `projects/<dir>/<sessionId>/` 之下，不在项目目录一级；分类按完整相对路径判断） | Auto30d | ✓（第 6.6 节方式）【D-6】 |
| Unknown（项目内） | `projects/<dir>/` 下不属于以上任何类别的文件，合并为**单一**类别 `projects/other`（界面显示为「项目内其他文件」）（不得按目录名逐个生成类别，避免随会话数无限增长） | Unknown | ✗ |
| AutoMemory | `projects/<dir>/memory/**` | MemoryRule | ✗（第一版不开放） |
| FileHistory | `file-history/**` | Auto30d | ✗（第一版仅展示） |
| PasteCache / Uploads / Debug / Plans / Tasks / SessionEnv | 同名顶层目录 | Auto30d | ✗（第一版仅展示） |
| HistoryLog | `history.jsonl` | Permanent | ✗ |
| StatsCache | `stats-cache.json` | Permanent | ✗ |
| Legacy(name) | `todos/`、`statsig/`、`logs/`、`image-cache/` | LegacyRemoved | ✗ |
| Protected(name) | 第 6.2 节白名单内的目录/文件 | — | ✗（展示体积，标「受保护」） |
| Unknown(name) | 其余任何顶层目录/文件 | Unknown | ✗（标「未知数据」） |

**算法**

- 顶层目录级增量：缓存中记录每个顶层目录的 `(mtime, 递归结果)`。注意 NTFS 目录 mtime 只反映**直接子项**变化，深层文件改动不会冒泡；因此对 `projects/` 采用二级增量（按每个编码目录的 mtime + 其 `.jsonl` 文件列表哈希），对其他顶层目录采用「mtime 未变且距上次全量扫描不超过 24 h → 复用」的策略【假设：可接受最多 24 h 的体积滞后】。
- 活跃时间：`max(mtime of projects/<dir>/*.jsonl, projects/<dir>/subagents/**/*.jsonl)`，显式排除 `memory/`；无转录时回退到 `~/.claude.json` 条目 `lastSessionId` 对应转录的 mtime，再回退为空。
- 清扫模拟：对 `Auto30d` 类别，列出 `mtime < now - cleanup_days` 的文件，汇总数量与字节数；`cleanup_days` 读 `settings.json`（全局）→ 无则 30（F7），UI 标注来源。
- 进度：按顶层目录推送 `(已完成目录, 总目录数, 当前累计字节)`。

**错误与边界**：无权限文件计入「无法访问」计数，不中断；符号链接/重解析点不跟随；扫描过程中文件消失忽略。

### 6.4 `core.usage`

**职责**：分项目 token 统计（解析转录 + 增量缓存）；读取 `stats-cache.json` 作全局口径。

**接口**

```
update_project_usage(project, cache: &mut UsageCache) -> UsageStat
read_global_stats(root) -> GlobalStats   // 直接映射 stats-cache.json，version 不认识时降级为「不可用」
```

**算法**

- 遍历项目目录下的 `*.jsonl`（含 `subagents/`）。缓存以 `(路径, mtime, size)` 为键；三者不变即复用。
- 逐行流式解析；仅处理 `type == "assistant"` 且 `message.usage` 存在的行。**按 `message.id` 去重**：同一 assistant 消息在流式写入时可能出现多行，usage 以该 `message.id` 最后一行为准【假设，需在实现时用真实转录验证】。
- 累加四字段：`input_tokens`、`output_tokens`、`cache_creation_input_tokens`、`cache_read_input_tokens`；其他键忽略。缺字段按 0。
- `session_count` = 转录文件数（不含 `subagents/`）；`message_count` = 去重后 assistant 消息数。
- UI 口径文案固定为：「现存转录的用量（转录受 N 天清扫），不是历史总量；全局总量见 stats-cache（截至 lastComputedDate）」。

**错误与边界**：某行 JSON 损坏 → 跳过并计数；文件被占用（CC 正在写）→ 本次跳过、缓存不更新。

### 6.5 `core.purge`

**职责**：探测 `claude` 可执行文件；dry-run 展示；执行 purge；错误分支。

**接口**

```
detect_cli() -> Result<CliInfo { path, version, kind: Enum { NpmCmd, NativeExe, Other } }, CliError>
dry_run(project) -> Result<DryRunReport { raw_text, item_count: Option<u32> }, PurgeError>
execute(project, token: PreflightToken) -> Result<PurgeOutcome { raw_text, exit_code }, PurgeError>
```

**规则**

- 探测顺序：`where claude` 的第一个 `.cmd`/`.exe` 结果 → `%USERPROFILE%\.local\bin\claude.exe` → 都无则禁用删除功能。对 `.cmd` 必须通过 `cmd.exe /c` 调用。记录 `claude --version` 供诊断页显示。
- 传给 CLI 的路径使用配置 key 的原始形式（正斜杠）；子进程继承当前环境变量（保证 `CLAUDE_CONFIG_DIR` 一致）。
- dry-run：`claude project purge <path> --dry-run`，原文展示（F1）。`item_count` 仅从末行 `Dry run: N item(s)` 正则提取用于按钮文案，提取失败不影响流程。
- 执行：`claude project purge <path> --yes`，超时 120 s。退出码非 0 → 展示 stderr 原文，并触发该项目重扫以反映实际状态。
- 流程顺序（对应需求 3.3）：`detect_cli` → `guard.preflight(Purge)` → 提示导出 auto memory（有 `memory/` 且非空时默认勾选）→ dry-run → 确认框（含 dry-run 原文、导出选项、「我已了解 purge 不可恢复」勾选）→ 执行 → 结果页。
- `ConfigOnly` 与 `Orphan` 项目也允许 purge（CLI 会删除配置项与 history 行）；`Unowned`/`LegacyEncoded` 无配置 key，**CLI 无法定位**，第一版不提供删除入口，只提供「打开数据目录」和「导出记忆」【D-2】。

### 6.6 `core.memory`

**职责**：记忆迁移/重绑定、导出/导入 zip、`~/.claude.json` 改写与备份、分类删除的文件级执行。

**接口**

```
migrate(project, new_real_path, token) -> Result<MigrateReport, MemoryError>
export_zip(project, dest: Path) -> Result<ExportReport, MemoryError>
inspect_zip(zip) -> ImportPlan                       // 列出将写入的文件与冲突
import_zip(zip, target_project, decisions: Map<Path, Overwrite|Skip>, token) -> Result<ImportReport>
delete_category(project, Category::Transcripts, token) -> Result<DeleteReport>   // 【D-6】
```

**迁移算法**（需求 4.2 的落地）

1. `guard.preflight(Migrate)`：全局无活跃会话；`migration_enabled == true`；目标路径存在且是目录；目标路径的配置 key 与编码目录**都不存在**（否则拒绝，避免覆盖；合并场景不在第一版）。
2. 备份 `~/.claude.json` 到工具备份目录，文件名带时间戳与操作 ID。
3. 计算 `new_dir = encode_path(new)`；`rename(projects/old_dir, projects/new_dir)`（同卷原子重命名）。
4. 读入 `~/.claude.json`（保序解析），把 `projects[old_key]` 移到 `projects[new_key]`，写到临时文件后原子替换；缩进 2 空格与原文件一致（F2）。
5. 第 3 步成功、第 4 步失败 → 自动回滚重命名；回滚也失败 → 结果页给出手工恢复指引（备份路径、两个目录名）。
6. 确认框文案明示：信任状态、MCP 设置随条目迁移；`history.jsonl` 中旧路径的 prompt 历史**不改写**【D-7】。
7. 操作记录写入工具日志（旧 key、新 key、备份路径、结果）。

**导出 zip 结构**（需求 4.3）

```
manifest.json    { tool_version, exported_at, source_real_path, encoded_dir, state, cc_version, contents: [...] }
auto-memory/     ← projects/<dir>/memory/ 全部
user-memory/     ← <real_path>/CLAUDE.md、CLAUDE.local.md、.claude/rules/**（路径存在时）
```

zip 内路径统一正斜杠；空目录不写；导出前对 `user-memory` 部分做白名单检查（`.claude/settings*.json` 明确排除）。

**导入**：先 `inspect_zip` 生成计划（每个目标文件：新建 / 冲突），UI 逐个决定覆盖或跳过（支持「全部覆盖」「全部跳过」）；`import_zip` 先 preflight，然后按计划写入；写入前对每个将被覆盖的文件做工具级备份。目标项目为 `Orphan` 时只允许导入 `auto-memory`。

**分类删除（转录）【D-6】**：需求 3.4 允许点删转录，但 CLI 没有「只删转录」的子命令，这一步无法沿用「只走官方 CLI」原则。设计为：展开文件清单 → preflight → 移入 **Windows 回收站**（`IFileOperation` / `SHFileOperation`，允许用户手工恢复）而非直接删除 → 记录日志。确认框列出文件数与总大小，并提示「将失去 resume/continue」。

## 7. UI 设计

### 7.1 窗口结构

单窗口，左侧或顶部四个视图切换：

1. **项目**：主列表 + 右侧详情面板。
2. **全局**：分类空间、清扫模拟、token 总量、快捷入口（`/insights` 报告、打开数据根）。
3. **记忆**：迁移向导、导出/导入。
4. **诊断**：数据根来源、CLI 探测结果、编码自校验结果与未匹配清单、运行中会话（含 Stale 列表）、工具缓存与备份目录、日志。

顶部常驻状态条：数据根路径、CLI 状态、自校验状态、后台扫描进度。

### 7.2 项目列表

- 列：状态标签、真实路径（Unowned 显示编码目录名并置灰）、最近活跃、空间总量、token（现存转录）、会话数。
- 默认按最近活跃降序；可切换按空间降序；筛选：全部 / 孤儿 / 无主 / 仅配置 / 运行中 / 长期未用且占用大（默认阈值 90 天且 ≥ 200 MB，可调）【D-8】。
- 行内操作按钮及禁用规则：

| 操作 | Normal | ConfigOnly | Orphan / Unreachable | Unowned / Legacy | 运行中 |
| --- | --- | --- | --- | --- | --- |
| 新会话 / 恢复会话 | ✓ | ✓ | ✗ | ✗ | ✓ |
| 打开数据目录 | ✓ | ✗（无目录） | ✓ | ✓ | ✓ |
| 打开项目路径 | ✓ | ✓ | ✗ | ✗ | ✓ |
| 导出记忆 | ✓ | ✗ | ✓（仅 auto） | ✓（仅 auto） | ✓ |
| 迁移 | ✓ | ✗ | Orphan ✓ / Unreachable ✗ | ✗（第一版） | ✗ |
| 删除（purge） | ✓ | ✓ | Orphan ✓ / Unreachable ✗ | ✗（第一版） | ✗ |

- 「运行 CC」实现：`Start-Process powershell -ArgumentList "-NoExit","-Command","Set-Location -LiteralPath '<path>'; claude"`（恢复会话追加 `--resume`），路径引号转义处理。

### 7.3 项目详情面板

- 分类占用条形/表格：转录、auto memory、其他；每类附「删除后果」说明与（仅转录）删除按钮。
- token 四项 + 口径提示；会话数、记忆文件数、`MEMORY.md` 行数（第一版只显示数字，不做 200 行预警逻辑）。
- 两类记忆分区展示：auto memory 文件列表（只读）；用户记忆文件存在性（`CLAUDE.md` / `CLAUDE.local.md` / `.claude/rules/` 各一行「存在/不存在」）。
- 明文风险提示固定显示在转录区域。

### 7.4 确认对话框统一规范

所有写操作共用一种对话框：标题写明操作与目标 → 中部展示**原文**（dry-run 输出 / 文件清单 / 迁移前后对照）→ 后果说明（固定文案）→ 需要用户勾选「我已了解」→ 主按钮文案包含数量（如「删除 3 项」）。默认焦点在取消。

### 7.5 语言与外观

第一版仅简体中文界面【D-9】；跟随系统深浅色。

## 8. 应用层与后台任务

- 任务类型：`FullScan`、`ProjectRescan(id)`、`UsageUpdate(id)`、`Export`、`Import`、`Migrate`、`Purge`、`DeleteCategory`。
- 调度规则：只读任务可并行（最多 2 个）；写任务串行且独占，写任务执行期间禁止启动新的只读任务，完成后自动触发相关项目的 `ProjectRescan`。
- 应用启动流程见 3.3；窗口关闭时若有写任务进行中，阻止关闭并提示。

## 9. 工具自身数据【D-10】

工具**永不**在数据根目录内创建文件（迁移的目录重命名与 `~/.claude.json` 改写除外）。自有数据放在 `%LOCALAPPDATA%\CCProjectManager\`：

```
settings.json          工具设置（阈值、窗口状态、上次数据根）
cache/scan.json        扫描缓存（含 schema_version）
cache/usage.json       token 增量缓存
backups/<ts>_<op>/     ~/.claude.json 备份、导入前被覆盖文件的备份
logs/ops.log           写操作审计日志（时间、操作、目标、结果、备份位置）
logs/app.log           运行日志（滚动，最多 5 × 5 MB）
```

缓存文件带 `schema_version`，版本不符直接丢弃重建。备份保留策略：保留最近 20 份或 30 天（取宽者），诊断页可一键打开目录。

## 10. 错误处理与防御性解析

- 所有外部数据（`~/.claude.json`、转录行、`stats-cache.json`、sessions 文件）解析时**未知字段忽略、缺失字段取默认、类型不符整条跳过并计数**；从不因单条数据异常中断整体流程。
- `stats-cache.json` 的 `version` 大于已知版本时，全局 token 区显示「格式未识别（version N）」而非猜测。
- 每个模块返回结构化错误（错误码 + 用户可读文案 + 技术细节），UI 展示前两者，第三项进日志。
- 写操作三段式记录：`begin` / `done` / `failed`，失败时日志写明已完成的步骤与备份位置，便于手工恢复。

## 11. 测试策略

- **核心库单元测试**：以「伪造数据根目录」为夹具（从本机脱敏采样生成，含中文、UNC、旧编码目录、僵尸 session、损坏 JSON 行），覆盖 discovery 状态判定、自校验降级、guard 白名单与三重校验、scan 分类与增量、usage 去重与增量、memory 迁移回滚与 zip 往返。
- **CLI 交互测试**：用桩脚本替代 `claude`，模拟 dry-run 正常输出、退出码 1、超时、stderr 乱码。
- **真实环境只读验收**：在本机真实数据根上跑只读功能，对照实测事实表 F1～F12 逐条核对。
- **破坏性操作人工清单**：purge、迁移、分类删除、导入各准备一个一次性测试项目，逐项验证备份产生、日志完整、失败回滚。
- 任何改动合入前，核心库测试必须全绿；UI 层以手工冒烟为主。

## 12. 决策记录（2026-09-22 已定）

需求方于 2026-09-22 确认：全部决策点按推荐选项执行。

| 编号 | 问题 | 曾考虑的选项 | **最终决定** |
| --- | --- | --- | --- |
| **D-1** | 技术栈 | A Tauri 2 + Rust 核心 + Vue/TS UI；B Python + PySide6；C Electron + TS | **A：Tauri 2**（落地细节见第 4 节） |
| **D-2** | 旧编码残留目录（如 `win_project` 变体）与无主数据的处置 | ① 只展示 + 导出记忆；② 合并到新目录；③ 重建 `~/.claude.json` 条目；④ 工具自行删除 | **①**；②③ 列为第二期候选 |
| **D-3** | 「仅配置无数据」态（本机 21/38 条）是否进主列表 | ① 进列表可 purge；② 单独入口；③ 不显示 | **①**，标签「仅配置」，可筛选隐藏 |
| **D-4** | 网络路径不可达（`Y:`/`Z:`/UNC 离线）是否按孤儿处理 | ① 单独标「不可达」；② 按孤儿处理 | **①**，禁用迁移与 purge，其他同 Normal |
| **D-5** | 白名单扩充范围 | 是否把 `CLAUDE.md`、根 `memory/`、`skills/`、`hooks/`、`sessions/`、`daemon*`、`cache/`、`telemetry/`、`feedback/` 列为受保护 | **全部受保护**（第 6.2 节表即为最终白名单） |
| **D-6** | 转录分类删除违背「只走 CLI」原则 | ① 移入回收站；② 第一版不做；③ 直接硬删 | **①**：工具自行删除，但移入 Windows 回收站 |
| **D-7** | 迁移时是否同步改写 `history.jsonl` 中的 `project` 字段 | ① 不改；② 改写 | **①** 不改，确认框中说明旧行残留 |
| **D-8** | 「长期未用且占用大」默认阈值 | 天数与体积阈值 | **90 天 / 200 MB**，可在设置调整 |
| **D-9** | 界面语言 | 仅中文 / 中英双语 | **仅简体中文** |
| **D-10** | 工具自身缓存与备份位置 | `%LOCALAPPDATA%` / exe 同目录 / 数据根内 | **`%LOCALAPPDATA%\CCProjectManager`** |
| **D-11** | 批量重绑定（整盘符 `D:`→`E:`）是否进第一版 | 进 / 不进 | **不进**，预留 `migrate_batch(rules)` 接口 |
| **D-12** | 分发形态 | 便携单 exe / NSIS 安装包 / 两者 | **便携单 exe** |
| **D-13** | `file-history` 归属项目的映射是否提前到第一版 | 提前 / 留第二期 | **留第二期**（数据源已确认为转录中的 `file-history-snapshot` 行） |

## 13. 假设与待验证项

| 编号 | 假设 | 验证方式 |
| --- | --- | --- |
| A-1 | 当前编码规则为「正斜杠化后，非 `[A-Za-z0-9-]` 字符逐个替换为 `-`」 | 本机 17/17 有目录条目匹配；实现后用自校验持续监控 |
| A-2 | 同一 assistant 消息在转录中可能多行出现，按 `message.id` 去重取最后一行 | **已于 2026-09-22 实测确认**：一份转录 22 行 assistant 对应 9 个 `message.id`，6 个 id 重复出现。实现时再抽样对比 `~/.claude.json` 条目 `lastTotal*Tokens` |
| A-3 | 运行中 CC 的宿主进程映像名为 `claude.exe`（原生）或 `node.exe`（npm） | **已于 2026-09-22 实测修正**：自更新后运行中的进程映像名为 `claude.exe.old.<时间戳>`，匹配前截到第一个 `.exe`（见 6.2 第 3 条）；`bun.exe` 作为兼容项 |
| A-4 | `~/.claude.json` 在 CC 未运行时不会被其他进程（daemon）改写 | 迁移前后比对文件 mtime；若 daemon 会改写，迁移需额外要求 daemon 未运行 |
| A-5 | 顶层目录体积最多 24 h 滞后可接受 | 与需求方确认（属于 D-8 类阈值） |
| A-6 | 大转录（数十 MB）流式解析在后台线程内数秒完成 | 用本机最大转录文件基准 |

## 14. 分阶段实施建议

| 阶段 | 内容 | 退出标准 |
| --- | --- | --- |
| M0 工程骨架 | 安装 rustup 与 VS Build Tools、Cargo workspace（`cc_core` + `src-tauri`）、Vue 前端壳、伪造数据根夹具 | `cargo test` 全绿、`tauri dev` 能打开空窗口 |
| M1 只读 | discovery + scan + usage + 诊断页 + 项目列表/详情 + 全局视图 | 在本机真实数据上核对 F1～F12 全部一致 |
| M2 purge | guard + purge 全流程 + 导出记忆 | 用一次性测试项目完成 dry-run → purge → 备份/日志校验 |
| M3 记忆 | 迁移 + 导入 + 分类删除（视 D-6） | 迁移后 CC 能在新路径 `--resume`，回滚路径验证通过 |
| M4 打磨 | 筛选、阈值设置、深浅色、便携打包 | 便携 exe 在干净 Win11 上可运行 |

M1 完成即可日常使用（看清）；M2、M3 的安全策略已由 D-2～D-7 定死，实施时不得偏离。
