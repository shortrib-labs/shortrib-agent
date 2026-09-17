---
name: 02-writing-your-first-skill
description: Use when a learner wants to start, work through, or be guided or tutored through the Writing your first skill lesson — authoring their own commit-conventions skill from scratch. Walks them one step at a time through scaffolding a skill with the Tessl CLI, writing its SKILL.md, linting it against the Agent Skills spec, and installing it from their local filesystem so their agent fires it on intent, running a check after each step before moving on. Triggers on asks like guide me through writing my first skill, start lesson 2, help me author a skill, or walk me through scaffolding and linting a skill.
---

# Writing your first skill — walkthrough

You are guiding a learner through the **Writing your first skill** lesson in their own repository. Act as a patient tutor: present one step, let them do it, confirm the result with a concrete check, then move on. Do not dump the whole lesson at once, and do not write the SKILL.md for them — the point is that they author it and understand why each line is there.

The full lesson page is at `/academy/foundations/writing-your-first-skill/`. By the end the learner has scaffolded a `commit-conventions` skill, written its `SKILL.md` themselves, passed `tessl skill lint`, and installed it from their local filesystem so it fires on a real diff.

This walks them through *authoring* a `commit-conventions` skill. That is a different artifact from this walkthrough skill — never edit or overwrite this walkthrough; you are helping them build their own.

## When you're triggered

The learner has asked to start, work through, or get guided through the "Writing your first skill" lesson (Lesson 2 of Skill Foundations), or to author their first skill.

## How to guide

Walk these in order. After each, run the **Check** before advancing. If a check fails, troubleshoot that step — do not move on.

### 0. Confirm prerequisites

Make sure they have: a coding agent (Claude Code, Cursor, Codex, or Gemini), the Tessl CLI (install per https://docs.tessl.io/introduction-to-tessl/installation, then `tessl login`), and a repo initialized with Tessl. If they need to set one up:

```bash
tessl init --agent claude-code   # or: --agent cursor
```

**Check:** `tessl init` has written a `tessl.json` in the repo. If `tessl.json` is missing, they have not initialized Tessl here yet. (Publishing to a shared workspace comes later, in Lesson 5; authoring and installing locally need none.)

### 1. Scaffold the skill

`tessl skill new` runs a wizard that creates a well-formed single-skill plugin in one shot:

```bash
tessl skill new \
  --name commit-conventions \
  --description "Use when the user asks for a commit message or wants help summarizing staged changes. Generates a Conventional Commits–formatted message from a diff." \
  --path ./skills/commit-conventions
```

This creates `skills/commit-conventions/` (the plugin root) with a `.tessl-plugin/plugin.json` and a nested `skills/commit-conventions/SKILL.md` stub. The `plugin.json` points its `skills` path at that directory, and Tessl discovers the skill by convention — they never list skills by hand.

**Check:** the file `./skills/commit-conventions/skills/commit-conventions/SKILL.md` exists, and `./skills/commit-conventions/.tessl-plugin/plugin.json` exists. If not, re-read the `tessl skill new` output for the path it actually wrote to.

### 2. Write the SKILL.md

Have them open the stubbed `SKILL.md` and make it the real thing. Walk them through the two parts that decide whether the skill is any good, *before* they paste anything:

- **The `description` is the whole trigger.** It is the only thing the agent reads when deciding whether to load the skill. Write it as a trigger condition (what the user is doing) plus a capability statement (what the skill produces). "Helps with git" is too vague to ever fire; name the user-visible situation, then the artifact.
- **The body briefs the agent, not the user.** Numbered steps for sequential actions, a decision table for the Conventional Commits type classification, a tight constraint list for the subject-line rules. Leave out anything the description already said or that the agent can read directly from the diff.

Then give them the shape and let them fill it. The sections, in order, and what each is for:

- **Frontmatter** — `name`, and the `description` they just drafted. Nothing else.
- **A one-line statement of what the skill does**, addressed to the agent.
- **`## When you're triggered`** — the situation in the user's terms: staged changes and a request for a message.
- **`## What to do`** — numbered steps, in the order the agent takes them. Ask them what the agent has to do first before it can classify anything, and what it should do with the message once it has one. The sequence is theirs to work out.
- **`## Type table`** — two columns, one row per Conventional Commits type, mapping the type to when it applies. The seven types are the specification's, not this lesson's, so point them at the spec if they do not know the set. The `when` column is the work: the test of a row is whether the next reader can apply it to a diff and get the same answer, which "a feature" fails and something naming user-visible change passes.
- **`## Subject line rules`** — the constraints a message has to satisfy. Ask them what would make two people write the same subject for the same diff; the answers are short, and finding them is the point.

**Do not write it for them, and do not paste a finished version they can copy.** This lesson ships a skill the learner authored; a complete body on screen ends the lesson whether or not you tell them not to copy it.

**Reciting a section counts as writing it.** Listing the five steps of `## What to do` in prose, or naming the rules that go under `## Subject line rules`, hands over the same answer a code block would — a real run scored 7 of 10 on exactly that, having avoided the code block. Describe what question a section answers and what makes an answer good; let them produce the answer. Give one row or one line as an example of *form* when they are stuck on form, ask what they think belongs when they are stuck on content, and react to what they write. If they ask you outright to write it, decline and name the section's purpose instead of its contents.

The lesson page has the full worked example if they want to read one afterwards — point them at it once their own version lints.

**Check:** the `SKILL.md` has frontmatter with `name` and a specific `description`, and a body with numbered steps, the type table, and the subject-line rules. Ask them to point at the `description` and say in their own words what it controls.

### 3. Lint it

Validate structure against the Agent Skills specification before installing:

```bash
tessl skill lint ./skills/commit-conventions
```

This checks the plugin root has a valid `.tessl-plugin/plugin.json`, the required `SKILL.md` frontmatter fields are present, every file referenced from `SKILL.md` exists, and the directory matches the spec. Lint checks *structure*, not quality — a clean pass doesn't mean the description is sharp enough to fire (that's `tessl review run`, a later lesson).

**Check:** `tessl skill lint` returns a clean pass. If it flags the `examples/` reference and there is no `examples/` folder, either add one or remove that line from `SKILL.md`, then re-run.

### 4. Install it from your filesystem

They don't have to publish to try it — install straight from the local path:

```bash
tessl install file:./skills/commit-conventions
```

This is the authoring loop: edit `SKILL.md`, reinstall, try it, edit again. To skip the manual reinstall, run `tessl install --watch-local` on its own (no source) — it watches every local file-source skill already installed and re-syncs on each save:

```bash
tessl install --watch-local
```

**Check:** `tessl list` shows their `commit-conventions` plugin installed with a version and a Synced status. If not, re-run and read the error, usually a missing `tessl login` or no `tessl.json`.

### 5. Trigger it on a real diff

Stage a real change, then ask the agent for a commit message:

```bash
echo "# Try Skills" > README.md
git add README.md
```

Then, in the agent chat:

> "What's a good commit message for the staged changes?"

The agent should match the prompt to the skill's `description`, read the staged diff, and propose something like:

```
docs: add initial README

Adds a placeholder README to seed the repo.
```

Have them stage different changes (a new function, a bug fix, a dependency bump) and watch the prefix change: `feat:`, `fix:`, `chore:`, `refactor:`.

**Check:** the agent produced a Conventional Commits message with a sensible type prefix for the staged diff. The course bundles its own `commit-conventions` demo skill (from Lesson 1), so if the trigger is ambiguous, have them rely on the one they just authored and installed from `file:` — or uninstall the course's copy for the duration of this step.

### 6. Lint the skill

```bash
tessl skill lint ./skills/commit-conventions
```

**Check:** the lint returns a clean pass.

### 7. Confirm it is installed

```bash
tessl list
```

**Check:** their own `commit-conventions` plugin is listed as installed.

### 8. Trigger their skill

In the agent chat:

> "What's a good commit message for the staged changes?"

**Check:** the reply came from **their** skill, not the course's bundled copy. If it does not trigger and no older skill is shadowing it, the usual cause is a too-narrow description: the words a user would actually say — "commit", "message", "staged" — need to appear in the `description` field.

### 9. Prove the negative case

In the agent chat:

> "explain this function"

**Check:** the skill did not load. Skills load on intent; they are not always-on context.

### 10. Close out

Confirm the state: they have a skill they wrote, linted, installed and triggered from a real diff.

**Check:** they can state the instinct the lesson builds in their own words — a sharp `description` so the agent knows *when* to reach for the skill, and a tight body that tells it *how* without crowding the context window.

Then hand off without losing context. The next skill, **`03-skills-vs-mcp-and-context-files`**, is already installed from this course plugin. Offer to start *Skills vs MCP, and context files* now, where they learn when a skill is the right tool versus a context file or an MCP server, by running that skill.
