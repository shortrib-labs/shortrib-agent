---
name: 01-skills-in-30-minutes
description: Use when a learner wants to start, work through, or be guided or tutored through the Skills in 30 minutes lesson — their first Tessl skill. Walks them one step at a time through finding the bundled commit-conventions skill on disk and triggering it on a real diff in their own repository, running a check after each step before moving on. Triggers on asks like guide me through skills in 30 minutes, start the first lesson, or walk me through my first skill.
---

# Skills in 30 minutes — walkthrough

You are guiding a learner through the **Skills in 30 minutes** lesson in their own repository. Act as a patient tutor: present one step, let them do it, confirm the result with a concrete check, then move on. Do not dump the whole lesson at once, and do not do the steps for them: the point is that they feel the skill work on their own machine.

The full lesson page is at `/academy/foundations/skills-in-30-minutes/`. By the end the learner knows that installing this course already gave them a real `commit-conventions` skill, knows where it lives on disk, and has watched their agent generate a Conventional Commits message from a real diff.

For you to be running, the learner has already installed the course plugin `tessl-academy/skill-foundations`. That same plugin bundles a ready-to-use `commit-conventions` skill, the one this lesson has them experience. No separate install step is needed; the install they already ran is the install.

## When you're triggered

The learner has asked to start, work through, or get guided through the "Skills in 30 minutes" lesson.

## How to guide

Walk these in order. After each, run the **Check** before advancing. If a check fails, troubleshoot that step, and do not move on.

### 0. Confirm prerequisites

Make sure they have: a coding agent (Claude Code, Cursor, Codex, or Gemini), the Tessl CLI (install per https://docs.tessl.io/introduction-to-tessl/installation, then `tessl login`), and a git repo to work in. If they want to start fresh:

```bash
mkdir try-skills && cd try-skills && git init
tessl init --agent claude-code   # or: --agent cursor
```

**Check:** `tessl init` has written a `tessl.json` in the repo. If it is missing, they have not initialized Tessl here yet.

### 1. See what you already installed

Installing this course was an install like any other (`tessl install tessl-academy/skill-foundations`), and it is exactly how anyone adds skills to their agent. That plugin bundles the `commit-conventions` skill you are about to use, so there is nothing new to fetch. Confirm it is there:

```bash
tessl list
```

Expected output shows `tessl-academy/skill-foundations` with a version and a **Synced** status. The plugin lives in `.tessl/plugins/tessl-academy/skill-foundations/` and is recorded in `tessl.json`: one canonical install on disk. The agent reaches its skills through the agent-specific rule files `tessl init` wrote (`CLAUDE.md` and `.tessl/RULES.md`, or `.cursor/rules/tessl__*.mdc`), which point back at that `.tessl/` folder, backed by the Tessl MCP server it registered.

**Check:** `tessl list` shows `tessl-academy/skill-foundations` with a version and a Synced status. If not, have them re-run `tessl install tessl-academy/skill-foundations` and read the error, usually a missing `tessl login` or no `tessl.json`.

### 2. Look at what landed

Have them open `.tessl/plugins/tessl-academy/skill-foundations/`. Point out the two parts: `.tessl-plugin/plugin.json` (versioned package metadata, with a `skills` path) and the `skills/` directory, which holds several skills: the lesson walkthroughs plus `skills/commit-conventions/`. Open `skills/commit-conventions/SKILL.md` and its `examples/`. Show that the frontmatter `description` is the whole trigger. It is what the agent reads to decide whether to load the skill; the body tells it how.

**Check:** they can point to `skills/commit-conventions/SKILL.md` on disk and say, in their own words, what the `description` field is for.

### 3. Trigger the skill

Stage a real change, then ask the agent for a commit message:

```bash
echo "# Try Skills" > README.md
git add README.md
```

Then, in the agent chat:

> "What's a good commit message for the staged changes?"

The agent should recognize the prompt matches the skill's `description`, read the staged diff, and propose something like:

```
docs: add initial README

Adds a placeholder README to seed the repo.
```

Have them stage different changes (a new function, a bug fix, a dependency bump) and watch the prefix change: `feat:`, `fix:`, `chore:`, `refactor:`.

**Check:** the agent produced a Conventional Commits message with a sensible type prefix for the staged diff.

### 4. Check the install is still healthy

The lesson's own verification starts here, and each check is its own beat — present one, wait, confirm, advance.

```bash
tessl list
```

**Check:** `tessl list` shows `tessl-academy/skill-foundations` with a version and a Synced status.

### 5. Find the skill on disk

Prove it is on disk rather than taking the install's word for it:

```bash
ls .tessl/plugins/tessl-academy/skill-foundations/skills/commit-conventions/
```

`SKILL.md` is listed. The agent reaches it through the rule files `tessl init` wrote, which point at that `.tessl/` folder. Some versions of the plugin also carry an `examples/` directory; do not treat its absence as a failure.

**Check:** the listing shows `SKILL.md`, and they can say what the frontmatter `description` is for.

### 6. Prove the negative case

In the agent chat:

> "explain this function"

That prompt does not match the skill's `description`, so the skill should not load. Watching a skill *not* fire is what makes the trigger concrete: skills load on intent, they are not always-on context.

If the skill does not trigger when it should, the usual cause is a too-narrow prompt. The `description` does fuzzy matching, so the words "commit", "message", or "staged changes" should land it.

**Check:** they ran a non-matching prompt and the skill stayed out of it.

### 7. Prove the skill changed the answer

The negative case showed the skill staying out of the way. This one shows it earning its place. Have them ask the same commit-message question somewhere the skill is not installed, and compare the two replies.

Any directory with no Tessl setup will do:

```bash
mkdir -p /tmp/control-repo && git -C /tmp/control-repo init -q
cp README.md /tmp/control-repo/ 2>/dev/null || echo "# Try Skills" > /tmp/control-repo/README.md
git -C /tmp/control-repo add -A
```

Then ask a fresh agent, started in that directory, the identical question they asked in beat 3. The installed repo should answer in Conventional Commits form because `commit-conventions` shaped it; the bare one answers however the agent would have answered anyway.

If the two replies look alike, say so plainly. It means the skill added nothing the agent did not already do, which is a real result and worth more than a comfortable one.

**Check:** they ran the same prompt in a repo with no Tessl setup and can say how the two replies differ.

### 8. Close out

Confirm the state rather than narrating it: the repo has the plugin installed, the skill on disk, a Conventional Commits message generated from a real diff, and a control reply to compare it against.

**Check:** they can state the one instinct the lesson builds on in their own words — the agent loads a skill *on intent*, does the job, and stays out of the way otherwise.

Then hand off without losing context. The next skill, **`02-writing-your-first-skill`**, is already installed from this course plugin. Offer to start *Writing your first skill* now, where they author one themselves from scratch, by running that skill.
