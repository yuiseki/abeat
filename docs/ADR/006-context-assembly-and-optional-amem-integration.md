# ADR 006: Context Assembly and Optional `amem` Integration

## Status
Accepted

## Context
`abeat` must operate without `amem`, but the environment may provide `amem` and benefit from richer context and activity logging.
A single code path that assumes `amem` is always present would violate `abeat`'s independence.

## Decision
Define a layered context assembly model with optional `amem` integration modes.

### 1. Two-layer context model
- Core context (always available)
  - job metadata
  - current timestamp/timezone
  - recent `abeat` runs for the same job
  - workspace/repo paths
  - declared skills and paths
  - action prompt and no-op contract
- Integration context (best-effort or required)
  - `amem` outputs and other local CLI data when configured

### 2. `amem` integration modes
Per-job `context.amem_mode` supports:

- `auto`
  - use `amem` if available
  - continue with warning if unavailable or failing
- `on`
  - require `amem`
  - treat missing/broken `amem` as job error
- `off`
  - do not attempt `amem` access

### 3. `amem` context inputs (when enabled)
Examples:

- `amem today --json`
- `amem which`
- `amem get tasks ...`
- `amem get acts ...`
- `amem get diary ...`

### 4. Temporary context artifact
For each run, `abeat` composes a context file (for example under `~/.abeat/cache/contexts/...`) and passes it to the adapter.
The temporary context artifact is part of the debug/audit trail.

### 5. Optional `amem` activity export
If configured and available, `abeat` may write activity summaries via:

- `amem keep "... " --kind activity --source abeat`
- `amem keep "..." --kind activity --source abeat`

This export is best-effort unless the job explicitly requires `amem` (`amem_mode = "on"`).

## Consequences
- `abeat` remains portable and independent
- Jobs can opt into stronger `amem` coupling only when useful
- Context builder implementation becomes more complex due to mode handling
