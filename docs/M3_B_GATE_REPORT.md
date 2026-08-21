# M3-B Gate Report — Agent Suggestions & Workspace Actions

Gate status at this revision:

```text
M3_B_GATE_STATUS: AWAITING_PRODUCT_AND_TECH_REVIEW
PROPOSAL_STATUS: FOUNDATION_READY
AGENT_GATEWAY_STATUS: NOT_STARTED
```

M3-B adds a local Proposal boundary between Agent suggestions and workspace mutations. No Chat UI, Agent Gateway, remote model adapter, MCP, OAuth, scheduler, automation, semantic search, or M3-C work was started.

## Delivered Scope

- Added append-only migration `0007_agent_proposal.sql` and left `0001`–`0006` unchanged.
- Added independent `AgentProposal` lifecycle states: `pending`, `accepted`, and `rejected`.
- Added the locked MVP proposal types and payloads: Todo, Record, and Reading.
- Added Reader-only paper-style suggestion cards with explicit accept and ignore actions.
- Reused Sticky and Reading service validation/preparation rules.
- Made acceptance atomic: workspace creation and `accepted` status commit in one SQLite transaction.
- Made rejection status-only; it cannot create or modify workspace content.

## Architecture Truth

```text
Agent / local producer
  → create Proposal (pending)
  → Reader card
  → explicit user decision
      ├─ accept → existing service rules → Todo / Record / ReadingSession
      │          + Proposal accepted (one transaction)
      └─ reject → Proposal rejected only
```

The Proposal domain does not issue SQLite statements. React uses a Proposal port and Tauri adapter. The SQLite proposal repository coordinates the transaction and reuses the existing Sticky and Reading repository transaction primitives; workspace content is never created before user acceptance.

## Migration and Persistence

- Fresh databases apply migrations `[1, 2, 3, 4, 5, 6, 7]`.
- `agent_proposals.source_delivery_id` preserves the Delivery/Reader context when supplied.
- A 0006 upgrade test proves ReadingSession, Delivery, and ReaderDocument data remain unchanged while 0007 is added.
- Duplicate resolution is rejected: accepting an already resolved proposal creates no second workspace item.
- Restart tests preserve all three accepted proposal types and rejected status.

## Automated Gates

Exact local toolchain: Node `22.23.2`, pnpm `11.16.0`, Cargo lockfile, and MSVC x64.

| Check | Result |
| --- | --- |
| Prettier | PASS |
| TypeScript | PASS |
| ESLint `--max-warnings 0` | PASS |
| Frontend Vitest | PASS — 78/78 |
| Frontend production build | PASS — 219 modules |
| Rust fmt | PASS |
| Rust Clippy `-D warnings` | PASS |
| Rust tests | PASS — 42/42 |
| Fresh non-bundled Tauri release | PASS |

Coverage includes Proposal create/persistence/status, all three accepted workspace actions, reject-without-mutation, duplicate resolution, restart persistence, the three-Todo meeting demo, migration upgrade, and prior Sticky/Reader/Inbox/Delivery/Reading regressions.

## Native Windows Smoke

Truth executable:

```text
D:\agent-desk-m3b-truth-target\release\agent-desk.exe
SHA-256: 52A6B513549FD39DEF7383C6881C874484F8E1F4ABE352AABDB1BD3D8312D18B
```

Formal smoke used isolated SQLite, Store, window state, and WebView2 data under `D:\agent-desk-m3b-smoke-08a5095-20260822-05`.

1. First launch remained alive beyond 10 seconds.
2. A meeting-summary Delivery and Todo Proposal were created through Tauri commands.
3. Reader displayed the pending suggestion; the workspace still had no proposed Todo.
4. The user-facing `加入待办` action was invoked.
5. Expanded Sticky displayed exactly one accepted Todo.
6. After full process shutdown and restart, the source Reader document, accepted proposal, and exact Todo were restored.
7. Final audit found zero smoke Agent Desk and zero smoke WebView2 processes.
8. Product Owner Roaming data and Local WebView2 data matched before/after for the formal smoke.

Evidence: [Windows native smoke](evidence/m3-b/windows/native-smoke-runtime.json), [Reader proposal](evidence/m3-b/windows/01-reader-proposal.png), [accepted Todo](evidence/m3-b/windows/02-todo-accepted.png), and [restart restored](evidence/m3-b/windows/03-restart-restored.png).

During a superseded diagnostic smoke, SQLite/Store were isolated but the WebView2 user-data-folder variable was initially omitted. That invalid run changed only the Product Owner Local WebView2 cache; the Roaming database/Store digest stayed identical. No user file was deleted, reset, restored, or rolled back. The formal evidence run corrected the isolation and produced identical before/after digests for both locations.

## Artifact Truth

| Artifact | SHA-256 |
| --- | --- |
| `dist/index.html` | `1360F893020304623E1A4CC7C725C405297A39CA0622432FD81957A67B875437` |
| `dist/assets/index-Bz225wox.css` | `8D71DC37B54E3565140A8B54AD5B919E4F9F15C5F90E0554B8AB711AB71BAB2F` |
| `dist/assets/index-BEYg7U0K.js` | `F981DE9C98FB4F6FE1ABB9DA57E20EAD79B194EA2D64E101705E99D52C617701` |
| truth executable | `52A6B513549FD39DEF7383C6881C874484F8E1F4ABE352AABDB1BD3D8312D18B` |
| canonical executable | `52A6B513549FD39DEF7383C6881C874484F8E1F4ABE352AABDB1BD3D8312D18B` |

`PRODUCT_OWNER_PREVIEW_EXECUTABLE`: `D:\agent-desk-target\release\agent-desk.exe`

Truth and canonical files are byte-identical, size `15,002,112` bytes. Product source commit: `08a5095aab2c48c32f9e76cab9e9e04c8270514d`.

## Public Repository Safety

- Repository visibility remains Public; no visibility operation was performed.
- New commit content contains no secret, credential, token, personal user path, `.env`, local database, build output, `target/`, `node_modules/`, or executable.
- Runtime evidence replaces the local username with `%APPDATA%` / `%LOCALAPPDATA%` aliases.
- Existing README portfolio commits were preserved.

## Hosted CI

GitHub Actions run [32534848404](https://github.com/dawnsongbest-create/agent-desk/actions/runs/32534848404) validated implementation commit `08a5095aab2c48c32f9e76cab9e9e04c8270514d` through the Pull Request workflow.

| Job | Job ID | Result |
| --- | --- | --- |
| Windows | `96933709653` | PASS |
| macOS | `96933709823` | PASS |

Both jobs used `.node-version`, frozen pnpm install, frontend format/typecheck/lint/tests/build, Rust fmt/Clippy/tests, and a real non-bundled Tauri build.

No M3-C work may begin from this report.
