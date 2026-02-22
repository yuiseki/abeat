# ADR 011: Reliability Baseline and Failure Isolation

## Status
Accepted

## Context
Periodic agent execution can fail in multiple ways:
- overlapping runs
- hung agent processes
- malformed outputs
- transient tool failures (including optional integrations such as `amem`)

The MVP should be reliable enough for daily operation without implementing a full queue/scheduler subsystem.

## Decision
Adopt a minimal reliability baseline for MVP and defer advanced controls to later phases.

## MVP Reliability Baseline

### 1. Locking (duplicate execution prevention)
- Use filesystem locking (for example `flock`)
- At minimum, enforce per-job lock during execution
- Optional global lock may be used for `tick --due` orchestration

### 2. Timeouts
- Every agent run executes with a configured timeout
- Timeout expiration records an error run and releases the lock

### 3. Runtime state tracking
Persist per-job runtime metadata (for example in `jobs-state.json`):
- `last_run`
- `last_status`
- `next_due`
- `fail_count`

### 4. Failure isolation
- Failure of one due job must not prevent processing other due jobs in the same `tick --due`
- Each job produces its own run record and status update

### 5. Optional integration fault policy
- In `amem_mode = "auto"`, `amem` failures are warnings, not fatal job failures
- In `amem_mode = "on"`, `amem` failures are fatal for that job

## Deferred Hardening (Future Phases)
- retry backoff
- cooldown/deduplication
- stale lock cleanup
- busy-skip behavior for still-running jobs

## Consequences
- Practical safety baseline without major complexity
- Predictable behavior under common failure modes
- Some noisy repeats or retry inefficiencies remain until hardening features are added
