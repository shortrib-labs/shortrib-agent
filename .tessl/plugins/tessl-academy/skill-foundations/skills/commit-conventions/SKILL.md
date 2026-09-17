---
name: commit-conventions
description: Use when the user asks for a commit message, or has staged changes and wants help summarizing them. Generates a Conventional Commits–formatted message from a diff.
---

# Commit Conventions

You help the user write commit messages that follow the Conventional Commits specification.

## When you're triggered

The user has staged changes and wants a commit message, or has asked for help summarizing recent changes.

## What to do

1. Read the staged diff (`git diff --cached`) if you haven't already.
2. Identify the dominant change type. Use the table below.
3. Compose a message of the form `<type>(<optional scope>): <imperative subject>`.
4. Add a short body only if the diff has consequences a future reader would want flagged: breaking changes, migrations, security fixes.
5. Show the user the message before they commit. Don't run `git commit` unless they ask.

## Type table

| Type | When |
|---|---|
| `feat` | New user-visible capability. |
| `fix` | Bug fix the user can perceive. |
| `refactor` | Internal restructure with no behavior change. |
| `docs` | Documentation only. |
| `test` | Tests only. |
| `chore` | Tooling, deps, build, CI: anything below the user's perception. |
| `perf` | Measurable performance improvement. |

## Subject line rules

- Imperative mood ("add", not "added" or "adds").
- Lowercase first letter, no trailing period.
- Hard limit 72 characters.
- Scope is optional but useful when the repo has clear surfaces: `feat(auth):`, `fix(api):`.

## Examples

See `examples/` for sample diffs (`feat.diff`, `fix.diff`, `docs.diff`, `chore.diff`) to ground your reasoning about which type a change is.
