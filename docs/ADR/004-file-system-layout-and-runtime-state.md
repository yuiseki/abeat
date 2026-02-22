# ADR 004: File System Layout and Runtime State Separation

## Status
Accepted

## Context
`abeat` should be local-first and inspectable, following a POSIX/filesystem-oriented workflow similar to `amem`.
It should not depend on a repository-specific private directory (such as `./private/`) for core operation.
At the same time, configuration and runtime artifacts have different lifecycle needs:
- Configuration/definitions should be user-editable and stable
- State/logs/cache should be local, mutable, and not committed

## Decision
Keep `abeat` home-local by default, separating configuration from runtime state/log/cache.

### 1. Configuration root (home-local)
Recommended config root:

- `~/.config/abeat/`

Recommended subdirectories:

- `~/.config/abeat/config.toml`
- `~/.config/abeat/jobs/`
- `~/.config/abeat/prompts/`
- `~/.config/abeat/adapters/`

These files are intended for user editing and local configuration.
If users want Git-managed definitions, they may explicitly point `abeat` at another directory, but that is not the default architecture.

### 2. User runtime root (home-local)
Canonical runtime root:

- `~/.abeat/`

Recommended subdirectories:

- `~/.abeat/state/`
- `~/.abeat/state/locks/`
- `~/.abeat/logs/`
- `~/.abeat/logs/stdout/`
- `~/.abeat/logs/stderr/`
- `~/.abeat/cache/`
- `~/.abeat/cache/contexts/`

### 3. State/log split
- State stores current mutable runtime metadata (last run, next due, lock markers, runner log)
- Logs store append-only run records and captured outputs
- Cache stores generated context artifacts and temporary derived data

### 4. Home-local default and `amem` independence
`abeat` uses `~/.abeat` for its own runtime state even when `amem` is available.
`amem` paths are never the sole source of truth for `abeat`.
`abeat` configuration/definitions live under `~/.config/abeat/` by default, not under the active workspace.

## Consequences
- Strong inspectability and manual recoverability
- Clear distinction between configuration and runtime execution artifacts
- Avoids coupling `abeat` to repository-specific `./private/` conventions
- Requires path resolution between config root (`~/.config/abeat/`) and runtime root (`~/.abeat/`)
