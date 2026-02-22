# ADR 005: Job Model and TOML Definition Format

## Status
Accepted

## Context
`abeat` needs a declarative job format that is easy to inspect, edit, diff, and generate from CLI commands.
The system should not require a database for MVP job definitions.

## Decision
Use TOML files as the canonical job definition format, with explicit job kinds and schedule kinds.

## Job Kinds
- `heartbeat_check`
  - periodic cross-cutting checks
  - typically no-op capable (`HEARTBEAT_OK`)
- `scheduled_task`
  - time/cron-oriented scheduled executions
- `oneshot` (future phase)
  - one-time execution jobs

## Schedule Model
- `schedule_kind = "every"` with `every = "<duration>"`
- `schedule_kind = "cron"` with `cron = "<expression>"`

## File Model
- One file per job
- Recommended filename: `<job-id>.toml`
- Recommended location: `~/.config/abeat/jobs/`

## Required/Typical Fields
- `id`
- `kind`
- `enabled`
- schedule fields (`schedule_kind`, `every` or `cron`)
- `agent`
- `workspace`
- `timeout`
- `skills` (optional but common)
- `no_op_token` (default `HEARTBEAT_OK`, overridable)
- `[context]` section
- `[action]` section

## CLI Interaction
`abeat set jobs add/update/...` writes or modifies these TOML definitions under `~/.config/abeat/jobs/` by default.
Manual editing remains a supported workflow.

## Consequences
- Human-readable and script-friendly job definitions
- Works without requiring any repository-local private directory conventions
- Requires schema validation and clear error messages for malformed TOML
