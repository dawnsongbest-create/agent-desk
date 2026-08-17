# Agent Desk

> **A quiet desktop surface for Agents.**
> 让 Agent 在后台工作，把真正值得看的内容安静地送到你的桌面。

Agent Desk 是一个面向 **Windows / macOS** 的桌面 Agent 应用。

它尝试解决一个很具体的问题：

**Agent 已经可以替我们搜索、整理、总结和定时执行任务，但 Agent 产出的内容，应该在哪里被好好地阅读和使用？**

我的答案不是再做一个 Chat UI，而是给 Agent 一张真正的“桌面”。

Agent 可以在后台完成任务，把阅读材料、研究报告、定时 Brief 等内容投递到 Agent Desk；用户则可以在一个轻量、长期悬浮的桌面窗口中完成：

**接收 → 阅读 → 摘录 → 记录 → 待办**

---

## Preview

<p align="center">
  <img src="docs/images/reader.png" width="31%" alt="Agent Desk Reader" />
  <img src="docs/images/sticky.png" width="31%" alt="Agent Desk Sticky" />
  <img src="docs/images/record.png" width="31%" alt="Agent Desk Record" />
</p>

> 建议将 README 中的三张截图替换为仓库内实际图片路径：
>
> <img width="401" height="748" alt="image" src="https://github.com/user-attachments/assets/7035b496-0d2c-4995-844a-53e6817851b5" />
> <img width="648" height="741" alt="image" src="https://github.com/user-attachments/assets/1e9b8c07-6928-4b36-95ed-4d7f7a43d458" />
> <img width="526" height="780" alt="image" src="https://github.com/user-attachments/assets/87b43705-3af3-4f73-b340-97fb53264564" />


---

# Why Agent Desk?

这个项目来自我自己使用 Agent 定时任务时遇到的一个真实问题。

我经常让 Agent 定时整理资料、生成 Brief 或推送阅读内容。

但慢慢发现：

**内容生成出来，并不意味着我真的愿意读。**

很多 Agent 的结果最终被送进 IM 或聊天窗口：

* 长内容缺少舒服的排版
* 新内容不断把旧内容往上顶
* 未读消息越积越多
* 很难区分“需要现在看”和“以后再看”
* 阅读、记笔记、Todo 又散落在不同应用里

最后 Agent 虽然替我节省了“获取信息”的时间，却产生了新的：

> **信息消费负担。**

所以我开始思考：

如果 Agent 已经逐渐成为一个后台工作的数字协作者，那么它是不是也需要一个专门的 **Delivery Surface**？

于是有了 Agent Desk。

---

# Product Idea

Agent Desk 的核心原则是：

> **Agent 负责理解、处理和生产，Desk 负责呈现、交互和沉淀。**

我不希望它变成另一个：

* ChatGPT 客户端
* AI 聊天窗口
* 笔记软件
* Todo App
* 电子书阅读器

它更像是这些产品之间的一层连接。

```text
Agent
  ↓
获取 / 搜索 / 分析 / 生成
  ↓
Delivery
  ↓
Agent Desk
  ↓
阅读 / 摘录 / 记录 / Todo
```

Agent 在外面工作。

Agent Desk 是这些工作结果最终安静落到用户面前的地方。

---

# Core Experience

## 1. Reader + Floating Sticky

整个产品采用：

> **阅读页面 + 悬浮便利贴**

的双层交互。

视觉上希望它更像：

**桌面上放着一张纸，纸上贴着一张便利贴。**

而不是传统软件中的 Sidebar、Panel 和 Dashboard。

```text
┌─────────────────────────────┐
│                             │
│       Reader Document       │
│                             │
│                   ┌───────┐ │
│                   │Sticky │ │
│                   └───────┘ │
│                             │
└─────────────────────────────┘
```

Sticky 与 Reader 是两个独立图层。

便利贴可以自然盖在阅读页面上，不需要正文为了它重新排版。

---

# Sticky

Sticky 是 Agent Desk 中随时可以使用的个人信息层。

目前支持三种状态：

```text
Mini
 ↕
Compact
 ↕
Expanded
```

### Mini

最小化后的 Sticky 会固定在 Reader 顶部的 Safe Shelf 中。

只保留：

* TODAY
* 当前 Todo 数量
* 极轻量状态信息

尽可能减少对阅读的干扰。

### Compact

Compact Sticky 用于快速查看：

* 当前待办
* Sticky Quote
* 今日信息

支持拖动与位置保存。

### Expanded

Expanded Sticky 是完整工作状态，目前支持：

### 记录

* 创建长记录
* 编辑 / 删除
* 长文本输入
* Markdown 友好
* Markdown 导出
* 重启恢复

### Todo

* 创建 Todo
* 完成 / 恢复
* 截止日期
* 拖动排序
* 持久化
* Compact 状态快速查看

### Sticky Quote

可以单独留下一个简短句子。

它更像真正贴在桌边的一句话，而不是数据库中的另一条 Note。

---

# Reader

Reader 是 Agent 内容真正被消费的地方。

我没有把 Reader 做成 AI Dashboard，而是刻意向：

> **一张安静、耐看的纸**

靠近。

目前支持：

* Markdown 阅读
* H1 / H2 / H3
* 列表
* 引用
* Code Block
* Inline Code
* Link
* 字号调整
* 行距调整
* Light / Dark / System
* Grid Paper
* Paper Texture
* 阅读滚动位置恢复

---

## Blank Reader

用户可以点击：

**隐藏正文**

暂时把所有文章内容收起来。

页面会变成一张干净的网格纸，只留下 Sticky。

```text
Reader
↓
隐藏正文
↓
Blank Paper + Sticky
```

需要继续阅读时，再点击：

**显示正文**

即可回到之前的位置。

这个功能来自一个很简单的想法：

> 阅读器不一定永远需要显示内容，有时候用户只需要一张安静的纸。

---

# Selection Capture

阅读过程中可以直接选中文字。

选中后只出现两个操作：

```text
复制
保存到记录
```

不会弹出复杂的 AI Toolbar。

保存到记录时：

* 保留用户选中的原始文本
* 不自动改写
* 不自动总结
* 不自动润色
* 不把来源信息混入正文

同时系统会单独保存：

```text
Record
↕
Source Reference
↕
ReaderDocument
```

为未来：

**记录 → 回到原文**

留下数据基础。

---

# Inbox & Delivery

Agent Desk 已经进一步实现了内容投递与收件箱基础能力。

这里有一个我非常重视的设计：

## Content ≠ Delivery

一份内容是什么，与它什么时候被送到用户面前，是两件不同的事。

因此内部将它们拆分为：

```text
ReaderDocument
=
内容本身

Delivery
=
一次投递行为
```

例如：

```text
Agent
↓
生成 AI Daily Brief
↓
ReaderDocument

08:30 投递
↓
Delivery

Inbox
↓
用户打开
↓
Reader
```

这样未来即使出现：

* Agent 重试
* 定时任务
* Notification
* 多 Agent
* Cloud Relay

也不需要把“内容状态”和“投递状态”混在一起。

---

## Inbox

目前 Inbox 已支持：

* 收件入口
* 未读 / 已读
* 未读数量
* 按投递时间排序
* 内容来源
* 内容类型
* 投递时间
* Delivery → Reader
* 重启后状态恢复
* 防重复投递

用户点击：

```text
收件箱
↓
某份 Delivery
↓
ReaderDocument
↓
Reader
```

即可直接进入阅读。

---

# Idempotent Delivery

Agent 在未来真实投递时可能发生：

```text
请求
↓
网络超时
↓
Agent 重试
```

如果没有幂等机制，同一篇内容可能被投递很多次。

因此 Delivery 从第一版开始就加入：

```text
idempotencyKey
```

同一 key + 同一内容再次进入：

```text
不会创建重复 Delivery
```

同一 key 却携带不同内容：

```text
IDEMPOTENCY_CONFLICT
```

而不是静默覆盖。

这是为了让未来 Agent Gateway 可以建立在一个可靠的投递基础上。

---

# Technical Architecture

Agent Desk 当前使用：

* **Tauri 2**
* **React**
* **TypeScript**
* **Rust**
* **SQLite**
* **SQLx**
* **Tauri Store**
* **Vite**
* **Vitest**
* **GitHub Actions**

整体采用 Local-first 架构。

```text
┌──────────────────────────────┐
│            React             │
│                              │
│ Sticky / Reader / Inbox / UI │
└──────────────┬───────────────┘
               │ Application Port
               ▼
┌──────────────────────────────┐
│         Tauri Boundary       │
└──────────────┬───────────────┘
               │
               ▼
┌──────────────────────────────┐
│             Rust             │
│                              │
│ Application Services         │
│ Domain Validation            │
│ Transactions                 │
│ Native Capabilities          │
└──────────────┬───────────────┘
               │ Repository
               ▼
┌──────────────────────────────┐
│            SQLite            │
│                              │
│ Cards                        │
│ Records                      │
│ Todos                        │
│ ReaderDocuments              │
│ SourceRefs                   │
│ Deliveries                   │
└──────────────────────────────┘
```

---

# Architecture Principles

## 1. React 不直接访问 SQLite

依赖方向保持：

```text
React Feature
↓
Application Port
↓
Tauri Infrastructure
↓
Rust Application Service
↓
Repository
↓
SQLite
```

UI 不承担数据库业务逻辑。

---

## 2. Durable Data 与 UI Preference 分离

SQLite 保存：

* Record
* Todo
* ReaderDocument
* Delivery
* Source Reference
* 业务状态

Tauri Store 保存：

* Theme
* Window preference
* Reader font size
* Reader line spacing
* Blank Reader
* currentDocumentId
* 其他轻量 UI Preference

---

## 3. Append-only Migration

数据库从项目开始就采用：

> migration 只新增，不修改已经发布的历史 migration。

目前所有数据结构演进均通过连续 migration 完成。

这样可以真实验证：

```text
Old Version
↓
Upgrade
↓
New Version
```

而不是每次开发都重建数据库。

---

## 4. Transaction First

需要保持一致的数据操作尽可能放在同一事务中。

例如保存 Reader 摘录：

```text
Create Record
+
Create Source Reference
```

要么全部成功，要么全部失败。

Delivery ingestion 同样如此：

```text
Create ReaderDocument
+
Create Delivery
```

作为一个原子操作完成。

---

## 5. Presentation 与 Domain 分离

例如：

```text
隐藏正文
```

只是 Reader 的 UI 状态。

它不会：

* 删除 ReaderDocument
* 修改正文
* 改变数据库内容

类似地：

```text
currentDocumentId
```

表示“用户现在正在看什么”。

而：

```text
Delivery
```

表示“什么内容被送来过”。

两者不会因为 UI 方便而混成同一个状态。

---

# Why Not Another Chat UI?

我刻意没有设计一个输入框放在页面底部。

因为 Agent Desk 想解决的并不是：

> **“我应该在哪里继续和 AI 对话？”**

而是：

> **“Agent 已经工作完了，我应该在哪里舒服地消费它的结果？”**

很多 Agent 产品关注的是：

```text
User
→ Agent
```

Agent Desk 更关注：

```text
Agent
→ User
```

也就是 Agent 工作结果的最后一公里。

---

# Multi-Agent Ready

目前产品还没有真正连接外部 Agent。

但架构没有绑定任何具体模型。

未来计划通过：

```text
External Agent
↓
Adapter
↓
Agent Gateway
↓
Unified Event
↓
Delivery
↓
Agent Desk
```

接入不同来源。

例如：

* Local Agent
* OpenAI
* Claude
* Internal Agent
* HTTP
* MCP
* A2A

UI 不需要知道：

> 这条内容究竟来自哪一个模型或协议。

它只需要知道：

> 这是一个可以被 Agent Desk 理解的内容与投递事件。

---

# Progressive Reading

我还独立设计并验证了一个：

**Progressive Reading Skill**

它未来会成为 Agent Desk 的一个重要 Agent Use Case。

用户只需要：

1. 上传一本 PDF
2. 设置每天希望阅读多少分钟
3. 设置每天推送时间

Agent 自动完成：

```text
PDF
↓
解析结构
↓
Reading Map
↓
判断内容难度
↓
估算阅读时间
↓
拆分语义完整 Reading Units
↓
每天选择下一段
↓
Delivery
↓
Agent Desk
```

其中一个核心原则是：

> **语义完整性 > 精确阅读时间**

如果用户说每天阅读 10 分钟，并不是机械地在第 10 分钟的位置截断。

Agent 会综合：

```text
时间预算
+
内容难度
+
阅读速度
+
语义完整性
```

决定当天应该读到哪里。

同时：

> Agent 只能决定从哪里开始、在哪里结束，不能偷偷改写作者原文。

最终希望实现：

```text
Progressive Reading Skill
↓
Daily Reading Packet
↓
Agent Desk Delivery
↓
Inbox
↓
Reader
↓
用户完成阅读
↓
Agent 推进下一段
```

---

# What Makes Agent Desk Different?

我认为区别并不在于某一个单独功能。

便利贴、阅读器、Inbox 都已经存在很多年。

真正不同的是它们被重新组合起来服务于一个新的工作方式：

> **Agent 在后台工作，人只在需要的时候接收结果。**

因此 Agent Desk 更像是：

### Agent 的桌面收件口

而不是聊天客户端。

### Agent 结果的阅读器

而不是电子书 App。

### 阅读过程中的个人便利贴

而不是完整知识库。

### 多 Agent 共用的 Delivery Surface

而不是绑定某个模型的 UI。

---

# Current Status

目前已经完成可运行的 Desktop MVP。

### Sticky

* [x] Record
* [x] Todo
* [x] Due Date
* [x] Drag & Reorder
* [x] Sticky Quote
* [x] Mini / Compact / Expanded
* [x] Persistence
* [x] Markdown Export

### Reader

* [x] Markdown Renderer
* [x] Paper Typography
* [x] Grid Paper
* [x] Font Size
* [x] Line Spacing
* [x] ReaderDocument
* [x] Blank Reader
* [x] Scroll Restore
* [x] Text Selection
* [x] Copy
* [x] Save Selection to Record
* [x] Source Provenance

### Delivery / Inbox

* [x] Delivery Domain
* [x] ReaderDocument / Delivery Separation
* [x] Inbox
* [x] Unread / Opened
* [x] Ordering
* [x] Delivery → Reader
* [x] Idempotent Delivery
* [x] Atomic Persistence
* [ ] Final M2-C Native / CI Gate

### Agent

* [ ] Local Agent Gateway
* [ ] External Agent Push
* [ ] Native Notification
* [ ] Agent Adapter
* [ ] Unified Agent Event
* [ ] Remote Relay

---

# Engineering Status

当前 M2-C 本地验证：

```text
Frontend Tests   74 PASS
Rust Tests       33 PASS
Rust Clippy      PASS
Production Build PASS
```

项目已经建立 Windows / macOS CI 流程。

当前 Delivery / Inbox 主体实现完成，最终 M2-C Native Smoke 与 CI Gate 仍在收尾，因此这里不会把尚未完成的 Agent Gateway 标记为已实现。

---

# Roadmap

## M3 — Local Agent Gateway

下一阶段优先实现：

```text
Local Agent
↓
Agent Gateway
↓
Delivery
↓
Inbox
↓
Native Notification
↓
Reader
```

目标是第一次真正跑通：

> Agent 完成任务 → Agent Desk 自动出现一条新内容。

---

## Progressive Reading Integration

接入已经完成设计与验证的 Progressive Reading Skill。

```text
PDF
↓
Agent
↓
Daily Reading Packet
↓
Agent Desk
↓
Reader
```

---

## Native Notifications

未来 Agent Desk 不需要一直打开在用户面前。

Agent 完成重要任务以后：

```text
Native Notification
↓
点击
↓
对应 Delivery
↓
Reader
```

---

## More Agent Adapters

后续探索：

* Local HTTP Agent
* MCP
* A2A
* OpenAI
* Claude
* Internal Agent Platform

所有 Adapter 最终进入统一 Delivery Pipeline。

---

## Semantic Content Layout

当前 Reader 使用 Markdown。

未来希望进一步支持结构化内容：

```text
summary
key_point
quote
source
code
table
action
```

由 Agent 生成语义结构，

由 Agent Desk 决定最终视觉呈现。

即：

> **Agent 决定内容是什么，Desk 决定它应该怎么被阅读。**

---

# Design Philosophy

Agent Desk 的设计不是从：

> “AI 产品应该长什么样？”

开始的。

而是从：

> “人在桌面上到底愿意长期留下什么东西？”

开始。

纸、笔记本、便利贴之所以长期存在，是因为它们：

* 不要求持续互动
* 不抢注意力
* 可以放在那里
* 想看的时候再看
* 信息结构足够简单

所以我希望 Agent Desk 保留这种感觉。

不是：

```text
AI is always asking for your attention.
```

而是：

```text
Agents work quietly in the background.

The results wait for you on the desk.
```

---

# Development Method

Agent Desk 也是我用于探索 **AI Native Product Development / Vibe Coding** 的个人项目。

开发过程中，我主要负责：

* 产品定义
* User Flow
* Interaction Design
* 信息架构
* Domain Model
* 技术方案讨论
* Gate / Milestone 划分
* Product Review
* Native Validation
* Bug 判断与迭代

Coding Agent 负责具体工程实现。

整个过程不是：

```text
Prompt
↓
一次生成 App
```

而是：

```text
Product Hypothesis
↓
Architecture
↓
Gate
↓
Implementation
↓
Automated Test
↓
Native Smoke
↓
Product Review
↓
Next Gate
```

我希望通过这个项目验证：

> 在 Coding Agent 大幅降低实现成本以后，产品设计、问题拆解、架构判断、验收标准和持续迭代，会变得比“会不会亲手写完所有代码”更加重要。

---

# Local Development

## Requirements

建议开发环境：

```text
Node.js 22
pnpm
Rust
Tauri prerequisites
```

安装依赖：

```bash
pnpm install --frozen-lockfile
```

启动桌面开发环境：

```bash
pnpm tauri dev
```

运行前端测试：

```bash
pnpm test
```

构建：

```bash
pnpm tauri build
```

---

# Platform

```text
Windows   ✅
macOS     ✅ CI / Build Support
```

当前主要 Product Preview 与 Native Smoke 在 Windows 完成。

---

# Project Direction

Agent Desk 目前仍然是一个持续开发中的个人项目。

它最终想回答的问题很简单：

> **当未来一个人同时拥有很多 Agent，它们完成的工作应该去哪里？**

我的设想是：

不需要每次重新打开十个 Agent 聊天窗口。

Agent 可以各自在后台工作。

而重要的结果最终汇聚到：

**你的桌面。**

---

<p align="center">
  <strong>Agent Desk</strong><br/>
  Agents work outside. Results arrive quietly on your desk.
</p>
