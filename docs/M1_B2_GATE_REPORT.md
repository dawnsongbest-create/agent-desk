# Agent Desk — M1-B2 Sticky Redesign Gate Report

Date: 2026-08-12

Branch: `main`

Scope: M1-B2 Sticky Product Redesign only

## Executive Result

- M1-B2 implementation: **PASS**
- Product redesign: **PASS**
- SQLite persistence and restart restore: **PASS**
- Frontend automated Gate: **PASS**
- Rust automated Gate: **PASS**
- Windows release build: **PASS**
- Windows native smoke: **PASS**
- Windows GitHub Actions: **PASS**
- macOS GitHub Actions: **PASS**
- M1-B3 or later feature work: **NOT STARTED**

The result is a quiet reading board with a physical-feeling Sticky preview and an in-place expanded Sticky. Note and Todo are separate paper faces, both Captures follow their content, Todo creation is explicit, drag handles remain visible, and face turns include a short original paper sound.

## A. Implementation Summary

### Background board and Sticky states

- The application surface is now a light paper/reading board with a restrained notebook grid.
- The default state is a compact taped Sticky preview, not the former full software-like list.
- The preview shows Note/Todo counts, up to two open Todos and one real Note excerpt.
- Clicking the preview expands the same Sticky in place with a restrained scale/unfold transition.
- The board remains visible behind the expanded paper; no route, page or modal was introduced.
- The Sticky can be collapsed back to the populated preview.

### Separate Note and Todo faces

- Expanded Sticky exposes clearly labelled `记录` and `待办` paper faces with live counts.
- Note and Todo are filtered presentation faces over the existing unified SQLite placement model.
- Switching faces uses a short directional paper-turn transition: Note → Todo is forward; Todo → Note is backward.
- Existing multiline Note editing, `Ctrl/⌘ + Enter` save behavior, Todo Enter save, Escape cancel and `[ ]` quick syntax remain intact.

### Capture follows content — Product Owner decision B

- Note Capture renders after the current Note list.
- Todo Capture renders after the current Todo list.
- Capture is not fixed or floating at the window bottom and scrolls naturally with the paper content.
- Each face retains a small top `+ 新记录` / `+ 新待办` page affordance that scrolls to and opens the real content-following Capture. This keeps long lists discoverable without reintroducing a fixed software footer.
- Todo has a permanently visible, plainly worded `添加一个待办` entry; `[ ]` is convenience only.

### Todo drag usability

- Drag handles are permanently rendered at opacity `0.72` before hover/focus.
- Pointer activation distance is reduced from 6px to 5px.
- A dragged item lifts with rotation/shadow.
- The current insertion target receives a strong horizontal line and raised paper feedback.
- Reordering one face preserves the other kind's slots while still submitting the complete exact Note/Todo placement array to SQLite.

### Appearance and settings

- Follow system, Light and Dark remain available.
- The preference save queue now recovers after a failed save, so one failure no longer poisons every later theme change.
- Selecting a theme closes the popover and immediately exposes the visual result.
- The popover is a separate neutral paper layer with corrected spacing, hierarchy and z-index; at the 300×360 minimum viewport it remains entirely inside the window without text overlap.

## B. Page-turn Animation and Sound

Face changes are state changes first; animation and audio are non-blocking enhancements.

### Animation

- Duration: 290ms.
- Note → Todo begins with a light `rotateY(-7deg)` and 5px left offset.
- Todo → Note mirrors the direction with `rotateY(7deg)` and 5px right offset.
- `prefers-reduced-motion: reduce` removes the animation and other non-essential transitions without disabling face switching.

### Sound implementation and license

The paper sound is generated in [`pageTurnSound.ts`](../src/features/sticky/pageTurnSound.ts) by original project code using Web Audio noise, a small fibre component, a band-pass sweep and a low gain envelope. It is not a copied recording and has no third-party source or copyright dependency.

- Duration: 340ms.
- Peak gain: `0.075`, then decays to silence.
- No external URL, file load or network request.
- System output mute/volume continues to govern Web Audio normally.
- A new turn stops the previous source first, so rapid switching does not stack uncontrolled audio.
- A stopped/ended source race is caught safely.
- Missing Web Audio support, suspended context, audio failure and all thrown errors are non-blocking; the face still switches.

Native evidence used trusted pointer events against the real release WebView2 and enabled the DevTools WebAudio domain. Both directions produced a real `WebAudio.contextCreated` event plus source/filter/gain node creation and three `WebAudio.nodesConnected` events. The second turn created a fresh source/filter/gain chain after the first chain was destroyed.

Reproduction:

1. Run the Windows release.
2. Click the compact Sticky to expand it.
3. Click `待办`: the face turns forward and one soft paper-turn texture plays.
4. Click `记录`: the face turns backward and a new soft paper-turn texture plays.
5. Switch quickly several times: each new turn replaces the active source rather than overlapping it.
6. Mute Windows output and repeat: switching and animation continue while the OS suppresses audible output.

## C. Technical Changes

The required dependency direction remains unchanged:

```text
React Feature
→ Application Port
→ Tauri Infrastructure Boundary
→ Rust Command / Application Service
→ Repository
→ SQLite
```

- No SQL or filesystem persistence was added to React.
- No Tauri calls moved into presentation components.
- `0001_card_foundation.sql` and `0002_sticky_cards.sql` are unchanged.
- No migration or lockfile was added.
- SQLite remains the only durable Sticky source of truth.
- Face-local Todo reorder is converted back into a complete unified ID array before it reaches the application port.
- Existing optimistic rollback/refetch behavior remains active.

### M1-B1 preview database checksum compatibility

Native Gate uncovered a development-environment database created during the pre-commit M1-B1 native preview. Its `0002` SQLx checksum (`C961…9297`) differed from the final committed `0002` checksum (`98C6…2452`), although all three tables and all three integrity triggers were byte-for-byte schema-equivalent. SQLx correctly refused startup before M1-B2 could render.

The fix does **not** modify a frozen migration and does **not** broadly ignore checksum errors:

1. It recognizes only that one historical checksum and successful version 2 row.
2. It reads the six expected table/trigger definitions from `sqlite_master`.
3. It requires exact equality with the final committed schema definitions.
4. Only then does it update the migration-history checksum to the embedded current checksum.
5. Any unknown checksum or any schema difference remains untouched and SQLx continues to reject startup.

Automated Rust tests prove both sides: the exact legacy schema reopens successfully, while a database missing the Task type trigger continues to fail migration validation. The real original AppData then started with the final release and retained 7 cards, 5 Note payloads, 2 Task payloads and 4 active placements.

## D. Automated Tests

Frontend Gate ran with Node `22.23.2` and pnpm `11.16.0` directly, avoiding the desktop runtime's Node 24 wrapper.

| Command | Result | Evidence |
| --- | --- | --- |
| `pnpm format` | **PASS** | All matched files use Prettier style |
| `pnpm typecheck` | **PASS** | No TypeScript diagnostics |
| `pnpm lint` | **PASS** | ESLint completed with zero warnings/errors |
| `pnpm test` | **PASS** | 3 files, 16 tests passed |
| `pnpm build` | **PASS** | 46 modules transformed; production renderer generated |
| `cargo fmt --all -- --check` | **PASS** | No Rust formatting diff |
| `cargo clippy --all-targets --all-features --offline --locked -- -D warnings` | **PASS** | No Clippy diagnostics |
| `cargo test --all-targets --all-features --offline --locked` | **PASS** | 14 passed, 0 failed |
| Tauri non-bundled build | **PASS** | Real Windows release generated |

Frontend coverage includes:

- board + compact preview default state;
- preview → expanded transition;
- Capture after existing Note content;
- multiline Note creation and editing;
- Note → Todo and Todo → Note directional face state;
- audio trigger calls for both directions;
- explicit Todo creation path;
- `[ ]` quick syntax retention;
- Todo completion, date and deletion;
- redesigned theme/pin popover wiring;
- permanently rendered sortable handles;
- face-local reorder mapped to unified placement slots;
- mutation rollback/refetch.

Rust coverage includes all M1-B1 repository/domain cases plus:

- exact legacy preview checksum + exact schema reconciliation;
- refusal to reconcile when schema integrity differs.

MSVC emitted its known localized import-library informational stdout. Clippy, tests and release linking completed successfully.

## E. Windows Native Smoke

The target was the real release executable and real WebView2/Tauri/SQLite stack.

Final release:

- Path: `D:\agent-desk-target\release\agent-desk.exe`
- Size: 13,267,456 bytes
- SHA-256: `3098319B16D1ACBC5C501795B3CD13F79D4EE68650881D0B60E63C36475D2962`

Native results:

| Item | Result | Evidence |
| --- | --- | --- |
| Fresh `0001 → 0002` database | **PASS** | Both committed checksums applied successfully |
| Board + preview | **PASS** | Real release screenshot and compact real-content summary |
| Preview → expanded | **PASS** | Same window and board, no route/modal |
| Note Capture after content | **PASS** | Native Note created through the in-paper Capture |
| Explicit Todo path | **PASS** | Native Todo composer opened from clearly visible UI |
| Note → Todo turn | **PASS** | `data-turn=forward`; WebAudio graph created/connected |
| Todo → Note turn | **PASS** | `data-turn=backward`; new WebAudio graph created/connected |
| Always-visible drag handles | **PASS** | Two handles visible at computed opacity `0.72` |
| Trusted mouse drag | **PASS** | PointerSensor changed Todo order and SQLite positions |
| Insertion feedback | **PASS** | Captured active insertion line and lifted rows |
| Due date | **PASS** | SQLite and restored UI contained `2026-08-20` |
| Theme switching | **PASS** | System → Dark → Light completed in native release |
| Restart restore | **PASS** | Note, both Todos, mouse order and date restored from SQLite |
| Existing M1-B1 preview database | **PASS** | Final release stayed alive; all 7 cards remained |
| Final launch survival | **PASS** | Final executable stayed alive beyond 4-second launch check |

The desktop session was locked. Trusted local DevTools pointer events addressed the real release WebView2; these events exercised Web Audio user activation, dnd-kit PointerSensor, Tauri IPC, Rust commands and SQLite. This was not a Vite/browser substitute. The local port was temporary and disappeared with the process.

The original AppData was protected during fresh-database testing by a same-volume atomic backup/restore. Its pre-Gate database SHA-256 was recorded, restored exactly before compatibility validation, and no Agent Desk process remained at handoff.

## F. GitHub Actions

Implementation run: [GitHub Actions run 31601060400](https://github.com/dawnsongbest-create/agent-desk/actions/runs/31601060400)

- Head: `86f43e2ae0141a4204172034ccb6205695d43746`
- Event: `push`
- Windows job [`windows-latest` / `94128291865`](https://github.com/dawnsongbest-create/agent-desk/actions/runs/31601060400/job/94128291865): **PASS**
- macOS job [`macos-latest` / `94128291831`](https://github.com/dawnsongbest-create/agent-desk/actions/runs/31601060400/job/94128291831): **PASS**

Both jobs ran the existing workflow: Node from `.node-version`, frozen pnpm install, frontend format/typecheck/lint/test/build, Rust fmt, Clippy with `-D warnings`, Rust tests and non-bundled Tauri build. No job/test/check was skipped, deleted or marked `continue-on-error`.

This is real macOS compile/test/build CI, not a separate manual macOS UX smoke.

## G. Product Preview Evidence

All images were captured from the real Windows release at a 320×420 CSS viewport (400×525 physical pixels at DPR 1.25).

| Evidence | SHA-256 |
| --- | --- |
| [`01-board-sticky-preview.png`](evidence/m1-b2/windows/01-board-sticky-preview.png) | `44ABD657BA1B5D902C26631556216057200C6E20C197758AEF7CEA38FBE14501` |
| [`02-expanded-note-face.png`](evidence/m1-b2/windows/02-expanded-note-face.png) | `2EDAC82AD487401D9AFC4B4965F22FB97F567FEEF9ACB91981E9450215371037` |
| [`03-expanded-todo-face.png`](evidence/m1-b2/windows/03-expanded-todo-face.png) | `42A318C5F9004118237A114841761524FA2834C504BBBD5C8EA03BFDC5400877` |
| [`04-explicit-todo-create-path.png`](evidence/m1-b2/windows/04-explicit-todo-create-path.png) | `868A690AFFAB643C344DD069F02B329529F53139868C51AD1DAE2C5F2FCA9FFD` |
| [`05-always-visible-drag-handles.png`](evidence/m1-b2/windows/05-always-visible-drag-handles.png) | `5832B0ADA9C711AF3F8A1180420183C1AD22BCE4FBDC3C452668F1B0BAA13FFA` |
| [`06-mouse-drag-insertion-feedback.png`](evidence/m1-b2/windows/06-mouse-drag-insertion-feedback.png) | `23E5796A71F6B53F6B30E95E90267CCFCC46EC72128543FDFF832D13AFFD2BF4` |
| [`07-todo-to-note-page-turn.png`](evidence/m1-b2/windows/07-todo-to-note-page-turn.png) | `08F36BB5F7EFC8B01353601A7170A053619A4D6CA40330746211ACD5927582DF` |
| [`08-dark-theme.png`](evidence/m1-b2/windows/08-dark-theme.png) | `2F6FF8045A761906D139257659EFDABFA782B7FBD1435C36B27E61942830336E` |
| [`09-populated-sticky-preview.png`](evidence/m1-b2/windows/09-populated-sticky-preview.png) | `0F37FA56DB281F29DC6AB3D6B711490BC36555015E7A7C134455CA8B6A4A9145` |
| [`10-note-to-todo-turn-frame.png`](evidence/m1-b2/windows/10-note-to-todo-turn-frame.png) | `D5A37AF7C4EDEBFB9C4CD0099ACD1516FF019A1CC2C8DEC879E80398D47AA119` |
| [`11-todo-to-note-turn-frame.png`](evidence/m1-b2/windows/11-todo-to-note-turn-frame.png) | `E662A293EFBAC40D2C7E23B139590E2D81B031C12AD573D7F7E0E30F853A9168` |

`10` and `11` are deliberate captures 75ms into the two directional 290ms transitions. Static pixels alone cannot prove sound, so section B records the native WebAudio graph events and exact reproduction steps.

## H. Remaining Product Questions

These do not reopen the B1 shape and are not blockers:

1. Should the preview keep exactly two open Todos + one Note, or switch to one Todo on the 300×360 minimum window?
2. Is the current Sticky yellow final, or should Appearance later expose a small paper color palette?
3. Should completed Todos remain inline, move beneath open Todos, or enter a collapsed paper fold?
4. Is the board grid strength quiet enough beside real Reader content?
5. Should face preference remain ephemeral as now, or reopen on the last viewed face in a future scope?
6. Should the top page affordance remain text (`+ 新待办`) or become a smaller pencil/plus mark after users learn the model?
7. Is the 340ms page texture soft enough across laptop speakers and headphones, or should final gain be reduced below `0.075`?
8. Should a future Accessibility setting expose an explicit sound toggle in addition to system mute/volume?

## I. Git State

- Repository: `https://github.com/dawnsongbest-create/agent-desk`
- Visibility: **Private**
- Branch: `main`
- M1-B1 Gate baseline: `e447954774901959a0053f86c1a60218e5f1ea82`
- CI-validated M1-B2 implementation: `86f43e2ae0141a4204172034ccb6205695d43746`
- Implementation tree: `3804371099949ddfd4d83ec3aa1a62b02d50fa8e`
- Origin: `https://github.com/dawnsongbest-create/agent-desk.git`
- Standard Git push attempt: `Recv failure: Connection was reset`
- Authorized Git Data API fallback: private repo, parent/blob/tree/commit SHA verified
- Expected handoff: final report commit on `main`, local `main` = `origin/main`, clean worktree

The final report commit and its report-only confirmation CI are stated in the handoff because a commit cannot contain its own SHA or resulting run ID.

M1_B2_GATE_STATUS: AWAITING_PRODUCT_AND_TECH_REVIEW
