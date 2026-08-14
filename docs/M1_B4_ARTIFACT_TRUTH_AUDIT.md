# Agent Desk — M1-B4 Artifact Truth Audit

## Status

```text
M1_B4_GATE_STATUS: REVOKED
M1_B4_ARTIFACT_TRUTH_STATUS: PASS
M1_B4_RECORD_RUNTIME_STATUS: PASS
```

This audit does not reinstate the revoked product Gate and does not authorize B4.1, M2,
Reader, or any new product work. It establishes only artifact truth and the requested Record
runtime result.

## Root Cause of the Product Owner Conflict

The canonical file was not stale. Before this audit it matched the hash claimed by the old
Gate Report:

```text
D:\agent-desk-target\release\agent-desk.exe
SHA-256: 05402F8550F164CF2023E5B3F419E2EE0527F754D483B2D4C4F70FC1C2615965
Size: 13,771,776 bytes
Timestamp: 2026-08-13 12:32:23.446 +08:00
```

However, an older Agent Desk instance was already running when the audit began:

```text
PID: 15372
Executable: C:\Users\26374\Desktop\agent-desk.exe
Started: 2026-08-14 10:19:16.516 +08:00
SHA-256: 79FEE30A966D81A7A66B65DC84D75280DE4595407A654309BD6476917BDCFBFB
```

That hash is the prior B3 desktop artifact. Its WebView2 child was PID 32968. The running
B3 instance was exited through the Agent Desk tray **Quit** action before the build/runtime
audit. Agent Desk's single-instance behavior means launching the canonical B4 executable
while this B3 copy was alive surfaced the already-running B3 UI. This explains why the
Product Owner saw no Mini Tab and saw the old Record behavior even though the canonical file
itself had the reported B4 hash.

Classification: `STALE_RUNNING_INSTANCE`, not `STALE_CANONICAL_ARTIFACT`.

## Git Truth at Audit Start

```text
Branch: main
HEAD: 6140f9d3ac156434bc0d2d7dd4f3a847d1c1cbed
origin/main: 6140f9d3ac156434bc0d2d7dd4f3a847d1c1cbed
Working tree: clean
git diff: empty
git diff --cached: empty
B4 implementation commit: fd3ed203 (ancestor of HEAD: yes)
```

The source evidence is:

- Mini production component: `MiniStickyTab` in `src/features/sticky/StickyShell.tsx`.
- Compact → Mini and Mini → Compact: `onMinimize` / `onRestore` and
  `onStickyModeChange("mini" | "compact")` in `StickyShell.tsx`; persisted through
  `App.tsx` and `domain/preferences.ts`.
- large-text DOM path: `RecordEditor`, `draftRef`, textarea `defaultValue`, and DOM read in
  `save()` in `StickyShell.tsx`.
- ordinary Save and save-and-collapse: `.record-save`, `saveAndCollapse()`, and
  `.record-collapse` in `StickyShell.tsx`.
- three-row Compact viewport: `.compact-todo-viewport` at 84 px with 28 px rows in
  `App.css`.
- circular Compact checkbox: `.preview-check { border-radius: 50%; }` in `App.css`.

## Production Frontend Assets

The prior `dist` directory was treated as a disposable build artifact, removed, and rebuilt
with Node 22.23.2 and pnpm 11.16.0. The fresh production output is:

| Asset | Bytes | SHA-256 |
| --- | ---: | --- |
| `dist/index.html` | 446 | `6FA570F3601DC791B250D3DF0D2EE51B5658D43D04972AA7ABD0546D53B7AFB5` |
| `dist/assets/index-BDGEGBQ5.js` | 265,029 | `0C08F9F91AC5204BFAF318B74D2E5E2A240B79BD2A6595D003FF94F070BE71A4` |
| `dist/assets/index-CBQ9LWi6.css` | 28,025 | `AB4D222A4FD076504EB2F541FFE57511917B46029A65F6EDC73BD186FC6FC644` |

The minified production JS contains the Mini labels/classes, persisted `stickyMode`, Record
body label, ordinary Save path, and save-and-collapse path. The production CSS contains the
Mini/Compact selectors, the 84/28 px three-row viewport, the circular checkbox rule, and the
Record collapse action. The fresh output hashes matched a second production build, proving
the inspected bundle was deterministic current-source output rather than an old cache.

## Clean Build Provenance

The audit used:

```text
Node: 22.23.2
pnpm: 11.16.0
MSVC: x64 Developer Environment
Cargo.lock SHA-256: 075635A4168DA26E075DED6DD1B6C8A02F7E869D31CBE4950705D77A6E4D7DE5
CARGO_TARGET_DIR: D:\agent-desk-b4-truth-target
Cargo mode: --locked --offline
Command: pnpm tauri build --no-bundle -- --locked --offline
Result: PASS (fresh target; no old executable fallback)
```

The runtime fixture required user AppData to remain read-only. A narrow audit-only override,
`AGENT_DESK_AUDIT_DATA_DIR`, was added for SQLite, preferences, and window-state paths. It is
used only when explicitly set to an absolute path; normal Product Owner startup behavior is
unchanged. WebView2 used its official isolated user-data-folder environment setting.

Final binary provenance:

```text
Build-source commit: 890b329 (test: isolate native artifact truth data)
Truth executable: D:\agent-desk-b4-truth-target\release\agent-desk.exe
Canonical executable: D:\agent-desk-target\release\agent-desk.exe
SHA-256 (both): 936969C9EC2C59BB9717E8A06462F648F1DCD2F831B476C11D67C4C203F956B8
Size (both): 13,894,656 bytes
Build timestamp (UTC): 2026-08-14T09:07:38.1414224Z
Copy verification: exact hash match
```

## Exact Native Mini Proof

The exact canonical executable above was launched in Tauri/WebView2, not in Vite or a browser
substitute.

- Compact rendered visibly at 244 × 234 px and exposed the `缩成 Mini Tab` control.
- Clicking that control rendered Mini visibly at 78 × 46 px.
- Activating Mini restored Compact.
- The WebView target was `http://tauri.localhost/`, title `Agent Desk`.

Evidence, both captured from executable SHA
`936969C9EC2C59BB9717E8A06462F648F1DCD2F831B476C11D67C4C203F956B8`:

- [Compact screenshot](evidence/m1-b4-truth-audit/01-compact.png), SHA-256
  `3B615559EEA8E490112147A45E54EAB5A65758950436616E3F0516BED38250BD`
- [Mini screenshot](evidence/m1-b4-truth-audit/02-mini.png), SHA-256
  `CB2D457E6D5C652085E65739AF0204E5393EC25989852C2E5A0F79EEB0B0EA9E`

## Exact Native Record Reproduction

The Product Owner path was reproduced with isolated business data:

1. Open Record.
2. Create a Record.
3. Open it.
4. Put 7,591 Chinese/marker characters into the uncontrolled editor textarea.
5. Click the ordinary **保存** button (not **保存并收起**).
6. Exit to Records.
7. Collapse Expanded Sticky.
8. Terminate the isolated test process, restart the exact canonical executable with the same
   fixture, reopen the Record, and compare the full body.

Observed result:

```text
Save UI freeze: no
Ordinary Save completion: 126.6 ms
Animation frames during Save: 18
React save state: 已保存
Sticky mutation state: ready
Rust command/promise: returned successfully (the awaited onSave path returned true)
SQLite transaction: committed
Editor exit after Save: PASS
Collapse after editor exit: PASS
JS console errors: none
Unhandled promise rejections: none
Runtime/Log exceptions: none
Restart body length: 7,591
Restart body SHA-256: 07EA0E798C43183B0F2D4546E03E6D3D8393C8D6221D49DF723AFB0E93303409
SQLite body SHA-256: 07EA0E798C43183B0F2D4546E03E6D3D8393C8D6221D49DF723AFB0E93303409
```

SQLite migrations 0001, 0002, and 0003 were present and successful in the isolated fixture.
The requested bug did **not** reproduce in the exact newly built canonical binary. No Record
product fix was made.

## Product Owner Data Safety

The real Roaming AppData files were hashed before native testing and again after both runs.
All values were unchanged:

| File | SHA-256 before and after |
| --- | --- |
| `.window-state.json` | `4031F5F9CD643A5A23C591C582A16A4892908C36F12BE5A07476A9D6797C15B2` |
| `agent-desk.sqlite3` | `F846FDD16112E4BA84923E6CB62C85C22B141E391708B192C32F3A5C7BE925BD` |
| `preferences.json` | `CFB7122D7514BCD9867B4E2D5F3D3100872B287B6B0E85123636A496CB863026` |

No Product Owner Record, Todo, Quote, database, migration state, preferences, or window state
was modified. The isolated test Record exists only in the temporary audit fixture.

## Regression Checks

```text
pnpm format: PASS
pnpm typecheck: PASS
pnpm lint: PASS
pnpm test: PASS (27/27)
cargo fmt --check: PASS
cargo clippy --all-targets -D warnings: PASS
cargo test: PASS (20/20)
```

No Agent Desk process or audit WebView2 child remained after the audit.

## Final Gate Output

```text
M1_B4_ARTIFACT_TRUTH_STATUS: PASS
M1_B4_RECORD_RUNTIME_STATUS: PASS
```

Stop here. Do not enter B4.1, M2, Reader, or new product development.
