# ADR 001: Independent `abeat` CLI and Scope Boundary

## Status
Accepted

## Context
The goal is to build an agentic heartbeat mechanism that can periodically run existing coding AI agent CLIs with local context injection.
The system should remain simple, local-first, and reusable across environments.

`amem` is useful for memory/context/logging, but heartbeat orchestration has a different responsibility.
Making heartbeat execution depend on `amem` would reduce portability and blur ownership between runtime orchestration and memory storage.

## Decision
Create a separate CLI tool named `abeat` (Agentic Beat / Agentic Heartbeat) as an independent runtime/orchestration layer.

### Responsibilities of `abeat`
- Job definition management (`get/set/list`)
- Due detection and periodic execution (`tick --due`)
- Agent CLI invocation via adapters
- Context assembly for runs
- Run logging and runtime state management
- Minimal reliability controls (lock, timeout, failure isolation)

### Non-goals of `abeat`
- Implementing a custom LLM runtime
- Depending on external scheduler/queue/database services
- Re-implementing repository Skills

### Relationship with `amem`
- `amem` is an optional integration for context import and activity logging
- `abeat` must run correctly without `amem`

## Consequences
- Clear boundary between memory system (`amem`) and scheduler/orchestrator (`abeat`)
- `abeat` can be reused in environments that do not have `amem`
- Some integration behaviors must be explicitly modeled (`auto|on|off`) instead of assuming `amem`
