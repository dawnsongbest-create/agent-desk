# Agent Desk — M1-A Gate Report

Date: 2026-08-10
Branch: `main`
Scope: M1-A foundation only

## 1. Change Summary

- Added a reproducible pnpm/Tauri 2/React/TypeScript/Vite scaffold with committed JavaScript and Rust lockfiles.
- Added formatting, type-checking, linting, unit-test and production-build commands.
- Added a Windows/macOS GitHub Actions matrix that runs frontend checks, Rust formatting, Clippy, Rust tests and a non-bundled Tauri build.
- Established domain, application-port, infrastructure and native-shell boundaries without implementing M1-B or later product features.
- Added the append-only SQLite `0001_card_foundation.sql` migration, connection bootstrap and a transactional base-card repository.
- Added versioned theme and always-on-top preferences persisted through the native Tauri Store boundary.
- Added the single-instance-first native bootstrap, window position/size restoration, off-screen clamping, close-to-tray behavior and tray Show/Quit commands.
- Added a minimal Sticky visual shell with System/Light/Dark controls and an always-on-top control. It intentionally contains no Note/Todo product editor.
- Amended the M0 gate report and recorded the Tech Lead decisions in ADR 0001 before starting M1-A implementation.

## 2. Architecture Deviations

No implementation deviation from the approved M1-A architecture was introduced.

Validation is incomplete because this Windows host does not have the required MSVC linker. Installing Visual Studio Build Tools requires an administrator elevation that was declined, so no native binary could be compiled or smoke-tested. This is a gate blocker, not an approved architecture deviation.

## 3. Dependency Manifest

Exact resolved versions are taken from `pnpm-lock.yaml` and `src-tauri/Cargo.lock`.

### Runtime and build tools

| Dependency | Resolved version | Reason |
| --- | ---: | --- |
| pnpm | 11.16.0 | Deterministic package-manager entry point |
| TypeScript | 5.8.3 | Static checking for renderer and tests |
| Vite | 7.3.6 | Renderer development and production build |
| `@vitejs/plugin-react` | 4.7.0 | React transformation for Vite |
| `@tauri-apps/cli` | 2.11.4 | Native development/build orchestration |
| Tauri Rust crate | 2.11.5 | Desktop runtime and native window/tray APIs |
| `tauri-build` | 2.6.3 | Tauri build-time configuration |

### Renderer and quality dependencies

| Dependency | Resolved version | Reason |
| --- | ---: | --- |
| React / React DOM | 19.2.8 | Minimal renderer UI |
| `@tauri-apps/api` | 2.11.1 | Typed renderer-to-native invocation |
| Vitest | 3.2.7 | Renderer unit tests |
| Testing Library React | 16.3.2 | Behavior-oriented component tests |
| Testing Library jest-dom | 6.9.1 | DOM assertions |
| Testing Library user-event | 14.6.3 | User-level interaction tests |
| jsdom | 26.1.0 | Browser-like test environment |
| ESLint | 9.39.5 | Static linting |
| typescript-eslint | 8.66.0 | TypeScript ESLint integration |
| eslint-plugin-react-hooks | 5.2.0 | React hook correctness |
| eslint-plugin-react-refresh | 0.4.26 | Safe Fast Refresh exports |
| Prettier | 3.9.6 | Deterministic formatting |

### Native dependencies

| Dependency | Resolved version | Reason |
| --- | ---: | --- |
| SQLx | 0.9.0 | Async SQLite connection, migrations and repository queries |
| `tauri-plugin-single-instance` | 2.4.3 | One-process desktop behavior; registered first |
| `tauri-plugin-store` | 2.4.4 | Versioned local preferences |
| `tauri-plugin-window-state` | 2.4.1 | Window position/size persistence |
| async-trait | 0.1.92 | Async repository port implementation |
| serde / serde_json | 1.0.229 / 1.0.151 | Native preference and domain serialization |
| thiserror | 2.0.20 | Typed persistence errors |
| Tokio | 1.53.1 | Native async test runtime |
| tempfile | 3.27.0 | Isolated SQLite test databases |

Toolchain observed on this host: Node 24.14.0, rustc/cargo 1.97.1, rustfmt 1.9.0-stable and Clippy 0.1.97. CI standardizes Node 22 and Rust stable.

## 4. Migration Report

Migration `0001_card_foundation.sql` is the only schema migration.

It creates one common `cards` table with:

- stable card identity and constrained card type;
- lifecycle and attention fields;
- user/agent source integrity constraints;
- JSON-validated metadata;
- created/updated and lifecycle timestamps;
- type/lifecycle and attention indexes.

The migration intentionally does not add Note/Task payloads, Inbox, Reader, Agent Gateway, notification, reading-progress or scheduling tables. Migration execution and repository transaction tests are present in source, but could not be compiled on this host because `link.exe` is missing.

## 5. Automated Test and Build Results

| Command | Result | Evidence |
| --- | --- | --- |
| `pnpm install --frozen-lockfile --offline --store-dir D:\agent-desk-pnpm-store` after removing workspace `node_modules` | PASS | 271 packages installed; esbuild postinstall completed |
| `pnpm format` | PASS | All matched files use Prettier style |
| `pnpm typecheck` | PASS | TypeScript completed with no diagnostics |
| `pnpm lint` | PASS | ESLint completed with zero warnings/errors |
| `pnpm test` | PASS | 2 files, 5 tests passed |
| `pnpm build` | PASS | 36 modules transformed; production renderer generated |
| `cargo metadata --format-version 1 --no-deps` | PASS | Manifest and target graph resolved |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check` | PASS | No formatting diff |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings` | BLOCKED | MSVC `link.exe` not found while compiling build scripts |
| `cargo test --manifest-path src-tauri/Cargo.toml --all-targets --all-features` | BLOCKED | Same missing MSVC linker; tests did not execute |
| `pnpm tauri build --no-bundle` | BLOCKED | Renderer build passed; native compilation stopped because `link.exe` was not found |

The Rust test sources cover migration/bootstrap, foreign-key activation, transactional create/read, constraint rollback, off-screen geometry and preference JSON compatibility. They are not counted as passing until compiled and executed.

## 6. Windows Native Smoke Checklist

No native smoke item was run because no Windows executable could be produced.

| Check | Status |
| --- | --- |
| First launch opens one resizable 320×420 Sticky window with 300×360 minimum | NOT RUN |
| Second launch surfaces/focuses the existing instance | NOT RUN |
| Position and size restore after restart | NOT RUN |
| Off-screen saved geometry is clamped to an available monitor | NOT RUN |
| Always-on-top applies immediately and persists | NOT RUN |
| Close hides to tray instead of exiting | NOT RUN |
| Tray left-click/Show surfaces the window | NOT RUN |
| Tray Quit persists window state and exits | NOT RUN |
| System/Light/Dark preferences persist | NOT RUN |

## 7. macOS Build Evidence

`.github/workflows/ci.yml` defines a `macos-latest` job with the same frontend and Rust gates as Windows and the corrected `pnpm tauri build --no-bundle` command.

This repository has no configured Git remote, so the workflow could not be pushed or executed. There is no green macOS build evidence for this gate.

## 8. Screenshots

No native screenshots were produced. A renderer-only browser screenshot would not validate the required window chrome, always-on-top behavior, tray integration or native sizing, so it is not presented as native evidence.

## 9. Known Issues and Gate Recommendation

1. **Gate blocker:** Visual Studio Build Tools with the Visual C++ workload is absent; Tauri's supported Windows MSVC target therefore cannot link.
2. Rust type-checking beyond build-script compilation, Clippy, Rust test execution and Tauri native build remain unverified.
3. All Windows native smoke checks remain unverified.
4. The macOS CI job is configured but has not run because no remote is configured.
5. No native screenshots are available.

Recommendation: do not mark M1-A as passed. Install the MSVC Build Tools with approval, rerun all Rust/Tauri gates and the Windows smoke checklist, then push to a remote and require a green macOS CI job before Tech Lead acceptance.

## 10. Git State

- Branch: `main`
- M1-A implementation baseline: `466f0f4 feat: establish M1-A desktop foundation`
- Remote: none configured
- Worktree: clean immediately after the implementation baseline commit; this Gate Report state note is committed separately

M1_A_GATE_STATUS: AWAITING_TECH_LEAD_REVIEW
