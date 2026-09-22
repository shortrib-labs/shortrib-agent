#!/usr/bin/env bash
set -e

git init
git config user.email "dev@example.com"
git config user.name "Developer"

mkdir -p api

cat > api/routes.py << 'PYEOF'
from flask import Flask, jsonify

app = Flask(__name__)

users = [
    {"id": 1, "name": "Alice", "email": "alice@example.com"},
    {"id": 2, "name": "Bob", "email": "bob@example.com"},
    {"id": 3, "name": "Carol", "email": "carol@example.com"},
]


@app.route("/users/list")
def list_users():
    """Return all registered users."""
    return jsonify(users)


@app.route("/users/<int:user_id>")
def get_user(user_id):
    """Return a single user by ID."""
    user = next((u for u in users if u["id"] == user_id), None)
    if user is None:
        return jsonify({"error": "User not found"}), 404
    return jsonify(user)


if __name__ == "__main__":
    app.run(debug=True)
PYEOF

git add api/routes.py
git commit -m "initial commit"

cat > api/routes.py << 'PYEOF'
from flask import Flask, jsonify

app = Flask(__name__)

users = [
    {"id": 1, "name": "Alice", "email": "alice@example.com"},
    {"id": 2, "name": "Bob", "email": "bob@example.com"},
    {"id": 3, "name": "Carol", "email": "carol@example.com"},
]


@app.route("/users/index")
def list_users():
    """Return all registered users."""
    return jsonify(users)


@app.route("/users/<int:user_id>")
def get_user(user_id):
    """Return a single user by ID."""
    user = next((u for u in users if u["id"] == user_id), None)
    if user is None:
        return jsonify({"error": "User not found"}), 404
    return jsonify(user)


if __name__ == "__main__":
    app.run(debug=True)
PYEOF

git add api/routes.py

echo "Repository initialized with staged changes."
echo "Run 'git diff --staged' to review what will be committed."
