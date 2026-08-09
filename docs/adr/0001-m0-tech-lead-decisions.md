# ADR-0001: M0 Tech Lead decisions and M1 execution order

- **Status:** Accepted
- **Date:** 2026-08-10
- **Decision owner:** Tech Lead
- **Supersedes:** conflicting statements in the initial M0 Gate Report

## Context

The initial M0 architecture was approved with amendments. This ADR records the authoritative decisions so implementation does not depend on reconstructing review conversation history.

## Decisions

### 1. Approved architecture invariants

- React owns presentation and local interaction.
- Rust/Tauri owns trusted native, network and credential boundaries.
- SQLite is authoritative for durable Desktop domain state.
- External agents enter through Adapter → Agent Gateway → `UnifiedAgentEvent`.
- UI remains provider/protocol independent.
- Progressive Reading Skill owns Reading Map, Reading Units, `delivery_cursor` and `confirmed_cursor`.
- Desktop owns presentation, immutable delivered Packet snapshots, `local_read_position`, and the local completion action/outbox projection.
- Desktop never infers confirmed progress from scroll position and never re-segments, rewrites or replans Reading Packets.
- Delivery persistence commits before notification; idempotency is transactional.

### 2. Scheduled Tasks use two milestones

**Experiment MVP:** may rely on an already-configured external Agent scheduled task so the Delivery-to-confirmation loop can be validated without schedule-management UI.

**V0.1 Complete:** must later include a minimal Agent-owned Scheduled Tasks surface:

- list projected scheduled tasks;
- create through the connected Agent;
- edit delivery time;
- pause;
- resume.

The external Agent/Scheduler remains authoritative. Desktop stores only a projection and outbound action state.

Future-safe capabilities:

```text
scheduling.read
scheduling.write
scheduling.natural_language
```

Suggested outbound intents:

```text
CREATE_SCHEDULED_TASK
CREATE_SCHEDULED_TASK_FROM_TEXT
UPDATE_SCHEDULED_TASK
PAUSE_SCHEDULED_TASK
RESUME_SCHEDULED_TASK
```

The UI exposes an operation only when the connected Agent advertises its required capability. No Scheduled Tasks UI is implemented in M1-A.

### 3. Notification activation remains an M3 hard feasibility gate

Current Tauri Notification APIs expose action listeners and notification-associated data. The M3 spike must prove exact `card_id` recovery for:

- Windows installed application: warm, hidden-to-tray and cold launch;
- macOS application: warm, hidden and cold launch.

Activation must navigate to an already-persisted Card and never ingest/create a second Delivery/Card. A stale/deleted target falls back to Inbox or Sticky. Arrival never steals focus. `tauri dev` alone is insufficient Windows evidence; at least one validation uses an installed packaged application.

### 4. E2E approach

Use the current WebdriverIO Tauri service for cross-platform E2E. Do not infer that macOS E2E is unavailable from older direct `tauri-driver` limitations. Keep native E2E small and critical; keep most renderer coverage in unit/component/integration tests.

### 5. M1 development defaults

| Setting | Provisional value |
|---|---|
| product name | `Agent Desk` |
| package/workspace | `agent-desk` |
| bundle identifier | `com.agentdesk.desktop` |
| default Sticky size | 320×420 logical px |
| minimum Sticky size | 300×360 logical px |
| default Home | Sticky |
| theme | follow system |
| close | hide to tray |
| tray Quit | terminate process |
| branch before first baseline commit | `main` |
| package manager | pnpm |

The bundle identifier is for private MVP development. It must be reviewed before external distribution because a later change can affect app-data paths and credential identity.

### 6. Single instance

Use Tauri's official single-instance plugin and register it before every other plugin. A second launch surfaces the existing Agent Desk instance and does not create a duplicate runtime. The handler routes through normal shell/navigation restoration rather than bypassing it.

### 7. M1 sub-gates

**M1-A Foundation** includes only:

- reproducible Tauri 2 + React + TypeScript + Vite + pnpm scaffold;
- format, lint, typecheck, test and Windows/macOS CI build baseline;
- approved module/port boundaries;
- minimal SQLite `0001`, connection, foreign keys, Card base repository transaction;
- theme, always-on-top and window-behavior preferences;
- single instance, resize, geometry restore/clamp, tray show/quit, close-to-tray, Sticky Home;
- minimal visual shell sufficient to prove launch/theme/restart.

**M1-B** contains final Note/Todo UX, drag reorder, due dates and Sticky product-quality work. It cannot start until M1-A is reviewed.

M1-A explicitly excludes Inbox, Reader, Agent Gateway, HTTP Adapter, Notifications, Progressive Reading, Scheduled Tasks, Learning Enhancements and nonessential animation.

## Consequences

- The Experiment MVP and V0.1 Complete are distinct acceptance milestones.
- Scheduling abstractions must remain capability-based even though no scheduling code is built in M1-A.
- Notification routing cannot be declared solved from development-mode behavior.
- M1-A remains intentionally thin; product Note/Todo implementation waits for its own Gate.
