<!--
PULL REQUEST TEMPLATE

Every instruction in these comments is a requirement, not a suggestion. If you
are an agent drafting a pull request, read all of them before writing a word,
run the checklist at the bottom before submitting, and delete every HTML
comment from the final body. Only the headers, your prose, and the Test plan
list should remain.

VOICE

Write as an experienced developer sharing knowledge with colleagues:
conversational and practitioner-focused, direct and no-nonsense, but not dry or
formal. The body is a narrative for the reviewer and the start of a
conversation with them. Set context the diff cannot; never retrace the diff.

TITLE (every rule must pass)

- Starts with a third-person present-tense: "Adds", "Fixes", "Removes",
  "Tracks". Never imperative ("Add"), past ("Added"), or future ("Will add").
- 40 characters or fewer, counting spaces.
- No first person: no "I", "We", "My", "Our".
- Names the INTENT (the outcome or capability), not the mechanism. Good:
  "Tracks CMX credit purchases as deals" Bad:  "Adds webhook handler and event
  listener"
- Do not put the title in the body. It goes in the pull request title field.


BODY (every rule must pass)

- Reproduce the headers below exactly. `TL;DR` and `Details` use setext
  underlines: five dashes under `TL;DR`, seven dashes under `Details`. Never
  convert them to `##` headings. `## Test plan` is the one ATX heading, and it
  stays ATX.
- Present tense and active voice throughout. Never passive ("was deployed",
  "is rejected by").
- Forbidden phrases anywhere in the body: "this PR", "this change", "this
  commit", "this update", "the fix", "the solution", "the change". These are
  all ways of naming the work itself and are banned in every form.
- Subject rule: start sentences with either an implied subject (the pull
  request itself, left unnamed) or a concrete subject (actual code or
  infrastructure).
  Implied subject:  "Creates a bucket with versioning disabled because..."
  Concrete subject: "The bucket disables versioning because..."
- Put exact version numbers, commands, identifiers, environment variables,
  and file paths in `backticks`.
- Prose only in TL;DR and Details: no bullet points, no numbered lists, no
  tables, no headings of your own. Small code blocks are allowed in Details
  ONLY to quote a novel, clever, or tricky fragment that deserves explanation.
- Do not restate the TL;DR inside Details, and do not describe what the
  Files Changed tab already shows: file names, renamed files, "adds X
  function", which CSS class a template uses, which color token a date
  renders in. If a sentence would read as a line-by-line summary, cut it.

CHECKLIST (run before submitting; every box must be true)

- [ ] Title is present tense, 40 characters or fewer, and states intent
- [ ] TL;DR opens with a DIFFERENT verb than the title, same tense
- [ ] TL;DR is one or two sentences in a single paragraph
- [ ] Details opens by stating what the code does, then explains why
- [ ] Details ends with a concrete verification step
- [ ] No forbidden phrases, no passive voice, no first person
- [ ] Headers match the template exactly (setext dash counts, ATX Test plan)
- [ ] Versions, commands, paths, and identifiers are in `backticks`
- [ ] Every Test plan item is `- [x]` and cites evidence
- [ ] Tests pass and linting passes, with the commands recorded in Test plan
- [ ] All HTML comments are deleted from the final body
-->

TL;DR
-----
<!-- One or two sentences, one paragraph. Open with a verb that differs from
     the title's verb (title "Adds" -> TL;DR "Gives", "Lets", "Enables";
     title "Fixes" -> TL;DR "Prevents", "Eliminates", "Stops"). State the
     problem that gets solved or what becomes possible, not how it is built.
     Lead with the outcome so a reviewer can decide whether to read on.
     Good: "Gives the revenue team automatic CRM visibility into credit
           purchases."
     Bad:  "Implements a webhook handler that creates deal records via the
           REST API." -->

<!-- Issue reference: keep the next line only when an issue exists, placed
     here between TL;DR and Details. Always use "Closes", never "Fixes" or
     "Resolves". Delete the line if there is no issue. -->
Closes #

Details
-------
<!-- Tell the story behind the work. The first sentence states directly what
     the code does; lead with action, not background. Then answer, in
     narrative paragraphs, what the reviewer needs in order to judge the
     work:

     - Why it needed solving now, or what it unblocks going forward
     - Why this approach over the alternatives considered or rejected
     - What tradeoffs, constraints, or limitations shaped the decision;
       state them honestly
     - What existing patterns or conventions the work follows
     - Which documentation or issues to read for more context (link them)

     Each paragraph should advance the reader's understanding of a distinct
     aspect. If a novel, clever, or tricky idea lives in the code, call it
     out here and quote the fragment in a small code block so the reviewer
     knows where to look closely.

     End with a verification step: a URL to visit, a command to run, or a
     behavior to observe that confirms the work is correct. The reviewer
     should finish Details knowing "what is this about" and "how do I know
     it was done right". -->

<!-- Test plan: describe what WAS tested and the result, never what will be
     tested. Every item uses `- [x]`; an unchecked `- [ ]` means the pull
     request is not ready. Each item cites concrete evidence: the command
     run and its output, test suite pass counts, HTTP status codes, record
     counts, screenshots. Bullets are expected in this section. -->
## Test plan

- [x]

<!-- Optional sections. Add them after Test plan, ATX headings, only when
     they apply. Delete this comment otherwise.

## Breaking changes

Describe what breaks and the exact migration steps.

## Dependencies

Name each new or updated dependency with its exact version in `backticks`
and why it is needed. -->

<!-- Keep the attribution line below only when an agent drafted the
     description; delete it otherwise. -->
🤖 Generated with [Claude Code](https://claude.com/claude-code)

<!-- WORKED EXAMPLE (delete before submitting)

Title: Tracks CMX credit purchases as deals

TL;DR
-----
Gives the revenue team automatic CRM visibility into credit purchases so
nobody re-keys Stripe receipts into HubSpot by hand.

Closes #214

Details
-------
Creates a HubSpot deal the moment Stripe reports a successful
`checkout.session.completed` event for a CMX credit bundle. Sales currently
learns about credit purchases a week late, when finance reconciles Stripe
payouts, which means follow-up on expansion opportunities starts cold. The
webhook path in `apps/billing/webhooks.py` already verifies Stripe
signatures, so extending it costs far less than polling the Stripe API on a
schedule and avoids the duplicate-deal problem that polling introduces on
retries.

The one non-obvious piece is idempotency. Stripe retries webhooks for up to
three days, so the deal lookup keys on the Stripe session id rather than
the customer:

    deal = hubspot.deals.find_by_property("stripe_session_id", session.id)

Refunds do not close the deal yet; that is tracked in #219 and left out here
to keep the surface small.

Verify by running `just stripe-replay evt_1PZ...` against a dev tunnel and
confirming a deal named after the customer appears in the HubSpot sandbox
pipeline within a few seconds.

## Test plan

- [x] `make test` passes: 212 passed, 0 failed
- [x] `make lint` passes with no findings
- [x] Replayed `evt_1PZq...` three times; HubSpot sandbox shows exactly one
      deal (`HTTP 200` on the first call, `HTTP 200` no-op on retries)
-->
