# M2-C Gate Report — Delivery Inbox & Reader Handoff

Gate status at this revision:

```text
M2_C_GATE_STATUS: AWAITING_PRODUCT_AND_TECH_REVIEW
DELIVERY_DOMAIN_STATUS: FOUNDATION_READY
INBOX_STATUS: READY
AGENT_GATEWAY_STATUS: NOT_STARTED
```

The implementation, local automated gates, resumed native audit, public-release safety audit, and hosted Windows/macOS CI pass. M2-C is ready for Product Owner and Tech Lead review; M3 has not started.

## A. Implementation Summary

- Added an independent Delivery domain and append-only `0005` migration.
- Added atomic ReaderDocument + Delivery ingestion, unique idempotency handling, Inbox list/count, and first-open behavior.
- Added a quiet Header `收件` entry, Reader/Inbox local presentation state, paper Inbox list, empty/error states, unread semantics, and Delivery → Reader handoff.
- No Product UI can create a Delivery. The internal `ingest_delivery` command is the only native smoke/future application boundary.

## B. Delivery vs ReaderDocument Model

`ReaderDocument` remains the durable content identity. `Delivery` is the arrival envelope and stores only its linked document ID, idempotency key, delivery time, and first-open time. Unread/opened state is not stored on ReaderDocument.

## C. Migration 0005

- File: `src-tauri/migrations/0005_delivery_inbox.sql`
- Adds only `deliveries`, a ReaderDocument foreign key, a unique idempotency key, an Inbox ordering index, and a partial unread index.
- `0001`–`0004` were not modified.
- Fresh migration test: PASS (`[1, 2, 3, 4, 5]`, foreign keys enabled).
- Existing `0004` → `0005` preservation test: PASS for Sticky Record, placement, and ReaderDocument.

## D. Delivery Repository

`SqliteDeliveryRepository` implements atomic ingest, get, repository-ordered Inbox (`delivered_at DESC, id DESC`), unread count, and first-open marking. Repository tests cover insert/get/list/restart/FK/idempotency/conflict/rollback.

## E. Delivery Application Service

`DeliveryService` validates `CreateDeliveryInput` and exposes ingest/list/count/open operations through the Delivery repository port. Tauri commands are `ingest_delivery`, `list_inbox`, `get_inbox_unread_count`, and `open_delivery`.

## F. Atomic Document + Delivery Creation

ReaderDocument and Delivery inserts share one SQLite transaction. A test-only rejecting Delivery trigger proves that failure after the ReaderDocument insert rolls the complete operation back and leaves zero orphan documents.

## G. Idempotency

- First key + payload: creates one document and one Delivery.
- Same key + identical normalized payload: returns the original item with `created = false`.
- Same key + changed title/content/source/type or explicit delivery time: returns `IDEMPOTENCY_CONFLICT` and preserves the original.
- Sequential restart repeat and unique-key race fallback are handled without orphan documents.

## H. Inbox UI

- Header uses `收件` / `收件 N`; Inbox mode uses `阅读`.
- Inbox reuses the Reader paper canvas and fixed Safe Shelf.
- Items show title, source, Chinese type label, and local delivery time.
- The exact empty text is `暂时没有新内容。`; load and open failures offer simple retry-safe feedback.
- Items are semantic buttons with focus styles and accessible labels.

## I. Unread Semantics

`opened_at IS NULL` is the only unread definition. A small dot, slightly stronger title, `data-unread`, and explicit `未读`/`已读` text make the state perceivable without color alone. Opening decrements the Header count immediately; reopening preserves the first timestamp and count.

## J. Inbox / Reader Navigation

`ReaderSurfaceMode = reader | inbox` is React-local state and is not persisted. Header round trips preserve the current document and Reader scroll ref. Entering Inbox clears the Reader popover and DOM selection. Inbox and Reader use the same independently scrolling viewport below the fixed Shelf.

## K. Delivery → Reader Handoff

The Rust open operation first resolves the joined Delivery + ReaderDocument, then applies `opened_at = COALESCE(opened_at, now)` in the same transaction. Only a successful return updates `currentReaderDocumentId`, forces content visible for an explicit item click, switches to Reader, and resets new-document scroll to zero. Failed opens remain in Inbox and do not change navigation state.

## L. Blank Reader Interaction

Automated tests pass for both required paths:

- Blank Reader → Inbox → `阅读`: remains Blank.
- Blank Reader → Inbox → Delivery: content becomes visible and the selected document renders.

Both paths pass in the exact canonical Windows executable with isolated data, as recorded in section P.

## M. Safe Shelf / Sticky Coexistence

- Inbox Shelf left label: `收件箱`; pinned Mini remains on the right.
- Native Mini evidence: PASS.
- Native Compact-over-Inbox evidence: PASS; Reader layer `1`, Sticky layer `2`.
- No wrapping, collision avoidance, auto-move, or secondary Inbox toolbar was introduced.

## N. Architecture Check

```text
Reader React → Reader Port → Tauri → Rust Reader Service → Reader Repository → SQLite

CreateDeliveryInput → Delivery Service → atomic ReaderDocument + Delivery → SQLite

React Inbox → Delivery Port → Tauri → Delivery Service → Delivery Repository → SQLite
```

React does not access SQLite. Inbox domain data is not stored in Tauri Store. `currentDocumentId` remains a UI preference. Delivery and ReaderDocument are separate. Agent Gateway, HTTP, notifications, scheduler, adapters, cloud relay, archive, delete, search, filters, and future cursor/progress domains were not started.

## O. Automated Tests

- Frontend format/typecheck/lint: PASS.
- Frontend Vitest: PASS, 74 tests across 7 files.
- Rust fmt: PASS.
- Rust Clippy `-D warnings`: PASS.
- Rust tests: PASS, 33 tests.
- Frontend production build: PASS.
- Inbox coverage includes empty, one unread, three unread + two opened, semantics, metadata, navigation, failed open, Blank mode, same-document scroll restoration, new-document scroll zero, DOM selection cleanup, and all four presets.
- Existing long Record, Todo, Quote, Mini/Compact/Expanded, selection capture, export, page-turn sound/animation, Reader Markdown, typography, Safe Shelf, Blank mode, and four-preset tests remain green.

## P. Windows Native Smoke

Executable under test:

```text
D:\agent-desk-target\release\agent-desk.exe
SHA-256: D0AD9CA0F02E638768F914FEFE4F9E119366F45E38EF7164647EDD43769A73E8
```

Isolation:

```text
SQLite / Store / window state: D:\agent-desk-m2c-smoke-10aa275\data
WebView2 profile: D:\agent-desk-m2c-smoke-10aa275\webview2
Product Owner AppData: not used by the app; read-only aggregate audit unchanged
```

Completed native checks:

- Exact canonical process stayed alive >10 seconds and exposed the native `Agent Desk` window: PASS.
- WebView2 rendered current Reader, Header `收件`, Sticky, and Safe Shelf: PASS.
- Empty Inbox and fixed Shelf: PASS.
- A/B/C created through page-local Tauri `ingest_delivery` invoke (not direct SQL): PASS.
- Newest-first order C/B/A, source/type/local time, unread count 3: PASS.
- Open B through the native accessible item: PASS.
- B `opened_at` created; unread count 2; current document B; Reader visible; scrollTop 0: PASS.
- B selection → `保存到记录` through native UI; resulting note returned by Tauri Sticky command: PASS.
- Inbox returns with B `已读`, A/C `未读`: PASS.
- Mini Safe Shelf and Compact-over-Inbox layer behavior: PASS.
- The previously reported isolated PID `34252` no longer existed when the Gate resumed; no process was terminated under that PID: PASS.
- Open A from Inbox; current document A, Reader visible, scrollTop 0: PASS.
- Blank Reader → Inbox → exact Header `阅读`; Reader remained Blank and `readerContentVisible=false`: PASS.
- Blank Reader → Inbox → open C; Reader became visible with current document C and scrollTop 0: PASS.
- Duplicate C with the same idempotency key and identical payload returned the original IDs with `created=false`: PASS.
- Same C key with a changed payload returned exactly `IDEMPOTENCY_CONFLICT` and preserved the original: PASS.
- Before restart: 3 Deliveries, 3 unique Delivery IDs, 3 unique linked ReaderDocument IDs, unread count 0: PASS.
- Restart with the same isolated SQLite/Store/WebView2 profile: PASS.
- After restart: all 3 Deliveries and their opened timestamps persisted; unread count 0; current document C; Reader visible: PASS.
- No duplicate Delivery or ReaderDocument was introduced; the document repository contains the 3 delivery documents plus 1 built-in document: PASS.
- Resumed smoke PID `41256` was stopped before restart; restart PID `32148` was stopped after verification: PASS.
- Final process audit found 0 `agent-desk.exe` and 0 WebView2 processes referencing the isolated smoke profile: PASS.

Product Owner AppData was protected by directing all runtime writes to the isolated paths above. Aggregate file-manifest SHA-256 values were identical immediately before and after the resumed native audit:

| Product Owner path | Files | Before | After |
| --- | ---: | --- | --- |
| `%APPDATA%\\com.agentdesk.desktop` | 3 | `89341C50A3209782EAEB76F3651AE90117DF39C11104CAE6B1BF16351D77E523` | `89341C50A3209782EAEB76F3651AE90117DF39C11104CAE6B1BF16351D77E523` |
| `%LOCALAPPDATA%\\com.agentdesk.desktop` | 400 | `C0C0E5583872B88FB45D0F96DCF8ACE83EE66E42FD9EDB700B4DA0F5D309A11` | `C0C0E5583872B88FB45D0F96DCF8ACE83EE66E42FD9EDB700B4DA0F5D309A11` |

Native status: **PASS**.

## Q. Windows/macOS CI

The first post-merge run, [CI #27](https://github.com/dawnsongbest-create/agent-desk/actions/runs/32363325541), failed on both platforms at `pnpm format`. Root cause: the preserved public README-only commits had not been formatted by the repository's pinned Prettier configuration. No product code failed. The minimal fix changed only README formatting and was committed as `52e75dacd13891a36f0c7d73b69f5f886122ad1f`.

The replacement [CI #28](https://github.com/dawnsongbest-create/agent-desk/actions/runs/32363880138) ran against that exact commit and completed successfully:

| Job | Job ID | Result |
| --- | ---: | --- |
| `windows-latest` | `96409095544` | **PASS** |
| `macos-latest` | `96409095689` | **PASS** |

Both jobs used Node `22.23.2`, pnpm `11.16.0`, frozen install, frontend format/typecheck/lint/test/build, Rust fmt/Clippy/tests, and a real non-bundled Tauri build.

## R. Product Preview Evidence

All images below came from the exact canonical SHA above with isolated fixture data:

| Evidence | SHA-256 |
| --- | --- |
| `docs/evidence/m2-c/01-empty-inbox.png` | `EBAD92CE42AE49484E5FE168C005FC0E1829530FE6F3726541E7919B15F36725` |
| `docs/evidence/m2-c/02-inbox-three-unread.png` | `7BE1333EA19A2E4E4CC0CA3CA1D2E0B077EC5DCA7524ABB54DE1226E3AFC654C` |
| `docs/evidence/m2-c/03-open-delivery-b-reader.png` | `B49A1E86E6B7D93E30EEF5FC1340272CD75E435E1899FAF94373DBF6A34E75BD` |
| `docs/evidence/m2-c/04-inbox-b-opened-mini.png` | `7B7EE9A8E9BF5819AE36B7BE9A688A18BC05BD0E70C46C10ED49C165D3BD221A` |
| `docs/evidence/m2-c/05-inbox-compact-overlap.png` | `B52667A5EF5FA35EEB81C3991ED5D72D20D2F27C5AFB80242A860B932D15D2DD` |
| `docs/evidence/m2-c/06-open-delivery-a-reader.png` | `A25041E2B37408D381262856519418FADD527DA64A136811C896C0BF712A8103` |
| `docs/evidence/m2-c/07-blank-inbox-reading-return.png` | `789B8C90FE7B7ED8C4D36D4068A71B8C27B31A2E38D39AF5F0C57ACBBD3D01F3` |
| `docs/evidence/m2-c/08-open-c-restores-reader.png` | `30671ED1BFC3A64E530B3F3DCE971A558ABBDD80AAD9338F26C79B58F9DE8A3A` |
| `docs/evidence/m2-c/09-restart-persistence.png` | `33406876F05C1211FF1828AE0529BCB130652BAE26AA0DD0ABE343A47B463911` |

The A/B/C smoke fixture used literal PowerShell escape markers in its Markdown, which rendered visible `n` characters in delivery screenshots. This is fixture construction noise, not stored-content truncation or a Reader renderer defect; production and automated Markdown fixtures use real line breaks.

## S. Regression Results

Automated Sticky and Reader regression suites: PASS. Native Reader, Sticky, Mini, Compact, selection-capture, Delivery handoff, Blank interaction, idempotency, restart persistence, and final process shutdown checks all pass.

## T. Known Technical Issues

- No known M2-C product-code failure is currently reproduced.
- The native smoke fixture escaping issue affects delivery-screenshot presentation only.
- Agent Gateway and all M3 capabilities remain intentionally absent.

## U. Build Provenance

```text
Implementation commit: 10aa2751ea1751698b9433e9b06ee1c1832e5879
Public README merge commit: b9ca47cc3d177b229c9cf42bb1197a24e2851002
Final handoff commit: 52e75dacd13891a36f0c7d73b69f5f886122ad1f

Frontend bundles:
dist/index.html
  13ED91EACC70B31B7715AD636473AF6559DCA1F9A60A1C004CCDE38882FB07E0
dist/assets/index-D-LRs5dO.css
  8039FC80FA60E6279F93F22F0DD7740B48947E4C0A2EB9350035BAF9B23DDED4
dist/assets/index-D2IVfb_g.js
  0D28BA09F1FFC997880D8E2144958AE2B5429FBCE4F8C32534CC8DDDD9F18E06

Truth build:
D:\agent-desk-m2c-truth-10aa275\release\agent-desk.exe
  D0AD9CA0F02E638768F914FEFE4F9E119366F45E38EF7164647EDD43769A73E8

Canonical executable:
D:\agent-desk-target\release\agent-desk.exe
  D0AD9CA0F02E638768F914FEFE4F9E119366F45E38EF7164647EDD43769A73E8

Truth SHA = Canonical SHA: PASS
Product Preview path: D:\agent-desk-target\release\agent-desk.exe
CI run: 32363880138 (run 28) — Windows PASS / macOS PASS
```

Build used Node `22.23.2`, pnpm `11.16.0`, Cargo.lock with `--locked`, and a new isolated Cargo target. The release build did not reuse the prior canonical artifact.

## V. Git State

At the CI-validated handoff revision:

```text
branch: main
HEAD: 52e75dacd13891a36f0c7d73b69f5f886122ad1f
origin/main: 52e75dacd13891a36f0c7d73b69f5f886122ad1f
implementation commit: present in HEAD ancestry and on origin/main
working tree: clean before this final report-only update
repository visibility: Public by explicit Product Owner decision
```

The report itself is the immediately following documentation-only provenance commit. Final post-push ref equality and CI are verified after that commit rather than self-referencing its SHA inside its own content.

## W. Public Release Safety

- Current `main` tree high-confidence secret signatures: none.
- Credential assignment hints: none.
- Literal personal user-directory paths: none; three legacy Gate Report paths were normalized to `%USERPROFILE%` / `%LOCALAPPDATA%` in the public merge.
- Tracked `.env`, SQLite/database files, executables, libraries, archives, `node_modules`, Rust `target`, frontend `dist`, coverage, or local pnpm store: none.
- Large tracked files above 5 MiB: none.
- `.gitignore` covers `.env`, `.env.*`, `node_modules`, `.pnpm-store`, and `dist`.
- Native screenshots contain only isolated A/B/C smoke fixtures and no Product Owner data.
- Existing historical commit identities were preserved; no history rewrite or force-push was performed.

No M3 implementation has started.
