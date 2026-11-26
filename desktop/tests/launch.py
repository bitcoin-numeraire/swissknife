"""Launch the packaged binary and observe JavaScript running in its native wallet webview."""

from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
import json
import os
import secrets
import subprocess
import sys
import threading


def main(executable, initialize):
    state = Path(os.environ["RUNNER_TEMP"]) / "swissknife-origin.json"
    if initialize:
        if state.exists():
            raise RuntimeError("Expected a fresh desktop test profile")
        port, marker = 0, secrets.token_hex(16)
    else:
        port, marker = json.loads(state.read_text())
    ready = threading.Event()
    failures = []

    # A committed IndexedDB record proves that the native wallet retained its
    # origin storage. A JWT in the real dashboard uses sessionStorage instead.
    script = """
      const request = indexedDB.open('installer-test', 1);
      request.onupgradeneeded = () => request.result.createObjectStore('preferences');
      request.onerror = () => fetch('/failed');
      request.onsuccess = () => {
        const db = request.result;
        const transaction = db.transaction('preferences', initialize ? 'readwrite' : 'readonly');
        const store = transaction.objectStore('preferences');
        const read = store.get('marker');
        read.onsuccess = () => {
          if (initialize && read.result === undefined) store.add(marker, 'marker');
          else if (initialize || read.result !== marker) transaction.abort();
        };
        transaction.onabort = () => fetch('/failed');
        transaction.oncomplete = () => { db.close(); fetch('/ready'); };
      };
    """
    body = (f"<!doctype html><title>SwissKnife QA</title><script>"
            f"const initialize = {json.dumps(initialize)}, marker = {json.dumps(marker)};"
            f"{script}</script>").encode()

    class Dashboard(BaseHTTPRequestHandler):
        def do_GET(self):
            if self.path == "/":
                self.send_response(200)
                self.send_header("Content-Type", "text/html")
                self.send_header("Cache-Control", "no-store")
                self.send_header("Content-Length", str(len(body)))
                self.end_headers()
                self.wfile.write(body)
            elif self.path in ("/ready", "/failed"):
                if self.path == "/failed":
                    failures.append("The wallet origin did not retain its test preference")
                self.send_response(204)
                self.end_headers()
                ready.set()
            else:
                self.send_error(404)

    with ThreadingHTTPServer(("127.0.0.1", port), Dashboard) as server:
        threading.Thread(target=server.serve_forever, daemon=True).start()
        origin = f"http://127.0.0.1:{server.server_port}"
        process = subprocess.Popen([str(Path(executable).resolve()), "--instance", origin])
        try:
            for _ in range(60):
                if ready.wait(1):
                    if failures:
                        raise RuntimeError(failures[0])
                    if initialize:
                        state.write_text(json.dumps([server.server_port, marker]))
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
    main(sys.argv[1], "--initialize" in sys.argv[2:])
