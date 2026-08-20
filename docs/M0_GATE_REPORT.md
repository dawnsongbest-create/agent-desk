# Agent Desk — M0 Architecture & Execution Gate Report

**阶段：** M0 — Architecture & Foundation Planning
**角色：** Implementation Engineer
**日期：** 2026-08-10
**状态：** Tech Lead 已于 2026-08-10 评审通过并附带修正；详见 ADR-0001

## 0. Executive decision

Agent Desk 应采用“React 负责展示与本地交互、Rust/Tauri 负责可信边界与后台能力、SQLite 负责关键持久状态”的分层架构。外部 Agent 的任何输入必须先经过 Adapter 与 Agent Gateway，转换并校验为 `UnifiedAgentEvent`，再由 Card Engine 事务性地产生 Card、Delivery 和通知意图。UI 只认识 Agent 身份与统一领域类型，不认识 Claude、OpenAI、MCP 或 A2A 等 provider/protocol 类型。

Progressive Reading Skill 是阅读计划与进度推进的唯一权威。桌面端只保存 Reading Packet 的不可变快照、`local_read_position`、本地完成动作的投递状态，以及 Skill 返回的确认投影；绝不生成 Reading Unit、重切 Packet、改写正文或直接写 `confirmed_cursor`。

MVP 的 Generic Local / HTTP Adapter 明确定义为：**仅绑定 loopback 的本地 HTTP ingress，加上对已配置本地 Agent endpoint 的动作回传**。它不是 Remote Cloud Relay，也不接受 Event 内携带的任意 callback URL。未来远程 Relay、MCP、A2A 和 provider adapter 通过同一 `AgentAdapter` 边界增加。

## 0.1 Tech Lead amendments (authoritative)

以下修正覆盖本报告原始表述，完整决定记录在 `docs/adr/0001-m0-tech-lead-decisions.md`：

- M0 状态为 `PASS_WITH_AMENDMENTS`，无需重做。
- Scheduled Tasks 不是永久 V0.1 非目标。Experiment MVP 可依赖已配置的外部计划；V0.1 Complete 必须提供 Agent-owned 的最小计划列表、创建、修改时间、暂停和恢复界面。Desktop 始终不是 Scheduler source of truth。
- M3 通知 spike 的未知项是不同 Windows/macOS 生命周期下能否从 activation data 精确恢复 `card_id`，而不是回调 API 是否存在。Windows 至少一次使用安装后的 packaged app。
- E2E 使用当前 WebdriverIO Tauri service；不能基于旧版 `tauri-driver` 限制宣称 macOS E2E 不可用。
- M1 分成 M1-A Foundation 与 M1-B Sticky Product 两个独立 Gate；本次只执行 M1-A。
- M1 开发默认值：`Agent Desk`、workspace `agent-desk`、bundle id `com.agentdesk.desktop`、320×420、最小 300×360、follow-system、Sticky Home、close-to-tray、tray Quit，并在首个 baseline commit 前将分支改为 `main`。
- Tauri 官方 single-instance plugin 必须最先注册；第二次启动只唤起既有实例且不绕过正常状态恢复。

## 1. Input authority and conflict resolution

本报告完整阅读并以以下顺序解释输入：

1. `Agent Desk — M0 Architecture & Execution Planning`：当前执行与 Gate 的控制文档。
2. `Desktop Agent Window PRD`：产品体验、信息架构和长期方向。
3. `Progressive Reading Skill v1.0`：阅读数据、游标与原文保真契约。

发生范围差异时，以本轮 M0 控制文档为准：

- PRD 把 Local Adapter 与 Remote Adapter 都列为 P0；本轮明确排除 Remote Cloud Relay，因此 V0.1 只做 loopback HTTP Adapter。
- Scheduled Tasks 采用双里程碑：Experiment MVP 可依赖已配置的外部 Agent 计划；V0.1 Complete 必须交付 Agent-owned 的最小计划 surface。Desktop 只维护 projection，通过 capability-gated outbound intents 请求 Agent 创建/修改/暂停/恢复。
- PRD 的 Learning Enhancement 是正文后的第二层能力；本轮仅保留动作扩展点，不实现知识点、金句、Quiz、思考题或长摘要。
- “完成阅读”不是桌面端直接推进 Skill 状态。桌面端发出幂等 `CONFIRM_PACKET` action，只有 Agent/Skill 确认后，本地投影才进入 `confirmed`。

---

# A. Repo Audit

## A1. Current state

审计工作区：`%USERPROFILE%\Documents\ChatGPT\电子阅读便利贴`

- repo 是 **greenfield / effectively empty**。
- 只有 `.git/`；没有 tracked files，没有 commit，当前分支为 `master`。
- 没有 `package.json`、`Cargo.toml`、Tauri scaffold、源码、测试、CI、lint/format 配置、数据库或 migration。
- 没有 `AGENTS.md`、`.agents/AGENTS.md` 或 `.codex/AGENTS.md` 项目约束。
- 没有既有依赖或用户代码需要兼容，也没有可运行的测试基线。
- 本轮唯一新增物是本 M0 报告；未初始化应用、未安装依赖、未进入 M1。

## A2. Audit consequence

- M1 必须先建立可复现 scaffold、锁文件、格式化、lint、测试和 Windows/macOS CI，再开始 Sticky 功能。
- 默认分支名称、包管理器、应用 identifier、签名主体和发布渠道仍需 Tech Lead/Product Owner 在 M1 前确认；它们不阻塞本报告中的模块设计。
- 因无历史 schema，初始 SQLite migration 可以从 `0001` 建立，但从第一天起必须只追加 migration，不能编辑已发布 migration。

---

# B. Proposed Architecture

## B1. Runtime boundaries

```text
External Agent
    │ raw provider/local payload
    ▼
Adapter Layer (Rust, provider/transport specific)
    │ normalize only
    ▼
Agent Gateway (Rust, schema validation + idempotency + transaction)
    ├── Delivery Repository ───────────────┐
    ├── Card Factory / Card Repository ────┼── SQLite
    ├── Action Outbox ─────────────────────┤
    └── Notification Intent ───────────────┘
                         │ committed domain event
                         ▼
Tauri IPC application commands/events
                         │
                         ▼
React application layer
    ├── Sticky view
    ├── Inbox view
    └── Focus Renderer

Progressive Reading Skill owns:
Reading Map + Units + delivery_cursor + confirmed_cursor

Agent Desk owns:
presentation + local interaction + cached packet + local_read_position
+ CONFIRM_PACKET outbox/receipt projection
```

关键顺序为：**validate → idempotency claim → persist delivery/card → commit → notify UI/system**。通知失败不能回滚已接收的 Delivery；数据库失败则不能先显示通知。

## B2. Proposed directory structure

```text
/
├── docs/
│   ├── M0_GATE_REPORT.md
│   └── adr/                         # M1 起记录不可逆技术决策
├── contracts/
│   ├── unified-agent-event.schema.json
│   ├── fixtures/
│   │   ├── valid/
│   │   └── invalid/
│   └── README.md                    # wire compatibility/version policy
├── src/                             # React/TypeScript presentation process
│   ├── app/
│   │   ├── App.tsx
│   │   ├── navigation.ts            # Sticky/Inbox/Focus finite states
│   │   └── query-client.ts
│   ├── domain/
│   │   ├── cards/
│   │   ├── agent-events/
│   │   └── reading/
│   ├── application/
│   │   ├── ports/                   # TS-facing command contracts
│   │   └── use-cases/
│   ├── infrastructure/tauri/        # one IPC boundary; no scattered invoke()
│   ├── features/
│   │   ├── sticky/
│   │   ├── inbox/
│   │   ├── reader/
│   │   └── settings/
│   ├── components/                  # non-domain UI primitives only
│   ├── styles/                      # tokens, themes, typography
│   └── test/
├── src-tauri/
│   ├── capabilities/                # least-privilege per window
│   ├── migrations/                  # append-only SQLite migrations
│   ├── src/
│   │   ├── lib.rs
│   │   ├── commands/                # narrow application commands
│   │   ├── domain/
│   │   │   ├── card.rs
│   │   │   ├── agent_event.rs
│   │   │   └── reading.rs
│   │   ├── application/
│   │   │   ├── ingest_delivery.rs
│   │   │   ├── execute_card_action.rs
│   │   │   └── update_read_position.rs
│   │   ├── adapters/
│   │   │   ├── mod.rs
│   │   │   └── generic_http.rs
│   │   ├── gateway/
│   │   ├── persistence/
│   │   │   ├── sqlite.rs
│   │   │   └── repositories/
│   │   ├── credentials/
│   │   ├── notifications/
│   │   └── window/
│   └── tests/
├── e2e/                             # WebdriverIO Tauri flows
├── package.json
├── pnpm-lock.yaml
├── vite.config.ts
└── .github/workflows/
```

## B3. Module responsibilities

| Module | Owns | Must not own |
|---|---|---|
| Desktop Shell / Window Manager | single instance, tray, show/hide/quit, always-on-top, size/position restore, theme bridge | cards, Agent protocol, reading cursors |
| Card Engine | typed Card lifecycle, payload registry, actions, placements, Card factory | provider parsing, Reader typography, Reading Unit planning |
| Sticky | Note/Task editing, completion, due date, reorder view | arbitrary SQL, window persistence, Agent networking |
| Agent Inbox | delivery list, unread/opened/archive interactions | provider-specific fields or adapter status logic |
| Focus Renderer | deterministic `ContentBlock` rendering, progress, resume, completion CTA | source rewriting, Packet segmentation, `confirmed_cursor` mutation |
| Notification Manager | permission state, native notification after committed ingestion, notification-to-card route | delivery persistence or unread source of truth |
| Agent Gateway | schema/version validation, normalization boundary, idempotency, card creation transaction, action routing | provider UI, source text generation |
| Adapter Layer | transport lifecycle, raw payload mapping, connection health, outbound action delivery | writing cards directly, React state, generic domain policy |
| Local Persistence | migrations, repositories, transactions, recovery | credentials in plaintext, network behavior |
| Secure Credentials | create/read/delete secrets by opaque `credential_ref` | returning secrets to React, ordinary preferences |

## B4. Dependency direction rules

1. `features → application ports → infrastructure/tauri`; React components never call `invoke`, SQL or HTTP directly.
2. Rust `adapters → gateway input port`; adapters cannot call Card repositories.
3. `gateway/application → domain + repository ports`; persistence implements ports.
4. Card Engine knows registered `CardType` and `CardAction`, but not reading planning rules.
5. Reading payload may use generic `ContentBlock`; generic Card code may not inspect `book_id`, anchors or Packet status.
6. Persistent repositories are authoritative. React Query is a refetchable cache, never a second durable source.
7. Network and credential operations stay in Rust; Tauri capabilities expose only narrow commands to the bundled `main` webview.

## B5. Navigation/state machine

```text
Sticky ── OPEN_INBOX ──> Inbox ── OPEN_CARD(cardId) ──> Focus(cardId)
  ▲                         │                              │
  └──── ESC / CLOSE ────────┴──── ESC / CLOSE / DONE ─────┘
```

- Startup and any invalid/deleted focus target resolve to `Sticky`.
- Notification click supplies only a `card_id`; the app loads it from SQLite, marks it opened transactionally, then enters Focus.
- Closing the OS window hides to tray. Explicit tray `Quit Agent Desk` terminates.
- Navigation is an explicit reducer/state machine, not URL routing and not three independent booleans.

---

# C. Technology Decisions

Version policy: M1 should install the latest mutually compatible stable releases within the selected major lines and commit both `pnpm-lock.yaml` and `Cargo.lock`. This report intentionally does not invent exact patch pins before a scaffold exists. Dependency updates require CI on Windows and macOS.

| Decision | Why | Alternative considered | Main risk / mitigation |
|---|---|---|---|
| Tauri 2 + Rust | mandated; native tray/window/notification/security boundary with a small desktop shell | Electron | WebView behavior differs by OS; add per-OS smoke and avoid browser-only assumptions |
| React + TypeScript + Vite | mandated React/TS; Vite is Tauri’s lean frontend build path | Next.js, CRA | no SSR benefit; keep renderer a static SPA and pin toolchain in lockfile |
| pnpm | deterministic lockfile, efficient local/CI installs | npm | contributor availability; declare `packageManager` and use Corepack |
| Rust application services behind narrow Tauri commands | keeps DB, network and credentials out of the webview; enforces trusted boundary | call official SQL/HTTP plugins directly from React | more Rust code; offset with repositories, command contract tests and thin commands |
| `sqlx` + bundled SQLite + embedded append-only migrations | async pool, transactions, portable SQLite, migration support | `@tauri-apps/plugin-sql`, `rusqlite` | binary/build cost and migration hash issues; pin version, force SQL files to LF, test upgrade fixtures |
| Tauri Store plugin for noncritical preferences only | simple atomic-ish KV for theme/onboarding/shortcut preferences | put all preferences in SQLite | JSON corruption/last-write semantics; defaults + validation + never store domain records here |
| Tauri Window State plugin for geometry | official cross-platform save/restore of size and position | hand-written per-OS storage | stale/off-screen coordinates; clamp to active monitor and save only intended flags |
| OS native secure stores via `keyring-core` + platform store crates | guarantees Windows Credential Manager and macOS Keychain; secrets never enter SQLite/React | Stronghold, encrypted JSON, environment variables | keychain prompts/locked store/API churn; conditional crates, opaque refs, denial/error tests |
| `serde`/`serde_json` in Rust and `zod` at TS IPC boundary | typed decoding plus runtime rejection of malformed/version-incompatible events | TypeScript compile-time types only | mirrored contract drift; golden valid/invalid fixtures must pass in Rust and TS |
| Canonical checked-in JSON Schema for the wire envelope | language/provider-neutral contract and reviewable compatibility | generate TS from Rust with Specta | schema/code drift; CI fixture suite and explicit schema-version rules |
| TanStack Query as renderer cache | consistent async load/mutation/refetch around IPC and push invalidation | Redux/Zustand/global context | accidental second source of truth; no persistence plugin, refetch after committed mutations |
| `dnd-kit` sortable primitives | keyboard-capable task reorder and collision control | HTML5 Drag and Drop, custom pointer logic | touch/desktop edge cases; persistence-on-drop tests and accessible keyboard path |
| Typed deterministic `ContentBlock` renderer | supports headings, paragraphs, lists, quotes, code, tables and images without HTML injection or source rewriting | render arbitrary HTML/Markdown with `dangerouslySetInnerHTML` | schema expansion; unknown blocks get safe unsupported UI, never silent text mutation |
| `axum` loopback server + `reqwest` action client | generic local HTTP bridge with request limits and explicit lifecycle | filesystem polling, raw TCP, remote WebSocket relay | port/auth/firewall/SSRF risk; loopback-only bind, bearer secret, endpoint allowlist, body limit |
| Tauri Notification plugin, with an M3 native activation feasibility spike | current APIs expose action listeners and notification-associated data | browser Notification API only; immediately write custom Windows/macOS notification integration | prove exact `card_id` recovery for warm/hidden/cold lifecycle on both OSes; use an installed Windows package, verify no duplicate Card and safe stale-target fallback |
| Vitest + Testing Library + Rust tests + current WebdriverIO Tauri service | fast domain/component tests plus supported Windows/macOS desktop E2E | Playwright-only browser E2E; direct legacy `tauri-driver` assumptions | native automation can still be flaky; keep critical native E2E small and do not infer current macOS support from older driver limitations |
| CSS custom properties + scoped CSS, native `Intl` | small custom “paper/card” UI without a design-system dependency | Tailwind, component framework, date library | style drift; central tokens/typography fixtures and Product Owner visual acceptance |
| `tracing` with redaction and rotating local logs | diagnosable adapter/persistence failures without logging content/secrets | console logs | privacy leakage; field allowlist, payload/hash only, retention cap |

Security defaults:

- CSP denies remote scripts and unsafe eval; remote images are not granted blanket filesystem/network privilege.
- Tauri capabilities are explicit for the `main` window; no wildcard remote capability.
- The localhost adapter binds only `127.0.0.1`/`::1`, authenticates every write, caps body size, validates media types and rate-limits failures.
- Adapter configs store endpoint metadata in SQLite; bearer tokens/API secrets live only in OS credentials under an opaque `credential_ref`.
- Event actions use `action_ref`, never an arbitrary URL from an event. Adapter configuration resolves the action destination against an allowlist.

Current official references checked during M0:

- [Tauri capabilities and security boundaries](https://v2.tauri.app/security/capabilities/)
- [Tauri Window State plugin](https://v2.tauri.app/plugin/window-state/)
- [Tauri Notifications plugin](https://v2.tauri.app/plugin/notification/)
- [Tauri WebDriver testing](https://v2.tauri.app/develop/tests/webdriver/)
- [SQLx embedded migrations](https://docs.rs/sqlx/latest/sqlx/macro.migrate.html)
- [Rust keyring native secure stores](https://docs.rs/keyring/latest/keyring/)
- [Tauri distribution/signing overview](https://v2.tauri.app/distribute/)

---

# D. Domain Schemas

All timestamps are RFC 3339 UTC instants. Date-only due dates use `YYYY-MM-DD`. IDs are opaque strings; app-created IDs should be UUIDv7 where available. JSON fields use `snake_case` on the wire. Unknown schema major versions are rejected; unknown optional fields in the same major are ignored and preserved only where explicitly stated.

## D1. Card model

```ts
type CardType = "note" | "task" | "reading" | "agent_message";

type CardLifecycle = "active" | "completed" | "archived" | "deleted";
type AttentionState = "unread" | "read" | null;

type CardSource =
  | { kind: "user" }
  | {
      kind: "agent";
      agentId: string;
      agentName: string;
      deliveryId: string;
    };

type CardAction = {
  id: string;
  intent: "COMPLETE_TASK" | "CONFIRM_PACKET" | "ARCHIVE" | string;
  label: string;
  actionRef?: string;                // opaque; never a URL
  availability: "enabled" | "disabled" | "pending" | "succeeded" | "failed";
  requiresConfirmation: boolean;
  metadata: Record<string, JsonValue>;
};

type Card<TType extends CardType, TPayload> = {
  schemaVersion: 1;
  id: string;
  type: TType;
  title: string | null;
  status: {
    lifecycle: CardLifecycle;
    attention: AttentionState;
  };
  source: CardSource;
  payload: TPayload;
  actions: CardAction[];
  metadata: Record<string, JsonValue>; // bounded, non-secret, non-authoritative extensions
  createdAt: string;
  updatedAt: string;
  completedAt: string | null;
  archivedAt: string | null;
  deletedAt: string | null;
};

type NoteCard = Card<"note", { text: string }>;

type TaskCard = Card<"task", {
  text: string;
  dueDate: string | null;
}>;

type AgentMessageCard = Card<"agent_message", {
  summary: string | null;
  content: ContentBlock[];
}>;
```

Design notes:

- `status.lifecycle` and `status.attention` are orthogonal; a Note is not forced into an Inbox-only `UNREAD` state.
- Sticky ordering is a `card_placements` concern, not embedded in every Card.
- Type-specific data remains a discriminated payload. Adding `ReportCard` registers a new payload/factory/renderer without changing existing Card persistence or lifecycle.
- `metadata` is never queried for core behavior and cannot contain credentials or provider response objects.

## D2. Structured content and source fidelity

```ts
type SourceRef = {
  anchorId: string;
  sourcePage?: number;
  blockIndex?: number;
  charStart?: number;
  charEnd?: number;
};

type ContentBlock =
  | { id: string; kind: "heading"; level: 1 | 2 | 3 | 4 | 5 | 6; text: string; source: SourceRef }
  | { id: string; kind: "paragraph"; text: string; source: SourceRef }
  | { id: string; kind: "quote"; text: string; citation?: string; source: SourceRef }
  | { id: string; kind: "list"; ordered: boolean; items: ContentBlock[][]; source: SourceRef }
  | { id: string; kind: "code"; code: string; language?: string; source: SourceRef }
  | { id: string; kind: "table"; columns: string[]; rows: string[][]; source: SourceRef }
  | { id: string; kind: "image"; assetRef: string; alt: string; caption?: string; source: SourceRef };
```

- Renderer maps typed fields to React text nodes/elements. It does not accept executable HTML.
- Every source block has a stable Packet-provided anchor. Exact text/code/table cell strings are treated as source data and must not be “cleaned up” by AI.
- M4 contract tests compare a canonicalized concatenation/hash against Packet fidelity fields. Failure creates a rejected delivery/error card, not altered source.
- Image `assetRef` is adapter-resolved/cache-controlled; it is not passed to unrestricted browser fetch.

## D3. ReadingCard payload

```ts
type ReadingCardPayload = {
  packetId: string;
  bookId: string;
  bookTitle: string;
  author: string | null;
  chapterTitle: string | null;
  sectionTitle: string | null;
  partLabel: string | null;
  unitIds: string[];
  startAnchor: string;
  endAnchor: string;
  estimatedMinutes: number;
  progressBefore: number;             // 0..1, Skill-provided snapshot
  progressAfter: number;              // 0..1, if this Packet is confirmed
  sourceTextSha256: string;
  provenance: "SOURCE_EXTRACTED";
  blocks: ContentBlock[];
  completion: {
    state: "not_completed" | "confirmation_pending" | "confirmed" | "failed";
    confirmActionId: string;
    requestedAt: string | null;
    confirmedAt: string | null;
    lastErrorCode: string | null;
  };
};

type ReadingCard = Card<"reading", ReadingCardPayload>;
```

`completion.state` is a local delivery/receipt projection, not a replacement for Skill `confirmed_cursor`. `confirmed` requires an acknowledgement correlated to the action invocation; an offline click becomes `confirmation_pending` in the outbox.

## D4. UnifiedAgentEvent

```ts
type UnifiedAgentEvent = {
  schemaVersion: "1.0";
  eventId: string;
  eventType: "delivery.created" | "delivery.updated" | "action.acknowledged";
  contentType: "reading" | "message" | "brief" | "report" | string;
  agent: {
    id: string;
    name: string;
    instanceId: string;
    skillId: string | null;
  };
  occurredAt: string;
  expiresAt: string | null;
  correlationId: string | null;
  idempotency: {
    key: string;
    scope: "agent_instance" | "agent";
  };
  title: string;
  summary: string | null;
  payload: Record<string, JsonValue>; // validated again by contentType factory
  actions: Array<{
    id: string;
    intent: string;
    label: string;
    actionRef: string;
    requiresConfirmation: boolean;
    inputSchema: Record<string, JsonValue> | null;
  }>;
  metadata: Record<string, JsonValue>;
};
```

Gateway-added receipt fields such as `received_at`, `adapter_instance_id`, validation result and raw hash are **not producer-controlled fields** and live in the Delivery record.

Validation policy:

1. enforce content type, UTF-8, body/array/string size limits and RFC 3339 timestamps;
2. reject unsupported schema major;
3. validate the event envelope, then the content-specific payload;
4. acquire unique idempotency claim before Card creation;
5. treat same key + same canonical hash as successful duplicate/no-op;
6. treat same key + different hash as conflict/security error;
7. never pass raw provider objects through `metadata` to UI.

## D5. AgentAdapter contract

The production boundary is a Rust trait because adapter lifecycle, transport and credentials must continue while the webview is hidden and must not expose secrets to React.

```rust
#[async_trait]
pub trait AgentAdapter: Send + Sync {
    fn kind(&self) -> &'static str;
    fn instance_id(&self) -> &AdapterInstanceId;

    async fn start(
        &self,
        config: AdapterConfig,
        sink: Arc<dyn RawAgentEventSink>,
    ) -> Result<(), AdapterError>;

    async fn stop(&self) -> Result<(), AdapterError>;
    async fn health(&self) -> AdapterHealth;

    async fn invoke_action(
        &self,
        action: OutboundAgentAction,
    ) -> Result<ActionReceipt, AdapterError>;
}
```

Responsibilities:

- establish/stop exactly one transport instance and report `connecting/connected/degraded/disconnected/error`;
- authenticate using a secret resolved by `credential_ref` inside Rust;
- map transport/provider fields into a raw candidate for the Gateway;
- forward events only to `RawAgentEventSink`; never persist Cards directly;
- deliver an outbox action using configured endpoints and return a correlated receipt;
- implement cancellation, bounded retry/backoff and clean reconnect.

Non-responsibilities:

- no UI state, notification, Card status policy, SQLite schema knowledge or Progressive Reading cursor mutation;
- no arbitrary Event-supplied callback URL;
- no assumption that all adapters support scheduling, files or structured delivery.

## D6. local_read_position

```ts
type LocalReadPosition = {
  readingCardId: string;
  packetId: string;
  contentHash: string;
  anchorId: string;
  blockId: string;
  blockOffset: number;                 // UTF-16 text offset where meaningful
  intraBlockRatio: number;             // 0..1 fallback within block
  documentScrollRatio: number;         // 0..1 last-resort fallback only
  updatedAt: string;
};
```

Recovery order is `contentHash match + blockId` → `anchorId` → nearest preceding known anchor → clamped `documentScrollRatio` → top. Saving is debounced during scroll and flushed on window blur, Focus exit and app shutdown.

The three cursors remain separate:

| State | Meaning | Authority | Advances when |
|---|---|---|---|
| `delivery_cursor` | Skill has sent source through which anchor | Progressive Reading Skill | Skill successfully records delivery |
| `confirmed_cursor` | user has confirmed completion through which anchor | Progressive Reading Skill | Skill handles valid `CONFIRM_PACKET` |
| `local_read_position` | viewport location inside one delivered Packet | Agent Desk SQLite | local Reader scroll/visibility changes |

No mapping from scroll percentage to `confirmed_cursor` is permitted.

---

# E. Persistence Plan

## E1. Storage allocation

| Data | Store | Reason / rule |
|---|---|---|
| Notes | SQLite | user-authored, transactional, queryable, restart-critical |
| Tasks, completion, due dates | SQLite | atomic mutation with Card lifecycle and order |
| Cards and placements | SQLite | shared domain source of truth across Sticky/Inbox/Focus |
| Valid deliveries and idempotency claims | SQLite | exactly-once Card creation and restart recovery |
| Rejected delivery diagnostics | SQLite, bounded/redacted | preserve error code/hash without exposing unlimited raw payload |
| Reading Packet snapshot | SQLite JSON/text columns | offline Reader requires exact delivered content; no regeneration |
| `local_read_position` | SQLite | per-card transactional durable state |
| completion action outbox/receipt | SQLite | offline/restart-safe `CONFIRM_PACKET` |
| Window size/position | Tauri Window State file | OS-aware geometry; only intended state flags |
| always-on-top, appearance, shortcut, onboarding | lightweight Tauri Store | small non-relational preferences with defaults |
| Agent configuration/capabilities | SQLite | queryable, migratable; contains only opaque `credential_ref` |
| Agent connection runtime | memory | live transport health is inherently runtime state |
| last connection snapshot | SQLite | diagnostic/UI fallback only; never claims current connectivity |
| scheduled-task representation | SQLite cached projection | external Agent/Scheduler is authoritative in MVP |
| API tokens, loopback bearer secret | OS credential store | never plaintext DB/KV/log/IPC |
| cached images/large assets | app data files + SQLite manifest | avoid DB blobs; validate hash/size/MIME and use controlled asset path |

## E2. Initial logical tables

```text
cards
  id PK, type, title, lifecycle, attention,
  source_kind, source_agent_id, source_delivery_id,
  metadata_json, created_at, updated_at, completed_at, archived_at, deleted_at

note_cards
  card_id PK/FK, text

task_cards
  card_id PK/FK, text, due_date

reading_cards
  card_id PK/FK, packet_id UNIQUE, book_id, book_title, author,
  chapter_title, section_title, part_label, unit_ids_json,
  start_anchor, end_anchor, estimated_minutes,
  progress_before, progress_after, source_text_sha256,
  blocks_json, completion_state, confirm_action_id,
  completion_requested_at, completion_confirmed_at, last_error_code

agent_message_cards
  card_id PK/FK, summary, blocks_json

card_actions
  id PK, card_id FK, intent, label, action_ref,
  availability, requires_confirmation, metadata_json

card_placements
  surface, card_id FK, rank, PRIMARY KEY(surface, card_id), UNIQUE(surface, rank)

deliveries
  id PK, adapter_instance_id, event_id, agent_id, agent_instance_id,
  idempotency_scope, idempotency_key, canonical_hash,
  schema_version, event_type, content_type, occurred_at, received_at,
  event_json, status, card_id FK NULL,
  UNIQUE(adapter_instance_id, idempotency_scope, idempotency_key)

delivery_rejections
  id PK, adapter_instance_id, received_at, payload_hash,
  error_code, safe_details, sample_truncated, purge_after

read_positions
  reading_card_id PK/FK, packet_id, content_hash, anchor_id,
  block_id, block_offset, intra_block_ratio, document_scroll_ratio, updated_at

action_outbox
  invocation_id PK, card_id FK, adapter_instance_id, action_id,
  intent, payload_json, idempotency_key UNIQUE, status,
  attempt_count, next_attempt_at, created_at, updated_at, last_error_code

agent_configs
  id PK, adapter_kind, display_name, endpoint_config_json,
  credential_ref, enabled, created_at, updated_at

agent_capabilities
  agent_config_id FK, capability, value_json, observed_at

agent_connection_snapshots
  agent_config_id PK/FK, last_status, last_seen_at, last_error_code

scheduled_task_projections
  external_task_id PK, agent_id, title, schedule_summary,
  status, next_delivery_at, payload_json, observed_at

asset_manifest
  asset_ref PK, relative_path, sha256, mime_type, byte_size, status, created_at
```

Important constraints:

- SQLite foreign keys enabled; WAL mode used after platform verification; busy timeout configured.
- Domain writes use transactions. Reorder updates all affected ranks atomically.
- `CHECK` constraints cover enum values, progress/rations `0..1`, positive estimates and valid completion transitions where practical.
- Packet `blocks_json` is immutable after accepted ingestion except through an explicit schema migration; Reader position and completion are separate columns/tables.
- Deleted Card is a tombstone in MVP to protect references and enable recovery diagnostics; UI delete is immediate. Purge is a later maintenance policy.
- Raw credentials and full HTTP headers are never persisted.

## E3. Migration and recovery policy

1. Migration filenames are monotonic, immutable and LF-normalized, e.g. `0001_initial.sql`.
2. `sqlx::migrate!` embeds migrations in the binary and runs them before adapters, tray receiving, UI queries or notifications start.
3. App startup order: open DB → integrity/open check → backup/restore decision → migrate → validate KV → start repositories → restore window → start enabled adapters → drain outbox.
4. A migration failure stops adapter ingestion and opens a recoverable error surface. It never creates a fresh empty DB over the user’s file.
5. Before destructive migrations, create a versioned local backup and enforce free-space checks. Migrations are forward-only in production; rollback means restore backup plus previous binary.
6. Keep fixture databases for every released schema. CI opens each fixture with the new binary, migrates, and verifies critical records.
7. Corrupted DB behavior: close connections, preserve the corrupt file with timestamp, attempt read-only integrity diagnosis, offer restore/export; never silently discard Notes/Tasks.
8. KV values have independent `preferences_schema_version`, typed defaults and validation. Invalid individual values reset to defaults with a diagnostic, not an app-wide crash.

## E4. State ownership

| State | Source of Truth | Renderer behavior |
|---|---|---|
| window position/size | Window State plugin file + current native window | observes/commands Window Manager; no duplicate React persistence |
| always-on-top/theme/shortcut | validated Tauri Store preference; native window is applied projection | optimistic UI only if native apply succeeds; then persist |
| sticky content | SQLite Card/type tables | React Query cache invalidated after transaction |
| todo ordering | SQLite `card_placements.rank` | drag preview is ephemeral; drop transaction decides final order |
| unread deliveries | SQLite `cards.attention` | badge is a query result, never a standalone counter |
| card state | SQLite | one Card query/mutation path for all surfaces |
| `local_read_position` | SQLite `read_positions` | debounced local draft is flushed; stored row wins after restart |
| reading completion | Skill `confirmed_cursor`; SQLite holds local action/outbox/ack projection | show pending offline; show confirmed only after correlated ack |
| Agent connection | live Adapter runtime | SQLite last snapshot is labelled stale/last seen, not current truth |
| scheduled-task representation | external Agent/Scheduler | SQLite is explicitly a cached projection; no unsupported local edits |

## E5. Scheduled Tasks future contract (deferred from Experiment MVP, required for V0.1 Complete)

The Desktop queries and displays an Agent-owned projection. It does not run or reconcile a local schedule. Capability discovery gates each control:

```text
scheduling.read
scheduling.write
scheduling.natural_language
```

Future outbound intents are `CREATE_SCHEDULED_TASK`, `CREATE_SCHEDULED_TASK_FROM_TEXT`, `UPDATE_SCHEDULED_TASK`, `PAUSE_SCHEDULED_TASK` and `RESUME_SCHEDULED_TASK`. Each request follows the same durable outbox/idempotent acknowledgement pattern as other Agent actions. If capability discovery does not advertise the required operation, its UI is unavailable rather than simulated locally.

---

# F. Gate-by-Gate Implementation Plan

Gate discipline: each gate starts only after the previous Gate Report is approved. A task is complete only when its acceptance criteria and listed tests pass. File paths are proposed and may be refined without changing module boundaries.

## M1 — Desktop Foundation + Sticky (two sub-gates)

- **M1-A Foundation:** M1.1–M1.3 plus only the preference and minimal visual-shell subset of M1.6. No final Note/Todo UX.
- **M1-B Sticky Product:** M1.4–M1.5 and remaining product-quality M1.6 work. It starts only after M1-A approval.

| ID | Goal | Files/modules likely touched | Acceptance criteria | Tests | Dependencies |
|---|---|---|---|---|---|
| M1.1 | Bootstrap reproducible Tauri/React/TS workspace and quality gates | `package.json`, lockfiles, `src/`, `src-tauri/`, configs, CI | clean Windows/macOS checkout installs, typechecks, lints, tests and builds; no product feature yet | scaffold smoke; CI matrix build | M0 approval; app identifier/package names decided |
| M1.2 | Establish SQLite, repositories, migrations and preference/credential ports | `src-tauri/migrations`, `persistence/`, `credentials/`, TS ports | migration `0001` creates constraints; repository CRUD transaction works; secrets cannot be read via frontend IPC | Rust repository tests on temp DB; migration fixture; credential mock contract | M1.1 |
| M1.3 | Implement native shell/window lifecycle | `window/`, `lib.rs`, `capabilities/`, tray assets/config | resizable/draggable; always-on-top toggle; close hides; tray show/quit; one instance; position/size restored and clamped | Rust/mock tests where possible; Windows/macOS manual smoke; restart | M1.1, preference ports |
| M1.4 | Build Sticky Home and unified Note/Task capture/edit/delete/complete | `domain/cards`, `features/sticky`, commands/repositories | default Home is Sticky; text creates Note; `[ ]` or explicit Task creates Task; edits survive restart; completion/delete correct | Card unit tests; component input/edit tests; DB integration; E2E capture/restart | M1.2–M1.3 |
| M1.5 | Add due dates and durable accessible drag reorder | `task_cards`, `card_placements`, Sticky DnD components | date-only due date persists; mouse and keyboard reorder; drop is atomic; cancelled drag makes no write | rank/reorder property tests; component keyboard DnD; restart E2E | M1.4, dnd-kit |
| M1.6 | Add appearance and foundational visual/accessibility QA | `styles/`, theme preference, shared UI primitives | light/dark/follow-system; 320×420 remains usable; keyboard focus/contrast; quiet paper/card direction accepted | theme unit/component; Windows/macOS visual/manual smoke; Product Owner review | M1.3–M1.5 |

**M1 exit:** Product Owner can use Sticky daily on Windows and macOS with restart-safe Notes/Tasks; no Inbox/Reader/Gateway production path is required yet.

## M2 — Inbox + Focus Reader with mock delivery

| ID | Goal | Files/modules likely touched | Acceptance criteria | Tests | Dependencies |
|---|---|---|---|---|---|
| M2.1 | Finalize Card registry and mock delivery fixtures | `domain/cards`, `contracts/fixtures`, mock ingestion command | mock reading/message deliveries deterministically create typed Cards; unknown type yields safe unsupported card | factory unit tests; fixture contract tests | M1 exit |
| M2.2 | Implement Inbox and unread/open/archive lifecycle | `features/inbox`, Card queries/commands | Inbox groups deliveries by date; unread badge derives from DB; open/read/archive persist; it is not a chat timeline | query/repository integration; component state tests; restart E2E | M2.1 |
| M2.3 | Implement Focus navigation and transition shell | `app/navigation`, Focus container, animations | Sticky↔Inbox→Focus and ESC/close→Sticky are deterministic; invalid/deleted ID falls back safely | reducer unit tests; keyboard/component tests; E2E path | M2.2 |
| M2.4 | Build deterministic typography renderer | `features/reader/renderers`, styles, safe asset resolver | all required heading/paragraph/list/quote/code/table/image blocks render; no raw HTML execution; unknown blocks visible but nonfatal | per-block snapshot/semantic tests; malicious payload tests; Product Owner typography review | M2.1, M2.3 |
| M2.5 | Persist and restore `local_read_position` | `reading` domain, `read_positions`, observer hook/command | position saves with anchor/hash, flushes on exit, resumes after restart, handles changed/missing anchor via defined fallback | algorithm unit tests; SQLite integration; long-document scroll/restart E2E | M2.4 |
| M2.6 | Implement local completion interaction with mock ack | ReadingCard action UI, action state command | CTA appears only at content end; click is idempotent; mock ack marks projection confirmed; returns to Sticky; does not expose/update cursor fields | action transition unit; double-click integration; E2E complete path | M2.5 |
| M2.7 | Validate long content and offline cached flow | reader performance fixtures, asset/error/empty states | large Packet opens/resumes without unacceptable input/scroll stalls; cached content works with network disabled; missing image degrades clearly | performance budget test; offline E2E; manual Windows/macOS smoke | M2.4–M2.6 |

**M2 exit:** With mock events, the complete Sticky → Inbox → Reader → Sticky flow works, including unread and durable Reader position.

## M3 — Agent Gateway

| ID | Goal | Files/modules likely touched | Acceptance criteria | Tests | Dependencies |
|---|---|---|---|---|---|
| M3.1 | Check in versioned `UnifiedAgentEvent` contract and dual-runtime validators | `contracts/`, TS/Rust event domain | valid fixtures accepted identically; malformed/oversized/unsupported-major rejected with stable error codes | Rust+TS golden contract suite; fuzz/property tests for bounds | M2 exit |
| M3.2 | Implement Gateway ingestion transaction and idempotency | `gateway/`, `ingest_delivery`, repositories | one valid event creates exactly one Delivery/Card; exact duplicate no-op; key/hash conflict rejected; commit precedes UI/notification events | concurrent duplicate integration; transaction failure injection; restart replay | M3.1, SQLite repositories |
| M3.3 | Implement `AgentAdapter` runtime registry and health model | `adapters/mod.rs`, app runtime, connection snapshots | adapters start/stop independently; status transitions observable; one failure does not crash shell; secrets remain Rust-only | trait fake unit tests; lifecycle/reconnect integration | M3.1 |
| M3.4 | Implement Generic loopback HTTP Adapter | `adapters/generic_http.rs`, credential/config commands, capabilities | loopback-only ingress accepts authenticated schema-valid POST; rejects non-loopback config, missing token, bad type/size; clean reconnect | HTTP integration with real ephemeral port; auth/rate/body tests; lost connection test | M3.2–M3.3, OS credential port |
| M3.5 | Implement durable outbound action outbox | `execute_card_action`, `action_outbox`, adapter action path | `CONFIRM_PACKET`/generic actions enqueue atomically, retry with bounded backoff, survive restart, and correlate ack; same idempotency key never invokes twice after ack | offline/reconnect/restart integration; duplicate click; poison action handling | M3.3–M3.4 |
| M3.6 | Spike, then implement native notification activation after successful ingestion | `notifications/`, route-to-card handling, tray/window, possible per-OS Rust module | prove exact `card_id` activation for Windows installed app and macOS app in warm/hidden/cold states; activation cannot create Delivery/Card; stale/deleted target opens Inbox/Sticky; arrival never steals focus; any fallback/deviation returns to Gate review | permission fake; commit/notify order; duplicate activation; Windows packaged and macOS cold/warm/hidden manual/automated evidence | M3.2, M1 shell; spike result is a hard sub-gate |
| M3.7 | Prove provider independence | dependency/contract tests across UI and Card factory | UI bundle/domain contains no provider response types/branches; two differently shaped fake providers normalize to same view | architecture/static test; adapter equivalence fixtures; E2E delivery | M3.1–M3.6 |

**M3 exit:** A local Agent can securely deliver a unified event into a provider-agnostic Inbox/Reader and receive a durable action response.

## M4 — Progressive Reading Integration

| ID | Goal | Files/modules likely touched | Acceptance criteria | Tests | Dependencies |
|---|---|---|---|---|---|
| M4.1 | Freeze Reading Packet → ReadingCard mapping contract with Skill owner | reading contract/fixtures, mapping ADR | required identity, anchors, blocks, hash, progress and confirm action agreed; missing fidelity fields fail closed | valid/invalid Skill fixture tests; schema compatibility | M3 exit; Tech Lead + Skill owner contract review |
| M4.2 | Ingest exact source Packet without reinterpretation | Gateway reading factory, asset manifest | accepted Packet persists exact structured source and hash; no LLM/rewrite/segmentation path exists; figure/table/code dependencies preserved or clearly unsupported | byte/string fidelity tests; hash mismatch rejection; table/image/code fixtures | M4.1 |
| M4.3 | Integrate notification/open/resume for real Packet | Gateway, Reader, read position | real Packet notifies, opens correct ReadingCard, and resumes local anchor across restart; local position never modifies Skill cursor fields | full flow integration; restart E2E; long Packet | M4.2, M2 reader |
| M4.4 | Integrate `CONFIRM_PACKET` handshake | outbox, Generic adapter action mapping, ReadingCard projection | explicit click queues packet ID/action ref; offline shows pending; correlated Skill ack marks confirmed; double action is idempotent; no direct cursor write exists | fake Skill integration; disconnect/reconnect; duplicate/late/wrong ack tests | M4.1, M3.5 |
| M4.5 | Enforce no-backlog/current-Packet behavior in presentation | Inbox/Reader projections | repeated reminder/update for waiting Packet does not create duplicate reading body; reopen shows same stored Packet; new distinct Packet remains a Skill decision | update/idempotency fixtures; reopen hash equality; integration | M4.2–M4.4 |
| M4.6 | Produce integration Gate evidence | test fixtures, trace/redaction checks, Gate Report | end-to-end evidence covers Reading Packet → Gateway → Card → notification → Reader → local position → confirm ack; logs contain no source body/token | Windows/macOS manual smoke; privacy log audit; full automated suite | M4.1–M4.5 |

**M4 exit:** Existing Progressive Reading Skill works through the desktop surface while all five Skill invariants remain intact.

## M5 — MVP Hardening

| ID | Goal | Files/modules likely touched | Acceptance criteria | Tests | Dependencies |
|---|---|---|---|---|---|
| M5.1 | Harden restart/crash and migration recovery | startup, persistence, migrations, fixtures | Notes/Tasks/Cards/position/outbox recover; old DB upgrades; failed migration preserves data and stops ingestion safely | crash points; schema fixture matrix; integrity/corruption drills | M4 exit |
| M5.2 | Harden duplicate/malformed/adversarial delivery | Gateway/adapter limits, rejection store | concurrent duplicates create one Card/notification; conflicts/malformed/oversized input cannot crash or poison queue; rejection retention bounded | concurrency, fuzz, rate/body limits, restart replay | M3 Gateway |
| M5.3 | Harden offline and adapter reconnect | adapter supervisor, outbox, error UI | all local features and cached reading work offline; reconnect uses backoff/jitter; pending action drains once; user can disconnect and clear secret | network fault injection; long outage/restart; credential deletion | M3/M4 action path |
| M5.4 | Harden notification permissions and routing | notification manager/settings | denied/unavailable notifications show nonblocking state; Inbox remains correct; stale/deleted notification target falls back safely | permission matrix fakes; real Windows/macOS manual test | M3.6 |
| M5.5 | Add empty/error/corrupt-state UX | all features, recovery UI | empty Sticky/Inbox, unsupported Card, missing asset, adapter error and corrupt preference/DB are understandable and non-destructive | component states; corruption fixtures; Product Owner review | M5.1–M5.4 |
| M5.6 | Meet long-reading performance/accessibility budgets | Reader virtualization/containment only if measured, styles | agreed large fixture opens and restores within budget; scrolling/input stay responsive; keyboard, focus, contrast and text scaling pass | benchmark on both OSes; accessibility checks; manual screen reader sanity | representative hardware and fixture |
| M5.7 | Cross-platform packaging, signing readiness and smoke | Tauri bundle config, CI/release docs | Windows installer and macOS app/DMG build on native runners; icons/identifier/version correct; tray, keychain, notifications, updates policy documented | clean VM smoke; install/upgrade/uninstall; macOS Intel/Apple Silicon decision | signing identities/certificates from owner |
| M5.8 | Final privacy/security and MVP acceptance audit | capabilities, logs, dependency audit, docs | least privilege; no plaintext secret/source body logs; disconnect clears credentials; all P0 Gate flows accepted | capability review, secret scan, dependency audit, Product Owner/UAT | all prior M5 tasks |

**M5 exit:** signed/readiness-grade MVP passes native Windows and macOS smoke, resilience scenarios and Product Owner acceptance. Shipping remains a separate owner decision.

---

# Testing Strategy

## Gate test matrix

| Gate | Unit | Integration | Manual smoke | Cross-platform validation |
|---|---|---|---|---|
| M1 | Card rules, rank allocation, navigation/window preference reducers | repositories, migrations, native command wrappers | drag/resize/tray/AOT/theme/Sticky CRUD/restart | Windows + macOS build and smoke; geometry across monitor/DPI changes |
| M2 | block renderers, read-position fallback, completion transitions | mock event→Card, Inbox lifecycle, position DB | typography, keyboard, long read, reopen/complete | native fonts/WebView/table/code/image behavior on both OSes |
| M3 | validators, idempotency decisions, adapter health/backoff | real loopback server, transaction order, outbox, notification fakes | notification permission/click, disconnect/reconnect | Windows/macOS firewall/keychain/notification differences |
| M4 | Packet mapping/fidelity, ack state machine | fake real Skill contract end-to-end | exact source audit, offline pending confirm, resume | same fixtures and source hashes on both OSes |
| M5 | corruption/recovery helpers, bounded queues | upgrade DB matrix, fault injection, concurrency, long content | clean install/upgrade/restart/offline/denial/error/empty states | native signed-package smoke on Windows and macOS |

Required scenario assertions:

- **Restart:** no data loss, duplicate notification or new Packet generation; pending outbox remains pending.
- **Duplicate event:** same idempotency key/hash returns original outcome; different hash is conflict, never overwrite.
- **Malformed event:** stable rejection, bounded diagnostic, no Card and no notification.
- **Lost connection:** local app remains usable; status becomes degraded/disconnected; retry bounded with jitter.
- **Notification denial:** Delivery commits and unread remains discoverable; no repeated permission harassment.
- **Long reading:** exact content and anchors survive storage; resume is stable; rendering remains responsive.
- **Persistence:** DB transaction rollback cannot leave orphan Delivery/Card/action/placement records.
- **Reader recovery:** anchor-first restore after close/restart; missing anchor falls back deterministically without confirming progress.

Testing rules:

1. Contract fixtures are immutable examples shared by Rust and TS tests.
2. Time, UUID, filesystem, credential store, notification and transport are injected ports in unit tests.
3. At least one integration test uses real SQLite and one uses a real loopback socket; mocks alone do not pass M3.
4. E2E selectors use roles/test IDs, not CSS layout. Keep native E2E small and critical.
5. Performance budgets must be set at M2 with Product Owner hardware/fixture, then treated as regression gates; this report does not invent unsupported millisecond targets.
6. Product Owner owns interaction/visual acceptance; Tech Lead owns architecture/engineering Gate; automated green tests do not replace either review.

---

# G. Risk Register

| Rank | Risk | Likelihood / impact | Early signal | Mitigation / owner |
|---:|---|---|---|---|
| 1 | Progressive Reading Packet contract is not machine-stable or lacks anchors/hash/action ack | High / Critical | hand-authored payload variants, missing fidelity fields | freeze M4.1 contract before integration; fail closed; Skill owner + Tech Lead |
| 2 | Windows/macOS Tauri differences break tray, notification click, window restore or keychain | High / High | behavior only tested on development OS | native CI plus per-Gate dual-OS smoke; Implementation Engineer |
| 3 | duplicate delivery or completion action advances/creates twice after retry/restart | Medium / Critical | duplicate Cards/notifications or multiple ack calls | DB unique claims, transactional ingestion, action idempotency, concurrency tests; Tech Lead review |
| 4 | source fidelity is lost while converting PDF/Skill structure into UI blocks | Medium / Critical | whitespace/text/table/image mismatch | Packet-supplied source hash/anchors, deterministic renderer, fixture audit, no AI transform; Skill owner + Product Owner |
| 5 | localhost HTTP bridge exposes ingestion/action endpoints to other local processes or SSRF | Medium / High | unauthenticated requests, event callback URLs | loopback-only, per-install keychain token, fixed configured endpoints, limits/rate logging; Tech Lead security review |
| 6 | SQLite migration/corruption causes silent loss of personal notes or reading state | Medium / Critical | schema edited in place, startup creates blank DB | append-only migrations, backups, old-schema fixtures, preserve-on-failure recovery; Implementation Engineer |
| 7 | long Reading Packets cause WebView memory/scroll jank and unstable anchors | Medium / High | input lag, resume lands wrong | representative fixtures at M2, block-level rendering/containment, measure before virtualization; Product Owner acceptance |
| 8 | state duplication between React cache, SQLite and external Skill causes misleading unread/completion | Medium / High | badge mismatch, “confirmed” before ack | explicit ownership table, query invalidation, pending state, no persisted frontend cache; Tech Lead review |
| 9 | notification activation data may not recover the exact Card uniformly across installed Windows/macOS warm, hidden and cold lifecycles | Medium / High | activation loses `card_id`, duplicates ingestion, or works only in `tauri dev` | M3 lifecycle matrix on packaged apps; activation only navigates to existing Card; stale target fallback; Inbox remains canonical |
| 10 | scope creep from full PRD (remote relay, schedules, AI learning, adapters) delays MVP core loop | High / High | work starts outside M1–M5 exit criteria | explicit non-goals, Gate approval required, change control by Product Owner/Tech Lead |

Open decisions required before or during M1, not guessed by Implementation Engineer:

- Product Owner: final external-distribution bundle identifier, signing/release channel and visual acceptance fixture. M1 uses Tech Lead provisional defaults recorded in ADR-0001.
- Tech Lead: approve loopback-only meaning of Generic Local / HTTP Adapter, dependency major lines after scaffold resolution, log retention, DB backup retention and CI runner policy.
- Progressive Reading Skill owner/Tech Lead before M4: exact structured Packet schema, hash canonicalization, action acknowledgement and asset transport contract.

---

# H. Explicit MVP Non-Goals

The following are intentionally excluded unless a later Gate change is approved:

- Claude-specific or OpenAI-specific adapters and UI.
- Production A2A or MCP integration.
- Remote Cloud Relay, inbound internet listener, NAT traversal or remote push service.
- Multi-device sync, accounts, cloud backup or conflict resolution.
- Mobile apps.
- Email, Calendar, collaboration, teams, shared Cards or marketplace.
- Scheduled Tasks are deferred from Experiment MVP and M1-A, not removed from V0.1 Complete. Desktop must later list projected tasks and capability-gate Agent-owned create, edit-time, pause and resume actions; local scheduler ownership remains excluded.
- Multiple parallel active reading plans UI.
- PDF ingestion, OCR, Reading Map, Reading Unit planning, difficulty estimation or source extraction in Desktop App.
- Any desktop mutation of Skill `delivery_cursor` or `confirmed_cursor`.
- Automatic confirmation based on reaching the bottom, time spent or scroll percentage.
- Source rewriting, translation, summarization mixed into source, or arbitrary HTML execution.
- Learning enhancements: knowledge points, quote extraction, Quiz, reflection question and long summary.
- Rich-note editor, tags, projects, subtasks, priority system, kanban/Gantt or Notion replacement.
- Complex theme marketplace or AI-styled dashboard.
- Remote asset proxy/cache service; MVP only handles controlled Packet assets/local cache.
- Telemetry/analytics cloud pipeline. Product metrics require a separately approved privacy design.
- Auto-update service, public release, store submission or production signing credentials as part of feature Gates; M5 only reaches packaging/signing readiness unless owners authorize release.

## M0 exit checklist

- [x] Repo audited and empty state recorded.
- [x] Architecture and dependency directions proposed.
- [x] Technology choices include why, alternative and risk.
- [x] Card, Event, Adapter, ReadingCard and local position schemas defined.
- [x] SQLite/KV/secure-store split and migrations defined.
- [x] State ownership, including three distinct reading cursors, defined.
- [x] M1–M5 decomposed into goal/files/acceptance/tests/dependencies.
- [x] Gate-level testing strategy covers all requested failure scenarios.
- [x] Ten highest technical risks recorded.
- [x] MVP non-goals explicitly frozen.
- [x] No M1 scaffold, dependency install or feature implementation performed.

M0_GATE_STATUS: PASS_WITH_AMENDMENTS
