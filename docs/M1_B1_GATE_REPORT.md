# Agent Desk — M1-B1 Sticky Core Gate Report

Date: 2026-08-10

Branch: `main`

Scope: M1-B1 Sticky Core only

## Executive Result

- M1-B1 implementation: **PASS**
- Migration and SQLite persistence: **PASS**
- Frontend automated Gate: **PASS**
- Rust automated Gate: **PASS**
- Windows release build: **PASS**
- Windows native E2E: **PASS**
- Windows Product Preview evidence: **PASS**
- Windows GitHub Actions: **PASS**
- macOS GitHub Actions: **PASS**
- M1-B2 or later feature work: **NOT STARTED**

M1-B1 provides a real daily-use Sticky Home with unified Note and Task content, native persistence and restart restoration. Product and Tech Lead review are required before any M1-B2 work begins.

## A. Implementation Summary

Sticky remains one quiet, paper-like Home surface rather than separate Notes and Todos pages.

Implemented Note behavior:

- create through the unified Capture;
- multiline text;
- click-to-edit inline;
- `Ctrl/⌘ + Enter` to save a Note while preserving normal Enter for new lines;
- Escape to cancel;
- subtle `···` delete action;
- SQLite persistence and restart restoration.

Implemented Task behavior:

- create through the explicit Task GUI path;
- `[ ]` quick-syntax recognition from the default Note Capture;
- inline single-line editing with Enter to save and Escape to cancel;
- complete and uncomplete without removing the Task;
- optional `YYYY-MM-DD` due date with local display;
- subtle delete action;
- mouse and keyboard dnd-kit reorder;
- persisted mixed Note/Task order and restart restoration.

The renderer treats React state as a cache and ephemeral interaction state. Sticky mutations are disabled while an operation is in flight. A failed mutation rolls the optimistic state back, refetches SQLite, and displays a lightweight error rather than pretending the change was saved.

No Inbox, Reader, Focus, Agent Gateway, HTTP Adapter, Notifications, Progressive Reading, Scheduled Tasks, AI features, priority, subtasks, projects, tags or cloud sync were introduced.

## B. Migration / Persistence Changes

`0001_card_foundation.sql` remains byte-for-byte unchanged.

Append-only migration [`0002_sticky_cards.sql`](../src-tauri/migrations/0002_sticky_cards.sql) adds:

- `note_payloads(card_id, body)`;
- `task_payloads(card_id, text, due_date)`;
- unified `card_placements(card_id, surface, position, timestamps)`;
- foreign keys, uniqueness and type-integrity triggers for Note/Task payloads and Sticky placement.

Persistence rules:

- SQLite generates card IDs and canonical timestamps;
- Note and Task payload writes are transactional with base-card and placement writes;
- completion uses the existing card lifecycle/completion timestamp;
- delete is a soft lifecycle change plus placement removal;
- reorder validates the complete exact ID set, offsets existing positions, writes the new order, reads it back and commits in one SQLite transaction;
- invalid reorder returns an error and rolls the transaction back;
- a cancelled drag never calls the persistence boundary.

Migration validation started from an empty database and confirmed the ordered `0001 → 0002` chain, foreign keys and both migration versions in `_sqlx_migrations`.

## C. Architecture Check

**PASS — required dependency direction is preserved:**

```text
React Feature
→ Application Port
→ Tauri Infrastructure Boundary
→ Rust Command / Application Service
→ Repository
→ SQLite
```

- React components contain no SQL or filesystem persistence.
- Tauri `invoke` calls are centralized in `src/infrastructure/tauri/sticky.ts`.
- Sticky JSX does not own durable persistence logic.
- Domain parsing, ordering and wire types are separated from JSX.
- Rust commands delegate validation and operations through the application service and repository port.
- SQLite is the Sticky domain source of truth.
- React mutation failure handling performs rollback/refetch.
- M1-A preferences, theme, always-on-top, window state, single-instance and tray behavior remain wired and regression-tested.

## D. Automated Tests

All frontend commands ran with Node `22.23.2` and pnpm `11.16.0`.

| Command | Result | Evidence |
| --- | --- | --- |
| `pnpm format` | **PASS** | All matched files use Prettier style |
| `pnpm typecheck` | **PASS** | No TypeScript diagnostics |
| `pnpm lint` | **PASS** | ESLint completed with zero warnings/errors |
| `pnpm test` | **PASS** | 3 files, 13 tests passed |
| `pnpm build` | **PASS** | 45 modules transformed; production renderer generated |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check` | **PASS** | No Rust formatting diff |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features --offline --locked -- -D warnings` | **PASS** | No Clippy diagnostics |
| `cargo test --manifest-path src-tauri/Cargo.toml --all-targets --all-features --offline --locked` | **PASS** | 12 passed, 0 failed |
| `pnpm tauri build --no-bundle` | **PASS** | Real Windows release executable generated |

Frontend coverage includes:

- quiet empty Sticky;
- multiline Note creation;
- Task GUI creation and `[ ]` recognition;
- inline Note/Task editing and their distinct keyboard behavior;
- complete/uncomplete;
- due date;
- delete;
- unified reorder boundary;
- mutation failure rollback/refetch UI;
- M1-A theme and always-on-top callbacks.

Rust coverage includes:

- Note create/read/update/delete;
- Task create/read/update/delete;
- completion/uncompletion and due date;
- unified Note/Task placement creation and reorder;
- invalid reorder rollback;
- date and text domain validation;
- `0001 → 0002` migration chain;
- existing M1-A persistence and shell regressions.

MSVC emitted its known localized informational import-library message during test/build linking. Links, tests, Clippy and release build all completed successfully.

## E. Windows Native Smoke

The test target was the real release binary and real WebView2 renderer, not a Vite/browser substitute.

Release build:

- Path: `D:\agent-desk-target\release\agent-desk.exe`
- Size: 13,378,048 bytes
- SHA-256: `96137475D284634C6EF28F9FF3ABBA415F9A8DE9638B8740F5C1D843FB77985D`

Native E2E sequence:

```text
launch
→ create multiline Note
→ create Task through GUI
→ create Task through [ ] syntax
→ keyboard reorder
→ mouse reorder
→ cancel keyboard drag
→ complete Task
→ set due date
→ close to tray
→ open real tray menu
→ Quit Agent Desk
→ restart
→ verify full SQLite-backed state
```

Results:

| Native item | Result | Evidence |
| --- | --- | --- |
| Release launch and renderer load | **PASS** | Real Agent Desk PID, Tauri HWND and WebView2 page |
| Note and Task creation | **PASS** | Renderer and SQLite contained all three records |
| Keyboard reorder | **PASS** | dnd-kit keyboard sensor changed the unified order and SQLite placement |
| Mouse reorder | **PASS** | Pointer drag changed and persisted the unified order |
| Cancelled drag | **PASS** | Placement IDs, positions and update timestamps remained unchanged |
| Completion | **PASS** | Completed Task remained visible and persisted |
| Due date | **PASS** | SQLite stored `2026-08-18`; UI displayed local `8月18日` |
| Minimum window | **PASS** | 300×360 CSS client; internal scrollbar, Task Capture and date input remained usable and contained |
| Close to tray | **PASS** | HWND hidden while the original process remained alive |
| Real tray menu | **PASS** | Exposed `Show Agent Desk` and `Quit Agent Desk` |
| Tray Quit | **PASS** | Selecting the real Quit menu item exited the process with code 0 |
| Restart restore | **PASS** | Text, mixed order, completion and due date all restored from SQLite |

The Windows session was locked during final automation. The Gate controlled the real release WebView2 through its local DevTools protocol and captured the real Tauri HWND through `PrintWindow`; it also addressed the default-desktop tray through UI Automation. This was an automation transport choice only—the tested renderer, IPC commands, Rust process, SQLite file, native window and tray menu were all the release application.

## F. macOS CI

Implementation validation run: [GitHub Actions run 31393525793](https://github.com/dawnsongbest-create/agent-desk/actions/runs/31393525793)

- CI head: `ca2d6706f4e549cef057c335e4e72ece632dd227`
- Event: `push`
- Windows job [`windows-latest` / `93470742317`](https://github.com/dawnsongbest-create/agent-desk/actions/runs/31393525793/job/93470742317): **PASS**
- macOS job [`macos-latest` / `93470742358`](https://github.com/dawnsongbest-create/agent-desk/actions/runs/31393525793/job/93470742358): **PASS**

Both jobs completed the existing workflow with Node from `.node-version`, frozen pnpm install, frontend format/typecheck/lint/test/build, Rust fmt, Clippy with `-D warnings`, Rust tests and non-bundled Tauri build. No platform job, test, Clippy check or Tauri build was skipped or marked `continue-on-error`.

This macOS result is real compile/test/build CI. It is not claimed as a separate manual macOS UX smoke.

## G. Product Preview Evidence

The six images below were captured from the real Windows release. Empty, populated and minimum-window images use the release WebView2 viewport capture so the full bottom Capture surface is visible; completed, due-date and dark images include the native Tauri window frame.

| Preview | Dimensions | SHA-256 |
| --- | ---: | --- |
| [`01-empty-sticky.png`](evidence/m1-b1/windows/01-empty-sticky.png) | 400×567 | `40EA019BCFE52EEF1C2A7F77AC67136193508CE04D0810BAEF5FD8F2173EFF3F` |
| [`02-note-and-tasks.png`](evidence/m1-b1/windows/02-note-and-tasks.png) | 400×567 | `3C492CDDFC4B51D76D4CFD2E2C4FF3DAECE2C7964D7AB0B915D715A166571DC4` |
| [`03-task-completed.png`](evidence/m1-b1/windows/03-task-completed.png) | 334×490 | `B147A9C36B41E860458B1A24CBDD26CF42A351679E06B2985FF9E802C8FAD503` |
| [`04-due-date.png`](evidence/m1-b1/windows/04-due-date.png) | 334×490 | `4C688A6E99A3338772BF88942116D6A86ED76F76294BFC097703DE914FA55F2F` |
| [`05-minimum-window.png`](evidence/m1-b1/windows/05-minimum-window.png) | 375×449 at DPR 1.25 (300×360 CSS client) | `9A7A6EFF941F2EB09B35F73EC5B2485AED0DA6BB902B227938436E06C50C78BC` |
| [`06-dark-mode.png`](evidence/m1-b1/windows/06-dark-mode.png) | 334×490 | `AC69BFC0CF64F95C982022A054AAEAC79B56D5B2CFE1425DC3FCDA1F6BA539A6` |

The release executable is intentionally not committed because build outputs remain ignored. The Product Owner can run the exact local release path stated in section E.

## H. Known Product UX Questions

These are deliberate B1 assumptions for Product Owner review, not final product decisions:

1. **Capture placement:** keep the Capture anchored at the bottom, or let it follow the content after a short list?
2. **Note/Task switching:** retain the small segmented `随手记 / 任务` control, or make Task creation even lighter?
3. **Quick syntax:** keep `[ ]` as an invisible convenience, and should any other syntax be recognized later?
4. **Inline edit affordance:** is clicking the text discoverable enough, or should hover expose a subtle edit hint?
5. **Note save behavior:** keep `Ctrl/⌘ + Enter`, or prefer blur-only/manual save while Enter always inserts a line?
6. **Delete behavior:** is direct delete from `···` appropriate for short content, or should B2 add a lightweight undo?
7. **Due date placement:** is the date control inside `···` sufficiently discoverable, and is the compact local label enough?
8. **Completed Task treatment:** keep completed Tasks inline with mild fading, or move them into a collapsible quiet section?
9. **Reorder affordance:** should the drag handle remain hover/focus-subtle, or be permanently visible?
10. **Mixed Note/Task ordering:** keep one freely mixed order, or add optional lightweight grouping while retaining one placement model?
11. **Visual density:** confirm the serif/paper direction, row spacing, separators, corner radius and amount of texture.
12. **Settings:** confirm whether theme and pin controls should remain inside the top-right `···` menu.

## I. Known Technical Issues

### DEV_ENV_GIT_TRANSPORT_ISSUE

Standard Git transport was attempted before fallback and failed twice on this machine:

```text
fatal: unable to access 'https://github.com/dawnsongbest-create/agent-desk.git/': Failed to connect to github.com port 443 after 42 ms: Could not connect to server
fatal: unable to access 'https://github.com/dawnsongbest-create/agent-desk.git/': Recv failure: Connection was reset
```

No SSL verification, repository network code or dangerous Git configuration was changed. No token was written to the repository, remote URL, logs or a plaintext file.

The authorized Git Data API fallback used Git Credential Manager only in process memory. It verified:

- Private repository visibility;
- remote `main` parent `a96bf88add103c25377abb0881525a9c04a05632`;
- all 32 uploaded blob SHA values against local Git blobs;
- final tree `598e1343e69e6acd7e94eafed3875368003a49c3`;
- commit and remote `main` `ca2d6706f4e549cef057c335e4e72ece632dd227`;
- local `main` and `origin/main` alignment.

This remains a development-environment transport issue, not a product-code Gate blocker.

Other notes:

- A separate manual macOS UX smoke is still appropriate before a public macOS release; this Gate provides macOS compile/test/build CI.
- Product Preview data currently present in the local Agent Desk AppData was created by the native Gate so the Product Owner can open the release and inspect the persisted B1 state.
- No known M1-B1 product-code correctness blocker remains.

## J. Git State

- Repository: `https://github.com/dawnsongbest-create/agent-desk`
- Visibility: **Private**
- Branch: `main`
- M1-A baseline: `a96bf88add103c25377abb0881525a9c04a05632`
- CI-validated M1-B1 implementation commit: `ca2d6706f4e549cef057c335e4e72ece632dd227`
- Implementation tree: `598e1343e69e6acd7e94eafed3875368003a49c3`
- Origin: `https://github.com/dawnsongbest-create/agent-desk.git`
- Upstream: local `main` tracks `origin/main`
- Expected handoff worktree: clean after the report/evidence calibration commit

The final report commit and its report-only confirmation CI run are stated in the handoff because a commit cannot contain its own SHA or resulting run ID.

M1_B1_GATE_STATUS: AWAITING_PRODUCT_AND_TECH_REVIEW
