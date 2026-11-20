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
| 百度搭子 1.0.93 真实业务调用 | NOT_TESTED：未获外部 Header 配置许可，未接触百度搭子配置；M5-B0 不作为 M5-B1 业务验收 |

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

## 安全检查、待完成与回滚

- 改动路径扫描未发现凭据模式、Windows 用户绝对路径、用户数据库或构建产物。新测试仅使用合成文本/凭据；不提交原始请求日志或用户数据。
- 生产替换条件尚未具备：需真实百度搭子测试许可和业务验收、Windows/macOS CI、产品与技术审批。macOS CI 不能证明真实 macOS 百度搭子已测试。
- 无需修改旧 schema 即可回滚启用状态：停止 Bridge、撤销百度搭子连接，并用关闭预览的同 schema 构建运行。若降级到不认识 0011 的旧二进制，应使用隔离或经批准的升级前备份；不得在用户 DB 上删除迁移记录、删批次或覆盖升级后的数据。

```text
M5_B1_PHASE_A_DESIGN_GATE: PASS_WITH_CONDITIONS
M5_B1_IMPLEMENTATION_STATUS: IMPLEMENTED_PENDING_GATE
M5_B1_PRODUCT_GATE_STATUS: AWAITING_REVIEW
DUMATE_REAL_CLIENT_STATUS: NOT_TESTED
CANONICAL_REPLACEMENT_STATUS: NOT_AUTHORIZED
CHATGPT_INTEGRATION_STATUS: NOT_STARTED
```
