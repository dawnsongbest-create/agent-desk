# Agent Desk — M1-B4.1 Gate Report

## Gate Scope

M1-B4.1 is a narrow corrective Gate only. It changes Record post-Save navigation release,
Record editor vertical layout, Expanded Todo checkbox presentation, and the Compact
Todo-to-Quote separator. It does not add Reader, Inbox, Agent integration, new Mini behavior,
new Sticky features, migrations, or unrelated visual redesign.

## Save Busy-State Root Cause

The persistence lifecycle was not stuck:

1. `RecordEditor.save()` awaited `onSave(record.id, draft)`.
2. `useStickyCards.updateText()` awaited the Tauri `update_sticky_text` command.
3. The successful command published the returned card and executed `setState("ready")`.
4. `RecordEditor` then executed `setSaved(true)`.

The exact persistent lock was a second, independent condition. After successful normal Save,
`selectedRecord` intentionally remained non-null so the editor stayed open. The Todo tabs,
`+ 新 Record`, and the Expanded collapse button were all still disabled whenever
`selectedRecord !== null`, regardless of the now-ready mutation state. A global
`button:disabled { cursor: wait; }` rule made that selection lock look like a pending Save.

A regression test was added before the fix and failed with the Todo tab still carrying its
native `disabled` attribute after the UI displayed `已保存`. This separated the root cause
from SQLite, the Tauri command, refetch behavior, or an unresolved promise.

## Exact Save Fix

- `RecordEditor` now reports dirty transitions to `StickyShell` through `onDirtyChange`.
- While a Record is dirty or a real mutation is running, Todo, new-entry, and collapse remain
  protected from accidental unsaved-data loss.
- After ordinary Save resolves successfully, dirty becomes false and all three navigation
  paths are immediately enabled while the editor remains open.
- Clicking Todo leaves the saved editor and performs the existing page turn.
- Clicking `+ 新 Record` leaves the saved editor, waits for the Record list/capture DOM to
  commit, then opens the existing Capture control.
- Clicking collapse clears the saved editor selection and returns to Compact.
- `aria-busy` and `data-dirty` expose the real editor state for runtime verification.
- Generic disabled controls now use `not-allowed`, not the false `wait` cursor. The existing
  persistence indicator remains the Save-progress affordance.
- The optional `scrollIntoView` enhancement can no longer prevent Capture from opening when
  that browser API is unavailable.

No timeout, skipped persistence, reduced text limit, database reset, or fake loading cleanup
was introduced.

## Record Editor Layout

The Record path now establishes a complete flex chain from `.face-scroll` through the Record
face and `.record-editor`:

```text
Header / navigation
Record status
flex: 1 Record textarea (min-height: 0; overflow: auto)
compact bottom footer
```

The face-level outer scroller is disabled only while the Record editor is active, and the
textarea becomes the long-body scroll owner. The footer remains at the bottom with character
count/type on the left and Export/Delete/Save/Collapse actions on the right.

Native final-SHA geometry:

```text
face-scroll height: 240.80 px
Record editor height: 230.89 px
Record textarea height: 159.69 px
footer height: 35.57 px
textarea share of editor: 69.16%
outer overflow-y: hidden
textarea overflow-y: auto
```

## Expanded Todo Checkbox

- Compact preview markers are unchanged circles (`border-radius: 50%`).
- Expanded Todo uses a restrained 2 px-radius square outline.
- Incomplete state has no visible interior mark.
- Completed state displays an explicit `✓` inside the same square.
- The previous circular filled-dot/radio-button presentation was removed from Expanded Todo.

## Compact Quote Separator

The Compact Quote's top border and top padding were removed. The last Compact Todo row also
has no bottom border, so no remaining line can read as a Todo-to-Quote divider. Separation is
now only a 20 px whitespace gap plus Quote typography. Earlier Todo row separators remain
available when multiple Todo rows are shown.

## Automated Checks

Toolchain:

```text
Node 22.23.2
pnpm 11.16.0
MSVC x64 Developer Environment
Cargo: --locked --offline
```

Results:

```text
pnpm format: PASS
pnpm typecheck: PASS
pnpm lint: PASS
pnpm test: PASS (29/29)
cargo fmt --check: PASS
cargo clippy --all-targets -D warnings: PASS
cargo test: PASS (20/20)
pnpm tauri build --no-bundle: PASS (fresh isolated target)
```

New automated coverage proves:

- navigation remains protected while an edited Record is dirty;
- ordinary Save releases Todo, `+ 新 Record`, and collapse;
- `+ 新 Record` actually opens Capture after Save;
- Todo can be opened immediately after Save;
- 6000+ Chinese-character ordinary Save releases collapse and restores the exact body.

## Build Provenance and Product Preview

```text
Implementation commit: c2e010fb65ccd10234b2687c1a87329aaf8cf236
Fresh target: D:\agent-desk-b4-1-target
Truth executable: D:\agent-desk-b4-1-target\release\agent-desk.exe
PRODUCT_OWNER_PREVIEW_EXECUTABLE: D:\agent-desk-target\release\agent-desk.exe
Executable SHA-256 (truth and canonical): BC1DEA2F57CA3FC893B0AB3C8D6F42603CFE139A6C9A70C7556BE8CF1EF56466
Executable size: 13,894,656 bytes
Executable timestamp UTC: 2026-08-14T15:52:29.3266999Z
```

Production assets:

| Asset | Bytes | SHA-256 |
| --- | ---: | --- |
| `dist/index.html` | 446 | `4B457FE60E5E8848DEEA4EE99D9DDC8EE8234D2C308F035179A7F2C887D35A17` |
| `dist/assets/index-DpbXVkGx.js` | 265,457 | `C4A5E3DAF213FEB3790576B6C086B8EEDAC09D71E62927804FB90738B706B465` |
| `dist/assets/index-BOC1fa8a.css` | 28,351 | `00E60BB3421EAFB71C59B59E826FF132140C3F093606098B39EC5D675231D737` |

## Exact Native Product Smoke

The final canonical executable above ran in Tauri/WebView2 at `http://tauri.localhost/` with
isolated business data and an isolated WebView2 profile. The Product Owner AppData remained
read-only.

The smoke executed this real sequence:

```text
launch
→ Expanded Record
→ create/open short Record
→ ordinary Save
→ immediately open Todo
→ create two Todos
→ return Record
→ + New Record
→ create/open 7,616-character Chinese Record
→ ordinary Save
→ collapse
→ reopen and verify exact body
→ inspect Record layout
→ Expanded Todo unchecked square
→ complete Todo and inspect square + check
→ Compact circular marker
→ verify no Quote separator
```

Save observations:

| Check | Short Record | 7,616-character Record |
| --- | ---: | ---: |
| Save completion | 96.4 ms | 104.9 ms |
| responsive animation frames | 5 | 15 |
| mutation state after Save | `ready` | `ready` |
| editor `aria-busy` after Save | `false` | `false` |
| editor dirty after Save | `false` | `false` |
| Todo disabled | no | no |
| `+ 新 Record` disabled | no | no |
| collapse disabled | no | no |
| Todo cursor | `pointer` | `pointer` |

Long Record verification:

```text
saved length: 7,616
reopened length: 7,616
expected/reopened/SQLite SHA-256:
88D81BE4631AA67BE3F6290E92654C54F09B6D0022DAB44A435F778216AF4342
exact equality: yes
```

Todo/Compact computed styles:

```text
Expanded unchecked: 2px square; check transform = scale(0)
Expanded completed: 2px square; content = ✓; check visible
Compact marker: border-radius = 50%
Quote border-top: none / 0px
Quote padding-top: 0px
Quote whitespace margin: 20px
```

No JS console error, unhandled rejection, runtime exception, or log error occurred. SQLite
migrations 0001, 0002, and 0003 were successful in the fixture. The two Records and both Todo
completion states were committed. Product Owner DB, preferences, and window-state hashes were
identical before and after the smoke.

Evidence, all captured from executable SHA
`BC1DEA2F57CA3FC893B0AB3C8D6F42603CFE139A6C9A70C7556BE8CF1EF56466`:

- [Record editor layout](evidence/m1-b4-1/01-record-layout-after-long-save.png) —
  `FEDBAA36E74B06968FE9028B8C6E882166725E5C2A7876082DE2CC123B720DF5`
- [Expanded Todo unchecked](evidence/m1-b4-1/02-expanded-todo-unchecked.png) —
  `D31FB5C7254FA9F09CCD36CD4CB7821735515F27D45AF632F2700B24A4A4A7F7`
- [Expanded Todo completed](evidence/m1-b4-1/03-expanded-todo-checked.png) —
  `F5972DCECEA97CAF6778B7CA4F57B8A2E55EAB084C1B083285324474B1440DB2`
- [Compact circular marker and no Quote separator](evidence/m1-b4-1/04-compact-no-quote-separator.png) —
  `FCB657E5CBE165ABB9CB5CA6C79B2C393332D49A973E853B92B35E4A8ED254F6`

## GitHub Actions CI

```text
Implementation commit: c2e010fb65ccd10234b2687c1a87329aaf8cf236
Run: 31818687865
Run URL: https://github.com/dawnsongbest-create/agent-desk/actions/runs/31818687865
Run conclusion: completed / success
Windows job: 94826449982 / completed / success
Windows completed: 2026-08-14T16:25:02Z
macOS job: 94826449856 / completed / success
macOS completed: 2026-08-14T16:21:50Z
```

Both jobs executed frontend format/typecheck/lint/tests, Rust fmt/Clippy/tests, and a real
non-bundling Tauri build. No test, Clippy rule, platform job, or build step was skipped.

## Git State

```text
Branch: main
Implementation HEAD: c2e010fb65ccd10234b2687c1a87329aaf8cf236
origin: https://github.com/dawnsongbest-create/agent-desk.git
Pushed branch: main
Remote implementation HEAD: c2e010fb65ccd10234b2687c1a87329aaf8cf236
```

## Final Gate Output

```text
M1_B4_1_GATE_STATUS: AWAITING_PRODUCT_AND_TECH_REVIEW
```

Stop here. Do not enter M2.
