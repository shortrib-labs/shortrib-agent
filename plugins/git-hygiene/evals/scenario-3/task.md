# Draft a PR Description for the Task Search Feature

## Background

Your team maintains a task management library — a set of Python functions in `src/api.py` that provide task CRUD operations backed by an in-memory store. The module also includes utility helpers in `src/utils.py`. The codebase is in the `repo/` directory.

Users have been asking for a way to find tasks by keyword without iterating through the entire list. The team decided this is the next thing to ship: a `search_tasks` function that filters tasks whose `title` or `description` contains a given keyword (case-insensitive). This is tracked under ticket #47 in the project backlog.

## Setup

Initialize a Git repository inside the `repo/` directory and create a `main` branch with an initial commit of all existing files.

Then create a feature branch named `feature/search-tasks`. On this branch, implement the `search_tasks(keyword)` function in `src/api.py` and commit your work. You may make more than one commit if that feels natural.

## Output

Once the feature is committed on the feature branch, draft a pull request description for merging `feature/search-tasks` into `main`. Save the PR description to `pr-description.md` in your working directory.
