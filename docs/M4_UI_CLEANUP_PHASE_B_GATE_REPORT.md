# M4 UI Cleanup — Phase B

Final implementation: `31115025ce634779e65d5160dd0fe0b4031a0b87`.
Ready for Tech Lead and Product Owner review, not product acceptance.
This gate is limited to the approved UI cleanup. No agent integration, import,
AI Reading, Todo priority, schema migration, dependency upgrade, or README change.

## Checkpoints

- **B1 — `e3e0d84`**: retired right-edge snapping and its ports, IPC handlers,
  window, capability, bootstrap route, component, CSS, and hover/focus/timer
  paths. Kept native free drag, position persistence, ordinary display bounds,
  tray, single-instance, and always-on-top. Legacy placement is normalized
  before startup policy: desktop and old-edge last-state open a free Desktop
  Compact; explicit classic startup opens Reader Compact. Valid normal position
  wins over old edge coordinates; usable old coordinates are bounded to the
  work area, otherwise the window centers safely.
- **B2 — `6186e70`**: removed Reading Plan management navigation, panel,
  loading hook, styles, and view branch. Old `plans` becomes `reader`. Reading
  Session from selection and accepted proposals, plan APIs, persisted plans,
  and deliveries retain their existing service and database paths.
- **B3 — `3b65dae`**: added a top-row book icon and 阅读 in Desktop Compact;
  isolated its pointer and keyboard handling. Both paper folds go to Mini.
  Reader Mini reuses the existing component in the Safe Shelf; it has no
  second fold or extra window. Expanded 收起 goes to Compact. Removed the
  obsolete Sticky visibility toggle and Quote hint, kept 正文 visibility.
  All four kinds of dirty draft block transitions that would discard them.
- **Native follow-up — `3111502`**: the Reader save persisted but the already
  mounted Desktop window continued to show an old Quote. The renderer had no
  listener for the window-return refresh event. The native target now emits
  a single Sticky refresh event on window handoff; the target reloads cards and
  Quote from existing storage. No write or schema path changed. Both target
  windows have automated refresh regression coverage.

## State-transition coverage

| Start | Action | Expected result | Frontend coverage |
|---|---|---|---|
| Desktop Compact | 阅读 | Reader Compact | click / Enter / Space; native request |
| Desktop Compact | fold | Desktop Mini | mode cycle |
| Desktop Compact | body | Desktop Expanded | mode cycle |
| Desktop Mini | body / keyboard | Desktop Compact | mode cycle |
| Desktop Expanded | 收起 | Desktop Compact | mode cycle |
| Desktop Expanded | fold | Desktop Mini | mode cycle |
| Reader Compact | fold | Reader Mini | pinned Mini and persistence |
| Reader Expanded | 收起 | Reader Compact | existing ordinary long Record Save regression |
| Reader Expanded | fold | Reader Mini | document and scroll unchanged |
| Reader Mini | body | Reader Compact | persistent Mini restore |
| Reader Mini | 只留便利贴 | Desktop Mini | same-shape command |
| Reader Compact | 只留便利贴 | Desktop Compact | same-shape command |
| Reader Expanded | 只留便利贴 | Desktop Expanded | same-shape command |

## Compatibility and verification

- Legacy `readerStickyVisible=false` in Reader recovers to visible Compact.
  The retired key is read for compatibility and is no longer written. A valid
  Reader Mini remains Mini under last-state startup. The Rust merge regression
  protects the mode against a late scroll-only preference save.
- No data-domain/migration changes. The existing `ReaderService::open_current`
  continues to reopen an existing document and seed an empty database.
- Checkpoint test counts: B1 90 frontend / 52 Rust; B2 89 / 53; B3 94 / 53.
  After the native follow-up: **96 frontend / 53 Rust**. Final TypeScript,
  ESLint, Prettier, Vitest, rustfmt, locked Rust tests,
  strict Clippy, and production frontend build passed.
- Removed the Edge hover test and native right-edge threshold test because their
  behavior is retired. Replaced runtime expectations with legacy placement
  and startup policy coverage. Removed the Reading Plan panel tests and its
  view assertions because the view is retired; retained domain/session tests
  and added old-page normalization and no plan-list invocation coverage.
- Replaced prior hide/show, no-Reader-Mini, visibility-preference, and
  disabled-collapse assertions with actual fold/restore and restart semantics,
  same-shape leave, click/Enter/Space isolation, and blocked transitions with
  draft retained (Record edit/create, Todo text/date, and Quote).

## Exact artifact and native smoke

A new isolated Windows release target built successfully from the implementation
commit with Node 22.23.2, pnpm 11.16.0, MSVC x64, and Cargo `--locked`.

| Artifact | SHA-256 |
|---|---|
| Production JS | `0B70947B025904D52C3EC9B001F281E6C0BF2A9265A68ADC3B5F941FFADA67F5` |
| Production CSS | `5694B0285627E155E6C2A67CA1364486DC02BFA72958098155A9AB8D1C421948` |
| Windows truth executable | `0D48DAF69B97DBCBDBCCD8F7AACFC3C7E5DAA112111613BF276C781869D121E4` |
| Windows canonical executable | `0D48DAF69B97DBCBDBCCD8F7AACFC3C7E5DAA112111613BF276C781869D121E4` |

The truth executable is 15,779,328 bytes (2026-11-19 14:01:21 UTC). The
canonical preview is `D:\agent-desk-target\release\agent-desk.exe`. It
was atomically replaced after smoke, with the previous executable preserved
locally for rollback. The new canonical itself was then launched with the
isolated profile; Reader Compact loaded the fixture document and Quote, and
the Pocket preset could fold it to Reader Mini.

Windows smoke used real Tauri/WebView2, separate SQLite, Store, WebView2 user
data and window-state, and synthetic fixture content only.

- All 13 transitions were exercised on the initial B3 binary. On the final
  release, the affected handoff path was repeated with an already mounted
  Desktop window: save Quote → Reader Expanded → Desktop Expanded with
  updated Quote → Compact → Reader. A dirty Quote blocked leaving and stayed
  editable. The frontend suite re-ran the full transition coverage.
- Legacy Edge placement → ordinary Desktop Compact, legacy plans → Reader,
  and legacy hidden Reader Sticky → visible Reader Compact were confirmed in
  the isolated Store. No Edge surface was observed in those runs.
- Last-state Reader Mini restored after process restart with the same document
  and 290px saved scroll. Todo and long Record digests stayed identical;
  the newly saved Quote also stayed identical across restart. Opening Reader
  and switching surfaces kept a single ReaderDocument.
- Product Owner SQLite, preferences and window-state SHA-256 matched before
  and after; SQLite WAL/SHM remained absent. No Product Owner data was
  published. Isolated Agent Desk and its WebView2 children were shut down.
- Native visual samples cover 320px Desktop Compact with 3 of 12 Todo rows
  and a three-line Quote, Reader Mini in the Safe Shelf, Expanded with a
  7,172-character Record, blocked Quote draft, and the refreshed Desktop.
  Reader Mini was visible both at 320px and at the 360px Pocket preset.
- A synthetic drag did not establish window movement. The same Compact
  synthetic input also expanded instead of moving on the pre-Phase B
  executable under its own test profile. Physical drag, right edge, focus,
  multi-monitor capture, Windows scaling and macOS native interaction remain
  manual review items, not automated PASS claims.

Exact-release visual evidence (all from the truth SHA above):
[Reader Mini](evidence/m4-ui-phase-b/windows/01-reader-mini-320.jpg),
[Compact layout](evidence/m4-ui-phase-b/windows/02-reader-compact-three-of-twelve.jpg),
[long Record](evidence/m4-ui-phase-b/windows/03-reader-expanded-long-record.jpg),
[draft guard](evidence/m4-ui-phase-b/windows/04-reader-quote-draft-guard.jpg),
[window refresh](evidence/m4-ui-phase-b/windows/05-desktop-expanded-refreshed-quote.jpg),
[desktop entry](evidence/m4-ui-phase-b/windows/06-desktop-compact-320.jpg),
[installed canonical/Pocket](evidence/m4-ui-phase-b/windows/07-canonical-reader-mini-pocket.jpg).

## GitHub Actions and public-safety review

The implementation fast-forwarded public `origin/main`. No visibility or README
change. The Phase B diff and tracked filenames were checked for common secrets,
private-key markers, personal paths, local databases, environment files and
build artifacts; no added exposures were found.

Final implementation CI: [run 44539110601](https://github.com/dawnsongbest-create/agent-desk/actions/runs/44539110601).

| Platform | Actual job | Status |
|---|---|---|
| Windows | [135612385067](https://github.com/dawnsongbest-create/agent-desk/actions/runs/44539110601/job/135612385067) | PASS |
| macOS | [135612385594](https://github.com/dawnsongbest-create/agent-desk/actions/runs/44539110601/job/135612385594) | PASS |

Both OS jobs run the locked frontend dependency install, formatting, type
checking, lint, tests, rustfmt, strict Clippy, Rust tests, and a Tauri
non-bundled release build. CI is not a substitute for native UI smoke.

The handoff adds only this report and isolated screenshots after the exact
implementation commit; it does not change the built sources.

## Product review checklist

- Windows: click the header Reader entry, both fold corners, 收起, and
  只留便利贴; confirm no accidental drag, expansion, or Reader focus switch.
- Check the Reader Safe Shelf at 320 CSS px and ordinary widths; verify that
  Mini stays on-screen and away from the toolbar and text. Try Windows
  100%, 150%, and 200% scale when available.
- Physically drag Mini, Compact, and Expanded, including to the right edge and
  across monitors. Check position restore and no focus stealing.
- Check zero, three, and many Todos; empty, short, and three-line Quote;
  and a long Record. Exercise the same window controls on macOS native.

```text
M4_UI_TECH_GATE_STATUS: AWAITING_TECH_REVIEW
M4_UI_PRODUCT_GATE_STATUS: AWAITING_PRODUCT_REVIEW
EDGE_DOCK_STATUS: RETIRED
READING_PLAN_UI_STATUS: RETIRED
READER_MINI_STATUS: READY_FOR_REVIEW
COMPACT_READER_ENTRY_STATUS: READY_FOR_REVIEW
EXPANDED_COLLAPSE_STATUS: READY_FOR_REVIEW
AGENT_INTEGRATION_STATUS: UNCHANGED
```
