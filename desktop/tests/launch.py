"""Launch the packaged binary and observe JavaScript running in its native wallet webview."""

from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
import subprocess
import sys
import threading


def main(executable):
    ready = threading.Event()

    class Dashboard(BaseHTTPRequestHandler):
        def do_GET(self):
            if self.path == "/":
                body = b'<!doctype html><title>SwissKnife QA</title><script>fetch("/ready")</script>'
                self.send_response(200)
                self.send_header("Content-Type", "text/html")
            elif self.path == "/ready":
                body = b""
                print("Native webview:", self.headers.get("User-Agent"), flush=True)
                ready.set()
                self.send_response(204)
            else:
                self.send_error(404)
                return
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)

    with ThreadingHTTPServer(("127.0.0.1", 0), Dashboard) as server:
        threading.Thread(target=server.serve_forever, daemon=True).start()
        origin = f"http://127.0.0.1:{server.server_port}"
        process = subprocess.Popen([str(Path(executable).resolve()), "--instance", origin])
        try:
            for _ in range(60):
                if ready.wait(1):
                    return
                if process.poll() is not None:
                    raise RuntimeError(f"Desktop exited with status {process.returncode}")
            raise RuntimeError("The packaged client did not load the wallet in 60 seconds")
        finally:
            if process.poll() is None:
                process.terminate()
                try:
                    process.wait(timeout=10)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait()
            server.shutdown()


if __name__ == "__main__":
    main(sys.argv[1])
