# cc-companion

A research platform for exploring new **human-AI interaction paradigms**, built around [Claude Code](https://docs.anthropic.com/en/docs/claude-code).

## Vision

The dominant pattern in today's AI-assisted development is **reactive**: the human asks, the AI answers. cc-companion explores this space by building infrastructure for a **proactive** AI companion — one that has context, knows when resources are available, and can take initiative.

This project is not just a tool. It is an **infrastructure** for exploring how humans and AI agents can collaborate more effectively — moving from a question-answer dynamic toward genuine partnership.

### Three Pillars

1. **Context** — The companion needs to understand what you're working on. By indexing all Claude Code session logs, project structures, and CLAUDE.md files, it builds a persistent awareness of your work across projects and over time.

2. **Resources** — Proactive action requires knowing *when* to act. The companion monitors API usage windows (5-hour and 7-day limits) and their reset schedules in real-time, identifying moments when compute resources are available and would otherwise go unused.

3. **Agency** — With context and resource awareness in place, the companion can move beyond responding to commands. It can schedule tasks, generate insights, and surface recommendations — acting as a strategist rather than an executor.

## Current Features

### Project-Aware Session Browser
Browse all Claude Code projects discovered from `~/.claude/projects/`. View session histories with metadata (date, message count, git branch), read past conversations with inline tool-call annotations, search across session content via ripgrep, and review project CLAUDE.md files — all within the TUI.

### Advisor Chat
An interactive chat mode powered by the Claude CLI with a built-in **Advisor** system prompt. The Advisor is designed as a *strategist* — it doesn't execute code, but helps you think through decisions, draft instructions for your execution agent, and spot blind spots. The human bridges the two agents, maintaining control while benefiting from a dual-perspective workflow.

### Real-Time Resource Monitoring
Continuous tracking of Anthropic API usage across both the 5-hour and 7-day windows. Displays utilization percentages with reset countdowns, automatically refreshes every 60 seconds, and retrieves OAuth tokens from the platform keychain. This data feeds directly into the scheduler to determine when surplus resources are available.

### Auto-Task Scheduler
A framework for automatically dispatching tasks when API usage is low and reset windows are approaching — turning otherwise wasted quota into productive work. The scheduler evaluates usage thresholds (e.g., <90% utilization with <30 min to 5h reset) and launches pre-defined tasks from a queue.

### Background Task Execution
Run shell commands without leaving the interface. Tasks execute in background threads with status tracking and inline output display, enabling parallel workflows alongside the advisor chat.

### Gateway Support
Route requests through a LiteLLM proxy for team environments, configurable via environment variables.

## Typical Workflow

1. **Launch** — cc-companion starts with a project selector, listing all Claude Code projects sorted by activity.
2. **Select a project** — The companion sets the working directory and automatically generates a project overview via the Advisor, establishing shared context.
3. **Consult** — Chat with the Advisor about strategy, architecture decisions, or next steps. The Advisor reads your project files and provides concise, opinionated guidance.
4. **Execute** — Take the Advisor's recommendations to your Claude Code execution agent. Return with results for review.
5. **Monitor** — Keep an eye on API usage. When resources are available, the scheduler can automatically dispatch queued tasks.
6. **Review** — Browse session logs to revisit past decisions, search for specific discussions, or build on previous work.

## In Development

### Autonomous Task Pipeline
With context (project understanding + session history) and resource awareness (token availability + timing) already in place, the next step is enabling **long-running, resource-intensive tasks** that execute autonomously. These tasks — too time-consuming and token-heavy for interactive sessions — are queued and dispatched when the scheduler detects available capacity.

An end-to-end scientific research pipeline is being built as the first use case: automated literature review, experiment design, and result analysis that runs across multiple sessions without human intervention.

## Roadmap

### Proactive Heuristic Guidance
The companion will move beyond answering questions to **actively initiating conversations** — offering suggestions on Claude Code usage patterns, surfacing relevant past sessions, and providing teaching-style guidance. This enables research into human-AI pedagogical interaction: how an AI can effectively coach a developer rather than simply serve them.

One promising direction is **Graph-RAG** (Graph-based Retrieval-Augmented Generation): by constructing a knowledge graph from session logs, project structures, and code relationships, the companion can traverse semantic connections to surface contextually relevant suggestions — such as recommending related past solutions when a similar problem is detected, or identifying architectural patterns the developer hasn't yet explored. Graph-RAG enables richer, more associative retrieval than flat vector search, making proactive guidance more precise and serendipitous.

### Cross-User Knowledge Sharing
By analyzing session logs across team members working on related projects, the companion can identify **domain knowledge gaps and overlaps** — enabling cross-pollination of insights, shared best practices, and collaborative learning without requiring direct communication between users.

## Build

```bash
cargo build --release
```

## License

MIT
