# ADR 002: CLI Structure and Job Commands

## Status
Accepted

## Context
The tool should feel ergonomic for frequent interactive use while remaining explicit and scriptable for cron/systemd execution.
The desired command style follows the practical `amem` conventions (`get/set/list/ls`) while reflecting `abeat`'s runtime responsibilities.

## Decision
Adopt a domain-oriented CLI with stable convenience aliases and a one-shot scheduler entry point.

### 1. Root command
- Binary: `abeat`
- Version: `abeat --version`

### 2. Core execution commands
- `abeat init`
- `abeat tick --due`
- `abeat run <job-id>`
- `abeat logs`

### 3. Domain-oriented job commands
- `abeat get jobs`
- `abeat list` (alias)
- `abeat ls` (alias)
- `abeat set jobs ...`

`abeat list` and `abeat ls` are canonical aliases for `abeat get jobs`.

### 4. `set jobs` sub-actions
- `abeat set jobs add ...`
- `abeat set jobs update <id> ...`
- `abeat set jobs enable <id>`
- `abeat set jobs disable <id>`
- `abeat set jobs rm <id>` (or `delete`)

The CLI may also support `upsert` as a convenience command, but explicit add/update remains the default UX.

### 5. Recommended auxiliary commands
- `abeat get runs` (machine-readable counterpart to `logs`)
- `abeat get job <id>`
- `abeat status`
- `abeat which`
- `abeat doctor`
- `abeat install cron`
- `abeat install systemd-user`

### 6. Output conventions
- Human-readable default output for `list` and `logs`
- Stable `--json` output for automation-oriented commands (`get jobs`, `get runs`, `status`)

## Consequences
- Low cognitive overhead for day-to-day use (`list`, `ls`, `get/set`)
- Clear separation between "run scheduler once" (`tick --due`) and "run one job now" (`run`)
- Slightly wider command surface requiring documentation and CLI tests
