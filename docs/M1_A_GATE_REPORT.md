# Agent Desk — M1-A Recovery & Native Validation Gate Report

Date: 2026-08-10
Branch: `main`
Scope: M1-A foundation recovery and native validation only

## Executive Result

- Windows Rust/Tauri automated validation: **PASS**
- Windows GitHub Actions CI: **PASS**
- macOS GitHub Actions CI: **PASS**
- Windows native smoke: **PASS**
- Native screenshot evidence: **PASS**
- Architecture regression check: **PASS**
- GitHub remote/private upload verification: **PASS**
- M1-B or later feature work: **NOT STARTED**

Windows and macOS CI completed successfully on the private GitHub repository. M1-A is ready for Tech Lead final review; no M1-B work has started.

## A. Environment Recovery

The Visual Studio x64 developer environment was initialized inside each Cargo/Tauri command process by calling:

```text
D:\迅雷下载\c++\Common7\Tools\VsDevCmd.bat -arch=x64 -host_arch=x64
```

No global `PATH` change was made.

Resolved native tools:

| Tool | Actual path/version |
| --- | --- |
| `cl.exe` | `D:\迅雷下载\c++\VC\Tools\MSVC\14.51.36231\bin\Hostx64\x64\cl.exe`; C/C++ compiler 19.51.36252 for x64 |
| `link.exe` | `D:\迅雷下载\c++\VC\Tools\MSVC\14.51.36231\bin\Hostx64\x64\link.exe`; Incremental Linker 14.51.36252.0 |
| `lib.exe` | `D:\迅雷下载\c++\VC\Tools\MSVC\14.51.36231\bin\Hostx64\x64\lib.exe` |
| `rc.exe` | `C:\Program Files (x86)\Windows Kits\10\bin\10.0.26100.0\x64\rc.exe` |
| Windows SDK | 10.0.26100.0 |
| Rust host | `x86_64-pc-windows-msvc`; rustc/cargo 1.97.1 |

Node was aligned to Node 22 without replacing the machine-wide Node 24 installation:

- Added `.node-version` with `22.23.2`.
- Added `package.json` engines for Node `>=22 <23` and pnpm `11.16.0`.
- Changed CI to read `.node-version` through `actions/setup-node`.
- Used the official portable Node 22.23.2 Windows x64 runtime for this Gate.
- Verified the portable archive against the official SHA-256: `1177b4137ba5adaa56354ae40f1080c7450e8ae09cecb47da459d1c52ac99f97`.
- Verified the validation process resolved Node 22.23.2 and pnpm 11.16.0.

The temporary local pnpm wrapper used to preserve Node 22 through Tauri's `beforeBuildCommand` was removed after validation and is not part of the repository.

## B. Compilation Fixes

**No source changes required after native compilation.**

The recovered MSVC environment compiled the existing M1-A Rust/Tauri source successfully. Clippy found no source warnings under `-D warnings`.

Two validation-harness observations did not require source changes:

1. `cargo metadata --format-version 1 --no-deps` has no manifest argument, so it failed when first invoked from the repository root. It passed when rerun from the `src-tauri` crate directory.
2. MSVC prints an informational import-library creation message while linking the library test/build target. Cargo surfaces that localized linker stdout as a warning, but the links, tests and Tauri build complete successfully.

The only committed implementation/configuration change in this recovery is Node 22 version alignment. No dependency was upgraded.

## C. Full Automated Results

All frontend commands below ran with Node 22.23.2 and pnpm 11.16.0. All native commands ran after x64 `VsDevCmd.bat` initialization in the same command process.

| Command | Result | Evidence |
| --- | --- | --- |
| `pnpm install --frozen-lockfile` | **PASS** | Lockfile already up to date; completed with pnpm 11.16.0 |
| `pnpm format` | **PASS** | All matched files use Prettier style |
| `pnpm typecheck` | **PASS** | TypeScript completed without diagnostics |
| `pnpm lint` | **PASS** | ESLint completed with zero errors/warnings |
| `pnpm test` | **PASS** | 2 files, 5 tests passed |
| `pnpm build` | **PASS** | 36 modules transformed; production renderer generated |
| `cargo metadata --format-version 1 --no-deps` from `src-tauri` | **PASS** | Cargo metadata resolved |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check` | **PASS** | No Rust formatting diff |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings` | **PASS** | Completed successfully with MSVC |
| `cargo test --manifest-path src-tauri/Cargo.toml --all-targets --all-features` | **PASS** | 6 passed, 0 failed |
| `pnpm tauri build --no-bundle` | **PASS** | Release executable generated at `src-tauri/target/release/agent-desk.exe` |

Native Rust tests executed successfully for:

- preference wire serialization;
- visible/off-screen geometry rules;
- SQLite migration and foreign-key initialization;
- transactional base-card create/read;
- source-integrity constraint rollback.

## D. Windows Native Smoke

Tests used the real release `agent-desk.exe`, the real WebView2 renderer, Win32 process/window state and native mouse/menu interactions. No browser renderer was substituted.

| Smoke item | Result | Evidence |
| --- | --- | --- |
| First launch creates one Agent Desk process/window | **PASS** | One process; responsive native window |
| Default 320×420 logical client size | **PASS** | UI Automation client 400×525 physical px at 125% DPI; outer frame 418×572 |
| Minimum 300×360 logical client size | **PASS** | `WM_GETMINMAXINFO` returned 393×497 physical outer tracking minimum at 125% DPI |
| Resizable native frame | **PASS** | `WS_THICKFRAME` present; native size changed from 418×572 to 518×697 |
| Window drag | **PASS** | Native title-bar drag moved the window by exactly +80,+50 physical px |
| Single instance | **PASS** | Second launch exited 0; original PID remained; existing window surfaced |
| Immediate always-on-top | **PASS** | `WS_EX_TOPMOST` changed from false to true after clicking Pin window |
| Always-on-top persistence | **PASS** | Preference remained true and `WS_EX_TOPMOST` was restored after clean restart |
| Position/size restore | **PASS** | Clean restart restored x=1388, y=242, 518×697 physical outer frame |
| Off-screen clamp | **PASS** | Saved x=3000, y=2000; clean restart restored to x=751, y=228, fully inside 1920×1080 |
| Close-to-tray | **PASS** | Native close hid the window while the original process remained alive |
| Tray Show | **PASS** | Clicking the actual hidden tray icon restored the existing window |
| Tray menu | **PASS** | Native menu exposed Show Agent Desk and Quit Agent Desk |
| Tray Quit | **PASS** | Native Quit terminated the process; process count became zero |
| Relaunch after Quit | **PASS** | Fresh process launched normally with intact state |
| System theme | **PASS** | Initial System selection followed the Windows dark appearance |
| Light theme | **PASS** | Immediate visual change; `preferences.json` persisted `theme: light` |
| Dark theme | **PASS** | Immediate visual change; clean restart persisted `theme: dark` |

Final smoke cleanup used the real tray Quit command. No Agent Desk process was left running.

## E. Native Screenshot Evidence

All screenshots are captures of the real Tauri release window or the real Windows tray UI.

| Evidence | Dimensions | SHA-256 |
| --- | ---: | --- |
| [`default-sticky.png`](evidence/m1-a/windows/default-sticky.png) | 418×572 | `b63361f0636812277489a9fa93c93ab05799cae30f6f52354513ec05750660a9` |
| [`light-mode.png`](evidence/m1-a/windows/light-mode.png) | 418×572 | `76af7601ab17e057b614540eb8cf0e8f3aae3d89c37dbf4aad9020e7dab5aeb4` |
| [`dark-mode.png`](evidence/m1-a/windows/dark-mode.png) | 418×572 | `9b5d366a6b4f72a0789f7e0c020dc732103516d1de9b96546db995022a8446e7` |
| [`tray-menu.png`](evidence/m1-a/windows/tray-menu.png) | 340×350 | `74bcfee9dd3264bd4038cc5487f0cd024e2d71bed011ff326540a3a119dc2536` |

The Dark screenshot was recaptured after clean restart and off-screen recovery; it also shows the persisted Pinned state.

## F. GitHub Remote

- Repository: `https://github.com/dawnsongbest-create/agent-desk`
- Visibility: **Private**, verified through the authenticated GitHub API before upload and after CI.
- Origin: `https://github.com/dawnsongbest-create/agent-desk.git`
- Pushed branch: `main`
- Remote history: all local commit/tree/blob SHA values were verified against GitHub; remote `main` matched local HEAD.
- Remote tree audit: workflow and M1-A report present; no tracked `node_modules/`, `dist/`, Rust `target/`, `.env`, database, or other checked build artifacts.

The standard `git push -u origin main` command was attempted, but this machine's Git/libcurl channel could not connect to `github.com:443` while authenticated GitHub API access remained healthy. The upload therefore used GitHub's authenticated Git Data API as a transport fallback. Every blob, tree, commit and final ref SHA was checked against local Git before the remote `main` ref was accepted; local `main` tracks `origin/main`.

## G. GitHub Actions

Final validation run: [CI run 31379834124](https://github.com/dawnsongbest-create/agent-desk/actions/runs/31379834124)

- Event: `push`
- CI-validated HEAD: `63f4673b6424023f62efb9aa8afd08ff2aa2055e`
- Windows job `windows-latest` (job `93427303848`): **PASS**
- macOS job `macos-latest` (job `93427303903`): **PASS**
- Node: `.node-version` resolved to `22.23.2` on both runners.
- Both jobs completed `pnpm install --frozen-lockfile`, frontend format/typecheck/lint/test/build, Rust fmt, Rust Clippy with `-D warnings`, Rust tests, and `pnpm tauri build --no-bundle`.
- No test, Clippy, platform job, or build step was skipped or marked `continue-on-error`.

Initial run `31379573974` exposed a real Windows-only failure at `pnpm format`: the Windows checkout converted text files to CRLF because the repository-wide `.gitattributes` rule did not specify an EOL. Prettier then reported 14 files as incorrectly formatted.

CI fix:

- Root cause: repository text EOL was platform-dependent under `* text=auto`.
- Minimal file changed: `.gitattributes`.
- Fix: `* text=auto eol=lf`.
- Commit: `63f4673 fix: normalize CI text line endings`.
- Local verification: affected files resolved to `eol=lf`; index/worktree EOL inspection passed; `pnpm format` passed.
- Remote verification: final Windows and macOS jobs both passed the complete workflow.

This CI result does not claim a separate macOS manual UX smoke; the current Gate requirement was real macOS build/CI execution.

## H. Architecture Regression Check

**PASS — no architecture deviation detected.**

- React contains no direct SQL access.
- React contains no scattered raw HTTP calls.
- Rust/Tauri still owns persistence, native shell and trusted boundaries.
- Preference writes remain serialized through the existing React queue and native Tauri Store boundary.
- React state is not the sole source of truth for durable preferences or domain data.
- M1-B functionality was not introduced.
- Inbox, Reader, Agent Gateway, HTTP Adapter, Notifications, Progressive Reading, Scheduled Tasks and Learning Enhancements were not started.
- `0001_card_foundation.sql` is byte-for-byte unchanged from implementation baseline commit `466f0f4`.
- JavaScript and Rust dependency lockfiles are unchanged; no unrelated dependency upgrade occurred.

## I. Git State

- Branch: `main`
- Existing implementation baseline: `466f0f4 feat: establish M1-A desktop foundation`
- Existing initial Gate evidence commit: `674b1d6 docs: finalize M1-A gate evidence`
- Native recovery/evidence commit: `816b487 chore: complete M1-A native validation`
- Upload ignore hardening: `dd85813 chore: harden upload ignores`
- CI-validated HEAD: `63f4673b6424023f62efb9aa8afd08ff2aa2055e`
- Remote: `origin` → `https://github.com/dawnsongbest-create/agent-desk.git`
- Upstream: `main` tracks `origin/main`
- Expected handoff worktree: clean after this report-only commit
- The final report commit SHA and its confirmation run are stated in the handoff because a commit cannot contain its own SHA or resulting run ID.

## Remaining Blocker and Recommendation

Windows and macOS M1-A engineering validation is complete. Do not begin M1-B yet.

The Gate can move to Tech Lead final review. GitHub Actions macOS success does not substitute for the separate pre-release macOS manual UX smoke requirement.

M1_A_GATE_STATUS: AWAITING_TECH_LEAD_FINAL_REVIEW
