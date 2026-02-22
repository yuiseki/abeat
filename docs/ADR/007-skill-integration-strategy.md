# ADR 007: Skill Integration Strategy

## Status
Accepted

## Context
The repository already provides a rich Skills system (`AGENTS.md`, `SKILL.md` files, tool-specific workflows).
`abeat` should leverage these assets without duplicating their logic or embedding skill-specific behavior into the scheduler core.

## Decision
Treat Skills as declared dependencies of a job and integrate them primarily through context/prompt injection.

### 1. Job-level declaration
Jobs may declare required/recommended skills, for example:

- `skills = ["gog-gmail", "gog-task"]`

### 2. MVP integration mode: prompt-level activation
`abeat` injects into context/prompt:

- skill names
- resolved `SKILL.md` paths
- instruction that the job should use those skills

The agent CLI remains responsible for following repository rules and reading the skill definitions as needed.

### 3. Deferred integration mode: deterministic preflight/postflight
`abeat` may later support explicit local commands before/after the agent run for stable, non-LLM-dependent tasks.

Examples:
- preflight data collection (`amem today --json`, weather, money snapshots)
- postflight append-only logging (`amem keep`, local JSONL writes)

### 4. Non-goal
`abeat` will not re-implement the repository Skill runtime or parse all skill internals into a separate execution engine.

## Consequences
- Reuses existing operational knowledge with minimal scheduler complexity
- Keeps `abeat` core generic and portable
- Some job behavior remains dependent on agent CLI compliance with repo instructions
