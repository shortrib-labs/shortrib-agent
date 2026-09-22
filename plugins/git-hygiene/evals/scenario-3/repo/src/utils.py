def paginate(items: list, page: int = 1, per_page: int = 10) -> dict:
    total = len(items)
    start = (page - 1) * per_page
    end = start + per_page
    return {
        "items": items[start:end],
        "total": total,
        "page": page,
        "per_page": per_page,
        "pages": (total + per_page - 1) // per_page,
    }


def normalize_text(text: str) -> str:
    return text.strip().lower()
