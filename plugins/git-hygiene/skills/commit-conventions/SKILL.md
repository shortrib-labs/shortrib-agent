---
name: commit-conventions
description: |
  Drafts a commit message for our staged changes using the conventional commits conventions, classifies the change type, and adds a commit body for breaking/tricky changes. Use when the user asks to commit or for help with a commit message.
---

# commit-conventions

## When you're triggered

* The user wants to commit their staged changes to provide
a commit message.
* The user asks for a commit message for their current staged changes.

## What to do

1. Review the staged changes against the currently committed files.
2. Classify the type of the changes based on the conventional commits
   conventions described under "Type table".
3. Draft a subject line following the "Subject line rules".
4. Include a longer commit body if:
     * The intent of the change is unclear from the subject line
     * You are introducing a breaking change
5. Show the message to the user (see examples) and ask them if they would like
   to use it for a commit.

## Type table


|Type|When|
|----|----|
| feat | introduces a new feature to the system that provides value to the user | 
| fix | change repairs a defect           | 
| refactor | a refactoring that introduces zero changes to functionality |
| chore | general code maintenance excluding refactorings |
| docs |  updates to documentation |
| ci | changes to the CI/CD system |
| perf | performance enhancements |
| build | local build changes|
| test | additions to or refactoring of tests |


## Subject line rules

1. Use the type determined in the type table.
2. Begin with the type as a prefix followed by a colon. Examples: `feat:`,
   `perf:`, `ci:`.
3. No longer than 50 characters including prefix.
4. Write in active voice with the commit as the implied subject.
5. Include the scope of the change in the prefix if the change is confined to
   a particular scope. Example: `feat(api):`, `perf(ui):`
6. Use lowercase.
7. Do not end with punctuation.

## Examples

### Change

```
feat: enables metadata edit
```

### Scoped change

```
feat(ui): moves metadata to its own tab
```

### Change requiring more detail

```
chore: points git-hygiene at local source

Publishing 0.1.1 replaced the file source with a registry version pin, so edits
under plugins/git-hygiene stopped reaching the agent. The vendored copy comes
along at 0.1.1, carrying the rewritten descriptions and the new validation step.
```


