"""
REST API server + in-memory state for MPC party/dealer state.

Endpoints:
  GET  /api/health            — Health check
  GET  /api/state             — Current state (steps, log, phase)
  POST /api/step              — Trigger the next step
  POST /api/configure         — Send configuration and reset state for a new computation
"""

from __future__ import annotations

import json
import threading
import time
from http.server import HTTPServer, BaseHTTPRequestHandler


class PartyState:
    """Thread-safe in-memory state container."""

    def __init__(self, party_id: int, role: str = "party"):
        self._lock = threading.Lock()
        self._party_id = party_id
        self._role = role
        self._started_at = time.time()
        self._step_requested = False
        self._config: dict | None = None
        self._config_event = threading.Event()

        # Session state
        self._phase = "idle"
        self._status = "Waiting for configuration..."
        self._current_step = 0
        self._total_steps = 0
        self._bytes_sent = 0
        self._bytes_received_offline = 0
        self._elapsed_ms = 0.0
        self._result: str | None = None
        self._steps: list[dict] = []
        self._log: list[dict] = []

    def update(self, **kwargs):
        with self._lock:
            for k, v in kwargs.items():
                attr = f"_{k}"
                if hasattr(self, attr):
                    setattr(self, attr, v)

    def log(self, msg: str, values: dict | None = None, level: str = "info"):
        with self._lock:
            t = round(time.time() - self._started_at, 3)
            entry: dict = {"time": t, "msg": msg, "level": level}
            if values:
                entry["values"] = _serialize_values(values)
            self._log.append(entry)
            print(f"[{t:.3f}s] [{level}] {msg}", flush=True)

    def set_steps(self, steps: list[dict]):
        with self._lock:
            self._steps = list(steps)
            self._total_steps = len(steps)
            self._current_step = 0

    def request_step(self):
        with self._lock:
            self._step_requested = True

    def wait_for_step(self, step_idx: int, timeout: float = 3600):
        deadline = time.time() + timeout
        while time.time() < deadline:
            with self._lock:
                if self._step_requested and self._current_step == step_idx:
                    self._step_requested = False
                    self._current_step = step_idx + 1
                    if step_idx < len(self._steps):
                        s = self._steps[step_idx]
                        self._phase = s.get("phase") or "running"
                        self._status = s["name"]
                    return True
            time.sleep(0.05)
        return False

    def reset(self):
        """Clear all state back to idle. Does NOT unblock wait_for_config."""
        with self._lock:
            self._started_at = time.time()
            self._step_requested = False
            self._phase = "idle"
            self._status = "Waiting for configuration..."
            self._current_step = 0
            self._total_steps = 0
            self._bytes_sent = 0
            self._bytes_received_offline = 0
            self._elapsed_ms = 0.0
            self._result = None
            self._steps = []
            self._log = []
            self._config = None
            self._config_event.clear()

    def set_config(self, config: dict):
        """Called by POST /api/configure — resets state and unblocks wait_for_config()."""
        self.reset()
        with self._lock:
            self._config = config
            self._config_event.set()

    def wait_for_config(self, timeout: float = 3600) -> dict:
        """Block until POST /api/configure is called. Returns the config dict."""
        self._config_event.wait(timeout=timeout)
        # Reset the event so the next wait_for_config blocks again
        self._config_event.clear()
        with self._lock:
            return self._config

    def snapshot(self) -> dict:
        with self._lock:
            return {
                "party_id": self._party_id,
                "role": self._role,
                "phase": self._phase,
                "current_step": self._current_step,
                "total_steps": self._total_steps,
                "steps": [
                    {"idx": i, "name": s["name"], "phase": s.get("phase", ""), "description": s.get("description", "")}
                    for i, s in enumerate(self._steps)
                ],
                "status": self._status,
                "log": list(self._log),
                "bytes_sent": self._bytes_sent,
                "bytes_received_offline": self._bytes_received_offline,
                "elapsed_ms": self._elapsed_ms,
                "result": self._result,
                "configured": self._config is not None,
                "step_requested": self._step_requested,
            }


def _serialize_values(values: dict) -> dict:
    import numpy as np
    out = {}
    for k, v in values.items():
        if isinstance(v, np.ndarray):
            flat = v.ravel()
            if len(flat) <= 16:
                out[k] = [_int_hex(int(x)) for x in flat]
            else:
                out[k] = {
                    "type": "array", "dtype": str(v.dtype),
                    "shape": list(v.shape) if v.ndim > 1 else [len(flat)],
                    "size": len(flat),
                    "first_8": [_int_hex(int(x)) for x in flat[:8]],
                    "last_4": [_int_hex(int(x)) for x in flat[-4:]],
                }
        elif isinstance(v, list):
            if len(v) <= 16:
                out[k] = [_int_hex(x) if isinstance(x, int) else str(x) for x in v]
            else:
                out[k] = {
                    "type": "array", "size": len(v),
                    "first_8": [_int_hex(x) if isinstance(x, int) else str(x) for x in v[:8]],
                    "last_4": [_int_hex(x) if isinstance(x, int) else str(x) for x in v[-4:]],
                }
        elif isinstance(v, (int,)):
            out[k] = _int_hex(v)
        elif isinstance(v, float):
            out[k] = round(v, 4)
        elif isinstance(v, bytes):
            out[k] = v[:32].hex() + (f"... ({len(v)}B)" if len(v) > 32 else "")
        else:
            try:
                if isinstance(v, np.integer):
                    out[k] = _int_hex(int(v))
                    continue
            except Exception:
                pass
            out[k] = str(v)
    return out


def _int_hex(v: int) -> str:
    return str(v)


def _make_handler(state: PartyState):
    class Handler(BaseHTTPRequestHandler):
        def do_GET(self):
            if self.path == "/api/state" or self.path == "/api/sessions/current":
                self._json(state.snapshot())
            elif self.path == "/api/health":
                self._json({"ok": True})
            else:
                self._respond(404, b"Not found")

        def do_POST(self):
            if self.path == "/api/step":
                state.request_step()
                self._json({"ok": True})
            elif self.path == "/api/configure":
                length = int(self.headers.get("Content-Length", 0))
                body = self.rfile.read(length)
                config = json.loads(body)
                state.set_config(config)
                self._json({"ok": True})
            elif self.path == "/api/reset":
                state.reset()
                self._json({"ok": True})
            else:
                self._respond(404, b"Not found")

        def do_OPTIONS(self):
            self.send_response(200)
            self._cors()
            self.end_headers()

        def _json(self, obj):
            body = json.dumps(obj).encode()
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self._cors()
            self.end_headers()
            self.wfile.write(body)

        def _respond(self, code, body):
            self.send_response(code)
            self._cors()
            self.end_headers()
            self.wfile.write(body)

        def _cors(self):
            self.send_header("Access-Control-Allow-Origin", "*")
            self.send_header("Access-Control-Allow-Methods", "GET, POST, OPTIONS")
            self.send_header("Access-Control-Allow-Headers", "Content-Type")

        def log_message(self, fmt, *args):
            pass

    return Handler


def start_api_server(state: PartyState, port: int = 8080) -> HTTPServer:
    handler = _make_handler(state)
    server = HTTPServer(("0.0.0.0", port), handler)
    t = threading.Thread(target=server.serve_forever, daemon=True)
    t.start()
    print(f"API server on http://0.0.0.0:{port}", flush=True)
    return server
