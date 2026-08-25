# M4-A1.1 + M4-A2 Gate Report — Desktop Experience Rework

## Scope Result

This Gate establishes the desktop-first workspace sequence:

```text
Desktop Sticky
    ↓ click
Compact Tab (entry only)
    ↓ 打开阅读器
Reader Workspace
```

It also adds the native right-edge recall surface:

```text
Desktop Sticky dragged into right safe zone
    ↓
Edge Dock collapsed
    ↓ hover / leave
Edge Dock expanded / collapsed
    ↓ 打开阅读器
Reader Workspace
```

No information dashboard, AI reading feature, new Agent API, Context, Memory, new Sticky data model, size system, or M4-B work was added.

## Implementation Truth

- Desktop experience implementation commit: `61eef6871588a4fb147bdb9f2b9d7a4f7ce57e53`
- Native Edge Dock deadlock fix commit: `65b78cd27a81edcc53f1f6432c1389a67567b318`
- Persistence remains in the existing preference Store; no database migration or copied workspace data was introduced.

| Boundary | File / symbol |
| --- | --- |
| Startup and workspace modes | `src-tauri/src/domain/preferences.rs` — `StartupDisplayMode`, `WorkspaceDisplayMode`, `startup_workspace_mode` |
| Preference persistence | `src-tauri/src/preferences/mod.rs` — `initialize`, `set_workspace_and_sticky_mode`, `save_edge_dock_position` |
| Native Desktop / Compact / Reader transitions | `src-tauri/src/shell/mod.rs` — `surface_main_window`, `enter_desktop_sticky`, `open_agent_desk_from_desktop`, `open_reader_workspace` |
| Native Edge Dock window | `src-tauri/src/shell/mod.rs` — `edge_dock_window`, `place_edge_dock`, `restore_edge_dock_window` |
| Post-drag right-edge docking | `src-tauri/src/shell/mod.rs` — `dock_desktop_sticky_if_near_right_edge` |
| Edge hover expansion | `src-tauri/src/shell/mod.rs` — `set_edge_dock_expanded` |
| Compact entry-only UI | `src/features/workspace/CompactTab.tsx` — `CompactTab` |
| Edge entry-only UI | `src/features/edgeDock/EdgeDock.tsx` — `EdgeDock` |
| Native surface routing | `src/main.tsx` — `surface=desktop-sticky`, `surface=edge-dock` |
| Startup setting UI | `src/features/sticky/StickyShell.tsx` — `SettingsMenu` startup group |
| Native application boundary | `src/application/ports/desktopSticky.ts` — `DesktopStickyWindowPort` |

## Product Behavior

### Startup

The persisted startup setting provides exactly three choices:

- `便利贴` (`desktop_sticky`) — default and recommended.
- `上次状态` (`last_state`) — restores Desktop, Compact, Reader, classic overlay, or Edge Dock.
- `便利贴贴在阅读器上` (`sticky_over_reader`) — restores the classic Reader + Sticky Overlay experience.

### Compact Tab

- Desktop Sticky click opens Compact Tab rather than Reader directly.
- The folded shrink control is at the top-right.
- The remaining surface expands Compact Tab.
- `打开阅读器` is the sole workspace entry action.
- Inbox, Todo, Reading, Agent status, counts, and the old `轻触展开` hint are absent.

### Edge Dock

- A real Tauri `WebviewWindow` named `edge-dock` is used; this is not an HTML imitation inside the main window.
- It is transparent, undecorated, non-resizable, always on top, and omitted from the taskbar.
- Drag completion checks a 36-physical-pixel right safe zone on the current monitor.
- Collapsed width is 34 physical pixels in the smoke environment; hover expands to 176 and leave returns to 34.
- Expansion anchors to the monitor right edge on high-DPI displays.
- Stored monitor and physical `y` restore after restart when startup is `上次状态`.
- The expanded Dock contains only `Agent Desk` and `打开阅读器`; it exposes no dashboard content.

## Runtime Defect Found and Fixed

The first native Edge smoke reproduced a real deadlock: the right-edge docking command created a new `WebviewWindow` synchronously while servicing the source WebView invoke. The Dock handle was created but remained at `about:blank`, the source invoke never returned, and Desktop Sticky remained visible.

The minimal fix changed only `dock_desktop_sticky_if_near_right_edge` from a synchronous Tauri command to an asynchronous command. The exact rebuilt executable then returned immediately, loaded the Edge WebView, hid Desktop and main windows, and completed all hover/restart/Reader-entry flows. No UI redesign or domain change was included in the fix.

## Windows Environment Recovery

The release rebuild was initially blocked because a manual disk migration split the registered Visual Studio instance:

- Visual Studio Installer still registered the instance on `D:`.
- The `D:` tree retained Hostx64/x64 `cl.exe` and `link.exe` but lacked environment scripts, includes, and libraries.
- The `E:` tree retained environment scripts, MSVC include/lib, and Hostx86 cross tools but lacked Hostx64/x64 binaries.

Recovery copied the `E:` tree back into the Installer-registered `D:` root while preserving the complete `E:` source. Before copying, the two trees were proven disjoint (`0` overlapping file paths). No files were deleted, no ACL was changed, and Visual Studio was not reinstalled.

Post-recovery proof:

- `vswhere` resolves both `Microsoft.VisualStudio.Component.VC.Tools.x86.x64` and the Windows SDK requirement to the registered instance.
- `VsDevCmd -arch=x64 -host_arch=x64` resolves the expected Hostx64/x64 `cl.exe` and `link.exe`.
- Compiler version: `19.51.36252`; linker version: `14.51.36252`.
- A real 64-bit C++ probe including `windows.h` compiled, linked, and ran with exit code `0`.
- Full Rust tests and the final Tauri release subsequently linked successfully in the restored environment.

## Automated Gates

Exact toolchain: Node `22.23.2`, pnpm `11.16.0`, Rust `1.97.1`, `x86_64-pc-windows-msvc`, MSVC Hostx64/x64.

| Check | Result |
| --- | --- |
| Prettier | PASS |
| TypeScript | PASS |
| ESLint `--max-warnings 0` | PASS |
| Frontend Vitest | PASS — 87/87 |
| Frontend production build | PASS — 227 modules |
| Rust fmt | PASS |
| Rust Clippy `-D warnings` | PASS |
| Rust tests | PASS — 50/50 |
| Non-bundled Tauri release | PASS |

The regression suites retain Sticky, Todo, 6000+ character Record save, Reader, Inbox, Delivery, Reading, Proposal, Agent Bridge, and OpenClaw workflow coverage.

## Native Windows Smoke

The exact final canonical executable used isolated SQLite, preference Store, window state, and WebView2 data under:

```text
D:\agent-desk-m4a2-smoke-65b78cd-20260826-01
```

Product Owner AppData was not opened for smoke. Pre/post SHA-256 comparison proved its database, preferences, and window-state files remained byte-identical.
The isolated fixture was removed after screenshots and runtime evidence were finalized.

The final executable SHA `6AA3655672EFF0DD3E040BA99351EEF3E21EECD54F552DF7341D55CA8C678586` proved:

1. Default launch showed only the native Desktop Sticky and stayed alive beyond 10 seconds.
2. Desktop click opened Compact Tab; no dashboard content or obsolete hint was present.
3. `打开阅读器` opened the full Reader Workspace.
4. Classic Reader + Sticky Overlay restored after a complete process restart.
5. Moving Desktop Sticky into the right safe zone created a native, topmost Edge Dock and hid both Desktop and main windows.
6. Hover changed the native rect from `(1502,77)-(1536,189)` to `(1360,77)-(1536,189)`; leave restored the original rect.
7. `上次状态` restart restored Edge Dock at the exact same rect and monitor position.
8. Edge Dock `打开阅读器` surfaced the existing main Reader and hid the Dock.
9. Final shutdown left zero smoke Agent Desk and zero fixture WebView2 processes.

Evidence: [runtime JSON](evidence/m4-a2/windows/native-smoke-runtime.json), [Desktop default](evidence/m4-a2/windows/01-default-desktop.png), [Compact entry](evidence/m4-a2/windows/02-compact-entry.png), [Reader Workspace](evidence/m4-a2/windows/03-reader-workspace.png), [classic restart](evidence/m4-a2/windows/04-classic-restart.png), [Edge collapsed](evidence/m4-a2/windows/05-edge-collapsed.png), [Edge expanded](evidence/m4-a2/windows/06-edge-expanded.png), [Edge restart](evidence/m4-a2/windows/07-edge-restart.png), and [Edge → Reader](evidence/m4-a2/windows/08-edge-open-reader.png).

## Artifact Truth

| Artifact | SHA-256 |
| --- | --- |
| `dist/index.html` | `6A0072202741E167107B4CE0B9A68AD8BA479B855CDF4AA66FDEADECCA566B31` |
| `dist/assets/index-DbbW_svI.css` | `27AC1BEDCE23FC2347D79055D2F3F06A1EE21D1E5BD0BE1719F2FB2333315A02` |
| `dist/assets/index-DMnj7JnD.js` | `80C6C852F6437B1BB2F091184F4B66A951BC98AD57E0F5B818A6650C9D8D4938` |
| truth executable | `6AA3655672EFF0DD3E040BA99351EEF3E21EECD54F552DF7341D55CA8C678586` |
| canonical executable | `6AA3655672EFF0DD3E040BA99351EEF3E21EECD54F552DF7341D55CA8C678586` |

`PRODUCT_OWNER_PREVIEW_EXECUTABLE`: `D:\agent-desk-target\release\agent-desk.exe`

Truth and canonical are byte-identical, size `15,775,232` bytes. Full provenance: [Windows provenance](evidence/m4-a2/windows/provenance.json).

## Public Repository Safety

- Repository visibility remains `public`; no visibility change was made.
- Public-tree scan found no credential value, `.env`, user database, Product Owner data, build output, or personal user-profile path.
- `node_modules/`, `dist/`, Rust `target/`, `.env`, and local runtime data remain ignored and untracked.
- Native evidence contains only isolated fixture metadata and screenshots.

## Hosted CI

GitHub Actions run: [32871182646](https://github.com/dawnsongbest-create/agent-desk/actions/runs/32871182646)

- Windows (`windows-latest`): PASS — [job 97878313820](https://github.com/dawnsongbest-create/agent-desk/actions/runs/32871182646/job/97878313820)
- macOS (`macos-latest`): PASS — [job 97878314198](https://github.com/dawnsongbest-create/agent-desk/actions/runs/32871182646/job/97878314198)

```text
M4_A1_1_GATE_STATUS: AWAITING_PRODUCT_AND_TECH_REVIEW
DESKTOP_FIRST_STATUS: READY
EDGE_DOCK_STATUS: FOUNDATION_READY
AI_READING_STATUS: NOT_STARTED
```

Do not enter M4-B.
