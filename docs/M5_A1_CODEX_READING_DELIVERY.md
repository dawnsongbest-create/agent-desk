# Codex Reading Delivery

Agent Desk accepts a report from a local Codex session through one STDIO MCP tool,
`send_reading`. The connector cannot execute commands, read files or the
database, or write Todo, Record, Sticky, Proposal, or Reading Plan.

## Connect

1. Install Agent Desk in its normal application location. The connector is a
   second launch mode of the installed Agent Desk binary, not a separate service
   or a checkout-relative script. Moving or uninstalling the application can
   invalidate the registered path.
2. Start Agent Desk, open the Agent Bridge settings, enable the Bridge and
   choose **连接 Codex**. Use the executable and argument fields shown there.
3. In the local Codex app's MCP settings, add a **STDIO** server, for example
   `agent-desk-reading`. Enter the installed executable as the command and
   `--codex-mcp` and the shown port as separate arguments. Do not use a shell,
   pass a token, edit any other MCP registration, or overwrite a server with that
   name. Restart Codex and check that the server discovers only `send_reading`.
4. Back in Agent Desk, choose **已注册，授权 Codex**. This stores the secret in
   Windows Credential Manager (local machine persistence) or macOS Keychain
   (user/login domain), not in Codex settings. macOS may request OS permission
   to access the item; denying it fails safely without a plaintext substitute.
5. Ask Codex to send a synthetic report. Confirm the Inbox item, or use the
   actionable error to resolve a disconnected Bridge or credential. Agent Desk
   reports the connection ready after its first committed send.

To uninstall the connection, click **断开 Codex** first, then remove only the
registration you added in Codex. The app deliberately does not edit Codex
configuration. Restarting a connector will reread the OS entry. If the local
credential has disappeared or no longer matches the DB, reauthorize once; it
will not rotate the legacy OpenClaw token.

## Contract

`send_reading`: required `title`, `content` (full Markdown, not a path), and
`type` (`article`, `report`, `brief`); optional `subtitle` and
`idempotency_key`. The connector supplies a random key when omitted and sends
UTF-8 JSON to the authorized loopback `POST /api/v1/reading-deliveries` route.
The route requires a key. The serialized body is limited to 1 MiB, title and
subtitle to 500 characters each. There is no silent truncation or split.
Caller-supplied identity and source metadata are not accepted. The receipt is
returned after the existing Delivery transaction and matches the request key,
operation and both stored identifiers; health alone proves no delivery.

New independent tool calls with no key are separate deliveries, even with the
same text. The one bounded in-process retry reuses the same key/JSON. An
explicit key (or a request ID returned by an uncertain attempt) can be reused
after reconnecting; same payload returns the original result, while changed
payload causes 409. If the process loses its generated key after transmission,
check Inbox before attempting another send. This does not promise exactly-once.

## Security and rollback

The `0009` migration is a small dedicated credentials table with hash, scope,
state and stable private dedupe namespace. There are no changes to legacy
migrations, Relay, or Delivery/Reader tables. The old credential cannot write
to the Codex endpoint, and the new credential cannot write to legacy endpoints.
The old v1 envelope and regular idempotency keys are preserved. The exact
Codex-internal namespace cannot be written through the legacy route.

The connector connects only to IPv4 loopback and never follows redirects,
consults a proxy, or prints token, authorization headers or report body to
logs. Loopback does not authenticate a process: a malicious local listener can
impersonate a Bridge if it controls the port, and OS storage is not isolation
from all malicious same-user code. Codex may process report content remotely
as part of the user's existing session. The Codex source label indicates use of
the paired credential, not proof of the connecting process.

Revocation invalidates the DB authorization before deleting the OS entry.
If local deletion fails, the credential remains unusable and cleanup can be
retried. For a schema-compatible disable/rollback, start the current build
with `AGENT_DESK_DISABLE_CODEX=1`, revoke Codex and remove its registration.
Do not run a binary that lacks migration 0009 on an upgraded database. Keep
all received content, the migration, and the original OpenClaw configuration.
