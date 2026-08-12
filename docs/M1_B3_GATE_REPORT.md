# Agent Desk — M1-B3 Sticky Deepening & Reader Coexistence Foundation Gate Report

Date: 2026-08-13

Branch: `main`

Scope: M1-B3 only

## A. Implementation Summary

- Expanded Sticky Record face now supports a single Sticky Quote, a Record list and a paper-native long Record editor.
- Compact Sticky is smaller and shows only open Todo items plus Sticky Quote; long Records no longer compete with future Reader content.
- Compact Sticky supports click-to-expand, free pointer drag, four-corner magnetic snap and normalized position persistence.
- Settings exposes Sticky, iPhone 5, Pocket and Book Page logical-size presets without restricting manual resize.
- Existing Note/Todo page-turn animation and sound are preserved unchanged.
- No Reader, Agent, Inbox, summarization, rich text or later-scope feature was implemented.

## B. Record Model Changes

The UI concept is now Record while the verified underlying CardType remains `note`. The first non-empty line is a derived display title; source text is not modified. Lists show title, modification date and a short derived excerpt. Selecting a Record opens a plain-text, Markdown-friendly editor inside the Expanded Sticky.

The editor supports create, open, edit, save, delete, scroll and restart restore. Ctrl/⌘ + Enter remains the save shortcut. While the editor contains unsaved changes, face switching and collapse are disabled so the draft cannot be silently discarded.

`0003_sticky_record_profile.sql` raises the Note payload limit from 4,000 to 100,000 characters by rebuilding only the Note payload table. Existing Task and unified placement architecture remain unchanged. Automated and native validation used a 6,104-character Chinese Record.

## C. Sticky Quote

Sticky Quote is independent user content in SQLite `sticky_surface_profile`, not Card metadata and not a UI preference. Exactly one `surface='sticky'` row exists. Quote update is transactional, allows long source text up to 10,000 characters, and Compact display clamps visually to two lines without truncating storage.

Tests cover empty initialization, update, restart restore and invalid-profile rollback.

## D. Record Export

The export boundary is:

```text
React Record action
→ Sticky Application Port
→ Tauri command
→ read confirmed SQLite Record by ID
→ native Save As
→ Rust exact-byte write
```

React receives no arbitrary filesystem permission. The dialog is the official Tauri Save Dialog plugin. Rust reads the already-saved Record from SQLite before showing Save As and writes the exact UTF-8 body to `.md`; no LLM, rewrite, formatting or network is involved. Cancel returns a non-error `false`; failure does not mutate the database or source body. A Rust test reopens an exported 5,000+ Chinese-character Markdown file and proves exact content equality.

## E. Compact Sticky Redesign

- Compact Sticky width is approximately 218–230 logical px and height approximately 181 logical px across the four presets.
- Only active/open Todos are shown, in user placement order.
- The Todo viewport displays roughly three rows and scrolls for additional rows.
- Wheel events stop propagation only while the Todo viewport can move in that direction; at top/bottom boundaries they propagate to the future Reader surface.
- Large Record counts/previews were removed. The optional count is a light `visible / open` mark.
- Sticky Quote remains visible at the bottom and clamps to two display lines.

Automated coverage includes 0/1/3/4+ data shapes through empty and populated render paths, completed-Todo exclusion, scroll container presence, Quote and long display clamp styles.

## F. Sticky Drag & Snap

Only Compact Sticky is draggable. Expanded Sticky remains fixed to avoid conflicts with text selection, Record editing and Todo reorder.

- Movement below 5px is a click and expands.
- Movement at or above 5px repositions and does not expand.
- Free position is clamped to the visible board.
- Four optional snap targets are supported: top-left, top-right, bottom-left and bottom-right.
- A nearby target adds subtle outline feedback; settling has no game-like bounce.
- Persisted state uses normalized `xRatio/yRatio` plus optional semantic snap anchor in the existing preference Store.
- Semantic anchors recalculate directly on resize; free positions use normalized coordinates and CSS clamp.

Pure domain tests cover free normalization, all four snaps and off-screen clamp. The Windows release used trusted pointer drag and persisted `bottom_left` before restart validation.

## G. Window Size Presets

Implemented logical sizes:

| Preset | Logical size |
| --- | --- |
| Sticky | 320 × 420 |
| iPhone 5 Reader | 320 × 568 |
| Pocket Reader | 360 × 640 |
| Book Page | 420 × 594 |

`apply_window_preset` resizes the current Tauri window with `LogicalSize` and clamps its physical origin to the current monitor. Preset selection is a shortcut only. Manual resize remains enabled; the window-state plugin continues to persist size/position, and a future manual-size observer can label the state `custom` without changing the stored layout model.

Browser responsive validation measured every Compact Sticky fully inside its exact viewport. At 320×568 the Expanded Sticky measured `294 × 444` at `(13,106)` and the settings panel remained fully in bounds.

## H. Mini Tab Concept A / B / C

```text
DESIGN_ONLY
NOT_IMPLEMENTED
```

No concept is connected to production Sticky state, persistence, drag, database, default UI or click behavior.

### Concept A — Bookmark Strip

28×92 vertical bookmark on the Reader edge; displays paper color, a small open-Todo count and ruled mark. It blocks the least text and has the strongest book association, but needs a carefully accessible narrow target.

### Concept B — Folded Paper Corner

58×58 triangular page fold with a subtle count. It visually merges into a paper page and unfolds naturally, but can compete with future corner controls.

### Concept C — Tiny Sticky Tab

108×45 taped Sticky sliver with TODAY and two Todo ticks. It preserves the strongest Agent Desk identity and normalized placement, but covers more Reader content.

Full rationale: [`mini-tab-concepts/README.md`](evidence/m1-b3/mini-tab-concepts/README.md)

## I. Persistence / Migration

- Frozen `0001_card_foundation.sql`: unchanged.
- Frozen `0002_sticky_cards.sql`: unchanged.
- New append-only `0003_sticky_record_profile.sql`: long Note payload and one-row Sticky profile.
- Fresh path `0001 → 0002 → 0003`: PASS.
- Existing real AppData `0001/0002 → 0003`: PASS; seven pre-existing active/completed cards retained.
- Legacy preview reconciliation remains exact-checksum + exact-six-schema verification and fail-closed. No broad ignore was added.
- Quote is SQLite user content; Compact position and size preset are Tauri Store layout preferences.

## J. Architecture Check

Required direction remains intact:

```text
React Feature
→ Application Port
→ Tauri Infrastructure Boundary
→ Rust Application Service
→ Repository
→ SQLite
```

Layout preferences remain:

```text
React
→ Tauri Preference Boundary
→ Store / Window API
```

React does not access SQLite or the filesystem. Tauri calls remain in infrastructure modules. Sticky presentation does not write Store directly. Core Record/Quote meaning does not use metadata. No Reader fixture entered the database.

React best-practice review found derived list/title/excerpt state computed during render, high-frequency drag coordinates isolated in refs/local state, parallel independent initial I/O, direct imports, no heavy frontend dependency and no global listener duplication.

## K. Automated Tests

Frontend used exact Node `22.23.2`:

| Gate | Result |
| --- | --- |
| Prettier | PASS |
| TypeScript | PASS |
| ESLint `--max-warnings 0` | PASS |
| Vitest | PASS — 4 files / 21 tests |
| Vite build | PASS — 46 modules |

Rust:

| Gate | Result |
| --- | --- |
| `cargo fmt --check` | PASS |
| Clippy all targets/features `-D warnings` | PASS |
| Rust tests | PASS — 19 passed / 0 failed |
| Tauri non-bundled release | PASS |

Rust coverage includes Quote update/restart/rollback, 5,500-character create/update/delete, exact Markdown reopen, migrations, exact legacy reconciliation, invalid schema refusal, existing task behavior and unified reorder rollback.

## L. Windows Native Smoke

Release:

- Path: `C:\Users\26374\AppData\Local\Temp\agent-desk-b3-final-target\release\agent-desk.exe`
- Size: 13,761,024 bytes
- SHA-256: `79FEE30A966D81A7A66B65DC84D75280DE4595407A654309BD6476917BDCFBFB`

Real WebView2/Tauri/SQLite flow validated:

- launch and `0003` application;
- Record face and Quote edit;
- create/open/scroll/save 6,104-character Chinese Record;
- Compact Quote and five additional open Todos;
- Compact scroll viewport;
- trusted pointer drag to bottom-left snap;
- persisted normalized position and semantic anchor;
- Sticky/iPhone 5/Pocket/Book preset application;
- release restart against the restored real AppData;
- seven original active/completed cards retained.

Original AppData was first moved to same-volume backups with SHA-256 records. Fresh Smoke state was removed and all three files restored byte-for-byte before the compatibility run. UI Smoke samples were then deleted by exact Card IDs, Quote and layout preferences were reset, and the debug process/port was closed. The append-only `0003` upgrade remains, as expected.

Native Save As exact-content behavior is proven at the Rust/native file boundary because automating a system file dialog would be less reliable than the underlying exact-byte test; the real release successfully loads the dialog plugin and export command.

## M. Windows/macOS CI

Implementation run `31621073762` compiled and completed all macOS gates successfully. Windows reached frontend tests, where the first cold-start Compact render took 6.447 seconds and exceeded Vitest's default 5-second per-test timeout; its remaining 20 tests passed. The focused follow-up raises only that complete test's timeout to 15 seconds without changing assertions, skipping a check or weakening the workflow. Final confirmation results are recorded in the handoff.

## N. Product Preview Evidence

Windows release evidence:

1. [`01-compact-sticky.png`](evidence/m1-b3/windows/01-compact-sticky.png)
2. [`02-long-record-editor.png`](evidence/m1-b3/windows/02-long-record-editor.png)
3. [`03-record-face-quote.png`](evidence/m1-b3/windows/03-record-face-quote.png)
4. [`04-compact-three-todos-quote.png`](evidence/m1-b3/windows/04-compact-three-todos-quote.png)
5. [`05-free-position.png`](evidence/m1-b3/windows/05-free-position.png)
6. [`06-top-right-snap.png`](evidence/m1-b3/windows/06-top-right-snap.png)
7. [`06-size-sticky.png`](evidence/m1-b3/windows/06-size-sticky.png)
8. [`07-size-iphone5.png`](evidence/m1-b3/windows/07-size-iphone5.png)
9. [`08-size-pocket.png`](evidence/m1-b3/windows/08-size-pocket.png)
10. [`09-size-book.png`](evidence/m1-b3/windows/09-size-book.png)

Mini Tab design-only previews in real 320×568 Reader Board context:

11. [`concept-a.png`](evidence/m1-b3/mini-tab-concepts/concept-a.png)
12. [`concept-b.png`](evidence/m1-b3/mini-tab-concepts/concept-b.png)
13. [`concept-c.png`](evidence/m1-b3/mini-tab-concepts/concept-c.png)

## O. Remaining Product Questions

Record:

1. Record list density 是否合适？
2. Sticky Quote 位置是否喜欢？
3. Long Record editor 是否像“备忘录”而不是“软件表单”？

Compact Sticky:

4. 约 3 条 Todo 的可见密度是否合适？
5. Quote 是否让便利贴更有个人感？
6. Todo scroll 与边界 wheel propagation 是否自然？

Drag:

7. 28px Magnet Snap 范围是否舒服？
8. 除四角外是否还需要 Edge Snap？
9. 是否确认 Sticky 默认长期记住最后位置？

Size Presets:

10. 四个模板是否都值得保留？
11. iPhone 5 是否是最喜欢的 Reader 比例？
12. 未来是否增加 Kindle / A5 / 新 iPhone 模板？

Mini Tab — required:

```text
PRODUCT_OWNER_MINI_TAB_SELECTION: A / B / C / REQUESTED_REVISION
```

## P. Known Technical Issues

- Local desktop pnpm fallback wrapper uses Node 24 internally; exact frontend Gate was therefore run by invoking project tools with the cached Node 22.23.2 binary. GitHub CI reads `.node-version` normally.
- The Tauri `beforeBuildCommand` invoked the desktop pnpm wrapper and printed an engine warning, while the separately verified exact Node 22 build and resulting release both passed.
- Windows Cargo emits a localized MSVC import-library informational linker line; Clippy and builds pass with zero diagnostics.
- Windows retained Cargo build-lock handles in some external target directories after process exit; the final full test/release used an isolated Temp target.
- Native automated window preset screenshots cover real logical sizing; a fully automated manual mouse resize is not exposed by the app boundary, while the unchanged `resizable: true` window and window-state persistence preserve manual resize.
- Export system dialog itself was not coordinate-automated; exact saved SQLite content → native file → reopened equality is covered directly in Rust.

## Q. Git State

- Repository: `https://github.com/dawnsongbest-create/agent-desk`
- Visibility: Private
- Branch: `main`
- M1-B2 baseline: `a30a8315ae349f68b7508b85a34c9ebc5c64301f`
- Expected handoff: implementation/report commit on `main`, local `main = origin/main`, clean worktree.

Final commit SHA and Windows/macOS CI run are stated in the handoff because a commit cannot contain its own SHA or resulting run ID.

M1_B3_GATE_STATUS: AWAITING_PRODUCT_AND_TECH_REVIEW

MINI_TAB_STATUS: AWAITING_PRODUCT_OWNER_SELECTION
