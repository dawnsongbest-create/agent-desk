# M4-A1 Gate Report — Mini Tab Desktop Mode

## Scope Result

M4-A1 upgrades the existing Mini Tab into a native desktop-resident surface:

```text
Full Reader + Sticky
    ↓
Mini Tab
    ↓
Desktop Sticky (native Tauri window)
    ↓ click
Existing Full Agent Desk
```

No Edge Dock, automatic screen-edge expansion, new Sticky system, new size system, AI reading feature, Agent API, Context, Memory, or M4-A2 work was added.

## Implementation Truth

- Implementation commit: `5370a396cfec90f0eed3569cb1bef15d62b9c40d`
- Native evidence commit: `ed760ba7ae71da0eeba075e9bd8ce56a00bcdaca`
- macOS transparency fix commit: `53543def2d16d276da6b49cf09e49297b5f18b9f`
- State model: `StickyMode = compact | mini | desktop`
- Desktop content kind: `todo | record | quote`
- Persistence: existing preferences/store only; no database migration and no copied workspace data.

| Boundary | File / symbol |
| --- | --- |
| Native window lifecycle | `src-tauri/src/shell/mod.rs` — `enter_desktop_sticky`, `open_agent_desk_from_desktop` |
| Native drag and menu | `src-tauri/src/shell/mod.rs` — `start_desktop_sticky_drag`, `show_desktop_sticky_menu` |
| Restart restore | `src-tauri/src/lib.rs` — Desktop mode startup branch |
| Persisted mode, kind, position | `src-tauri/src/domain/preferences.rs` — `StickyMode`, `DesktopStickyKind`, `DesktopStickyPosition` |
| Desktop presentation | `src/features/desktopSticky/DesktopSticky.tsx` — `DesktopSticky` |
| Application boundary | `src/application/ports/desktopSticky.ts` — `DesktopStickyWindowPort` |
| Tauri adapter | `src/infrastructure/tauri/desktopSticky.ts` — `tauriDesktopSticky` |
| Full → Mini → Desktop transition | `src/features/sticky/StickyShell.tsx` — `changeStickyMode`, Mini `onDesktop` |
| Surface routing | `src/main.tsx` — `surface=desktop-sticky` |

The desktop surface reads Todo, Record, and Quote through the existing Sticky application port and the same SQLite database. It does not copy or fork user data.

## Native Desktop Behavior

- A real Tauri `WebviewWindow` named `desktop-sticky` is created; this is not an HTML imitation inside the main window.
- The window is transparent, undecorated, non-resizable, always on top, and omitted from the taskbar.
- It retains the existing Mini Tab paper size and visual language; no Small/Medium/Large sizing model exists.
- Pointer movement crosses a drag threshold before invoking Tauri system window drag, preventing click jitter.
- Native move events persist physical `x`, `y`, and monitor name with debounce.
- Restart restores `desktop` mode, the prior monitor/position, and the same Todo/Record/Quote content.
- Single click restores the existing Agent Desk main window; it does not launch another application instance.
- Double click restores the app and enters the editor for the selected Sticky kind.
- Native context menu provides `打开 Agent Desk`, `切换 Sticky 类型`, `隐藏`, and `退出`.

## Automated Gates

Exact local toolchain: Node `22.23.2`, pnpm `11.16.0`, Rust `1.97.1`, Cargo.lock locked, `x86_64-pc-windows-msvc`, MSVC Hostx64/x64.

| Check | Result |
| --- | --- |
| Prettier | PASS |
| TypeScript | PASS |
| ESLint `--max-warnings 0` | PASS |
| Frontend Vitest | PASS — 83/83 |
| Frontend production build | PASS |
| Rust fmt | PASS |
| Rust Clippy `-D warnings` | PASS |
| Rust tests | PASS — 48/48 |
| Fresh non-bundled Tauri release | PASS |

The frontend tests cover the Mini → Desktop action, Desktop Sticky rendering, click/double-click behavior, and pointer threshold → native system drag. Rust tests cover the new preference values and persisted desktop position. Existing regression suites retain Sticky, Todo, Record large-text, Reader, Inbox, Delivery, Proposal, Reading, Bridge, and OpenClaw workflow coverage.

## Native Windows Smoke

The exact canonical executable used isolated SQLite, preference Store, window state, and WebView2 data under:

```text
D:\agent-desk-m4a1-smoke-5370a39-20260825-01
```

Product Owner AppData was not opened or modified. The full native flow used executable SHA `A7CC17AAE955B86241EA24DA81E35B755F21B10047A0E4FDEAFF72DC10B5642B` and proved:

1. Full Reader, Compact Sticky, Mini Tab, and the independent Desktop Sticky were visible in the expected sequence.
2. The desktop child had native class `Tauri Window`, was visible and topmost, while the main window was hidden in Desktop mode.
3. A native move changed the window rectangle from `(154,154)-(290,216)` to `(420,260)-(556,322)`; Tauri persisted DPI-aware physical position `x=525`, `y=325`, monitor `DISPLAY1`.
4. Because the Windows workstation was locked, movement was injected through Win32 `SetWindowPos` on the Default desktop. The real pointer → Tauri system-drag path is separately covered by the component test; Product review remains the manual fluidity check.
5. A Todo named `M4 桌面同步验证` was created through the real UI and immediately appeared in the Desktop Sticky from the same SQLite data.
6. The real native right-click menu exposed all four required actions.
7. Double-click restored the existing main window and opened the Todo editor; Reader remained available underneath.
8. After a complete process restart, Desktop mode, exact position, topmost state, Todo content, Reader, and Compact Sticky all restored.
9. A final post-fix build from commit `53543de` replaced canonical with SHA `12E22F7EBE41A5ED685F27B07F441503FD5C18C566DC05173D4477787C157C88`.
10. The exact post-fix canonical stayed alive, loaded WebView2, and completed Compact → Mini → Desktop with two WebView2 targets including exactly one Desktop surface.
11. Final shutdown left zero isolated Agent Desk processes.

Evidence: [runtime JSON](evidence/m4-a1/windows/native-smoke-runtime.json), [Full + Compact](evidence/m4-a1/windows/01-full-compact.png), [Mini in Reader](evidence/m4-a1/windows/02-mini-in-reader.png), [Desktop Sticky](evidence/m4-a1/windows/03-desktop-sticky-todo.png), [same-data Todo sync](evidence/m4-a1/windows/04-desktop-sticky-synced-todo.png), [double-click Todo editor](evidence/m4-a1/windows/05-desktop-double-click-edit.png), [restart Desktop restore](evidence/m4-a1/windows/06-restart-restored-desktop.png), and [restart Reader + Compact](evidence/m4-a1/windows/07-restart-reader-and-compact.png).

## Artifact Truth

| Artifact | SHA-256 |
| --- | --- |
| `dist/index.html` | `1F15EC9BCE48ACBEAA0F439CC4A7B08B1942D2A692BF27A46A4362B139AD9647` |
| `dist/assets/index-BImTNJf-.css` | `05596644B4C2E30F4A5DD1B431895C11F87A2CC89E69B0A203B8ECB21B599A2E` |
| `dist/assets/index-ECIkk77c.js` | `A424EFA46C7DECE6A0D47DE3739C45C8282D11DC22D536DB8D0709586A1174E9` |
| truth executable | `12E22F7EBE41A5ED685F27B07F441503FD5C18C566DC05173D4477787C157C88` |
| canonical executable | `12E22F7EBE41A5ED685F27B07F441503FD5C18C566DC05173D4477787C157C88` |

Truth executable:

```text
D:\agent-desk-m4a1-final-53543de-target\release\agent-desk.exe
```

`PRODUCT_OWNER_PREVIEW_EXECUTABLE`: `D:\agent-desk-target\release\agent-desk.exe`

Truth and canonical are byte-identical, size `15,667,712` bytes. Provenance: [Windows provenance](evidence/m4-a1/windows/provenance.json).

## Public Repository Safety

- Repository visibility remains expected `public`; no visibility change is part of this Gate.
- Public-tree scan found no credential value, `.env`, user database, Product Owner data, build output, or user-profile path.
- `node_modules/`, `dist/`, Rust `target/`, `.env`, and local runtime data remain ignored and untracked.
- Native evidence contains only isolated fixture content created for this Gate.

## Hosted CI

Initial run [`32834685208`](https://github.com/dawnsongbest-create/agent-desk/actions/runs/32834685208) proved Windows PASS but found a real macOS compile failure: Tauri's `WebviewWindowBuilder::transparent` is feature-gated on macOS. The minimal fix enabled the official `macos-private-api` feature in `src-tauri/Cargo.toml` and matching `app.macOSPrivateApi` configuration in `src-tauri/tauri.conf.json`; no product behavior or UI was changed.

Replacement run [`32835881532`](https://github.com/dawnsongbest-create/agent-desk/actions/runs/32835881532) validated fix commit `53543def2d16d276da6b49cf09e49297b5f18b9f` on public `main`:

- macOS job [`97764511401`](https://github.com/dawnsongbest-create/agent-desk/actions/runs/32835881532/job/97764511401): PASS.
- Windows job [`97764511623`](https://github.com/dawnsongbest-create/agent-desk/actions/runs/32835881532/job/97764511623): PASS.

Both jobs completed frontend format/typecheck/lint/tests, Rust fmt/Clippy/tests, and the real non-bundled Tauri build.

```text
M4_A1_GATE_STATUS: AWAITING_PRODUCT_AND_TECH_REVIEW
DESKTOP_STICKY_STATUS: FOUNDATION_READY
EDGE_DOCK_STATUS: NOT_STARTED
AI_READING_STATUS: NOT_STARTED
```

Do not enter M4-A2 before Product and Tech review.
