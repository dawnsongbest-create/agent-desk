# M3-A Gate Report — Reading Agent Experience & Delivery Loop

Gate status at this revision:

```text
M3_A_GATE_STATUS: AWAITING_PRODUCT_AND_TECH_REVIEW
READING_AGENT_STATUS: FOUNDATION_READY
AGENT_GATEWAY_STATUS: NOT_STARTED
```

M3-A implements and verifies the first local Reading Agent delivery loop. No Agent Gateway, remote adapter, PDF parser, cloud service, notification, scheduler, or M3-B work was started.

## A. Delivered Scope

- Added the ReadingPlan and ReadingSession domains through append-only migration `0006_reading_plan.sql`.
- Added plan creation, status, persisted progress, Chinese/English/technical reading-time estimation, and deterministic daily section generation.
- Reused the existing Delivery application service for atomic ReaderDocument + Delivery creation.
- Added Reading metadata to Inbox and Reader without redesigning either surface.
- Added Reader selection action `加入今日阅读`, which creates a ReadingSession and opens its new local ReaderDocument.
- Preserved the existing `保存到记录` flow and its source relationship.
- Added Header `收件 N` polish and a 36 px Sticky collapse hit area with a 16 px visual icon.

## B. Architecture Truth

```text
ReadingPlan + progress
  → ReadingService.generate_today
  → DeliveryService.ingest
  → ReaderDocument + Delivery (one transaction)
  → Inbox
  → Reader

Reader selection
  → ReadingService.create_session
  → ReadingSession + ReaderDocument (one transaction)
  → Reader open
```

Reading Agent is a local producer of the existing Delivery boundary. It does not write Delivery rows directly, and React does not access SQLite.

## C. Migration and Persistence

- `0006_reading_plan.sql` is append-only; `0001`–`0005` were not modified.
- Fresh native database applied migrations `[1, 2, 3, 4, 5, 6]`, all successful.
- Restart truth: 1 completed ReadingPlan, 1 opened Delivery, 1 plan/delivery link, 1 sourced Record, 1 ReadingSession, and 3 ReaderDocuments.
- The generated delivery and session document are distinct; no duplicate Delivery, plan link, Record, or ReadingSession was created.

## D. Automated Gates

- Node `22.23.2`, pnpm `11.16.0`: exact versions.
- Frontend format, typecheck, lint, production build: PASS.
- Frontend Vitest: 77/77 PASS.
- Rust fmt and Clippy with `-D warnings`: PASS.
- Rust tests: 40/40 PASS.
- Tests cover plan create/status/persistence, time estimation, Delivery-service generation, Inbox/open behavior, selection copy/save/session, migration/restart, and prior Sticky/Reader/Delivery behavior.

The first hosted run found one Rust 1.98 Clippy-only warning in the pre-existing checksum helper. Commit `8df03f3ef39de27fe311e996b449a7f54b68869b` made the equivalent minimal helper expression Clippy-clean; no product behavior changed.

## E. Native Windows Smoke

The exact canonical native executable was launched with isolated SQLite, Store, window state, and WebView2 profile. Product Owner AppData was never supplied to the process.

Verified through the real Tauri/WebView2 UI:

1. Launch remained alive beyond 10 seconds; Mini Sticky pinned to the Safe Shelf.
2. Create `M3-A 隔离原生验收` ReadingPlan.
3. Generate today's reading; Inbox displayed exactly 1 unread Reading delivery.
4. Open Delivery; Reader displayed `今日阅读 · Day 1 · 预计 1 分钟`; unread became 0.
5. Select the exact visible Chinese text and click `保存到记录`; exactly 1 Record and 1 source reference existed.
6. Select the text again and click `加入今日阅读`; Reader opened `今日阅读 · M3-A 隔离原生验收`; exactly 1 ReadingSession existed.
7. Click `完成阅读`; the plan became `completed` at 100%.
8. Stop the first PID, relaunch the same isolated profile, and verify the current ReadingSession document, plan, opened Delivery, Record, unread count, and all counts persisted.
9. Stop the restart PID; final audit found 0 `agent-desk.exe` and 0 WebView2 processes referencing the smoke profile.

Native status: **PASS**.

## F. Product Owner Data Safety

Read-only before/after audits matched exactly:

| Product Owner path | Files | Bytes | Metadata SHA-256 before/after | Content SHA-256 before/after |
| --- | ---: | ---: | --- | --- |
| `%APPDATA%\com.agentdesk.desktop` | 3 | 168,664 | `48DDAE43AD3AF7C2B935B667FC153D17853403E772C777BD882190583AC2F317` | `B377C41B5CD1ECF29C8B67A580FB70881EE38456DD367A54C7226DFCBC6333FA` |
| `%LOCALAPPDATA%\com.agentdesk.desktop` | 398 | 60,971,632 | `0657E850A9F0A4B6CC880D18A84EFDE36D52751B3534D46F4199043A27E2178A` | `3A6CE836B78448483C3C704012BB510A091CD54E36A894292BD1A2AA0302B388` |

No Product Owner Record, Todo, Quote, SQLite row, preference, window state, or WebView2 profile changed during the Gate.

## G. Regression

- M1 Sticky Mini/Compact/Expanded, Todo, Record, Quote, Markdown export, drag/snap, circular completion, long text, and page-turn behavior: automated suite PASS.
- M2 Reader Markdown, Blank Reader, Safe Shelf, selection copy/capture, Inbox, unread semantics, Delivery handoff/idempotency/persistence: automated suite PASS.
- Native Mini/Safe Shelf, Reader, Inbox, selection capture, restart, and process shutdown: PASS.

## H. Build Provenance

```text
Implementation commit:
30324fee8f6d78a8452a0a1b9c343e7d49f6c784

Executable truth commit:
8df03f3ef39de27fe311e996b449a7f54b68869b

Truth executable:
D:\agent-desk-m3a-truth-8df03f3\release\agent-desk.exe

PRODUCT_OWNER_PREVIEW_EXECUTABLE:
D:\agent-desk-target\release\agent-desk.exe

Truth SHA-256 = Canonical SHA-256:
BFB1F90BD6A36ABFE86C50A2B8C92182C8A48481A03BEDFDADC0A59F2B305FFB

Size: 14,687,744 bytes
Timestamp UTC: 2026-08-21T05:52:35.1194254Z
```

Frontend production bundle SHA-256:

- `dist/index.html`: `02C8CFE6FF13EAD2814E51C6952323331A201089586B7749A17B528F0CFCD26A`
- `dist/assets/index-Bx6m9v8Q.css`: `13AB27F97443B312009F6BDC0131748E109C0C9CB13447FAA8C4384595426FB3`
- `dist/assets/index-CdqQL-fR.js`: `15B1D640F69AAD38A1A02EE6D8FC63A438A27015D6C7A5ADEB0B8333767562CF`

The JavaScript production bundle contains the Reading Agent, ReadingPlan, `生成今日阅读`, and `加入今日阅读` paths. The truth build used a new isolated Cargo target and was copied byte-for-byte to the canonical path before native verification.

## I. Hosted CI

[GitHub Actions run 32451905186](https://github.com/dawnsongbest-create/agent-desk/actions/runs/32451905186) ran on truth commit `8df03f3ef39de27fe311e996b449a7f54b68869b`:

| Job | Job ID | Result |
| --- | ---: | --- |
| `windows-latest` | `96681813573` | **PASS** |
| `macos-latest` | `96681813694` | **PASS** |

Both jobs used the pinned Node/pnpm versions, frozen dependency install, frontend checks/tests/build, Rust fmt/Clippy/tests, and a real non-bundled Tauri build. The immediately following report/evidence-only handoff commit is rechecked by the same hosted workflow; its final run is recorded in the external Gate handoff.

## J. Evidence

Native images and machine-readable truth are under `docs/evidence/m3-a/windows/`:

- `01-plan-created.png`
- `02-inbox-reading-delivery.png`
- `03-reader-delivery.png`
- `04-selection-saved-to-record.png`
- `05-reading-session.png`
- `06-reading-completed.png`
- `07-restart-persistence.png`
- `native-smoke-runtime.json`
- `provenance.json`

Every screenshot contains only isolated fixture content and comes from canonical executable SHA `BFB1F90BD6A36ABFE86C50A2B8C92182C8A48481A03BEDFDADC0A59F2B305FFB`.

## K. Public Repository Safety

- Repository visibility remains **Public** by Product Owner decision; visibility was not changed.
- No secrets, credentials, API keys, tokens, `.env`, personal user-directory paths, user databases, build artifacts, runtime profiles, or Product Owner data are included.
- Evidence contains only the explicit `M3-A Native Fixture` isolated content.
- No history rewrite, force push, dependency upgrade, M3-B feature, or gateway work occurred.

## L. Git Handoff

The implementation and truth-fix commits are present in `main` ancestry and on `origin/main`. This report and evidence form the next documentation-only handoff commit. Final local/remote ref equality, clean working tree, and post-handoff CI are verified after push so the report does not self-reference a commit that does not yet exist.

```text
M3_A_GATE_STATUS: AWAITING_PRODUCT_AND_TECH_REVIEW
READING_AGENT_STATUS: FOUNDATION_READY
AGENT_GATEWAY_STATUS: NOT_STARTED
```

Stop condition reached. Do not enter M3-B.
