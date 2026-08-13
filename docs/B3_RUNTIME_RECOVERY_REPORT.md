# Agent Desk — B3 Runtime Recovery Report

## Recovery Result

The current M1-B3 release is available from the stable Product Owner preview path and has passed a real Windows launch check.

```text
PRODUCT_OWNER_PREVIEW_EXECUTABLE:
D:\agent-desk-target\release\agent-desk.exe
```

- File exists: PASS
- Size: 13,761,024 bytes
- SHA-256: `79FEE30A966D81A7A66B65DC84D75280DE4595407A654309BD6476917BDCFBFB`
- Process created from the canonical path: PASS
- Native `Agent Desk` window appeared: PASS
- Process remained responsive for 12 consecutive seconds: PASS
- WebView2 loaded `http://tauri.localhost/` with `document.readyState = complete`: PASS
- Sticky rendered from existing AppData: PASS
- A second launch activated the single existing instance and exited normally with code `0`: PASS

## Root Cause

The Product Owner's perceived “cannot open” behavior was caused by a hidden background instance / single-instance wake-up scenario. The previous executable remained alive and responsive in the background, but it had no enumerable top-level window. Launching that Temp executable again produced a short-lived second process while the hidden original instance remained unavailable to the user.

This was not an expired artifact, startup crash, database failure, migration failure, WebView2 load failure, or missing executable:

- The original Temp B3 executable still existed.
- Its SHA-256 matched the M1-B3 Gate artifact.
- SQLite `PRAGMA integrity_check` returned `ok`.
- Migrations `0001`, `0002`, and `0003` all remained successful.
- The stable-path launch produced a visible native window and a loaded Sticky WebView.

## Data Safety

No database reset, migration reset, AppData deletion, or empty database replacement was performed. Product source code was not changed.

Record, Todo, and Quote changes made during the user's interaction with the running application were treated as user operations and were not rolled back. The recovery process must not overwrite those changes.

## Stable Preview Rule

`AppData\Local\Temp\...` is not a long-term Product Owner preview location. The canonical Windows Product Owner preview path for this B3 release is:

```text
D:\agent-desk-target\release\agent-desk.exe
```

Temporary targets may still be used for isolated tests, but they must not be the only preview artifact delivered to the Product Owner.

## Build Environment Note

The frontend release build passed with Node `22.23.2` and pnpm `11.16.0`, and MSVC x64 `link.exe` was available. A clean Rust rebuild did not complete because the local Cargo source cache was incomplete and the crates.io network/index fetch did not finish in the available recovery window.

This does not affect use of the current validated B3 release at the canonical path. The canonical executable is an exact SHA-256 match of the previously validated current-M1-B3 release artifact. No dependency, lockfile, or source change was made to manufacture a build result.

## Git and Scope

- Branch: `main`
- Recovery baseline: `a0d0a03d07f8142c6e043c41a865b6e0910093d2`
- Product source changes: none
- Recovery scope ended here; M1-B4, Mini Tab implementation, Reader, and Agent integration were not started.

```text
B3_RUNTIME_RECOVERY_STATUS: PASS
READY_FOR_M1_B4: YES
```
