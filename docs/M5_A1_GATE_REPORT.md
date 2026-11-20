# M5-A1 — Gate handoff

Implementation: `64f24b44222fe97f3e86627c56bbae52d4ee1725`.
Status: isolated Windows Codex-to-Reader smoke and canonical artifact verification
complete; awaiting Tech Lead and Product Owner review.

## Implementation

- Added only `0009_codex_connection.sql`; legacy migrations and Delivery /
  Reader tables are unchanged. Codex uses its own credential hash, scope,
  revocation state, and private idempotency namespace. Connecting Codex does
  not rotate the OpenClaw credential.
- Added `POST /api/v1/reading-deliveries` and the single STDIO tool
  `send_reading`. The release executable runs the connector with
  `--codex-mcp <bridge-port>`; the desktop and connector are two modes of
  the same binary. See [pairing, contract and rollback](M5_A1_CODEX_READING_DELIVERY.md).
- Each new send receives a random request ID unless explicitly supplied.
  One in-process retry uses the same ID and body. Reusing a key with different
  content conflicts; a new key can send the same Markdown again. Inbox
  invalidation is emitted after commit only on newly created deliveries.

## Verified

| Layer | Result |
|---|---|
| Frontend (Windows) | 98 tests passed; TypeScript, ESLint, Prettier and Vite production build passed |
| Rust (Windows) | 58 tests passed; rustfmt and Strict Clippy passed |
| Windows Credential Manager | A separate opt-in integration test wrote, read and removed a random synthetic local-machine entry |
| macOS Keychain | The CI job created an ephemeral keychain and the native adapter round trip passed; no macOS desktop pairing smoke |
| OpenClaw | Legacy authenticated Delivery / Proposal / Reading Plan contract tests and legacy credential isolation passed; no new native OpenClaw client run |
| Windows release STDIO process | MCP 2025-11-25 negotiated; discovered only `send_reading`; missing isolated credential returned `credential_unavailable`; no stderr |
| Real local Codex CLI | Temporary STDIO registration and ephemeral sessions exercised first send, same-key replay, stopped-Bridge rejection, and rejection after revocation with Bridge still listening |
| Windows Tauri Native Smoke | Isolated migration 0009 and pairing worked; open Inbox showed the new unread row without re-entering Inbox; Reader rendered the synthetic heading, list, Unicode and bold text; restart restored the report and opened state; revocation preserved the report |

Implementation CI: [Windows PASS](https://github.com/dawnsongbest-create/agent-desk/actions/runs/44555459140/job/135670210776),
[macOS PASS](https://github.com/dawnsongbest-create/agent-desk/actions/runs/44555459140/job/135670209984).
The local CLI first verified the unpaired error path on `0.169.0-alpha.11.1`.
The native delivery checks used `0.169.1` and did not persist a Codex MCP registration.

### Sanitized native evidence

- A real Codex session sent a synthetic Markdown report using `send_reading`.
  The committed receipt returned `created=true`, Delivery
  `delivery_ba5970d14aefbb0385e7c342bbd99ff6` and Reader
  `reader_d61436f4211670a6cfb48e4e1fdc2ae5`.
- A second real Codex session with the same key and identical payload returned
  those same two IDs and `created=false`. Isolated SQLite still held one delivery.
- After the isolated process stopped, port 47791 had no listener and the real
  connector returned `bridge_unavailable` without a receipt.
- A fresh process of the same build restored the same report in Reader.
  SQLite matched the pre-restart pairing state, delivery ID, document ID,
  exact Markdown and opened state.
- The Product Owner revoked the isolated pairing. The stored state became
  `revoked` while the process still listened on IPv4 loopback port 47791.
  A real Codex tool call returned `credential_unavailable` with no receipt,
  and the delivery set did not change. Separate automated tests cover
  server-side rejection and preservation of the legacy authorization.

## Artifact provenance

Clean Windows truth build: Node 22.23.2, pnpm 11.16.0, MSVC x64,
`tauri build --no-bundle -- --locked`, fresh target.

| Artifact | SHA-256 |
|---|---|
| Truth release and MCP launch mode | `F179C05565428569E3F4A28041C45E9FB91C55855260F9A3AEAB03927676D3FD` |
| Production JS | `23CD319455E1A04198BDCB1A317818256F7441A822BEBD09021F682BB9F64D32` |
| Production CSS | `5694B0285627E155E6C2A67CA1364486DC02BFA72958098155A9AB8D1C421948` |
| Installed canonical release and MCP launch mode | `F179C05565428569E3F4A28041C45E9FB91C55855260F9A3AEAB03927676D3FD` |
| Archived previous artifact (M4; not a schema-compatible rollback) | `0D48DAF69B97DBCBDBCCD8F7AACFC3C7E5DAA112111613BF276C781869D121E4` |

Truth binary size: 16,188,928 bytes, written 2026-11-19 15:34:35 UTC.

## Installation, isolation and review boundaries

The Product Owner exited the previous instance. The Windows smoke used
isolated SQLite, Store, window state, WebView2 and an isolated OS credential.
After revocation passed, the tested artifact atomically replaced canonical,
and the installed binary restarted with the same isolated fixture. The
isolated process was then stopped. SHA-256/presence checks for the five
formal SQLite, Store and window-state files matched before and after.
No formal credential or persistent Codex MCP registration was changed.

Manual registration in Codex Desktop and native pairing on a macOS machine
remain unverified. Keychain CI is not a macOS desktop acceptance result.
No new native OpenClaw client run was performed; its compatibility result
comes from the authenticated contract and isolation tests above. Do not
restore the archived M4 executable over a database already migrated to 0009.
Use the schema-compatible disable path documented in the delivery guide.

Public safety: scoped changes scanned for secrets, personal paths, local
config, databases and release files; no findings in the pushed changes.
The repository stays public. No next phase was started.

```text
M5_A1_TECH_GATE_STATUS: AWAITING_TECH_REVIEW
M5_A1_PRODUCT_GATE_STATUS: AWAITING_PRODUCT_REVIEW
CODEX_MCP_STATUS: READY_FOR_REVIEW
CODEX_PAIRING_STATUS: READY_FOR_REVIEW
READING_DELIVERY_STATUS: READY_FOR_REVIEW
OPENCLAW_COMPATIBILITY_STATUS: VERIFIED
TODO_PRIORITY_STATUS: NOT_STARTED
BAIDU_DUMATE_STATUS: NOT_STARTED
```
