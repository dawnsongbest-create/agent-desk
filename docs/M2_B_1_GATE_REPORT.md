# Agent Desk — M2-B.1 Gate Report

Gate: **Pinned Mini Safe Shelf & Blank Reader Mode**  
Implementation / Final Handoff Source Commit: `4107da4e5507b543a0135cd92eae4400a2475815`

## A. Implementation Summary

- Replaced the scrolling Reader top spacer with a fixed, non-scrolling Safe Shelf.
- Split Reader into a fixed Shelf and an independent `.reader-scroll-viewport`.
- Pinned Mini to the right side of the Shelf while preserving Compact's saved normalized position.
- Added the quiet Chinese control `隐藏正文 / 显示正文`.
- Added preference-backed Blank Reader Mode without touching ReaderDocument or SQLite.
- Preserved same-session Reader scroll position across hide/show.
- Closed the selection popover and cleared the DOM selection when Reader content is hidden.
- Kept Sticky at layer 2 above the Shelf and Reader at layer 1.

No M2-C, Delivery, Inbox, Agent, Progressive Reading, or new Sticky feature was implemented.

## B. Product Problem — Mini / Reader Overlap

M2-B placed its safe space inside the scrolling content. Once that spacer scrolled away, body text could enter the fixed Mini region and be obscured. M2-B.1 removes that geometry conflict: the Shelf and Mini stay outside the scroll container, and the Reader viewport begins at or below the Shelf's bottom edge for the entire scroll range.

## C. Safe Shelf Architecture

The production hierarchy is now:

```text
desk-board
├── Reader Canvas (layer 1)
│   ├── reader-safe-shelf             fixed / non-scrolling
│   │   ├── reader-visibility-control
│   │   └── reader-mini-shelf-slot
│   └── reader-scroll-viewport        independent scrolling
│       └── ReaderDocument presentation
└── sticky-overlay-layer (layer 2)
    ├── Mini pinned to the Shelf geometry
    ├── Compact
    └── Expanded
```

Shelf geometry is 60 px at the 320×420 height and 64 px for the larger presets. The old `.reader-top-safe-area` scrolling spacer no longer exists.

## D. Mini Pinned Behavior

Mini uses `data-pinned-to-shelf="true"` and fixed Shelf variables rather than the persisted free-position style. Pointer drag is disabled only for pinned Mini; click and keyboard activation continue to restore Compact. The open-Todo count remains live.

Native measurements at iPhone 5 size remained identical at scrollTop 0, 300 and 1000:

- Shelf: top 76, bottom 140
- Mini: top 85, bottom 131
- Reader viewport: top 140
- Mini/Reader overlap: false

## E. Compact / Mini Position Separation

Compact remains draggable and uses the existing normalized `stickyPosition`. Compact → Mini now changes only `stickyMode`; it does not rewrite `stickyPosition`. Mini ignores that free position while pinned. Mini → Compact therefore restores the original Compact coordinates without contamination from Shelf placement.

Automated tests cover both transitions, click restoration, count rendering, pinned no-drag behavior, and the absence of position writes during mode changes.

## F. Reader Hide / Show

The Shelf button exposes direct Chinese terminology:

- visible: `隐藏正文`
- hidden: `显示正文`

The control is low-contrast text with stronger hover/focus treatment; it does not introduce a Reader toolbar. Font-size and line-spacing controls remain in Appearance.

## G. Blank Paper Mode

When hidden, the entire ReaderDocument presentation is removed from the interactive DOM: metadata, subtitle, title, headings, paragraphs, lists, quote, code and links. The app header, date, Appearance, grid, paper texture, Shelf, `显示正文`, and all Sticky forms remain available. No empty state, illustration, or onboarding is rendered.

The Reader scroll viewport becomes non-scrollable and has no multi-thousand-pixel blank range. Native Blank evidence reported:

```text
article=false
title=false
popover=false
selectionRangeCount=0
viewport overflow-y=hidden
grid paper=true
show control=true
```

## H. Scroll Position Restoration

The viewport's scrollTop is captured in an ephemeral React ref immediately before hiding. On show, a layout effect restores it on the next animation frame. This is presentation-only session state; no `local_read_position`, progress model, cursor, or SQLite row was added.

Native result: `732.8 → hide → show → 732.8`, delta `0`.

## I. Selection Cleanup

Hide performs all of the following without invoking Copy or Capture:

- cancels any pending popover timer;
- removes the selection popover;
- calls `removeAllRanges()` on the active DOM selection;
- unmounts ReaderDocument presentation.

Native verification showed a popover before hide, no popover and zero ranges in Blank Mode, and a fresh selection popover after show. Automated tests also assert that neither clipboard nor Record capture is called during hide.

The required positive capture regression was run separately against the same isolated canonical instance. The UI returned `✓ 已保存到记录`; read-only SQLite inspection found Record `card_746677aa8495d72601ba30801295c5dc` and its `record_source_refs` row pointing to `reader_builtin_foundation_v1` with `source_type=reader_selection` and exact selected text `Agent De`.

## J. Preference Persistence

`readerContentVisible: boolean` follows:

```text
React
→ PreferencesPort
→ Tauri command
→ tauri-plugin-store preferences.json
```

The TypeScript and Rust defaults are `true`. Legacy preference JSON defaults to visible. The isolated store was observed on disk with `readerContentVisible=false` and `stickyMode=mini`; a full process restart restored Blank + Mini while keeping `currentDocumentId=reader_builtin_foundation_v1`. Clicking `显示正文` restored the unchanged document.

## K. Z-index / Sticky Coexistence

Verified ordering:

```text
Sticky overlay = layer 2
Reader/Shelf   = layer 1
paper          = background
```

Dragging/restoring Compact over the visibility control made `elementFromPoint` resolve inside `.sticky-surface`, proving the control does not click through Sticky. Blank → Compact → Expanded was also exercised natively: Expanded remained visible, the 记录 tab contained the isolated clipping, and `便签一句` / `新记录` remained available.

## L. Four Presets

| Preset | Native viewport | Shelf | Mini | Reader viewport top | Overlap / overflow |
| --- | ---: | ---: | ---: | ---: | --- |
| Sticky | 320×420 | 70–130 (60 px) | 77–123 | 130 | none |
| iPhone 5 | 320×568 | 76–140 (64 px) | 85–131 | 140 | none |
| Pocket | 360×640 | 76–140 (64 px) | 85–131 | 140 | none |
| Book | 420×595 WebView client | 76–140 (64 px) | 85–131 | 140 | none |

The Book preset was requested as 420×594 logical pixels. At Windows devicePixelRatio 1.25, WebView2 reported a 420×595 client viewport due to native DPI rounding; the preset model and Tauri request remain 420×594. All four presets had `horizontalOverflow=false`, `shelfBeforeReader=true`, `miniInsideShelf=true`, and `miniReaderOverlap=false`.

## M. Architecture Check

- ReaderDocument still flows through Reader Port → Tauri → Rust → SQLite.
- Reader visibility is a lightweight Store preference only.
- `currentDocumentId`, Reader Markdown, title, and source provenance are unchanged by hide/show.
- No new SQLite migration was added.
- `0001`–`0004` have zero Git diff and retain their frozen SHA-256 values.
- `record_source_refs` and `0004_reader_documents.sql` were not modified.

## N. Automated Tests

Environment: Node `22.23.2`, pnpm `11.16.0`, locked/offline Cargo dependencies.

| Check | Result |
| --- | --- |
| Prettier | PASS |
| TypeScript | PASS |
| ESLint `--max-warnings 0` | PASS |
| Frontend Vitest | PASS — 62/62 |
| Rust fmt | PASS |
| Rust Clippy `-D warnings` | PASS |
| Rust tests | PASS — 26/26 |
| Frontend production build | PASS |
| Tauri release `--no-bundle --locked --offline` | PASS |

New coverage includes Shelf structure, four presets, Mini pin/click/count/position isolation, Blank presentation, grid/Shelf/Sticky survival, selection cleanup, scroll restoration, default/legacy preferences, save persistence, and restart restoration.

## O. Windows Native Smoke

All Native evidence came from exact canonical executable SHA-256 `0B6025EA29A13B0882B473DE3A07341BBDAF8036F96930BA565AB91F0FB355D1`, not Vite.

Verified in real Tauri/WebView2:

1. visible Reader, fixed Shelf, pinned Mini and visibility control;
2. iPhone 5 scrolling at 0, 300 and 1000 with invariant Shelf/Mini geometry;
3. all four window presets and no horizontal overflow;
4. selection popover, exact selection capture and success feedback;
5. Blank presentation removal, grid preservation, Mini and Compact;
6. Compact covering the visibility control with the correct hit-test layer;
7. Expanded Sticky over Blank paper with Record/Quote/capture UI;
8. exact session scroll restoration and fresh post-show selection;
9. full process restart restoring Blank + Mini and unchanged ReaderDocument;
10. restarted process remained responsive for 26.6 seconds.

Fixture isolation:

- SQLite / Store / window state: `D:\agent-desk-m2-b1-smoke-data`
- WebView2 profile: `D:\agent-desk-m2-b1-webview2`

Product Owner AppData stayed read-only. Before/after hashes were identical:

| Product Owner file | SHA-256 |
| --- | --- |
| `.window-state.json` | `D8FAC80CCAE8B82F52ADAE7AF0B3C55DDD3AECFF30D7D601CEDF3709C3ABA5FC` |
| `agent-desk.sqlite3` | `1A94BC86178FC01DC70729BAFA90372E2E78ADAB6C028A5D2037E208FEBED638` |
| `preferences.json` | `EDAAC921CB10861F364D5832DC15E12628EDA765610B239D236F9CFD38FC543F` |

No Agent Desk process remained after smoke completion.

## P. Windows/macOS CI

GitHub Actions run [31882961247](https://github.com/dawnsongbest-create/agent-desk/actions/runs/31882961247) is bound to implementation / Final Handoff Source Commit `4107da4e5507b543a0135cd92eae4400a2475815`.

| Job | Result | Completed UTC |
| --- | --- | --- |
| [macos-latest](https://github.com/dawnsongbest-create/agent-desk/actions/runs/31882961247/job/95007989386) | PASS | 2026-08-15 11:50:08 |
| [windows-latest](https://github.com/dawnsongbest-create/agent-desk/actions/runs/31882961247/job/95007989387) | PASS | 2026-08-15 11:54:58 |

Both jobs ran frozen pnpm install, format, typecheck, lint, frontend tests, Rust fmt, Clippy with warnings denied, Rust tests, and real non-bundled Tauri build.

## Q. Product Preview Evidence

- [320×420 Shelf](evidence/m2-b1/windows/01-sticky-320x420-safe-shelf.png)
- [iPhone 5 — Shelf + Mini + Reader top](evidence/m2-b1/windows/02-iphone5-top.png)
- [iPhone 5 — middle scroll](evidence/m2-b1/windows/03-iphone5-middle.png)
- [iPhone 5 — scrollTop 1000](evidence/m2-b1/windows/04-iphone5-near-bottom.png)
- [Book Page Shelf](evidence/m2-b1/windows/05-book-safe-shelf.png)
- [Selection before Blank](evidence/m2-b1/windows/06-selection-before-blank.png)
- [Blank Reader + Mini + 显示正文](evidence/m2-b1/windows/07-blank-mini.png)
- [Blank Reader + Compact](evidence/m2-b1/windows/08-blank-compact.png)
- [Sticky covering visibility control](evidence/m2-b1/windows/09-blank-sticky-over-control.png)
- [Blank Reader + restored Mini](evidence/m2-b1/windows/10-blank-mini-restored.png)
- [Restored Reader content near prior scroll](evidence/m2-b1/windows/11-restored-content.png)
- [Fresh selection after restore](evidence/m2-b1/windows/12-selection-after-restore.png)
- [Restart restored Blank + Mini](evidence/m2-b1/windows/13-restart-blank-mini.png)
- [Restart show restored ReaderDocument](evidence/m2-b1/windows/14-restart-shown.png)
- [Selection saved to Record](evidence/m2-b1/windows/15-selection-saved-to-record.png)
- [Expanded Sticky over Blank paper](evidence/m2-b1/windows/16-blank-expanded-sticky.png)
- [Native metrics](evidence/m2-b1/windows/native-smoke.json)
- [Restart metrics](evidence/m2-b1/windows/restart-smoke.json)
- [Capture result](evidence/m2-b1/windows/capture-smoke.json)
- [Build provenance](evidence/m2-b1/windows/provenance.json)

## R. Regression Results

- M1 Sticky: Compact/Mini/Expanded, Record create/edit/save/delete/export, 6000+ character ordinary Save, Todo completion/due date/reorder/wheel boundary, Quote, page-turn behavior and three-line Compact Quote remain covered and passing.
- M2-A: Grid/paper values, typography, Reader font/line-spacing preferences, four presets, Light/Dark/System and overlay layering remain unchanged or passing.
- M2-B: durable ReaderDocument, `currentDocumentId`, Markdown safety/typography, selection Copy/Save, transaction rollback, SourceRef and restart behavior remain passing. Native selection capture created one isolated Record and exact SourceRef.

## S. Known Technical Issues

- Windows WebView2 reports the 420×594 logical Book request as a 420×595 client viewport at devicePixelRatio 1.25. This is a one-pixel native DPI rounding difference; Shelf and overflow invariants pass.
- One initial local full Rust test run produced a non-reproducible `trigger already exists` error in the legacy migration reconciliation fixture. The exact test immediately passed, a subsequent default full run passed 26/26, and both Windows/macOS CI Rust suites passed. No migration or production workaround was applied.
- The Blank Compact and “Sticky covering control” screenshots are pixel-identical because the restored saved Compact position was already the same clamped top-left position reached by the drag. The native hit test independently proved `stickyOwnsHit=true`.

No open product-code blocker remains for this Gate.

## T. Build Provenance

| Item | Value |
| --- | --- |
| Implementation Commit | `4107da4e5507b543a0135cd92eae4400a2475815` |
| Final Handoff Source Commit | `4107da4e5507b543a0135cd92eae4400a2475815` |
| Frontend CSS | `index-BCxVtCnP.css` — `45A19C484BE3373BEEC2A57088A4772187AE3AECEDA27B0004A8ED4D11CA357C` |
| Frontend JS | `index-CNSLW5pZ.js` — `29587F387E47BB4FDFD30D6276FD18FA6DBFB4BDBC2C07ECC8C5076D4AD57FC2` |
| Truth Build | `D:\agent-desk-m2-b1-truth-target\release\agent-desk.exe` |
| Canonical | `D:\agent-desk-target\release\agent-desk.exe` |
| Truth / Canonical SHA-256 | `0B6025EA29A13B0882B473DE3A07341BBDAF8036F96930BA565AB91F0FB355D1` |
| Size | 14,156,288 bytes |
| Built UTC | `2026-08-15T11:31:25.0101058Z` |
| Truth = Canonical | PASS |
| CI Run | `31882961247` — PASS |

The truth target did not exist before build. `pnpm tauri build --no-bundle -- --locked --offline` generated the executable from the recorded production bundles. The exact truth file was then copied to canonical and both SHA-256 values were recomputed equal.

`PRODUCT_OWNER_PREVIEW_EXECUTABLE: D:\agent-desk-target\release\agent-desk.exe`

## U. Git State

- Branch: `main`
- Implementation / Final Handoff Source: `4107da4e5507b543a0135cd92eae4400a2475815`
- Source commit pushed to `origin/main`: yes
- Remote: `https://github.com/dawnsongbest-create/agent-desk.git`
- Runtime source changes at handoff: committed
- Gate report/evidence: packaged in a documentation-only descendant commit
- Temporary native scripts/wrappers: removed
- Agent Desk processes: none
- M2-C work: not started

## Product Review Questions

1. Mini 固定在 Safe Shelf 后，滚动阅读是否不再挡字？
2. Safe Shelf 高度是否自然？
3. 隐藏正文 / 显示正文 是否容易理解？
4. Blank Reader 是否足够像一张干净的纸？
5. Sticky 在 Blank Reader 上使用是否自然？
6. 显示正文后恢复原阅读位置是否舒服？
7. 是否准备正式进入 M2-C Delivery / Inbox？

```text
M2_B_1_GATE_STATUS: AWAITING_PRODUCT_AND_TECH_REVIEW
MINI_SAFE_SHELF_STATUS: READY
BLANK_READER_MODE_STATUS: READY
```
