# Draft a Pull Request Description for a Caching Feature

## Problem Description

Your team is shipping a performance improvement to your API service: an in-memory LRU cache in front of the database read path for user profile lookups. The feature is complete and ready for review, but nobody has written up the pull request yet. The lead engineer has asked you to prepare the PR description so the team can review and merge it.

To simulate the branch state, set up a local git repository as follows. Initialize a new git repo, establish a `main` branch with a base commit that contains a simple `api.py` file with a `get_user_profile` function that queries a database directly. Then create a `feature/user-profile-cache` branch off `main` and add two or three commits to it that progressively introduce the caching layer — importing an LRU cache library, wrapping the lookup function to check and populate the cache, and adding cache invalidation when a profile is updated.

Once the repository reflects this history, draft a pull request description for the `feature/user-profile-cache` branch against `main` and save it to `pr-description.md` in your working directory.

## Output Specification

- `pr-description.md` — a Markdown file containing the complete pull request description. The first line should be the subject line, followed by the body. Do not include any metadata or front matter — just the subject and body as they would appear when opening a pull request.
