# Agent Desk — M1-B4 Gate Report

## A. Implementation Summary

M1-B4 is implemented on `main` without starting Reader, Inbox, Agent integration, Scheduled Tasks, or any other post-B4 feature.

- Rebalanced Compact Sticky to a more comfortable paper size.
- Enforced a three-complete-Todo viewport with no fourth-row peek.
- Restored circular Todo controls and increased Todo/Quote separation.
- Fixed the Compact drag first-frame jump while retaining the 5 px click/drag threshold, snapping, and persisted normalized position.
- Removed large Record text from React's per-keystroke controlled render path and added an explicit save-and-collapse path.
- Implemented production Mini Tab C with Compact/Mini transitions, shared drag/snap behavior, and preference-store persistence.
- Kept the existing Note/Todo page-turn animation and locally generated page-turn sound implementation unchanged.

## B. Compact Sticky Visual Rebalance

Compact Sticky now uses a `244 × 250 px` stable surface (`244 × 234 px` in short windows), with the paper treatment applied to an inner element. It remains materially smaller than Expanded Sticky while recovering the visual breathing room lost in the earlier compacting pass.

The header retains only the lightweight `visible / total` Todo count. No dashboard-style Record/Todo summary block was reintroduced.

## C. Quote / Todo Spacing

The Quote has a `16 px` top gap after the Todo viewport plus a very light paper-line separator. This makes the Quote read as its own sentence on the paper rather than as another Todo row.

## D. Scrollbar Hover Behavior

- Todo viewport layout height: exactly `84 px`.
- Todo row layout height: exactly `28 px`.
- Therefore exactly three complete rows fit; row four begins at `84 px` and is fully clipped.
- Default scrollbar colors are transparent.
- `:hover` / `:focus-within` reveals a thin scrollbar.
- Native WebView2 validation reported the default color as transparent and the hovered thumb color as `rgba(82, 69, 31, 0.28)`.
- Existing boundary-aware wheel handling remains in place so the Todo surface consumes wheel input only while it can scroll in that direction.

## E. Checkbox Visual Restoration

Compact preview markers and Expanded Todo checkboxes use circular geometry (`border-radius: 50%`). The native Compact preview shows the restored small-circle paper style.

## F. Drag Jitter Root Cause & Fix

Root cause: the previous drag source was also the visually transformed paper. Passing the movement threshold replaced its centering/rotation/hover transform with an untransformed live-position style, changing coordinate space on the first drag frame and producing the visible jump.

Fix: a stable, untransformed absolute `StickySurface` now owns pointer capture, geometry, threshold handling, live position, snapping, and persistence. Compact paper rotation/hover transforms live only on the inner visual element. The first live frame is computed from the same outer surface rectangle that received pointer-down.

Native evidence: an inward drag moved from `[164, 76]` to `[129, 96]`, exactly `-35 px / +20 px`, without expanding. Unit tests also cover click-to-expand, drag-without-expand, first-frame continuity, snap settling, and Mini drag persistence.

## G. Long Record Large-Paste Root Cause & Fix

Root cause: large Record bodies were stored in React controlled input state, forcing the full editor subtree and repeated string work through the render path on every large paste/change. While a Record was selected, the global collapse action was also disabled to protect unsaved content, leaving no direct recovery path after editing.

Fix:

- Record create/edit textareas are now DOM-backed uncontrolled inputs referenced only when saving.
- React state tracks only small UI facts such as dirty/saved and whether capture text exists.
- Character count is updated directly on the small count node.
- The editor provides `保存并收起`, which saves successfully before collapsing to Compact.
- No autosave loop, database reset, migration edit, or reduced text limit was introduced.

Automated coverage uses both 5000+ and 6000+ Chinese-character input. Native release smoke created, edited, saved, collapsed, restarted, reopened, and verified an exact `6639`-character Record.

## H. Mini Tab C Implementation

The production state model is:

```text
Mini Tab ↔ Compact Sticky ↔ Expanded Sticky
```

- Compact's small minus control changes the preference to `mini`.
- Clicking Mini restores `compact`.
- Mini uses the same stable surface, 5 px threshold, normalized position, snapping, and persistence model as Compact.
- Mini renders as a `78 × 46 px` paper strip with `TODAY`, open-Todo count, fold, and one subtle status mark.
- Free positions are CSS-clamped per current surface size, so switching Mini/Compact cannot strand the larger surface offscreen.
- `stickyMode` is stored in the existing Tauri preference Store, not Card/Record domain data or SQLite.
- Legacy preference JSON without `stickyMode` deserializes to `compact`.

## I. Architecture Check

- React continues through application ports into Tauri commands and Rust services/repositories.
- React does not access SQLite directly.
- Mini/Compact mode and position remain layout preferences.
- No scattered new `invoke` calls were added.
- Migrations `0001`, `0002`, and `0003` were not changed and no migration was added.

Frozen migration SHA-256 values:

- `0001_card_foundation.sql`: `9CB2CD4DF9D4986D2341D1BA839C387B19D28E2A7F054C9FED70C014C06740DD`
- `0002_sticky_cards.sql`: `ABBE88782206AD7353DF5DDFF0AC34DF2C8DF0DBD0E29A21ED90112E4B98CDA1`
- `0003_sticky_record_profile.sql`: `294B93214B73E4B860F26C1A8A8EDD1EBF9EE2A40B88CE7AF7FA4C54CBE7A365`

## J. Automated Tests

Local frontend gate:

- Prettier: PASS
- TypeScript: PASS
- ESLint: PASS
- Vitest: PASS — 4 files, 27 tests
- Vite production build: PASS — 46 modules

Local Rust gate under the MSVC x64 Developer Environment:

- `cargo fmt --check`: PASS
- `cargo clippy --locked --offline --all-targets --all-features -- -D warnings`: PASS
- `cargo test --locked --offline --all-targets --all-features`: PASS — 20 tests

## K. Windows Native Smoke

Canonical Product Owner release:

```text
PRODUCT_OWNER_PREVIEW_EXECUTABLE:
D:\agent-desk-target\release\agent-desk.exe
```

- File exists: PASS
- Size: `13,771,776` bytes
- SHA-256: `05402F8550F164CF2023E5B3F419E2EE0527F754D483B2D4C4F70FC1C2615965`
- Process appeared: PASS
- Native `Agent Desk` window appeared: PASS
- WebView2 loaded Sticky: PASS
- Process remained running after each automated phase: PASS
- Two-launch restart flow: PASS
- Compact viewport: `84 px`; four fixture rows: `28 px` each; visible rows: `3`; fourth peek: `false`
- Scrollbar default/hover: transparent / visible thin thumb
- Compact checkbox radius: `50%`
- Quote gap: `16 px`
- Mini geometry: `78 × 46 px`
- Compact drag continuity: exact `-35 / +20 px`; no accidental expand
- Presets: Sticky `320×420`, iPhone 5 `320×568`, Pocket `360×640`, Book `420×595` (WebView viewport)
- Long Record: `6639` characters after edit; exact start/end and length survived process restart
- AppData safety: `.window-state.json`, `agent-desk.sqlite3`, and `preferences.json` were backed up for the isolated fixture run and restored byte-for-byte; all three pre/post SHA-256 hashes matched.

## L. Windows/macOS CI

Pending the first pushed M1-B4 commit. This section will be replaced with real GitHub Actions run and job results; workflow presence alone is not treated as PASS.

## M. Product Preview Evidence

Native Windows evidence is in `docs/evidence/m1-b4/windows/`:

1. `01-compact-three-todos.png` — Book view, exactly three Todo rows, circular markers, new Quote gap.
2. `02-mini-tab.png` — Book view, Tiny Sticky Tab.
3. `03-long-record-after-restart.png` — 6639-character Record reopened after restart.
4. `04-iphone5-compact.png` — iPhone 5 view, unobscured Compact Sticky.
5. `05-iphone5-scrollbar-hover.png` — iPhone 5 Todo hover with visible thin scrollbar.
6. `06-iphone5-mini.png` — iPhone 5 view, Tiny Sticky Tab with minimal obstruction.

Reproducible transition steps:

1. In Compact, click the small minus control to enter Mini.
2. Click Mini to restore Compact.
3. Drag either paper by more than 5 px to reposition; releasing near a corner snaps and persists.
4. Click Compact paper (without dragging) to open Expanded.

## N. Remaining Product Questions

- Product Owner visual approval is still required for the final `244 × 250` Compact balance and `78 × 46` Mini density.
- Product Owner may decide later whether Expanded should gain a direct one-step Mini action; the current intentional path is Expanded → Compact → Mini.

## O. Known Technical Issues

- `.node-version` specifies Node `22.23.2`, but that executable was not present on this machine during the final native build. pnpm `11.16.0` ran under installed Node `24.14.0`; pnpm emitted the expected engine warning, while typecheck, tests, frontend build, and Tauri release build all passed. CI remains the authoritative Node 22.23.2 cross-platform gate.
- Rust release/test linking emits an informational localized MSVC linker message about creating import artifacts; Clippy with `-D warnings` and all tests still pass.
- WebView2/Windows reports Book's requested 594 px logical height as a 595 px inner viewport on this machine; the product layout remains within bounds.

## P. Git State

Pre-push state:

- Branch: `main`
- Baseline: `a0d0a03d07f8142c6e043c41a865b6e0910093d2`
- M1-B4 implementation and evidence: pending commit
- Existing B3 Recovery Report is intentionally retained.
- Working tree and remote state will be finalized after real GitHub Actions results are recorded.

```text
M1_B4_GATE_STATUS: CI_PENDING
```
