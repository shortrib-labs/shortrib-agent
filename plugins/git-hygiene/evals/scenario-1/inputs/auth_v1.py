from http.server import BaseHTTPRequestHandler, HTTPServer
import json

VALID_TOKEN = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9"


class AuthHandler(BaseHTTPRequestHandler):
    def do_POST(self):
        if self.path == "/auth/verify":
            auth_header = self.headers.get("Authorization", "")
            if not auth_header:
                self.send_response(400)
                self.send_header("Content-Type", "application/json")
                self.end_headers()
                self.wfile.write(json.dumps({"error": "missing authorization header"}).encode())
                return

            token = auth_header.removeprefix("Bearer ")
            if token != VALID_TOKEN:
                self.send_response(400)
                self.send_header("Content-Type", "application/json")
                self.end_headers()
                self.wfile.write(json.dumps({"error": "invalid token"}).encode())
                return

            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.end_headers()
            self.wfile.write(json.dumps({"status": "authenticated"}).encode())
        else:
            self.send_response(404)
            self.end_headers()

    def log_message(self, format, *args):
        pass


if __name__ == "__main__":
    server = HTTPServer(("localhost", 8080), AuthHandler)
    server.serve_forever()
