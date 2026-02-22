# ADR 012: Rust CLI and File-Based MVP Stack

## Status
Accepted

## Context
`abeat` is a local-first heartbeat orchestrator for periodic execution of existing coding AI agent CLIs.
Its core concerns are:

- CLI UX and scriptability
- local filesystem state/log management
- subprocess execution
- time/schedule handling
- testable reliability behavior (lock/timeout/no-op handling)

Unlike `amem`, `abeat` does not require high-performance full-text search for MVP.
Therefore, introducing a database or async runtime too early would increase complexity without improving the initial value.

## Decision
Adopt Rust for `abeat` and start with a file-based MVP stack.

### 1. Primary implementation language
- Rust

Reasons:
- Strong typing for job schemas, command parsing, and runtime state
- Good CLI ergonomics and testability
- Easy alignment with `amem` implementation style and tooling
- Reliable subprocess and filesystem APIs for local orchestration

### 2. Storage strategy (MVP)
- Job definitions: TOML files (`~/.config/abeat/jobs/*.toml`)
- Runtime state: JSON (`~/.abeat/state/*.json`)
- Run history: JSONL (`~/.abeat/logs/runs.jsonl`)
- Captured outputs: plain text log files (`stdout/`, `stderr/`)

No database is used in MVP.

### 3. Scheduler strategy
- `abeat` provides one-shot execution (`tick --due`)
- Periodic triggering is delegated to OS schedulers:
  - `cron`
  - `systemd --user timer`

No built-in daemon is required in MVP.

### 4. Process execution model
- Invoke external agent CLIs as subprocesses (black-box integration)
- Use adapter abstraction for CLI differences
- Capture stdout/stderr and exit code
- Apply timeout per job (implementation detail may evolve)

### 5. Reliability baseline (MVP)
- Filesystem lock to avoid duplicate execution
- File-based runtime state updates
- Append-only run logs
- Failure isolation between jobs in a single `tick`

### 6. Deferred technology choices (explicitly postponed)
- SQLite / SQL query layer
- Async runtime (`tokio`) and concurrent scheduler execution
- Internal queue/broker
- Web UI / server component
- Plugin runtime

## Consequences
- Fast path to a robust, inspectable MVP aligned with `amem`'s engineering style
- Lower implementation complexity than database/daemon-first designs
- Some advanced capabilities (large-scale analytics, high-throughput scheduling) are intentionally deferred
