# Agent Desk — M2-B Gate Report

## A. Implementation Summary

M2-B completes the three locked M2-A polish items and replaces the Reader presentation fixture with a durable, ID-driven ReaderDocument path. Valid Reader text selections now expose a quiet two-action popover for exact copy or atomic save into the existing 记录 domain with a separate source reference. No M2-C, Inbox, Delivery, Agent, drag-to-Mini, highlight, annotation, search, progress, or AI feature was added.

## B. M2-A Product Polish

- Top safe area changed from `clamp(136px, 31vh, 190px)` to `clamp(124px, 24vh, 148px)` with a 128px short-height guard.
- At the priority 320×568 viewport the measured safe area is 136.3125px. Mini occupies y=75.19–122.81px; the real Reader header starts at y=136.31px. Mini has no intersection with metadata, H1, or subtitle.
- Compact ↔ Mini transitions preserve the Sticky top edge, keeping the reduced safe area reliable rather than moving Mini downward into the header.
- Grid opacity increased only from Light 5.6% to 6.2% and Dark 4.5% to 5.0%; line width, spacing, and saturation were not changed.

## C. ReaderDocument Domain

The minimal v1 model contains `id`, `documentType`, `title`, optional `subtitle`, `contentMarkdown`, `sourceType`, optional `sourceLabel`, `createdAt`, and `updatedAt`. Supported document types are article/brief/reading/report; source types are local/builtin/agent. No future delivery, cursor, annotation, summary, scheduler, or progress fields were introduced.

The former React fixture is now a stable builtin Markdown asset, bootstrapped once through the Rust service only when no ReaderDocument exists. The production UI does not import Reader Markdown directly.

## D. Persistence / Migration

`0004_reader_documents.sql` is append-only and creates only:

- `reader_documents` for durable Reader content;
- `record_source_refs` for one minimal Reader-selection provenance relation per existing note Card.

Historical `0001`, `0002`, and `0003` are byte-unchanged. Fresh migration tests apply 0001→0004. The 0003→0004 upgrade fixture proves existing 记录, Todo, Quote, placements, and preference-independent data remain intact. Legacy checksum reconciliation was not widened.

## E. ReaderDocument Repository

The SQLite repository implements create, get by ID, and list for multiple documents. Full update was deliberately omitted because M2-B has no Reader edit scenario. Repository tests cover exact Markdown round trips, list, restart persistence, Chinese, English, code content, and missing-document rollback.

## F. Current Document State

Reader rendering is driven by `currentDocumentId`. `currentReaderDocumentId` is the only Reader navigation state stored in preferences; Reader body content remains in SQLite. Startup resolves the stored ID, falls back to the first durable document, or bootstraps stable ID `reader_builtin_foundation_v1`. Native restart restored that ID and exact content.

## G. Reader Selection Detection

Selection detection uses the browser Selection/Range boundary but accepts only non-whitespace selections whose anchor and focus nodes both belong to `.reader-content`. Sticky, Appearance, and other external UI selections are rejected. The exact `Selection.toString()` value is retained. Scroll, resize, and lost/changed selection close the popover. Code block and multi-paragraph plain-text selections remain supported.

## H. Selection Popover

The fixed, viewport-clamped paper-style popover contains only `复制` and `保存到记录`. It prefers the selection's upper edge and falls below when required. Its z-index stays below Sticky, so it does not create click-through behavior under Compact/Expanded Sticky. It has keyboard focus styling, status semantics, error retry, and guarded timers so rapid subsequent selections are not closed by stale feedback.

## I. Copy

Copy runs only from the explicit user click through the secure WebView Clipboard API. No hidden textarea, `execCommand`, or other DOM hack is used. Native verification copied the exact 15-character Chinese selection `阅读并不总是为了更快得到答案。`; system clipboard readback matched exactly. Feedback briefly changes to `已复制` and does not interrupt Reader.

## J. Save to Record

`保存到记录` creates an existing note Card/记录 whose body is the exact selected text. It does not summarize, retitle, normalize punctuation, flatten line breaks, truncate, open Sticky, move focus, or scroll Reader. Success briefly shows `✓ 已保存到记录`; failure preserves the selection and allows retry. The created Card is inserted into the existing Sticky 记录 state and keeps edit/delete/export behavior.

Native verification saved a four-line Chinese selection, opened it from Sticky → 记录, and read exact equality from the existing editor. A code selection `return moment.question.trim();` was also saved. After process restart both records were present and the multiline body reopened exactly.

## K. Source Provenance

Record creation, note payload, placement, and `record_source_refs` insertion occur in one SQLite transaction. The relation stores `record_id`, `document_id`, source type `reader_selection`, exact `selected_text`, document-title snapshot, and `captured_at`. Provenance is not injected into the Record body. Missing-document tests prove the entire Record creation rolls back rather than leaving an orphan clipping.

## L. Record Chinese Terminology

All user-facing Record/Records/New Record labels in the touched product surfaces now use `记录` / `新记录`, including editor and export text. Internal `RecordEditor`, note Card type, repository symbols, migrations, and historical data remain unchanged.

## M. Architecture

Reader content follows:

```text
React Reader Feature
→ Reader Application Port
→ Tauri Infrastructure
→ Rust Reader Service
→ ReaderDocumentRepository
→ SQLite
```

Selection capture follows:

```text
Reader Selection
→ capture_reader_selection
→ Reader Service validation
→ atomic Record + SourceRef repository transaction
→ existing Sticky 记录 state
```

The React best-practices review influenced the final implementation by keeping transient selection/timer state in refs, using one bounded listener lifecycle, avoiding inline component declarations, preserving keyboard focus states, and preventing stale timer races.

## N. Automated Tests

All local gates passed with Node 22.23.2, pnpm 11.16.0, locked/offline Cargo dependencies, and MSVC x64:

| Gate | Result |
| --- | --- |
| Prettier | PASS |
| TypeScript typecheck | PASS |
| ESLint | PASS |
| Frontend tests | PASS — 59/59 |
| Vite production build | PASS — 210 modules |
| Rust fmt | PASS |
| Rust Clippy `-D warnings` | PASS |
| Rust tests | PASS — 26/26 |
| Tauri release `--no-bundle --locked --offline` | PASS |

Selection tests cover empty, whitespace, Reader-only bounds, Sticky rejection, top/right clamping, scroll, lost selection, Chinese, English, multiline, code, exact copy/save, error retry, and stale timer behavior. Repository tests include short/multiline/English/code/500+ capture content, exact Record/SourceRef equality, transaction rollback, and restart persistence. Existing Mini, Compact, Expanded, long Record save, Todo, Quote, export, Reader typography/preferences, and preset tests remain green.

The only emitted Rust test message was MSVC linker's informational library/object creation stdout; Clippy produced no warning and exited successfully.

## O. Windows Native Smoke

The exact canonical Tauri/WebView2 executable at `http://tauri.localhost/` passed:

1. 320×568 reduced-safe-area geometry and Light/Dark grid inspection;
2. durable builtin ReaderDocument ID and model-driven header;
3. real mouse Reader selection and visible two-action popover;
4. exact native clipboard equality;
5. multiline save confirmation without opening Sticky;
6. existing Sticky → 记录 list and exact editor body;
7. code selection capture;
8. full process restart with one ReaderDocument and both records restored;
9. canonical process remained alive for 28.3 seconds.

The isolated fixture used `D:\agent-desk-m2-b-smoke-data`. Product Owner AppData was never launched for testing and stayed read-only. Before/after SHA-256 values were identical:

| Product Owner file | SHA-256 |
| --- | --- |
| `.window-state.json` | `BB25845431F97286176B1C0EA6BA369A82A91C87775DAA7E4F40AC0B8F497704` |
| `agent-desk.sqlite3` | `8C7F505E8997DF1B1EAB2755BED9A0DBE3735D90D9D874727C4AC0502C8EF031` |
| `preferences.json` | `B1F9D9A55C8EB30F9137AC48A943F5CB89A528979B6E44C8D6B2460800FBDDDC` |

## P. Windows/macOS CI

GitHub Actions run [31877990812](https://github.com/dawnsongbest-create/agent-desk/actions/runs/31877990812) is bound to Final Handoff Commit `cd480d425f201f07c8699f589858df84930b5f73`.

| Job | Result | Completed UTC |
| --- | --- | --- |
| [macos-latest](https://github.com/dawnsongbest-create/agent-desk/actions/runs/31877990812/job/94996419629) | PASS | 2026-08-15 09:52:06 |
| [windows-latest](https://github.com/dawnsongbest-create/agent-desk/actions/runs/31877990812/job/94996419656) | PASS | 2026-08-15 09:57:04 |

## Q. Product Preview Evidence

Every file below came from executable SHA-256 `0D3B5739CA255E619AE45AFC56F29B34F55884483BDCE4953F2C8BF1BA7BF6B0` and source commit `cd480d425f201f07c8699f589858df84930b5f73`:

- [Light grid + reduced safe area](evidence/m2-b/windows/01-light-reduced-safe-area.png)
- [Dark grid](evidence/m2-b/windows/02-dark-grid.png)
- [Real Reader selection](evidence/m2-b/windows/03-trusted-reader-selection.png)
- [复制 / 保存到记录 popover](evidence/m2-b/windows/04-selection-popover.png)
- [Multi-line selection](evidence/m2-b/windows/05-multiline-selection.png)
- [✓ 已保存到记录](evidence/m2-b/windows/06-saved-feedback.png)
- [记录 / 新记录 and clipping list](evidence/m2-b/windows/07-record-terminology-and-list.png)
- [Open clipping in existing Record editor](evidence/m2-b/windows/08-open-clipping-record.png)
- [Code selection](evidence/m2-b/windows/09-code-selection.png)
- [Restart persistence](evidence/m2-b/windows/10-restart-persistence.png)
- [Native smoke data](evidence/m2-b/windows/native-smoke.json)
- [Restart data](evidence/m2-b/windows/restart-smoke.json)
- [Build provenance](evidence/m2-b/windows/provenance.json)

## R. Backlog — Selection → Mini

BACKLOG ONLY — NOT IMPLEMENTED IN M2-B:

```text
Reader Selection
→ drag selected text
→ Mini enters receive state
→ drop
→ confirm
→ save to 记录
```

M2-B contains no Mini drop target, drag payload, receive animation, or `放这里` affordance.

## S. Known Technical Issues

- ReaderDocument update UI/API is intentionally absent because M2-B has no document-editing scenario; create/get/list and durable current-document resolution are ready for later delivery work.
- SourceRef is durable but has no return-to-source UI or robust semantic anchor. That is intentionally outside this Gate.
- Clipboard failure depends on WebView/system policy and is surfaced as a retryable local error; selection/save state remains unaffected.
- Expanded Sticky remains intentionally translucent over Reader. This is the accepted M2-A layer relationship, not a selection click-through path.

## T. Build Provenance

| Item | Value |
| --- | --- |
| Implementation Commit | `cd480d425f201f07c8699f589858df84930b5f73` |
| Final Handoff Commit | `cd480d425f201f07c8699f589858df84930b5f73` |
| Frontend CSS SHA-256 | `E1355FF421FF182CFA28BE6AAB1A75C9CBD3C9A4E7C33D8ADE410EB308A6310C` |
| Frontend JS SHA-256 | `E219F8BE445301E911B78F73E8FA63813B6DD733120CF961E4DA5859CA6F0FC1` |
| Truth Build | `D:\agent-desk-m2-b-truth-target\release\agent-desk.exe` |
| Truth/Canonical executable SHA-256 | `0D3B5739CA255E619AE45AFC56F29B34F55884483BDCE4953F2C8BF1BA7BF6B0` |
| Executable size | 14,153,728 bytes |
| Executable timestamp | 2026-08-15 09:21:08 UTC |
| Canonical / Product Preview | `D:\agent-desk-target\release\agent-desk.exe` |
| CI Run | `31877990812` |

The truth target did not exist before the build. `pnpm tauri build --no-bundle -- --locked --offline` produced the executable from the current production bundle. After verification, that exact file was copied to canonical and both SHA-256 values were recomputed equal.

## U. Git State

- Branch: `main`
- Product source HEAD / origin main: `cd480d425f201f07c8699f589858df84930b5f73`
- Historical migrations: unchanged
- Product source working tree: clean after the implementation commit; only this Gate report/evidence remained for the documentation commit
- Remote: `origin https://github.com/dawnsongbest-create/agent-desk.git`

## Product Review Questions

1. Safe Area 现在是否更自然？
2. Grid 深度是否合适？
3. 「记录」中文化是否自然？
4. Selection popover 是否够轻？
5. 复制 / 保存到记录 是否容易理解？
6. 保存后不打断阅读的体验是否喜欢？
7. 保存到记录后的内容是否符合预期？
8. 是否准备进入 M2-C Delivery / Inbox？

```text
M2_B_GATE_STATUS: AWAITING_PRODUCT_AND_TECH_REVIEW
READER_DOCUMENT_STATUS: FOUNDATION_READY
SELECTION_CAPTURE_STATUS: READY
```
