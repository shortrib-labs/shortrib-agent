# Task Manager API

A lightweight in-memory task management library.

## Functions

- `create_task(title, description)` — create a new task
- `get_task(task_id)` — retrieve a task by ID
- `list_tasks()` — list all tasks
- `update_task(task_id, **kwargs)` — update task fields
- `delete_task(task_id)` — remove a task

## Utilities

- `paginate(items, page, per_page)` — slice a list into pages
- `normalize_text(text)` — lowercase and strip whitespace
