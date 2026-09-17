---
name: 05-publishing-across-a-team
description: Use when a learner wants to start, work through, or be guided or tutored through the Publishing and installing across a team lesson — taking a local plugin all the way to the registry. Walks them one step at a time through dogfooding their git-hygiene plugin against real work, publishing it to a workspace, installing it the way a teammate would, and shipping an update with a semver version bump, running a check after each step before moving on. Triggers on asks like guide me through publishing my plugin, start lesson 5, help me publish to the registry, walk me through team install, or how do I version-bump and update a plugin.
---

# Publishing & installing across a team — walkthrough

You are guiding a learner through the **Publishing & installing across a team** lesson in their own repository. Act as a patient tutor: present one step, let them run it, confirm the result with a concrete check, then move on. Do not dump the whole lesson at once.

The full lesson page is at `/academy/foundations/publishing-across-a-team/`. By the end the learner has dogfooded their `git-hygiene` plugin, published it to their workspace, installed it as a teammate would, and shipped an update with a version bump.

This is the last foundations lesson and builds on Lesson 4: the learner should have a `git-hygiene` plugin on disk at `./plugins/git-hygiene` with `commit-conventions` inside it. If they don't, send them back to Lesson 4 first.

Publishing pushes to a real registry. The learner needs their own workspace and `tessl login` — you cannot publish for them. Where a step needs their workspace, use `my-workspace` and have them substitute their own.

## When you're triggered

The learner has asked to start, work through, or get guided through the "Publishing & installing across a team" lesson (Lesson 5 of Skill Foundations), or to publish a plugin and ship updates.

## How to guide

Walk these in order. After each, run the **Check** before advancing. If a check fails, troubleshoot that step — do not move on.

### 0. Confirm prerequisites

They need: the Tessl CLI with `tessl login` done, a workspace they can publish into, and their `git-hygiene` plugin from Lesson 4 on disk.

**Check:** the `git-hygiene` plugin from Lesson 4 is on disk. Ask which directory they are in, then have them run `ls plugins/git-hygiene/.tessl-plugin/plugin.json` from the repository they are using for the course. If that comes back empty, ask which directory they ran it in and try that one path again from the right place — a wrong working directory looks exactly like a lesson never done. Do NOT have them search the tree: a `find` across the workspace turns up this walkthrough and invalidates the run. Only once they confirm the file is genuinely absent does Lesson 4 come into it.

### 1. Dogfood it before publishing

A clean lint and high review scores are necessary but not sufficient. Before publishing anything other people install from, install the plugin from their own filesystem and put every skill through its paces against real work:

```bash
tessl install file:./plugins/git-hygiene
```

For git-hygiene: make a commit (does `commit-conventions` fire?), and if siblings are present, open a PR (`pr-description`?) and cut a release (`release-notes`?). Two things to watch:

- **Triggering** — does each prompt load the right skill? If a prompt loads the wrong skill or nothing, the descriptions need tightening.
- **Tripping** — do skills tread on each other? A skill loading when a sibling should have is the classic overlapping-descriptions symptom — the same failure they guarded against in Lesson 4.

**Check:** they've staged a real change and confirmed `commit-conventions` fires on a commit-message prompt and produces a Conventional Commits message. A bad first release costs far more to recall than ten minutes of dogfooding.

### 2. Point the manifest at your workspace

Before the publish, have them check where it is aimed. Publishing takes no destination argument: the target is the `name` in `.tessl-plugin/plugin.json`, and the workspace is the part before the slash.

```bash
cat ./plugins/git-hygiene/.tessl-plugin/plugin.json
```

If that name still reads `my-workspace/git-hygiene` it is the Lesson 4 placeholder, and the publish will aim at a workspace called `my-workspace`. Have them replace the part before the slash with the workspace `tessl whoami` names, leaving `git-hygiene` alone. Do not let them reach for `--workspace` instead: it applies only when the manifest name is flat, so while the name carries a prefix the flag is ignored and the manifest wins.

If they publish to the placeholder anyway, the refusal will not name the cause — a `my-workspace` workspace exists, so they are told they lack permission rather than that the target is wrong. Read `✘ You do not have permission to publish to this workspace` as "wrong target" first.

**Check:** the `name` in `./plugins/git-hygiene/.tessl-plugin/plugin.json` names the workspace `tessl whoami` reports, not `my-workspace`.

### 3. Publish to your workspace

When it feels stable and the name is right:

```bash
tessl plugin publish ./plugins/git-hygiene
```

Explain what happens server-side, because it's the same quality gate every published plugin passes: the plugin is linted again; every skill is validated against the Agent Skills spec; a **review eval runs automatically** against each skill (the same one as `tessl review run`), with scores on the registry page within minutes; and the version (`0.1.0` for the first publish) is locked, so future publishes need `--bump`. Those auto scores are exactly why sharp, non-overlapping descriptions matter — the registry grades each skill on its own.

Then install it the way a teammate would — the same command from Lesson 1, now pointed at theirs:

```bash
tessl install my-workspace/git-hygiene
# or pin a version:
tessl install my-workspace/[email protected]
```

Check what landed, including the auto-generated review scores:

```bash
tessl plugin info my-workspace/git-hygiene
```

**Check:** `tessl plugin info <their-workspace>/git-hygiene` — substituting the workspace from beat 2, never `my-workspace` — shows the published plugin with a review eval score for each skill. If publish failed, read the error — usually a lint failure or a missing `tessl login`.

### 4. Ship an update

Editing a skill, re-publishing, and bumping the version is a one-line loop:

```bash
# edit SKILL.md for whichever skill changed
tessl plugin publish ./plugins/git-hygiene --bump patch
# new version: 0.1.1
```

`--bump` follows semver, and the level is a promise to everyone who installed it:

- **`patch`** — bug fixes, wording tweaks, anything backward-compatible.
- **`minor`** — new skills or capabilities; existing usage still works (adding `pr-description` is a `minor`).
- **`major`** — breaking changes; consumers should read release notes first.

Consumers pick up the new version with:

```bash
tessl update my-workspace/git-hygiene
```

`tessl outdated` shows them which plugins have updates available — the same shape as `npm outdated`, just for context.

**Check:** after the `--bump patch` publish, `tessl plugin info <their-workspace>/git-hygiene` shows `0.1.1`, and `tessl update <their-workspace>/git-hygiene` in a consuming repo pulls it.

### 5. Confirm the library is installed locally

```bash
tessl list
```

**Check:** `<their-workspace>/git-hygiene` is listed, with each skill inside it.

### 6. Read the published scores

```bash
tessl plugin info my-workspace/git-hygiene
```

**Check:** a review eval score is shown for each skill, set automatically on publish.

### 7. Prove each skill loads on its own prompt

Prompt for a commit message, then a PR description if present, then release notes.

**Check:** each prompt loaded a *different* skill. If two load on the same prompt, the fix is always the same: return to the descriptions, tighten the narrower one until the overlap disappears, and re-publish with `--bump patch`.

### 8. Install it somewhere else

```bash
tessl install my-workspace/git-hygiene
```

Run this in a *different* repo. That is the distribution test.

**Check:** it landed cleanly in one shot.

### 9. Close out

Confirm the whole distance: a skill installed, a skill written, a library bundled, and a plugin published that the team installs and updates with one command each.

**Check:** they can name the loop that keeps it alive: dogfood, publish, version, update.

That completes Skill Foundations. Point them at the next course, *Tuning Your Agent*, which ships as its own plugin. They install it once and run its lessons the same way as this one:

```bash
tessl install tessl-academy/tuning-your-agent
```

Then offer to kick it off by running its first skill, **`01-task-evals-and-scenarios`**, where they put a skill under test to improve it with numbers.
