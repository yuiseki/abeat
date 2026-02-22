# ADR 010: Run Logging and Observability

## Status
Accepted

## Context
Periodic automation is difficult to trust without a clear execution trail.
The design explicitly requires `abeat logs` as a first-class operator workflow.

The logging system must support:
- quick human inspection
- structured machine-readable history
- debugging of exact agent outputs

## Decision
Adopt a file-based observability model centered on structured run history plus captured stdout/stderr.

### 1. Canonical run history
- `~/.abeat/logs/runs.jsonl`
- Append-only JSON Lines format
- One record per run attempt

### 2. Captured process output
- `~/.abeat/logs/stdout/<run-id>.log`
- `~/.abeat/logs/stderr/<run-id>.log`

### 3. Runner operational log
- `~/.abeat/state/runner.log`
- Used for scheduler-triggered operational tracing and warnings

### 4. CLI surface
- `abeat logs` for human-friendly views (recent runs by default)
- `abeat logs --job <id>` and `--status <status>` for filtering
- `abeat get runs --json` for machine-oriented access

### 5. Minimum run record fields
- `run_id`
- `job_id`
- `status`
- `started_at`
- `ended_at`
- `agent`
- `exit_code`
- `no_op`

## Consequences
- Strong auditability and easy debugging with POSIX tools (`tail`, `grep`, `jq`)
- Works without external logging services
- Requires future log rotation/retention policies as history grows
