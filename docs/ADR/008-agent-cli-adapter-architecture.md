# ADR 008: Agent CLI Adapter Architecture

## Status
Accepted

## Context
Different coding AI agent CLIs (Codex, Gemini, OpenCode, others) have incompatible command-line flags, prompt input methods, session behavior, and output formats.
`abeat` needs a stable orchestration layer that can switch agents without changing scheduler logic.

## Decision
Introduce an adapter layer that treats each agent CLI as a black-box executable.

### 1. Adapter responsibilities
- Normalize command invocation for each agent
- Accept context file and prompt inputs
- Execute with timeout
- Capture stdout/stderr
- Return exit code and parsed execution status (`ok`, `no-op`, `error`)

### 2. MVP adapter implementation
Use shell-script adapters stored in the user config root by default:

- `~/.config/abeat/adapters/codex.sh`
- `~/.config/abeat/adapters/gemini.sh`
- `~/.config/abeat/adapters/opencode.sh`

This keeps adapter behavior independent from any specific workspace layout.

### 3. Scheduler/runner separation
`abeat` core decides:
- what job is due
- what context to build
- where to log output

Adapters decide:
- exact CLI flags
- input transport (stdin vs file)
- output parsing details

### 4. Future evolution
Adapters may later be implemented as built-in modules/subcommands, but the architectural boundary remains.

## Consequences
- Easy addition of new agent CLIs without rewriting scheduler code
- Isolates agent-specific breakage to adapter scripts
- Requires adapter maintenance as upstream CLIs evolve
