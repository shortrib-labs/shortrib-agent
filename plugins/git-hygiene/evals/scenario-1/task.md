# Auth Service PR Description

## Background

A teammate just wrapped up a bugfix for the internal authentication service. The `/auth/verify` endpoint had been returning the wrong HTTP status code for authentication failures — downstream API gateways and OAuth clients that inspect status codes precisely were misclassifying auth failures as malformed requests, causing silent errors in integrations that relied on correct HTTP semantics.

The change is small but important: the endpoint now returns the correct status code for missing or invalid tokens. Two versions of the auth module are in `inputs/`:

- `inputs/auth_v1.py` — the original, pre-fix version
- `inputs/auth_v2.py` — the corrected version

## Setup

Prepare a git repository that reflects the state of this work:

1. Initialize a new git repository in a directory called `repo/`
2. On the `main` branch, make an initial commit that includes `inputs/auth_v1.py` saved as `auth.py` at the root of the repo
3. Create and switch to a branch named `fix/auth-error-code`
4. Update `auth.py` to the contents of `inputs/auth_v2.py` and commit the change

Use `git config user.email` and `git config user.name` to set a dummy author if needed so commits succeed.

## Deliverable

Write a pull request description for the `fix/auth-error-code` branch (comparing against `main`) and save it to `pr-description.md` in your working directory.

The description should be suitable for submitting on GitHub. Include a subject line and a body.
