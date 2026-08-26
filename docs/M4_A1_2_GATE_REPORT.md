# M4-A1.2 Gate Report

## Scope

M4-A1.2 corrects the Sticky, Reader and Edge interaction model without entering M4-B. The implementation separates surface, Sticky size and placement state; removes Gateway from the normal product path; preserves Reader UI preferences; and keeps every Agent Desk native surface always on top without stealing keyboard focus.

## Implementation

- `144e67a` — separate Sticky / Reader / Edge state and correct transitions.
- `c83cfce` — keep Desktop Mini inside its native viewport.
- `98ad47a` — persist Reader surface and scroll position as UI preferences.
- `0560e51` — flush the final debounced Reader scroll position before surface exit or unmount.

Reader surface and scroll are stored only in Preferences. No SQLite migration was added and the durable ReaderDocument domain was not changed. Invalid or legacy display values are normalized to legal non-Gateway states.

## Automated Baseline

- TypeScript: PASS
- ESLint (`--max-warnings 0`): PASS
- Frontend tests: PASS — 84 / 84
- Rust tests: PASS — 50 / 50
- rustfmt: PASS
- strict Clippy (`--all-targets -- -D warnings`): PASS
- production frontend build: PASS — 225 modules
- production bundle contains neither the removed Gateway copy nor `轻触展开`: PASS

## Artifact Truth

- Published source commit: `5f93e38c15e339684bd6b373cbdc8c74489b9cc6`
- Source tree: `b7b6bdb6ee3e64b641f2afac0c586acaa6487164`
- Build environment: Node 22.23.2, pnpm 11.16.0, MSVC x64, Cargo locked
- Truth executable: `D:\agent-desk-m4a12-truth-0560e51\release\agent-desk.exe`
- Canonical executable: `D:\agent-desk-target\release\agent-desk.exe`
- Truth SHA-256: `51CFC559FDDCD26A2218CBBA06D7C503EF9B3F7464BC551611FCB9EC3F95E3D8`
- Canonical SHA-256: `51CFC559FDDCD26A2218CBBA06D7C503EF9B3F7464BC551611FCB9EC3F95E3D8`
- Byte length: 15,837,184
- Timestamp (UTC): 2026-08-26 08:12:37

Truth and canonical hashes are identical. No prior executable was used as a fallback.

## Native Smoke

All scenarios used the exact canonical SHA above with isolated SQLite, Store, window state and WebView2 profile.

- clean startup → Desktop Compact Sticky: PASS
- Compact → Expanded: PASS
- Compact → Desktop Mini → Compact: PASS
- Compact → Reader Compact: PASS
- Reader Compact → Reader Mini → Reader Compact: PASS
- Reader Mini fold → Desktop Mini: PASS
- Reader Compact → only Sticky Compact: PASS
- no Gateway or intermediate workspace page: PASS
- `last_state` restart restores Reader Expanded, Reader surface and scrollTop 240: PASS
- `sticky_over_reader` restart resolves to Reader Compact + free placement: PASS
- Edge free → collapsed → reveal → retract: PASS
- TOPMOST on Desktop Mini / Compact / Expanded, Reader, Reader Mini and revealed Edge: PASS
- ordinary window retained keyboard focus for two seconds; Agent Desk did not steal focus: PASS
- no Agent Desk smoke process remained after shutdown: PASS

Screenshots and machine-readable results are under `docs/evidence/m4-a1-2/windows/` and contain no user content, token or personal absolute path.

## Data Safety

The Product Owner AppData was read only. Before/after SHA-256 remained identical for:

- `.window-state.json`: `20DAB9E7DB363DAF4A67E87D3391AC9195710B44581BA9DF766F8CDFE270BDED`
- `agent-desk.sqlite3`: `4329A964C97377D6D33C52A00B453E7813A3FC3CB03022FE77C18A159471BE4C`
- `preferences.json`: `E276CEC4692D3D6C717CA6C14BB30E1CF0C392702277C9483B06A8221BC0BF3D`

## Public Repository Safety

PASS. The repository remains Public with `main` as its default branch. The final scan found no secrets, personal paths, Product Owner data, databases, WebView2 profiles or build outputs. The evidence intentionally contains only isolated, synthetic product state.

## CI

- Run: [32949054043](https://github.com/dawnsongbest-create/agent-desk/actions/runs/32949054043)
- Windows (`windows-latest`): PASS
- macOS (`macos-latest`): PASS

## Gate

Implementation, artifact truth, Native Smoke, public safety and two-platform CI: PASS.

`M4_A1_2_TECH_GATE_STATUS: AWAITING_TECH_REVIEW`

`M4_A1_2_PRODUCT_GATE_STATUS: AWAITING_PRODUCT_REVIEW`

`STICKY_STATE_MODEL_STATUS: READY_FOR_REVIEW`

`READER_TRANSITION_STATUS: READY_FOR_REVIEW`

`READER_PERSISTENCE_STATUS: READY_FOR_REVIEW`

`ALWAYS_ON_TOP_STATUS: READY_FOR_REVIEW`

`EDGE_DOCK_STATUS: READY_FOR_PRODUCT_REVIEW`

`AI_READING_STATUS: NOT_STARTED`
