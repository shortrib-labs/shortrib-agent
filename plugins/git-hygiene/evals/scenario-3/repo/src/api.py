from typing import Optional
from datetime import datetime

tasks = {}
_next_id = 1


def create_task(title: str, description: str = "") -> dict:
    global _next_id
    task = {
        "id": _next_id,
        "title": title,
        "description": description,
        "completed": False,
        "created_at": datetime.utcnow().isoformat(),
    }
    tasks[_next_id] = task
    _next_id += 1
    return task


def get_task(task_id: int) -> Optional[dict]:
    return tasks.get(task_id)


def list_tasks() -> list:
    return list(tasks.values())


def update_task(task_id: int, **kwargs) -> Optional[dict]:
    task = tasks.get(task_id)
    if not task:
        return None
    for key, value in kwargs.items():
        if key in task:
            task[key] = value
    return task


def delete_task(task_id: int) -> bool:
    if task_id in tasks:
        del tasks[task_id]
        return True
    return False
