# M3-C2 Gate Report — OpenClaw Adapter & First Agent Workflow

## Scope Result

M3-C2 establishes the first real external-agent workflow:

```text
OpenClaw
    ↓
OpenClaw Adapter
    ↓
Local Agent Bridge (127.0.0.1, API v1, Bearer Token)
    ↓
Application Services
    ↓
Delivery / Proposal / Reading Domains
    ↓
Inbox → Reader → User Accept → Reading Session
```

No Chat page, Agent input box, direct Agent workspace mutation, Claude/Codex adapter, MCP, A2A, OAuth, cloud Agent platform, server, login, scheduler, notification, or M3-D work was added.

## Implementation Truth

- Implementation commit: `1abda22ec79027f3f87a71266401e40004dde7f6`
- Native evidence commit: `03501ef3d8fe9b16baaaa2fa12837ebc33d74d75`

| Boundary | File / symbol |
| --- | --- |
| OpenClaw DTO and mapping boundary | `src-tauri/src/adapters/openclaw.rs` — `OpenClawAdapter`, `OpenClawDeliveryRequest`, `OpenClawProposalRequest`, `OpenClawReadingPlanRequest` |
| Delivery application path | `OpenClawAdapter::create_delivery` → `DeliveryService::ingest` |
| Proposal application path | `OpenClawAdapter::create_proposal` → `ProposalService::create` |
| Reading application path | `OpenClawAdapter::create_reading_plan` → `ReadingService::create_plan` |
| Authenticated identity | `AgentConnectionService::authenticate_identity` |
| Versioned HTTP routing | `agent_bridge/server.rs` — `route_request` |
| Runtime composition | `AgentBridgeState`, `lib.rs` setup |

The adapter owns only application services. It does not receive a SQLite pool or persistence repository and cannot bypass domain validation. No migration was required; append-only migration history remains `0001` through `0008`.

Public integration contract: [OpenClaw API v1](M3_C2_OPENCLAW_API.md).

## API v1 Result

All endpoints require the existing local Bearer Token.

| Endpoint | Result |
| --- | --- |
| `GET /api/v1/health` | Authenticated local health |
| `GET /api/v1/capabilities` | Delivery, Proposal, Reading discovery |
| `POST /api/v1/delivery` | OpenClaw result → Delivery → Inbox / Reader |
| `POST /api/v1/proposal` | OpenClaw suggestion → pending Proposal |
| `POST /api/v1/reading-plan` | OpenClaw plan → ReadingPlan |

Contract behavior is proven for minimal Product payloads and the complete first-demo payload. Delivery requests may include a stable `idempotency_key`; identical retries do not duplicate and changed payloads return `409`.

Error handling:

- missing or invalid Token → `401 Unauthorized`;
- direct `/todo`, `/record`, or `/sticky` mutation attempt → `403 Forbidden`;
- invalid JSON, unknown fields, invalid domain data, or client-supplied `agent_id` → `400 Bad Request`;
- changed payload for an existing Delivery idempotency key → `409 Conflict`;
- application/persistence unavailability → `503 Service Unavailable`.

## Identity and Security Gate

- The listener remains explicit IPv4 loopback `127.0.0.1`; it never binds `0.0.0.0`.
- Identity is returned by the authenticated `AgentConnection`; client input cannot establish trusted Agent identity.
- The existing random local Token and hash-only SQLite storage remain unchanged.
- The Bridge and adapter do not log Token values, Record/Todo/Quote content, or user workspace content.
- Request bodies are bounded, timed, UTF-8 JSON payloads; transfer encoding is rejected by the minimal local server.
- The OpenClaw adapter has no SQLite dependency and cannot directly create Todo, Record, Quote, or Sticky data.
- Formal smoke DB contains zero plaintext `adk_` matches.
- Public-tree scan found no credential value, `.env`, user database, build output, user profile path, or Product Owner data.
- Repository visibility remained `public`; default branch remained `main`.

## Automated Gates

Exact local toolchain: Node `22.23.2`, pnpm `11.16.0`, Rust `1.97.1`, Cargo.lock locked, `x86_64-pc-windows-msvc`, MSVC `14.51.36231` Hostx64/x64.

| Check | Result |
| --- | --- |
| Prettier | PASS |
| TypeScript | PASS |
| ESLint `--max-warnings 0` | PASS |
| Frontend Vitest | PASS — 79/79 |
| Frontend production build | PASS — 222 modules |
| Rust fmt | PASS |
| Rust Clippy `-D warnings` | PASS |
| Rust tests | PASS — 47/47 |
| Fresh non-bundled Tauri release | PASS |

Rust coverage includes OpenClaw Delivery / Proposal / Reading mapping, authenticated HTTP routes, capability discovery, `401` / `403` / `400`, idempotency, Proposal acceptance/rejection, and restart persistence. Existing frontend and Rust regression suites retain Sticky, Todo, Record large-text, Reader, Inbox, Delivery, Reading Agent, Proposal, and Bridge coverage.

## Native Windows Smoke

Formal smoke used only isolated SQLite, Store, window state, and WebView2 data under:

```text
D:\agent-desk-m3c2-smoke-1abda22-20260822-04
```

The exact canonical executable completed this real Tauri/WebView2 sequence:

1. Native process and window appeared; WebView2 loaded; process remained alive beyond 10 seconds.
2. Settings enabled the localhost Bridge and generated a one-time Token.
3. Reload removed plaintext Token from the UI; authenticated health and capability discovery succeeded.
4. Simulated OpenClaw created `AI Daily Research`; identical retry did not duplicate; changed payload returned `409`.
5. Direct Todo mutation returned `403`; spoofed `agent_id` returned `400`.
6. OpenClaw created one Reading Proposal linked to the Delivery and one `Transformer Study` ReadingPlan.
7. Inbox opened the Delivery into Reader and displayed the linked pending Proposal.
8. User action `加入今日阅读` accepted the Proposal and opened the resulting Reading Session.
9. Full process restart restored the same Token authentication, accepted Reading Session, Delivery/opened state, and ReadingPlan.
10. Final shutdown left zero smoke Agent Desk processes.

Read-only restart audit found exactly one Delivery (opened), one accepted Proposal, one ReadingPlan, one ReadingSession, and zero workspace cards. This proves the Agent workflow did not directly mutate Todo or Record.

Evidence: [runtime JSON](evidence/m3-c2/windows/native-smoke-runtime.json), [Bridge enabled without plaintext Token](evidence/m3-c2/windows/01-bridge-enabled-redacted.png), [Delivery and pending Proposal](evidence/m3-c2/windows/02-delivery-reader-proposal.png), [accepted Reading Session](evidence/m3-c2/windows/03-proposal-accepted-reading-session.png), [ReadingPlan](evidence/m3-c2/windows/04-reading-plan.png), [restart Reading Session](evidence/m3-c2/windows/05-restart-reading-session.png), and [restart ReadingPlan](evidence/m3-c2/windows/06-restart-reading-plan.png).

## Artifact Truth

| Artifact | SHA-256 |
| --- | --- |
| `dist/index.html` | `60571290D6FF6BE5F135BD0C66CD012D894E8AD268A288C01FB7CEBAC95E24BF` |
| `dist/assets/index-CKRSNhVG.css` | `CB3811CCA635265E9BC3C0F72FA602DEEAB683EA050F9613DC2A79F51EF3BA36` |
| `dist/assets/index-jZ25nxVs.js` | `EA307EC245A3857253F7F775B6EFA8D96C7588D6C2A53700AF5DE9E0FE202312` |
| truth executable | `380E75E14926D25395F02B4E939F7761DF1FA36503CA199CDE52EA6C6A3AB9B6` |
| canonical executable | `380E75E14926D25395F02B4E939F7761DF1FA36503CA199CDE52EA6C6A3AB9B6` |

Truth executable:

```text
D:\agent-desk-m3c2-truth-1abda22\release\agent-desk.exe
```

`PRODUCT_OWNER_PREVIEW_EXECUTABLE`: `D:\agent-desk-target\release\agent-desk.exe`

Truth and canonical are byte-identical, size `15,602,688` bytes. Provenance: [Windows provenance](evidence/m3-c2/windows/provenance.json).

## Hosted CI

GitHub Actions run [`32579818183`](https://github.com/dawnsongbest-create/agent-desk/actions/runs/32579818183) validated implementation commit `1abda22ec79027f3f87a71266401e40004dde7f6` on public `main`.

- macOS `97047386629`: PASS.
- Windows `97047386744`: PASS.

Both jobs ran the full workflow, including frontend format/typecheck/lint/tests/build, Rust fmt/Clippy/tests, and the real non-bundled Tauri build.

```text
M3_C2_GATE_STATUS: AWAITING_PRODUCT_AND_TECH_REVIEW
OPENCLAW_ADAPTER_STATUS: READY
AGENT_WORKFLOW_STATUS: FOUNDATION_READY
MCP_STATUS: NOT_STARTED
A2A_STATUS: NOT_STARTED
```

Do not enter M3-D before Product and Tech review.
