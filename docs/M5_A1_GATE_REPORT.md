# M5-A1 — Implementation checkpoint

Implementation: `64f24b44222fe97f3e86627c56bbae52d4ee1725`.
Status: isolated native handoff, successful Codex-to-Reader delivery, and
canonical replacement are not yet verified. This report is not a Gate PASS.

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
| Real local Codex CLI | With temporary command-line server overrides and an ephemeral synthetic session, Codex discovered and called `send_reading`; the unpaired isolation returned `credential_unavailable` and no receipt |

Implementation CI: [Windows PASS](https://github.com/dawnsongbest-create/agent-desk/actions/runs/44555459140/job/135670210776),
[macOS PASS](https://github.com/dawnsongbest-create/agent-desk/actions/runs/44555459140/job/135670209984).
The tested local Codex executable reports CLI version `0.169.0-alpha.11.1`.
This verifies a real client error path, not successful pairing or delivery.

## Artifact provenance

Clean Windows truth build: Node 22.23.2, pnpm 11.16.0, MSVC x64,
`tauri build --no-bundle -- --locked`, fresh target.

| Artifact | SHA-256 |
|---|---|
| Truth release and MCP launch mode | `F179C05565428569E3F4A28041C45E9FB91C55855260F9A3AEAB03927676D3FD` |
| Production JS | `23CD319455E1A04198BDCB1A317818256F7441A822BEBD09021F682BB9F64D32` |
| Production CSS | `5694B0285627E155E6C2A67CA1364486DC02BFA72958098155A9AB8D1C421948` |
| Existing canonical (M4, not replaced) | `0D48DAF69B97DBCBDBCCD8F7AACFC3C7E5DAA112111613BF276C781869D121E4` |

Truth binary size: 16,188,928 bytes, written 2026-11-19 15:34:35 UTC.

## Remaining gate

The Product Owner's Agent Desk instance is running. It was not stopped or
altered. The Tauri single-instance behavior makes a concurrent same-session
launch unsuitable for exact-binary proof. No real credentials, Codex MCP
registration, user AppData, or canonical executable were changed.

After it exits, use isolated SQLite, Store, window state, WebView2 and OS
credential reference. Verify native pairing and revocation, actual Codex
successful send with an open Inbox, complete Reader Markdown, same-key replay,
Bridge stopped, and restart persistence. Check Product Owner AppData hashes
before and after. Replace canonical only after the scoped native checks pass.
Manual registration in Codex Desktop and macOS native pairing remain unverified.

Public safety: scoped changes scanned for secrets, personal paths, local
config, databases and release files; no findings in the pushed changes.
The repository stays public. No next phase was started.
