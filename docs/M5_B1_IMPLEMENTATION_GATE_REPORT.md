# M5-B1 百度搭子本地 MCP 实施报告

## 范围与提交

- 隔离分支：`codex/m5-b1-local-mcp`；实现：`69a8bd922051d9b2f791530b02d80063ba16e4e2`。
- 原始基线：`993deb8ca0085f79d1273568f686c02d36d6960e`；M5-B0 本地归档 `e6542b6` 保留在历史中。未推送，未修改仓库可见性或正式发布配置。
- 原有未跟踪 `docs/M5_B1_DESIGN_REVIEW.md` 内容未改动，没有纳入实施提交。
- 实施仅提供隔离预览中的 `send_reading`、`suggest_todos`。不提供用户数据读取、直接写 Todo、系统操作、第三个工具、ChatGPT 接入或 Remote Gateway。

| 修改区域 | 文件和内容 |
| --- | --- |
| Schema 与兼容性 | `src-tauri/migrations/0011_local_mcp_du_mate.sql`；`src-tauri/src/persistence/sqlite.rs`：真实旧库升级、外键、历史收据测试 |
| Runtime 与传输 | `src-tauri/src/agent_bridge/{mod,server,mcp_http,dumate,clipboard,dumate_http_tests}.rs`、`src-tauri/src/lib.rs`、`src-tauri/Cargo.toml`：同一监听器、独立凭据、原生复制、协议及 TCP 回归 |
| 复用与来源 | `src-tauri/src/codex_delivery/{mod,mcp,credentials}.rs`、`src-tauri/src/domain/proposal/todo_batch.rs`、`src-tauri/src/persistence/proposal_repository/todo_batches.rs`：复用契约、事务和系统凭据实现；来源来自服务端/持久化注册表 |
| 设置与验证 | `src/application/ports/todoSuggestions.ts`、`src/features/agentBridge/{AgentBridgeSettings,DuMatePairingSettings,DuMatePairingSettings.test}.tsx`、`src/features/sticky/{TodoSuggestionTray,StickyHome.test}.tsx`、`src/infrastructure/tauri/dumatePairing.ts` |

## A — 0011 Migration

使用真实 0001–0010 建立三组隔离数据库：Codex grant active、revoked、无 grant。前两组各写入 pending、accepted、rejected 批次（包括已创建 Todo 与收据）；第三组验证没有 grant 时不合成批次，且可首次授权。每组都通过 0011 升级。

| 比较项 | 结果 |
| --- | --- |
| Batch ID、`dedupe_namespace`、`request_digest`、`items_json` | 迁移前后原值逐项相等 |
| `status`、`created_at`、`updated_at`、`accepted_todo_ids_json` | 迁移前后原值逐项相等 |
| 已有 Todo、排列位置及 accepted 收据 | 保留；重放返回原 Todo ID，不重复创建 |
| 两个独立来源及旧 Codex 首次/恢复授权 | Codex grant 首次写入与来源注册在同一事务；恢复保留同一命名空间，旧状态与外键约束未放松 |
| 空库迁移 / 升级后 `PRAGMA foreign_key_check` | 通过；空库达到 0011，升级未发现外键问题 |

0001–0010 没有变动。0011 保留原唯一键 `(dedupe_namespace, request_digest)`，Todo 批次改为引用独立来源注册表。撤销凭据不会删除来源、历史批次或已接受的 Todo。

## B–D — 鉴权、协议与用户操作

- `/mcp` 不新增端口，只在独立 audit 数据目录且显式启用 `AGENT_DESK_ENABLE_DUMATE_PREVIEW=1` 的进程提供。沿用 `127.0.0.1` IPv4 监听；原 OpenClaw REST、Codex STDIO/REST 请求及绑定行为不变。
- 百度搭子一个随机 Bearer、两项独立授权，固定 30 分钟，不因请求续期。DB 存 SHA-256 摘要；明文使用 Windows Credential Manager / macOS Keychain 单独条目。扩大授权或显式轮换会使旧凭据失效；收窄不延长有效期。到期后拒绝新调用。
- 鉴权及实际提交共享串行闸门。撤销、轮换和停用成功返回前等待已准入的提交结束；关闭后旧监听器不再准入。自然到期按准入时刻判定，已经准入的事务可以完成。应用不记录凭据或请求正文。
- 验证 Host 和 Origin，拒绝转发头与有歧义的重复头及冲突帧；有 HTTP 总体积限制和业务参数限制。Streamable HTTP 使用 JSON 响应，已鉴权 GET 返回 405（未提供 SSE），通知不执行工具。支持 2025-03-26、2025-06-18、2025-11-25，工具按权限发现。参见 [MCP Streamable HTTP 规范](https://modelcontextprotocol.io/specification/2025-11-25/basic/transports)。
- 两项工具必须提供幂等键。同键同内容复用已提交收据，内容变化返回冲突，跨 Agent 命名空间隔离。阅读通过既有 DeliveryService 保留 Markdown 并标记「百度搭子」；待办通过 ProposalService 建立 pending 批次，按已有规则排序，仅用户确认可原子加入正式 Todo。
- 设置提供按项授权、轮换、撤销、到期时间、Bridge 端口以及 `Agent-Desk-DuMate` 配置说明。Windows 通过原生剪贴板，不把凭据传给 React；macOS 使用显式复制时的一次性 WebView 回退，不将凭据放入组件状态、DOM 或本地持久存储。
- 百度搭子保存的自定义请求头是否云同步仍未知。设置明确提示外部应用未必只在本机保存配置、剪贴板可能被同步或保留历史；只允许合成内容和牺牲性授权，测试后要撤销并移除连接。

## E — 自动化、构建与运行证据

| 验证 | 本轮结果 |
| --- | --- |
| Windows 前端 | PASS：Node 22.23.2 / pnpm 11.16.0；format、typecheck、lint、104 项测试及 production build |
| Windows Rust | PASS：MSVC x64，locked/offline；66 项通过，Credential Manager 合成条目的写入、读取和删除另跑 1 项通过；Clippy `-D warnings` 通过 |
| 本地真实 TCP | PASS：initialize/协商、initialized、按授权发现、两个真实调用和提交收据；401/403、GET 405、413、错误头、chunked、重复及冲突、过期/轮换/撤销、停用/重启和并发竞态。检查提交前后状态、来源、稳定排序及「确认前零正式 Todo」 |
| Codex / OpenClaw | PASS（自动化）：Codex 两个工具的既有发现/schema、旧 REST 认证及收据/事务；OpenClaw health、既有业务及直写工作区拒绝；未实际调用外部客户端 |
| Inbox / Reader / Sticky | PASS（服务层、前端自动化和 Windows 原生隔离运行）：持久化来源、从 Inbox 打开文档、完整 Markdown、预览来源/排序及显式确认 |
| Windows / macOS GitHub Actions | 本次提交未运行。现有 workflow 由 push / pull request 触发；本次未授权推送或发布。Windows 本地验证不替代远端 CI，macOS 的 Keychain、413、复制与构建仍需独立 runner |
| 百度搭子真实业务调用（实施阶段） | 实施阶段未测试；见下方独立的真实客户端验收。M5-B0 不作为 M5-B1 业务验收。 |

### Truth Build

- Windows 非打包 release：`pnpm tauri build --no-bundle -- --locked`，使用新的隔离 target。
- Truth `agent-desk.exe`：17,043,968 bytes，SHA-256 `060469810C5D42B9B16D2010FE2EEF7DE296B372FCD14B6313A6A5BC7BD05BD8`。
- 前端包：`index-TPvXpUvK.js` SHA-256 `713C0074DA5E4357506AE9EC548EAAB6FA8B51588E18C975575235B514712B11`；`index-DEiiSrJc.css` SHA-256 `93C5454917986CA29A4F36201E843B9ECFA19A36F2D550DEC2458E3369D9B978`。
- 正式程序仅只读核对，SHA-256 `0C9E13094F57C54445FCC7D43D79A764D6A99BC325F577C10AA21DA18B946792`；未覆盖。未读取或改写正式 AppData。

### Windows Native Smoke（独立应用标识）

- 正式实例当时在运行，因此未启动具有相同单实例标识的 Truth exe。另用同一实现提交和同一前端包构建 `agent-desk-b1-smoke.exe`，只覆盖本次构建所用的应用标识、产品名和窗口标题/二进制名称；不修改仓库发布配置。其 SHA-256 为 `130CC794E597F27DEB8FB6AE77D65938AFD86A2BA28D280FF2B1E30AC89C45F3`（17,044,480 bytes）。
- SQLite、Store、WebView2 和端口均隔离。该 exe 在空 profile 实际启动并迁移至 0011。独立 Node HTTP 客户端凭合成授权完成初始化（2025-06-18）、通知（202）、工具发现、GET 405 和错误 Bearer 401。测试使用隔离数据库中仅有摘要的随机合成 grant；原生设置授权/剪贴板动作未在本次 GUI smoke 中执行，不用它替代独立的系统凭据存储往返测试。
- `send_reading` 提交 Delivery `delivery_b8eeb4206effbec6179c183baca133c4` / Document `reader_2208be92149badad9c9b7340be58f944`；`suggest_todos` 提交 Batch `todo_batch_16ef8593228f400480045f23d626106f`。两项重试返回相同 ID，没有新增文章或批次；确认前正式 Todo 数为 0。
- 原生 Inbox 显示未读的合成报告和「百度搭子」；点击该条目后 Reader 显示来源和 Markdown 标题、加粗正文与列表。Expanded Sticky 显示「待确认建议 · 1」；展开为「百度搭子 建议」，按高、中、低排列。点「确认加入」后出现「已加入 3 项」，SQLite 记录了同一批次的 accepted 收据和 3 个唯一 Todo ID；重启后同一 Reader 文档、阅读状态、批次和 Todo 均保留。
- 已结束所有本轮 smoke 进程，未留下使用测试 profile 的 WebView2 子进程，临时端口已关闭，隔离凭据摘要已清除。未结束或操作正式 Agent Desk 实例。

## 真实百度搭子验收 — 本地预检

启动前核对：分支 `codex/m5-b1-local-mcp`，HEAD `cc64c7a`，含实施提交 `69a8bd9`；原有未跟踪设计报告仍保留，无其他改动。Windows Truth 和独立 smoke 二进制的大小与 SHA-256 均与上文吻合。未启动原应用标识的 Truth exe。

| 证据类别 | 结果 |
| --- | --- |
| 自动化复核 | PASS：重新执行真实 0010 → 0011 升级、MCP 权限/幂等、撤销及停用竞争测试。Windows Credential Manager 随机合成条目的写入、读取、删除也再次通过；未读取正式凭据。 |
| 独立 MCP 客户端与原生隔离进程 | PASS：实际启动独立应用标识的 smoke exe；空数据库内部记录 0001–0011，外键检查为空，初始 Bridge 不监听。启用测试 fixture 后，操作系统确认唯一监听为 `127.0.0.1` 上的隔离临时端口。独立 Node 客户端完成 initialize（2025-06-18）、initialized（202）、发现两个工具并验证幂等键必填；GET 405，错误 Bearer 401。阅读独占和待办独占分别拒绝另一工具，过期和撤销 fixture 均返回 401。停止进程后端口不可达，重启后仍有效的同一测试凭据可用。 |
| 真实百度搭子个人版 | 预检未调用真实客户端；另见下方业务验收。外部应用可能保存或云同步自定义 `Authorization`，尚不能确认其存储边界。M5-B0 的客户端版本与 Echo 结果不计入此次业务验收。 |
| GitHub Actions Windows/macOS | NOT_RUN：未推送。 |

独立预检使用仅存摘要的合成数据库 fixture 测试权限和人为构造的过期状态，不等同于设置页签发、真实等待到期或外部客户端调用。没有写入业务数据；测试 profile 与进程已删除/停止，临时端口不再可用，未创建百度搭子连接器。正式 AppData、Codex/OpenClaw 授权未访问或写入，正式 exe 的 SHA-256 前后相同。

### 用户授权后的真实客户端准备

- 用户已明确同意本次验收使用独立环境、合成数据和 30 分钟临时凭据，并知晓外部 `Authorization` 可能被保存或同步。
- 已使用上文 SHA-256 匹配的独立 smoke exe 启动可见的 `Agent Desk B1 Smoke` 原生窗口。新隔离 SQLite 为 0011，外键检查通过，初始 Bridge 为 inactive，未签发凭据，Delivery／Todo 建议／正式 Todo 均为 0。本次使用独立 Store、WebView2 profile 和单独的临时端口。
- 用户在本机启用隔离 Bridge 并配置连接器。验收侧未读取或共享 Bearer 值，操作系统确认只监听隔离的 IPv4 loopback 端口；两项授权使用同一个有效期为 1,800 秒的凭据。运行中的百度搭子版本经 Windows 进程文件元数据核对为 1.0.93（ProductVersion 1.0.93.0）。

### 真实百度搭子个人版 — 业务回执与服务端核对

用户提供真实 `Agent-Desk-DuMate` 原始回执；独立只读查询隔离 SQLite 核对 ID、请求键摘要、来源、内容和正式 Todo 计数。这些不是 Node 测试客户端产生的写入。

| 请求 | 客户端回执 | 隔离服务端证据 |
| --- | --- | --- |
| `send_reading` / `b1-real-reading-v1`，同参调用两次 | 第一次 `created:true`，第二次 `created:false`；同一 Delivery `delivery_348cd8424040576e208054907cfc7ff9` 和 Document `reader_b758f4865ec0bb4ca8f9ebf32702243f` | 仅 1 条投递；报告已打开，来源「百度搭子」，保存的 Markdown 与测试参数完全一致。 |
| `suggest_todos` / `b1-real-reject-v1` | `created:true`、`pending`，Batch `todo_batch_3e0268e8a714466c9fe299e4ac331618` | 来源 `dumate`，三条均按 high → medium → low 存储。 |
| `suggest_todos` / `b1-real-accept-v1` | `created:true`、`pending`，Batch `todo_batch_24ef1475e9bc4ae9abc38b530b68635d` | 独立批次、同一可信来源；确认或拒绝前检查两批均 pending，正式 Todo 数为 0。 |

用户在隔离原生界面拒绝第一批后，服务端查询确认该批为 `rejected`、无已创建 Todo ID；第二批仍为 `pending`，正式 Todo 计数仍为 0，外键检查通过。

用户在隔离原生界面确认第二批后，服务端查询确认该批为 `accepted`，只生成 3 个不同的正式 Todo，按确认清单顺序保存到位置 0、1、2；第一批仍为 `rejected`，未产生任何 Todo。只重启隔离 Agent Desk 进程，确认停机期间端口拒绝连接；恢复后仍监听原 loopback 端口，原授权的代次和到期时间不变，打开状态、两批状态及 3 个 Todo ID 均持久化。

重启后，真实百度搭子使用原连接和相同的 `b1-real-accept-v1` 返回原批次 ID、`created:false`、`status:accepted`。再次对照隔离库：仍是 1 条投递、2 个批次、同一组 3 条正式 Todo，位置和 ID 不变，外键检查通过。真实重连、确认后幂等回放通过。

仅阅读权限验证通过：用户把现有授权收窄为 reading=1、todo=0，代次与到期时间均不变。真实搭子对 `suggest_todos` 收到 `forbidden_capability`，同一连接的 `send_reading` 仍可回放原收据；隔离 SQLite 中记录数、状态、正式 Todo ID 与位置均未改变。

经用户另行授权，真实搭子把既有文章的摘要投递到隔离 Inbox。按演示幂等键只读查询，确认 1 条投递、来源「百度搭子」、已在 Reader 打开；未把标题、正文或文档 ID 纳入报告。隔离库保留 3 条验收 Todo，另有 2 条不属于该验收批次的隔离 Todo；未读取文字或认定其来源。

自然到期验证通过：原凭据超出 30 分钟有效期，授权代次与监听实例未变化。真实百度搭子沿用配置调用两次，均报告客户端 transport 的 HTTP `401`；客户端连接器显示已授权不能代表本机凭据仍有效。实际客户端沿用演示摘要参数而不是合成报告内容，且使用了合成报告的键；因此这里只将结果用于凭据到期验收，不算同参幂等回放。该路径在应用解析工具参数前就校验凭据。隔离库仍为 2 条投递、相同两个批次和原有 5 条 Todo，没有记录变化。

仅待办权限验证通过：隔离库显示 reading=0、todo=1，新授权有效期为 1,800 秒。真实百度搭子发现列表只有 `suggest_todos`，`send_reading` 不可用；用原 `b1-real-accept-v1` 得到相同 Batch ID、`created:false`、`status:accepted`。这是已确认批次的回放，不需要再次确认。数据库仍为两条投递、两批、五条 Todo，外键检查通过。独立 HTTP 探针在新授权有效时向同一原生端口发送明确无效的合成 Bearer，收到 `401`，未传递真实凭据。

显式撤销验证通过：用户在隔离设置撤销后，真实百度搭子沿用原连接器及原 `suggest_todos` 请求得到 HTTP `401`。隔离 Bridge 当时仍在线；两项权限均关闭，摘要及到期值清空，授权代次由 3 变为 4。两条投递、两个批次和五条 Todo 均未增加，已确认收据仍保留。仅检查本轮独立 Windows Credential Manager 条目的存在性，确认该条目已不存在；未访问正式凭据。

### 清理与最终状态

- 已停止隔离验收实例，临时端口拒绝连接且无此 profile 的 WebView2 子进程。用户确认已删除百度搭子临时连接器 `Agent-Desk-DuMate`；未通过自动化读取或改动其他连接器，也无法核验第三方曾同步的副本，但对应凭据已撤销。
- 经用户明确同意，在核对绝对路径、撤销状态和进程归属后，已删除本轮独立 SQLite、Store 和 WebView2 profile，包含演示摘要和待办的本地隔离副本。删除后验证该目录已不存在；未删除原文章、正式 AppData、原有 Codex/OpenClaw 授权或构建产物。
- 正式 exe 与 Windows Truth SHA-256 前后一致。未 push，未运行本次提交的 Windows/macOS Hosted CI，未替换正式程序。Windows 真实客户端原生验收通过，产品 Gate 仍待评审。

```text
M5_B1_DUMATE_NATIVE_GATE: PASS
M5_B1_HOSTED_CI: NOT_RUN
M5_B1_PRODUCT_GATE: AWAITING_REVIEW
CANONICAL_REPLACEMENT: NOT_AUTHORIZED
```

## 安全检查、待完成与回滚

- 改动路径扫描未发现凭据模式、Windows 用户绝对路径、用户数据库或构建产物。新测试仅使用合成文本/凭据；不提交原始请求日志或用户数据。
- 生产替换条件尚未具备：Windows 真实百度搭子业务与授权验收已完成，仍需 Windows/macOS Hosted CI 和产品与技术审批。macOS CI 不能证明真实 macOS 百度搭子已测试。
- 无需修改旧 schema 即可回滚启用状态：停止 Bridge、撤销百度搭子连接，并用关闭预览的同 schema 构建运行。若降级到不认识 0011 的旧二进制，应使用隔离或经批准的升级前备份；不得在用户 DB 上删除迁移记录、删批次或覆盖升级后的数据。

```text
M5_B1_PHASE_A_DESIGN_GATE: PASS_WITH_CONDITIONS
M5_B1_IMPLEMENTATION_STATUS: IMPLEMENTED_PENDING_GATE
M5_B1_PRODUCT_GATE_STATUS: AWAITING_REVIEW
DUMATE_REAL_CLIENT_STATUS: PASS
CANONICAL_REPLACEMENT_STATUS: NOT_AUTHORIZED
CHATGPT_INTEGRATION_STATUS: NOT_STARTED
```
