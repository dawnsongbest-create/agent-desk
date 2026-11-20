# M5-B0 Baidu DuMate Personal MCP Connectivity Report

**Gate: PASS.** DuMate Personal reached an IPv4 loopback MCP endpoint on the same Windows host, supplied valid Bearer authorization, discovered the single tool, and invoked it both before and after restarting the client. This was an echo experiment, not an Agent Desk integration.

## Environment and configuration

- DuMate Personal 1.0.93, verified from the installed executable's file version; Windows 11 Home, build 26200; Node.js 24.14.0.
- Standalone, in-memory-token Streamable HTTP Echo Server and independent protocol probe in a temporary directory. No SDK or additional global packages. It listened on `127.0.0.1` and an OS-assigned temporary port, at `/mcp`; no tunnel, public listener, or gateway. Only `ping_test` was advertised.
- The installed client UI exposed **插件·技能 → 添加 → 自定义连接器**. The operator used manual configuration, `Agent-Desk-MCP-Connectivity-Test` (the UI rejected an earlier name containing spaces), Streamable HTTP, the active loopback URL, and an `Authorization` header containing a one-time test Bearer. No credential or complete config is retained here.
- [MCP transport specification](https://modelcontextprotocol.io/specification/2025-11-25/basic/transports) and [Baidu connector configuration reference](https://aca.bce.baidu.com/doc/Dumate/s/9mrdg87nq).

## Verification record

The independent local probe negotiated `2025-11-25`, validated initialization and notification handling, discovered exactly `ping_test`, and compared its synthetic nonce and run ID. Absent/wrong Bearer returned 401; Host, Origin, media types, size limit, protocol errors and post-stop refusal were checked. Probe logs contained neither the test credential nor the nonce. The **exact DuMate-negotiated protocol revision was not recorded**.

The table below is a selected projection of the Echo log cross-checked against the operator's DuMate observations and supplied tool replies. Times are UTC on 2026-11-20.

| Evidence | Server observation | Client evidence / interpretation |
| --- | --- | --- |
| Reachability, before authorization | 06:09:32.460 POST → `missing_authorization`; a later submission at 06:11:51.747 → `invalid_authorization` | 401 on discovery, proving a request reached this loopback listener, not a network block. |
| Discovery | 06:13:43.124 `initialize`, then initialized notification and 06:13:43.135 `tools/list`, all authorized | Operator reported successful save; the server's list contained only `ping_test`. |
| First invocation | 06:14:54.765 `tools/call`; authorized, invoked, success; anonymous request ID `99188b7b-516c-42f4-a3af-b1944261bb84` | Reply: `ok=true`, nonce `dumate-b0-local-test`, received at 06:14:54.764. |
| After DuMate restart | New authorized `initialize` at 06:17:39.654, then 06:17:39.664 `tools/call`; authorized, invoked, success; anonymous request ID `8f016ea4-c504-43a6-aabc-761886c6ec3d` | Operator's restart-test reply: `ok=true`, nonce `dumate-b0-local-restart-test`, received at 06:17:39.664. Both replies identify run `82fb4a1a-c024-4611-8281-756b15c9d071`. |

The restart itself was an operator action; the fresh initialization and successful call against the same listener were verified on the server. The log deliberately omitted request contents; the nonce and response timestamps above are from the operator-supplied tool replies, not a body dump. DuMate also attempted an authenticated GET for a server stream. This stateless server declines that optional stream with 405; the authenticated POST tool calls still succeeded. No SSE fallback was needed.

## Cleanup and limits

- The self-test server was stopped and the probe received connection refused. The real-client run was then stopped; process and listener rechecks found zero matching Echo processes and no remaining listener on the tested port. Its in-memory token is unusable once the service stops.
- The operator was asked to remove the test connector; removal from DuMate was **not independently confirmed**. No test program, raw log, credentials, private path, or AppData is checked in.
- Verified on the tested Windows build and DuMate 1.0.93 only. No macOS DuMate run; no Agent Desk business write, production build, or canonical replacement. Successful localhost traffic and restart do not prove that DuMate keeps its connector headers out of cloud sync, logs, or remote storage.
- Agent Desk was not changed. Baseline `993deb8ca0085f79d1273568f686c02d36d6960e`. Recommend evaluating a local Streamable HTTP adapter for B1; there is no evidence requiring a Remote Gateway.

```text
M5_B0_GATE_STATUS: PASS
DUMATE_MCP_DISCOVERY_STATUS: PASS
DUMATE_TOOL_INVOCATION_STATUS: PASS
LOCAL_LOOPBACK_ACCESS_STATUS: PASS
AUTHORIZATION_HEADER_STATUS: PASS
DUMATE_RESTART_RECONNECT_STATUS: PASS
M5_B1_IMPLEMENTATION_STATUS: NOT_STARTED
```
