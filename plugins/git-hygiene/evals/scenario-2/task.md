# Patch Upgrade Pull Request Description

## Problem/Feature Description

Your team's internal Node.js service uses `lodash` for utility functions across several modules. A security researcher recently disclosed a prototype pollution vulnerability affecting `lodash` versions below `4.17.21`, and the fix has been available since the patch release. Your security team has flagged the outdated dependency in `starter/package.json` and asked that it be addressed immediately so the service can be deployed to production.

You need to initialize a git repository in your working directory, set up a `main` branch with the service's current `package.json` from `starter/package.json` as the initial state, then create an `upgrade/lodash` branch and apply the version bump as a commit. Once the branch is ready, draft the pull request description that would be submitted when opening the pull request from `upgrade/lodash` into `main`. There is no pull request template in this repository.

Write the pull request description to `pr-description.md`. The file should contain the subject line on the first line, followed by a blank line and the body.

## Output Specification

- `pr-description.md` — the full pull request description, including subject line and body, formatted as it would appear when submitting the pull request.
