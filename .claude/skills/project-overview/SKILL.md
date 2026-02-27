---
name: project-overview
description: This skill should be used when the user asks about a project's purpose, architecture, current state, or possible future directions. Triggers on questions like "what does this project do", "give me an overview", "what's the goal", "what could be improved", or when onboarding to a new codebase.
---

# Project Overview

## Purpose

Provide a concise overview of the current codebase by reading key files, then output a short summary covering: what the project does, the user's goals, and possible next steps.

## Workflow

1. Read project config files to identify language and framework (e.g. `Cargo.toml`, `package.json`, `pyproject.toml`, `go.mod`, `pom.xml`, etc.)
2. Read `README.md`, `CLAUDE.md`, or similar project docs if they exist
3. Glob for source files and read the main entry point and core modules
4. Run `git log --oneline -15` to understand recent development direction
5. Run `git status` to see work in progress
6. Synthesize findings into the output format below

## Output Format

Keep the output **short and structured** using these sections:

### What This Project Is
1-2 sentences: project name, language/framework, what it does.

### User's Goals
3-5 bullet points: what the developer is trying to achieve.

### Architecture
Brief list of key source files and their roles (one line each).

### Current State
What's been done recently (from git log) and what's in progress (from git status).

### Possible Next Steps
3-5 concrete ideas for further development based on the codebase's trajectory.

## Guidelines

- Do not produce lengthy output. Aim for under 300 words total.
- Focus on actionable insights, not exhaustive descriptions.
- When suggesting next steps, ground them in patterns visible in the code and recent commits.
- Adapt to whatever language or framework the project uses — do not assume any specific stack.
