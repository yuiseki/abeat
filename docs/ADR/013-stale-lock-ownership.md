# ADR 013: A lock names its owner, and a lock without a living owner is stale

## Status
Accepted. Refines the locking in [ADR 011](011-reliability-baseline-and-failure-isolation.md).

## Context
Per-job locks were create-only: `OpenOptions::create_new` succeeded and the job
ran, or it failed with `AlreadyExists` and the job was skipped. The file
recorded the owning pid, but nothing ever read it back. Release happened in
`JobLock::drop`.

`Drop` does not run when a process is killed. An OOM kill, a SIGKILL or a power
cut leaves the file behind, and from then on that job is skipped forever. It
does not fail, it does not warn: it prints `Skipped (locked)` every tick, which
is the same line a healthy concurrent run prints.

Found in production on a swapless machine, where the kernel kills processes
under memory pressure. Five of seven jobs were wedged:

| job | stuck since | noticed after |
| --- | --- | --- |
| `owner-interest-diary-hourly` | 2026-02-26 | 6 months |
| `yuiclaw-daemon-watchdog` | 2026-03-16 | 6 months |
| `gyazo-sync-hourly` | 2026-05-09 | 4 months |
| `news-sync-hourly` | 2026-09-05 | 8 days |
| `hatebu-sync-hourly` | 2026-09-05 | 8 days |

It surfaced only because a person asked why a bookmark archive had a gap.
Nothing in the system was going to say so: the failure mode of the watchdog was
to be silently absent, which is the failure mode a watchdog exists to prevent.

## Decision
On `AlreadyExists`, read the pid the lock names and ask whether that process is
alive. If it is, the job is genuinely running and is skipped as before. If it is
not, remove the lock, take it, and say so on stderr:

```
Taking over the lock for gyazo-sync-hourly: pid 3222472 is gone
```

A lock that names no pid this can parse is also stale: nothing can be waiting on
a lock whose owner cannot be identified, and a half-written file is what a kill
during acquisition leaves.

Liveness is `/proc/<pid>` on Linux. Anywhere else the answer is "alive", so no
lock is ever taken: refusing to run is recoverable, running twice may not be.

The retry after removal is a single `create_new`. If two ticks decide to take
over the same stale lock at the same moment, one creates the file and the other
is told the job is locked, which is the correct answer for the loser.

## Consequences
- A job survives the death of the process that was running it. The worst case
  becomes one skipped interval instead of an indefinite silence.
- Taking over is announced. A lock that keeps being taken over means something
  is killing the runner, and that is worth seeing rather than smoothing away.
- A pid can be recycled. A stale lock whose number has been reused by an
  unrelated process reads as held, and the job waits another interval; the next
  tick is very unlikely to see the same coincidence.
- This does not protect against a hung but living process. That is what
  timeouts are for (ADR 011), and they release the lock through `Drop`.
