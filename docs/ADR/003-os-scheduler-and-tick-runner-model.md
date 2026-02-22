# ADR 003: OS Scheduler and One-Shot `tick` Runner Model

## Status
Accepted

## Context
`abeat` needs periodic execution but should not rely on external services and should avoid prematurely implementing a complex long-running daemon.
Operating systems already provide reliable local schedulers (`cron`, `systemd --user timer`, `launchd`).

## Decision
Use a two-layer scheduling model:

### 1. `abeat` as one-shot runner
- `abeat tick --due`:
  - loads job definitions and runtime state
  - determines due jobs
  - executes due jobs once
  - updates state and exits
- `abeat run <job-id>` remains the manual immediate execution path

### 2. OS scheduler as periodic trigger
- Preferred: `cron`
- Preferred: `systemd --user timer`
- Optional: `launchd` (platform-specific support)

### 3. Daemon policy
- No built-in always-on daemon in MVP
- A native daemon may be added later only if OS scheduler integration proves insufficient

### 4. Installation helpers
- `abeat install cron` / `abeat install systemd-user` are helper commands only
- They generate/install scheduler entries, but the scheduling authority remains the OS

## Consequences
- Simple and robust architecture with clear failure boundaries
- Easy debugging (scheduler trigger vs `abeat` execution logic)
- Platform-specific installation details move to helper commands/docs
