#!/usr/bin/env python3
"""
MP-SPDZ preprocessing server.

Starts idle and waits for a POST /api/run request with configuration.
When triggered, compiles and runs the MP-SPDZ protocol to generate
preprocessing data (Beaver triples, input masks, edaBits).

API:
  GET  /api/status  → current state (idle, compiling, running, done, error)
  POST /api/run     → trigger preprocessing with JSON body:
                       { "k": 64, "s": 64, "protocol": "spdz2k", "program": "bench_simple" }
"""

import json
import os
import subprocess
import threading
import time
from http.server import HTTPServer, BaseHTTPRequestHandler

PARTY_ID = int(os.environ.get("PARTY_ID", "0"))
N_PARTIES = int(os.environ.get("N_PARTIES", "2"))
PORT_BASE = int(os.environ.get("PORT_BASE", "14000"))
API_PORT = int(os.environ.get("API_PORT", "5000"))

# Global state
state = {
    "party_id": PARTY_ID,
    "protocol": "",
    "program": "",
    "phase": "idle",        # idle, compiling, running, done, error
    "start_time": None,
    "end_time": None,
    "elapsed_ms": 0,
    "output": "",
    "error": "",
    "k_bits": 0,
    "s_bits": 0,
}
state_lock = threading.Lock()


def run_preprocessing(config: dict):
    protocol = config.get("protocol", "spdz2k")
    program = config.get("program", "bench_simple")
    k_bits = config.get("k", 64)
    s_bits = config.get("s", k_bits)  # default s = k
    dim = config.get("dim", 4)  # matrix dimension

    with state_lock:
        state["protocol"] = protocol
        state["program"] = program
        state["k_bits"] = k_bits
        state["s_bits"] = s_bits
        state["phase"] = "compiling"
        state["output"] = ""
        state["error"] = ""
        state["elapsed_ms"] = 0

    # Compile (pass dim as program argument)
    compile_cmd = ["python3", "compile.py", "-R", str(k_bits), program, str(dim)]
    print(f"[mpspdz] Compiling: {' '.join(compile_cmd)}", flush=True)
    result = subprocess.run(compile_cmd, capture_output=True, text=True)
    if result.returncode != 0:
        with state_lock:
            state["phase"] = "error"
            state["error"] = result.stderr
        print(f"[mpspdz] Compile failed: {result.stderr}", flush=True)
        return

    # Generate SSL certs if needed
    if not os.path.exists(f"Player-Data/P{PARTY_ID}.pem"):
        print("[mpspdz] Generating SSL certificates...", flush=True)
        subprocess.run(["Scripts/setup-ssl.sh", str(N_PARTIES)],
                       capture_output=True, text=True)

    # Run protocol — select binary compiled for the matching ring size
    party0_host = os.environ.get("PARTY0_HOST", "party0-mpspdz")
    binary = f"./{protocol}-party-{k_bits}.x"
    if not os.path.exists(binary):
        with state_lock:
            state["phase"] = "error"
            state["error"] = f"No binary for ring size {k_bits}. Available: 5, 10, 32, 64."
        print(f"[mpspdz] Binary not found: {binary}", flush=True)
        return
    # Program args are compile-time (baked into bytecode by compile.py).
    # The compiled program is named "bench_simple-2" for dim=2, etc.
    compiled_name = f"{program}-{dim}"
    run_cmd = [
        binary,
        "-p", str(PARTY_ID),
        "-N", str(N_PARTIES),
        "-pn", str(PORT_BASE),
        "-h", party0_host,
        "-R", str(k_bits),
        "-S", str(s_bits),
        compiled_name,
    ]
    print(f"[mpspdz] Running: {' '.join(run_cmd)}", flush=True)

    with state_lock:
        state["phase"] = "running"
        state["start_time"] = time.time()

    result = subprocess.run(run_cmd, capture_output=True, text=True, timeout=600)
    end_time = time.time()

    with state_lock:
        state["end_time"] = end_time
        state["elapsed_ms"] = round((end_time - state["start_time"]) * 1000, 1)
        state["output"] = result.stdout
        if result.returncode == 0:
            state["phase"] = "done"
        else:
            state["phase"] = "error"
            state["error"] = result.stderr

    print(f"[mpspdz] Done in {state['elapsed_ms']}ms", flush=True)
    if result.stdout:
        print(f"[mpspdz] Output:\n{result.stdout}", flush=True)
    if result.stderr:
        print(f"[mpspdz] Stderr:\n{result.stderr}", flush=True)


class Handler(BaseHTTPRequestHandler):
    def do_GET(self):
        if self.path == "/api/status":
            with state_lock:
                body = json.dumps(state).encode()
            self._json_response(200, body)
        else:
            self.send_response(404)
            self.end_headers()

    def do_POST(self):
        if self.path == "/api/run":
            content_len = int(self.headers.get("Content-Length", 0))
            body = self.rfile.read(content_len)
            try:
                config = json.loads(body) if body else {}
            except json.JSONDecodeError:
                config = {}

            with state_lock:
                if state["phase"] == "running" or state["phase"] == "compiling":
                    self._json_response(409, json.dumps({
                        "error": "Preprocessing already in progress"
                    }).encode())
                    return

            # Run in background thread
            t = threading.Thread(target=run_preprocessing, args=(config,), daemon=True)
            t.start()

            self._json_response(202, json.dumps({
                "status": "started",
                "config": config,
            }).encode())
        else:
            self.send_response(404)
            self.end_headers()

    def do_OPTIONS(self):
        self.send_response(200)
        self._cors()
        self.end_headers()

    def _json_response(self, code, body):
        self.send_response(code)
        self.send_header("Content-Type", "application/json")
        self._cors()
        self.end_headers()
        self.wfile.write(body)

    def _cors(self):
        self.send_header("Access-Control-Allow-Origin", "*")
        self.send_header("Access-Control-Allow-Methods", "GET, POST, OPTIONS")
        self.send_header("Access-Control-Allow-Headers", "Content-Type")

    def log_message(self, fmt, *args):
        pass


def main():
    # Write HOSTS file
    os.makedirs("Player-Data", exist_ok=True)
    hosts = [
        os.environ.get("PARTY0_HOST", "party0-mpspdz"),
        os.environ.get("PARTY1_HOST", "party1-mpspdz"),
    ]
    with open("Player-Data/HOSTS", "w") as f:
        for h in hosts:
            f.write(h + "\n")

    # Start HTTP API and wait
    server = HTTPServer(("0.0.0.0", API_PORT), Handler)
    print(f"[mpspdz] Party {PARTY_ID} API on http://0.0.0.0:{API_PORT}", flush=True)
    print(f"[mpspdz] Waiting for POST /api/run to start preprocessing...", flush=True)
    server.serve_forever()


if __name__ == "__main__":
    main()
