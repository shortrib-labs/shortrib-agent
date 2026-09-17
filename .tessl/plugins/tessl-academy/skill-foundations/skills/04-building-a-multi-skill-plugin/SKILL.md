---
name: 04-building-a-multi-skill-plugin
description: Use when a learner wants to start, work through, or be guided or tutored through the Building a multi-skill plugin lesson, bundling related skills into one versioned, installable plugin. Walks them one step at a time through scaffolding an empty git-hygiene plugin with the Tessl CLI, copying their commit-conventions skill into it, linting the whole plugin, and keeping skill descriptions sharp enough that each fires on exactly one situation, running a check after each step before moving on. Triggers on asks like guide me through building a multi-skill plugin, start lesson 4, help me bundle my skills into a plugin, or walk me through making a skill library.
---

# Building a multi-skill plugin — walkthrough

You are guiding a learner through the **Building a multi-skill plugin** lesson in their own repository. Act as a patient tutor: present one step, let them run it, confirm the result with a concrete check, then move on. Do not dump the whole lesson at once.

The full lesson page is at `/academy/foundations/building-a-multi-skill-plugin/`. By the end the learner has scaffolded a `git-hygiene` plugin, added their `commit-conventions` skill to it, linted the whole plugin clean, and can state what makes skills belong in the same plugin and why their descriptions must not overlap.

This builds directly on Lesson 2 (Writing your first skill): the learner should already have a `commit-conventions` skill on disk at `./skills/commit-conventions`. If they don't, send them back to Lesson 2 first.

## When you're triggered

The learner has asked to start, work through, or get guided through the "Building a multi-skill plugin" lesson (Lesson 4 of Skill Foundations), or to bundle related skills into one plugin.

## How to guide

Walk these in order. After each, run the **Check** before advancing. If a check fails, troubleshoot that step, and do not move on.

### 0. Set the framing: a plugin is the unit you ship

Before any commands, make sure they hold the distinction the whole lesson turns on: **the unit of authorship is the skill; the unit of distribution is the plugin.** A plugin is what gets versioned, published, and installed, and it can bundle one skill, several related skills, or skills plus rules and docs.

Then the judgment call. A plugin is *not* "everything one team has written." Coherent skills share three things:

- **An audience** — the same people install all of them.
- **A problem space** — they help with adjacent jobs.
- **A release rhythm** — when one needs updating, the others usually do too.

The `git-hygiene` family (`commit-conventions`, `pr-description`, `release-notes`) is the clean example. Heuristic: if skill B's release cycle would always lag skill A's by weeks, they belong in separate plugins.

**Check:** ask them to say, in their own words, why `commit-conventions` and `release-notes` belong in one plugin but a `pdf-summarizer` skill would not.

### 1. Confirm prerequisites

They need the Tessl CLI (`tessl login` done), a workspace to publish into (personal is fine — the steps use `my-workspace`, have them substitute their own), and their `commit-conventions` skill from Lesson 2 on disk.

**Check:** the `commit-conventions` skill from Lesson 2 is on disk. Ask which directory they are in, then have them run `ls skills/commit-conventions/skills/commit-conventions/SKILL.md` from the repository they are using for the course — `tessl skill new` created a single-skill plugin, so the skill sits one level inside. If that comes back empty, ask which directory they ran it in and try that one path again from the right place. Do NOT have them search the tree: a `find` for SKILL.md turns up this walkthrough and invalidates the run. Only once they confirm the file is genuinely absent does Lesson 2 come into it.

### 2. Scaffold the plugin

For a library that bundles more than one skill, scaffold the plugin directly rather than with `tessl skill new` (which assumes a single-skill plugin). The CLI will not create a plugin with nothing in it: without a skill or a rules file it stops with `✘ Must specify at least one of: --skill or --rules`. A `git-hygiene` plugin wants a rules file anyway, so it goes in now and `skills/` is left for the next beat:

```bash
tessl plugin new \
  --name my-workspace/git-hygiene \
  --summary "Skills for clean git workflows: commits, PRs, release notes" \
  --path ./plugins/git-hygiene \
  --workspace my-workspace \
  --rules git-hygiene
```

This creates `plugins/git-hygiene/` holding `.tessl-plugin/plugin.json` and `rules/git-hygiene.md`. No `skills/` directory exists yet and the manifest carries no `skills` path; both are expected. The version starts at `0.1.0` (semver). Keep it in the `0.x` range while iterating; bump to `1.0.0` when the shape is stable enough for other teams to depend on.

**Check:** `./plugins/git-hygiene/.tessl-plugin/plugin.json` and `./plugins/git-hygiene/rules/git-hygiene.md` both exist. Do not expect a `skills/` directory yet, and do not have them add a `skills` field to the manifest — the next beat creates the directory and Tessl finds it by convention.

### 3. Add the skill to the plugin

Make the `skills/` directory the scaffold left out, then copy the `commit-conventions` skill into it:

```bash
mkdir -p ./plugins/git-hygiene/skills
cp -r ./skills/commit-conventions/skills/commit-conventions \
      ./plugins/git-hygiene/skills/commit-conventions
```

`tessl skill new` made a single-skill plugin in Lesson 2, so the skill is one level inside `./skills/commit-conventions`. Copying the outer directory nests a plugin inside a plugin, which lints but is not what the lesson is teaching.

That's all the wiring it needs, with no manifest edit. Because the manifest names no `skills` path, Tessl falls back to scanning `skills/`, so the skill is part of the plugin the moment its directory lands there. Adding `pr-description` and `release-notes` later is the same move.

**Check:** `./plugins/git-hygiene/skills/commit-conventions/SKILL.md` exists.

### 4. Lint the whole plugin

Lint after each addition to catch structural mistakes early:

```bash
tessl plugin lint ./plugins/git-hygiene
```

Plugin lint is `skill lint` applied across every skill in the manifest, plus checks that the plugin-level fields (`name`, `version`, `description`) are present and the `skills` path resolves to real skill directories.

**Check:** `tessl plugin lint` returns a clean pass. If it flags an unresolved file reference (e.g. an `examples/` line in the copied `SKILL.md`), fix it the same way as in Lesson 2 and re-run.

### 5. The thing that breaks libraries: overlapping descriptions

Walk them through the one problem a library has that a single skill never does. When two skills' descriptions overlap, the agent can't tell which to load. Each description must name a situation specific enough that no other skill in the same workspace could plausibly match:

- `commit-conventions` — *"the user wants a commit message"*
- `pr-description` — *"the user is opening a PR and wants a description"*
- `release-notes` — *"the user is preparing a release and wants notes between tags"*

Diagnostic: if they can read two descriptions and imagine the same user prompt matching both, they overlap. Tighten the narrower one by naming the trigger phrase the user would actually say, not the general topic.

**Check:** ask them to write a one-line `description` trigger for a hypothetical `pr-description` skill that could not be confused with `commit-conventions`. Confirm it names the PR situation specifically, not "helps with git."

### 6. Check the plugin manifest

```bash
cat ./plugins/git-hygiene/.tessl-plugin/plugin.json
```

**Check:** it carries `name`, `version` (`0.1.0`), `description` and `private`. The manifest carries no `skills` field, and should not: leaving it out is what makes `skills/` the default. If they added one by hand, have them take it out and confirm the lint in the next beat still passes.

### 7. Check the skill landed inside it

```bash
ls ./plugins/git-hygiene/skills/commit-conventions/
```

**Check:** `SKILL.md` is there.

### 8. Lint the plugin

```bash
tessl plugin lint ./plugins/git-hygiene
```

**Check:** the lint returns a clean pass.

### 9. Close out

Confirm the state: they have a plugin directory that lints, holding a skill they moved into it.

**Check:** they can state both takeaways in their own words — when bundling is right (shared audience, problem space, release cadence) and the description discipline that keeps a library working (each skill fires on exactly one situation).

Then hand off without losing context. The next skill, **`05-publishing-across-a-team`**, is already installed from this course plugin. Offer to start *Publishing & installing across a team* now, where they publish this `git-hygiene` plugin so the whole team can install it, by running that skill.
