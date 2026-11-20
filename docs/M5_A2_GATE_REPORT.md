# M5-A2 — Codex Todo Suggestions

Baseline: `cfdc245b7b4fadc75616bd0af5c4099d917bbb21` (M5-A1). This report separates
local tests from native Codex acceptance and hosted CI. Do not promote the gate
on automated tests alone.

Feature implementation and native screenshot source: `6382b84701ac63e477d80fb46e5fa91993eeb164`.
The final truth build additionally includes the bounded HTTP response fix
documented below.

## Checkpoints

| Checkpoint | Scope | Local evidence |
| --- | --- | --- |
| A | `0010`, independent Todo credential and explicit revocation | Windows migration, permission isolation and fail-closed CredentialPort tests; separate synthetic Windows Credential Manager round trip passed |
| B | Bounded MCP/Bridge input, ordered batch, idempotency and atomic acceptance/rejection | Concurrent duplicate sends, failed middle insert, repeated accept, Accept/Reject race, restart, and revocation during a queued write covered |
| C | Expanded Todo preview, pending count, local acceptance, opt-in positioning and cross-window refresh | TypeScript, ESLint, Prettier, production Vite build and 102 frontend tests passed on Windows; native/Codex evidence tracked separately below |

### Migration and permission boundary

`0010_todo_suggestion_batches.sql` appends exactly two tables:

- `codex_todo_suggestion_grant`: singleton per local database, fixed
  `suggest_todos` scope, active/revoked state, SHA-256 token hash, stable unique
  namespace and timestamps. No plaintext token.
- `todo_suggestion_batches`: canonical ordered items, keyed request digest,
  unique namespace/digest pair, timestamps, pending/accepted/rejected status and
  the accepted Todo IDs. Revocation changes the grant only, not batch history.

The same local STDIO registration discovers exactly `send_reading` and
`suggest_todos`. Pairing reads the matching OS credential only for that tool.
The Bridge listens only on IPv4 loopback. It recognizes scoped Codex credentials
before routing legacy tokens, then verifies the saved grant under the shared
Codex write/permission lock. The new HTTP route only submits suggestions.
List, accept and reject run through bundled-window native IPC, not HTTP or MCP.

| Credential | Reading delivery | Todo suggestions | Legacy v1 routes |
| --- | --- | --- | --- |
| Codex reading | Allowed | Forbidden | Forbidden |
| Codex Todo grant | Forbidden | Allowed while active | Forbidden |
| OpenClaw / legacy | Forbidden on Codex-specific route | Forbidden | Unchanged |
| Missing / revoked | Not authorized | Not authorized | Not authorized |

The reading and Todo entries in Windows Credential Manager / macOS Keychain are
separate. Granting or revoking Todo does not rotate reading. “断开 Codex” revokes
both hashes in a single DB transaction before attempting OS cleanup. OS cleanup
errors do not restore access. No new persisted Codex configuration is required.

### Contract and user flow

1. In the existing local Agent Bridge settings, enable the Bridge and keep the
   existing Agent Desk STDIO entry. If installing for the first time, use the
   executable and arguments shown by “连接 Codex”; restart the client so it
   discovers both tools. Do not paste a token.
2. Click “允许提交待确认的待办建议”. The new grant starts off. This lets Codex
   submit suggestions only; it cannot read, create, modify, delete or complete Todo.
3. Ask Codex to send an ordered group of tasks based on context you provided.
   `suggest_todos` accepts 1–20 items, each with `text` (1–4,000 characters),
   `priority` (`high | medium | low`), optional ISO date `due_date` and optional
   `reason` (200 characters). It rejects unknown fields and invalid items, with
   an overall 64 KiB UTF-8 JSON limit. No content is truncated.
4. Open Expanded Sticky → Todo → “待确认建议”. Preview one batch at a time.
   “暂不处理” closes the preview without writes; “拒绝” resolves it without Todo
   changes; “确认加入” appends the whole batch. “查看刚加入的待办” locates the
   first new task only when clicked. Compact deliberately has no new indicator.
5. “撤销待办建议权限” stops future submissions without affecting reading or
   removing pending suggestions or accepted tasks.

The server sorts high → medium → low stably and stores the normalized order.
Priority and reason are preview-only. Acceptance appends to existing Sticky
positions without modifying any old Todo or Record, and does not change manual
drag behavior. Both renderers refetch the Todo list after commit without
refetching Quote or resetting the current Reader.

An omitted `idempotency_key` is generated once per send. Reuse the returned ID
only when retrying the same send. Same key and canonical payload returns the
original batch and its current status; changed content returns `409`. A lost
key after a process restart cannot guarantee exactly-once delivery. Acceptance
locks before reading the batch and commits new cards plus their saved receipt
together. A repeated accept returns that receipt and does not insert again.

### Native and hosted evidence

- Final local Rust check: 63 passed, one opt-in test ignored in the normal run;
  the synthetic Windows Credential Manager test passed separately. Strict
  Clippy and rustfmt passed.
- Clean Windows truth release: Node 22.23.2 / pnpm 11.16.0 / MSVC x64,
  `pnpm tauri build --no-bundle -- --locked` in a fresh target. Tauri rebuilt
  the production frontend. Final binary size 16,600,064 bytes; written
  2026-11-20 04:56:41 UTC.

| Artifact | SHA-256 |
| --- | --- |
| Initial M5-A2 release used for real Codex and UI evidence | `EABD320528F2E11B031234165E645D72361062E5D5481060CBF609748AB2A229` |
| Production JS | `3E763D8A462EB6A8CFAA78F30B7FDB40127C64C04B9387FF10ADBF4FABBEF393` |
| Production CSS | `93C5454917986CA29A4F36201E843B9ECFA19A36F2D550DEC2458E3369D9B978` |
| Final M5-A2 truth and installed canonical | `0C9E13094F57C54445FCC7D43D79A764D6A99BC325F577C10AA21DA18B946792` |
| Archived M5-A1 canonical (not a schema-compatible rollback) | `F179C05565428569E3F4A28041C45E9FB91C55855260F9A3AEAB03927676D3FD` |

- Real local Codex CLI 0.169.1 used an ephemeral session and one-process
  MCP override, without persisting Codex configuration. The ungranted
  `suggest_todos` call failed with `credential_unavailable`, actionable
  authorization instructions and no submission receipt.
- After the owner closed the formal instance, the matching truth release
  launched with isolated SQLite, Store, WebView2 and OS credential scope.
  The owner enabled the Bridge and authorized Todo suggestions. The verified
  isolated process owned the IPv4 loopback listener.
- Two real Codex calls committed pending batches. With both visible in native
  Expanded Todo, the SQLite Todo count stayed at 0. “暂不处理” closed the preview
  and the same batch reopened; “拒绝” resolved that batch, still with 0 Todo.
- One native “确认加入” accepted the other batch. SQLite held four new tasks,
  in the exact accepted receipt order, with the first task's date preserved.
  The order was high, high, medium, low, preserving the relative order of the
  two high items. The renderer showed “已加入 4 项” and its locate action.
- Real Codex replays returned the same two batch IDs and `created=false`,
  with statuses `rejected` and `accepted`; the SQLite count stayed at four.
- [Native ordered preview](evidence/m5-a2/01-native-synthetic-ordered-preview.jpg)
  and [native acceptance](evidence/m5-a2/02-native-synthetic-acceptance.jpg) are
  from the initial M5-A2 release above and contain only synthetic content.
- The owner revoked only the Todo grant. The verified isolated Bridge stayed
  online. A real Codex `suggest_todos` call failed with `credential_unavailable`;
  batches and Todo rows were identical before and after. In that session,
  `send_reading` committed one synthetic Inbox delivery and its Reader document.
  Authenticated OpenClaw compatibility tests passed; no native OpenClaw client
  was run.
- Both M5-A2 builds were installed atomically after their corresponding
  isolated checks. The final canonical was launched against the same isolated
  profile for at least ten seconds. Its loopback listener and WebView2 children
  came from that run, and the four Todo rows, batch receipts and synthetic reading
  delivery survived the restart. The test processes were stopped; no
  isolated WebView2 children remained. The isolated reading test credential
  was removed. The five formal AppData files retained their pre-test hashes
  or absent status.
- Hosted verification: [main CI runs](https://github.com/dawnsongbest-create/agent-desk/actions?query=branch%3Amain).
  Use the Windows and macOS jobs for the push that includes this report;
  the local checks above are not hosted-CI evidence.
- [First hosted CI run](https://github.com/dawnsongbest-create/agent-desk/actions/runs/44676861126):
  Windows passed; macOS failed in the 64 KiB overflow test. The early 413
  closed the socket with unread request bytes; macOS reset the connection
  before the client could reliably read the status. The fix sends 413 and
  half-closes the write side, then discards a time- and size-bounded amount
  without parsing it. The existing overflow test is retained, and a
  headers-only case checks that the server replies without waiting for a
  body. Full Rust tests and strict Clippy passed again locally. A socket probe
  against the rebuilt Windows Tauri release verified HTTP 413 for both cases
  before canonical replacement. The limit and legacy OpenClaw routes are unchanged.
- Scoped public safety scan: no known credential signatures, personal
  absolute paths, or database/packaging/build files in the candidate changes;
  do not publish local logs, isolated state or binaries.

### Disable / forward rollback

Revoke the Todo grant, then start Agent Desk with
`AGENT_DESK_DISABLE_TODO_SUGGESTIONS=1` if the feature needs to be disabled.
This hides/disables the new controls and rejects new Todo submissions while
preserving history, accepted tasks and `0010`. Reading, OpenClaw, old Proposal
and Reader data are untouched. Do not replace a migrated installation with a
binary that predates `0010`. No Baidu Dumate integration was started.
