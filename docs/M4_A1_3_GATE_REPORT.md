# M4-A1.3 Reader Interaction Simplification

## Implementation

Baseline: `e7d30a82e1497ee03d7cc1de391060e19e3a8d4c`.

- `f072894`: Reader Sticky visibility, draft protection, preference compatibility and field-level merge.
- `618fb7c`: narrow-window controls remain on one line; restoring Sticky clears the transition protection notice.

Reader Compact and Expanded folds now hide the existing, still-mounted overlay. The Reader toolbar exposes a labeled, tooltip-equipped visibility toggle. Restoring reveals the prior Compact/Expanded editor and its in-memory draft. Reader never renders Mini; Desktop Mini/Compact/Expanded remain supported.

The existing only-Sticky action resolves visible Compact to Desktop Compact, visible Expanded to Desktop Expanded, and hidden Sticky to Desktop Compact. An unfinished Record, Todo or Quote editor blocks this cross-window transition and asks the user to save or cancel first. Hiding within Reader remains allowed. Unsaved drafts are not advertised as durable across process termination; the restart check saves content before quitting.

## Files and reasons

- `src/features/sticky/StickyShell.tsx`: visibility controls, mounted-overlay hiding, draft transition guard, Reader-only fold behavior and narrow-header title tooltip.
- `src/features/sticky/StickyHome.tsx`: forward the visibility callback.
- `src/App.tsx`: save visibility and include the pre-edit preference snapshot.
- `src/App.css`: hidden overlay, compact header fit and protection notice.
- `src/domain/preferences.ts`, `src-tauri/src/domain/preferences.rs`: default visibility, legacy Reader Mini normalization and startup policy.
- `src/application/ports/preferences.ts`, `src/infrastructure/tauri/preferences.ts`, `src-tauri/src/preferences/mod.rs`: merge only fields changed relative to the save snapshot into the current stored preferences.
- `src-tauri/src/shell/mod.rs`: Reader entry sets Compact, visible and free placement using the existing window lifecycle.
- Corresponding App, StickyHome and Rust preference tests: new behavior and regression coverage.

No dependency, migration, README, Bridge API, OpenClaw Adapter, Delivery Service or Proposal Service change.

## Preference compatibility

- New `readerStickyVisible` defaults to true when absent.
- Legacy Reader + Mini becomes Reader + Compact + visible.
- Desktop Mini remains Mini.
- Last-state startup preserves Reader visibility.
- Classic startup becomes Reader + Compact + visible.
- Opening Reader from Desktop explicitly produces Compact + visible.
- A stale scroll save changes scroll only; it cannot overwrite newer native display state or Sticky visibility. Existing whole-object callers remain supported when no base snapshot is supplied.

## Automated checks

- TypeScript: PASS
- ESLint: PASS
- Prettier: PASS
- Frontend tests: 91 / 91 PASS, up from 84
- Rust tests: 52 / 52 PASS, up from 50
- rustfmt: PASS
- Strict Clippy: PASS
- Frontend production build: PASS

No tests were deleted. Former Reader Mini interaction/Safe Shelf Mini expectations were updated for the approved removal of Reader Mini, while retaining Desktop Mini restore coverage. App save assertions now account for the base snapshot argument. Added seven frontend cases cover visibility persistence, both display sizes, Record draft protection and three composer types. Two Rust cases cover legacy/startup visibility and stale-save merge safety.

## Artifact truth and native verification

Final source: `618fb7c7b58565da0e8d0c696d4c7a4e067f9dfe`.

- Locked build environment: Node 22.23.2, pnpm 11.16.0, MSVC x64, Cargo.lock (`--locked`).
- Clean truth target: `D:\agent-desk-m4a13-truth-618fb7c\release\agent-desk.exe`.
- Canonical: `D:\agent-desk-target\release\agent-desk.exe`.
- Truth and canonical SHA-256: `056E053E6B36E5810D18340AFD4D8873D38EE95107B52E227059DB40E02E893E`.
- Size: 15,875,584 bytes. Timestamp: 2026-11-04 13:10:36 UTC.

Native Smoke PASS on that exact binary: clean Desktop Compact; direct Reader entry; Compact and Expanded hiding/restoring; no Reader Mini; dirty Record draft preserved and cross-window transition blocked; ordinary Save of a 6,437-character fixture; Desktop Expanded/Mini/Compact; hidden Reader to Desktop Compact; Reader document and scroll unchanged. True process restart restored the hidden state, scrollTop 240 and the saved Record, without duplicate cards.

Native Edge docking at an isolated right-edge position, reveal and retract passed (35 → 176 → 35 logical viewport pixels). Reader, Desktop Sticky and Edge native window TOPMOST flags were true. A normal application (Edge) remained foreground after a three-second observation; Agent Desk did not steal keyboard focus. All smoke Agent Desk processes were shut down.

Verification used the real production WebView2 DOM and Tauri commands with isolated data, never Vite. The Windows Computer Use helper failed initialization twice, including a reset/retry. Consequently physical mouse dragging/hover and the literal normal-app click were not automated: Edge hover events and safe-zone placement were supplied by the test harness, while the native docking/window code and focus state were exercised/read. Physical interaction feel remains on the Product Owner checklist; it is not represented as a completed manual test.

Only the final `618fb7c` binary's screenshots and results are included in `docs/evidence/m4-a1-3/windows/`.

## Product Owner AppData safety

This Gate uses isolated SQLite, Store, window state and WebView2 data. The current Gate's starting hashes are:

- Database: `894AB912AAF19B423F2895D2C0E0C756AD63A63011FB7240289DA36CD0C6ABD0`
- Preferences: `EE55DD5A7DE49EDB92D0E53D842F02ED248C6D607CA92590B4360FC299A38EFE`
- Window state: `F97C0ABFDC3C768E789C165096401BC92930473802D446B09ACDD9C07D37E354`

These are fresh pre-smoke hashes, not the older M4-A1.2 values.

All three hashes remained identical after the final Native Smoke. No user data was reset, restored or rolled back.

## CI and public handoff

- Evidence handoff commit: `303395cde74a0ee2d64bc38fefd47d2b4fcaa2de`.
- [GitHub Actions run 41836447626](https://github.com/dawnsongbest-create/agent-desk/actions/runs/41836447626): Windows PASS, macOS PASS. Both jobs completed successfully, including production non-bundled Tauri builds.
- Public repository safety scan: PASS. No secrets/tokens, personal profile paths, user data, SQLite files, WebView2 profiles or build artifacts were added. README is unchanged.
- Repository remains Public; default branch is main. Implementation and evidence are on origin/main.
- Temporary build wrapper, smoke script and downloaded Node portable runtime were removed. Verified truth and canonical releases remain available.

## Gate status

`M4_A1_3_TECH_GATE_STATUS: AWAITING_TECH_REVIEW`

`M4_A1_3_PRODUCT_GATE_STATUS: AWAITING_PRODUCT_REVIEW`

`READER_STICKY_VISIBILITY_STATUS: READY_FOR_REVIEW`

`DESKTOP_STICKY_STATUS: UNCHANGED`

`EDGE_DOCK_STATUS: REGRESSION_VERIFIED`

`CONTENT_IMPORT_STATUS: NOT_STARTED`

`AI_READING_STATUS: NOT_STARTED`

## Product review checklist

1. In narrow Reader windows, find the visibility toggle and confirm its tooltip and label.
2. Hide and restore Compact and Expanded; check that no Reader Mini appears.
3. Edit Record, Todo and Quote drafts, hide and restore them, then save normally.
4. Confirm that only-Sticky protects unfinished editing and follows the approved size mapping once editing is resolved.
5. Confirm physical desktop dragging, edge hover/retraction, normal-app focus and always-on-top comfort.

Content Import and AI Reading have not started. Stop after this Gate; do not enter M4-P1.
