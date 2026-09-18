---
name: pr-description
description: |
  Reviews the commits on the current branch, locates PR/MR templates, and drafts a subject line and body for the pull request/merge request. Use when the user wants to create or open a pull request, merge request, MR, or PR after any requisite commits have been made.
---

# pr-description

## When you're triggered

* The user wants to open or create a pull request (or merge request) for the
  commits on their current branch.
* The user asks for a pull request description for the commits on their
  current branch.

## What to do

1. If there are uncommitted changes, ask the user if they want to commit
   those changes.
2. Review the commits on the current branch against the `HEAD` of the default
   branch (usually `main`).
3. Check for a pull request/merge request template in the following locations.
   You must do this every time, even if you have previously read it.
      * .github/PULL_REQUEST_TEMPLATE.md
      * .github/pull_request_template.md
      * .github/PULL_REQUEST_TEMPLATE/*.md
      * .gitlab/merge_request_templates/*.md
4. If you find a template:
      * You must follow it, including all guidance included in comments.
   If you do not find a template:
      * Draft a subject line following the "Subject line rules".
      * Create the body for the pull request following the "Body rules".
5. Show the description to the user and ask them if they would like to use it for
   a new pull request.
6. Confirm the PR:
      * Conforms to the template, OR 
      * Follows the rules for subject line (50 character limit) and body
        (heading format matches, no forbidden phrases)

## Subject line rules

1. No longer than 50 characters.
2. Write in active voice with the set of changes in the pull request as the implied subject.
3. Describe the overall intent of the change in one phrase.
4. Do not end with punctuation.

## Body rules

Write as an experienced developer sharing knowledge with colleagues:
conversational and practitioner-focused, direct and no-nonsense, but not dry or
formal. The body is a narrative for the reviewer and the start of a
conversation with them. Set context the diff cannot; never retrace the diff.

1. Structure the body in two sections: `TL;DR` and `Details`.
2. Use setext underlines for both headers, five dashes under `TL;DR` and seven
   dashes under `Details`.
3. Maintain present tense and active voice throughout. Never passive ("was deployed",
   "is rejected by").
4. Forbidden phrases anywhere in the body: "this PR", "this change", "this
   commit", "this update", "the fix", "the solution", "the change". These are
   all ways of naming the work itself and are banned in every form.
5. Subject rule: start sentences with either an implied subject (the pull
   request itself, left unnamed) or a concrete subject (actual code or
   infrastructure).
      * Implied subject:  "Creates a bucket with versioning disabled because..."
      * Concrete subject: "The bucket disables versioning because..."
6. Put exact version numbers, commands, identifiers, environment variables,
   and file paths in `backticks`.
7. Prose only in TL;DR and Details: no bullet points, no numbered lists, no
   tables, no headings of your own. Small code blocks are allowed in Details
   ONLY to quote a novel, clever, or tricky fragment that deserves explanation.
8. Do not restate the TL;DR inside Details, and do not describe what the
   Files Changed tab already shows: file names, renamed files, "adds X
   function", which CSS class a template uses, which color token a date
   renders in. If a sentence would read as a line-by-line summary, cut it.

## Example

```
Removes troublesome preference files

TL;DR
-----

Deltes preferences files that caused issues with bootstrapping

Details
-------

Gets rid of two packaes that were causing too many issues in terms
of getting a bootstrap to run cleanly: `global` and `terminal`. The
likely causse was the files being overwritten during the time it
took to link them and to convert the links to hard links.
```
