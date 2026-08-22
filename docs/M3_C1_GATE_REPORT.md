# M3-C1 Gate Report — Local Agent Bridge Foundation

## Scope Result

M3-C1 establishes the local connection boundary:

```text
External Agent
    ↓
Local Agent Bridge (127.0.0.1, API v1, Bearer Token)
    ↓
Agent Desk application services
    ↓
Delivery / Proposal / Reading contracts
```

No Chat page, Agent panel, workspace auto-mutation, OpenClaw business adapter, MCP, A2A, OAuth, server, account system, cloud sync, scheduler, notification, or M3-C2 work was added.

## Implementation Truth

Implementation commit: `8acd8be85523187fd43a83d74ddf173332b022c8`

| Boundary | File / symbol |
| --- | --- |
| API and connection contract | `src-tauri/src/domain/agent_connection.rs` — `AgentRequest`, `AgentResponse`, `AgentCapabilities`, `AgentConnection` |
| Application boundary | `AgentConnectionService`, `CapabilityService` |
| Persistence port | `AgentConnectionRepository` |
| SQLite adapter | `SqliteAgentConnectionRepository` |
| HTTP lifecycle | `AgentBridgeState`, `RunningAgentBridge` |
| OpenClaw placeholder | `adapters/openclaw.rs` — `OpenClawAdapter` |
| Native settings entry | `AgentBridgeSettings`, `useAgentBridge` |

The HTTP server receives only versioned Bridge requests and calls application services. It has no SQLite dependency and cannot create a Delivery, Proposal, ReadingPlan, Todo, Record, Quote, or Sticky mutation in M3-C1.

## API v1 Contract

All implemented endpoints require `Authorization: Bearer <local-token>`.

| Endpoint | Result |
| --- | --- |
| `GET /api/v1/health` | `{ "version": "v1", "status": "ok" }` |
| `GET /api/v1/capabilities` | Delivery, Proposal, and Reading capability discovery |
| Missing/invalid token | `401 unauthorized` |
| Unknown versioned path | `404 not_found` |
| Unsupported method | `405 method_not_allowed` |

The future action names `createDelivery`, `createProposal`, and `createReadingPlan` are represented in the versioned contract only. No external mutation endpoint is implemented.

## Security Gate

- Listener bind is explicit IPv4 loopback: `127.0.0.1`; `0.0.0.0` is not used.
- The token is generated from two random UUID v4 values and has 244 random bits after UUID version/variant bits.
- Plaintext is returned only from the Generate/Rotate command and held only in transient UI state.
- SQLite stores a lowercase 64-character SHA-256 hash, never plaintext.
- Authentication compares equal-length hashes without early byte exit.
- Health and capability responses expose no Record, Todo, Quote, Reader, Delivery, or Proposal content.
- Bridge errors and runtime code do not log the token or workspace content.
- The public evidence screenshot replaces the one-time Token text before capture.
- Public-tree scan found no credential value, user database, local AppData, build artifact, or machine-specific user path.

## Migration and Persistence

`0008_agent_connection.sql` is append-only; migrations `0001` through `0007` are unchanged. The new table contains only connection identity, optional token hash, active/inactive state, and timestamps.

Fresh and restart tests prove:

- migration history is `[1, 2, 3, 4, 5, 6, 7, 8]`;
- one connection survives restart;
- active state and token hash survive restart;
- old Token fails immediately after rotation;
- successful authentication records `last_used_at`;
- isolated smoke DB contains no plaintext `adk_` prefix.

## Automated Gates

Exact local toolchain: Node `22.23.2`, pnpm `11.16.0`, Rust `1.97.1`, locked Cargo graph, `x86_64-pc-windows-msvc`.

| Check | Result |
| --- | --- |
| Prettier | PASS |
| TypeScript | PASS |
| ESLint `--max-warnings 0` | PASS |
| Frontend Vitest | PASS — 79/79 |
| Frontend production build | PASS — 222 modules |
| Rust fmt | PASS |
| Rust Clippy `-D warnings` | PASS |
| Rust tests | PASS — 46/46 |
| Fresh non-bundled Tauri release | PASS |
| Windows GitHub Actions | PASS — job `96997567709` |
| macOS GitHub Actions | PASS — job `96997567627` |

Regression coverage includes Sticky, Todo, Record large-text save/collapse, Reader, Inbox, Delivery, Reading Agent, and Proposal acceptance/rejection.

## Native Windows Smoke

Formal smoke used isolated SQLite, Store, window state, and WebView2 data under:

```text
D:\agent-desk-m3c1-smoke-8acd8be-20260822-02
```

1. The truth executable remained alive beyond 10 seconds.
2. The real Tauri settings UI enabled the Bridge and generated a Token.
3. Invalid auth returned `401`; authenticated health and capability calls returned `200`.
4. Capability discovery returned `delivery=true`, `proposal=true`, `reading=true`.
5. After full process shutdown and restart, the same Token remained valid and Bridge restored automatically.
6. The restarted UI did not reveal plaintext Token; Sticky and Reader remained visible.
7. The canonical copy then remained alive beyond 10 seconds, loaded WebView2, and restored the Bridge.
8. Final audit found zero residual smoke Agent Desk processes.

Evidence: [native smoke JSON](evidence/m3-c1/windows/native-smoke-runtime.json), [enabled and redacted Token UI](evidence/m3-c1/windows/01-bridge-enabled-token-issued-redacted.png), and [restart recovery UI](evidence/m3-c1/windows/02-bridge-restored-after-restart.png).

## Artifact Truth

| Artifact | SHA-256 |
| --- | --- |
| `dist/index.html` | `60571290D6FF6BE5F135BD0C66CD012D894E8AD268A288C01FB7CEBAC95E24BF` |
| `dist/assets/index-CKRSNhVG.css` | `CB3811CCA635265E9BC3C0F72FA602DEEAB683EA050F9613DC2A79F51EF3BA36` |
| `dist/assets/index-jZ25nxVs.js` | `EA307EC245A3857253F7F775B6EFA8D96C7588D6C2A53700AF5DE9E0FE202312` |
| truth executable | `84D7A89AD908B338D89446C1021BE71EA0D1DA27B246EED905F3248F0216A920` |
| canonical executable | `84D7A89AD908B338D89446C1021BE71EA0D1DA27B246EED905F3248F0216A920` |

Truth executable:

```text
D:\agent-desk-m3c1-truth-target\release\agent-desk.exe
```

`PRODUCT_OWNER_PREVIEW_EXECUTABLE`: `D:\agent-desk-target\release\agent-desk.exe`

Truth and canonical are byte-identical, size `15,324,160` bytes. Production bundle inspection proves the new Bridge UI, Tauri commands, loopback endpoint, API version, and one-time Token behavior are embedded in the shipped assets.

Provenance: [Windows provenance](evidence/m3-c1/windows/provenance.json).

## Hosted CI

GitHub Actions run [`32559004365`](https://github.com/dawnsongbest-create/agent-desk/actions/runs/32559004365) validated the Public `main` handoff commit `cc650dc0aa1599553929f07f6aa1f6e766b49698`.

- Windows `96997567709`: PASS. Format, typecheck, lint, 79 frontend tests, Rust fmt, Clippy, 46 Rust tests, and non-bundled Tauri build all passed.
- macOS `96997567627`: PASS. The same full workflow, including the real non-bundled Tauri build, passed.
- Repository visibility remained `public`; default branch remained `main`.

```text
M3_C1_GATE_STATUS: AWAITING_PRODUCT_AND_TECH_REVIEW
AGENT_BRIDGE_STATUS: FOUNDATION_READY
OPENCLAW_STATUS: READY_FOR_ADAPTER_IMPLEMENTATION
MCP_STATUS: NOT_STARTED
A2A_STATUS: NOT_STARTED
```

Do not enter M3-C2 until Product and Tech review.
