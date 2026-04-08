"""
TCP networking for two-party communication, matching the Rust Party struct.

Supports both localhost (original) and hostname-based connections (Docker).
Pass a ``hosts`` list to use Docker-style DNS names instead of 127.0.0.1.
"""

from __future__ import annotations

import socket
import threading
import time
from typing import Callable


RETRY_AFTER_SECONDS = 2
CHUNK_SIZE = 1 << 20  # 1 MB


# Optional event-hook callback type: (event_name, details_dict) -> None
EventCallback = Callable[[str, dict], None] | None


class Party:
    """Two-party TCP networking layer."""

    def __init__(
        self,
        my_id: int,
        num_parties: int,
        port_base: int,
        hosts: list[str] | None = None,
        on_event: EventCallback = None,
    ):
        self.my_id = my_id
        self.num_parties = num_parties
        self._bytes_sent = 0
        self._on_event = on_event

        # Default: all parties on localhost (original behaviour)
        if hosts is None:
            hosts = ["127.0.0.1"] * num_parties
        self._hosts = hosts

        self._send_streams: list[socket.socket | None] = [None] * num_parties
        self._recv_streams: list[socket.socket | None] = [None] * num_parties

        results: dict[str, dict[int, socket.socket]] = {"send": {}, "recv": {}}
        threads = []

        self._emit("connecting", {"my_id": my_id, "num_parties": num_parties})

        # Accept connections from other parties — listen on 0.0.0.0 so
        # remote containers can reach us.
        for from_id in range(num_parties):
            if from_id == my_id:
                continue
            port = self._which_port(port_base, from_id, my_id, num_parties)
            listener = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
            listener.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
            listener.bind(("0.0.0.0", port))
            listener.listen(1)

            def accept_fn(lst=listener, fid=from_id):
                conn, _ = lst.accept()
                conn.setsockopt(socket.IPPROTO_TCP, socket.TCP_NODELAY, 1)
                results["recv"][fid] = conn
                lst.close()

            t = threading.Thread(target=accept_fn)
            t.start()
            threads.append(t)

        # Connect to other parties (use hostname from hosts list)
        for to_id in range(num_parties):
            if to_id == my_id:
                continue
            port = self._which_port(port_base, my_id, to_id, num_parties)
            target_host = hosts[to_id]

            def connect_fn(host=target_host, p=port, tid=to_id):
                while True:
                    try:
                        s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
                        s.connect((host, p))
                        s.setsockopt(socket.IPPROTO_TCP, socket.TCP_NODELAY, 1)
                        results["send"][tid] = s
                        return
                    except (ConnectionRefusedError, OSError):
                        print(
                            f"Failed to connect to party {tid} at {host}:{p}, "
                            f"retry after {RETRY_AFTER_SECONDS} seconds..."
                        )
                        time.sleep(RETRY_AFTER_SECONDS)

            t = threading.Thread(target=connect_fn)
            t.start()
            threads.append(t)

        for t in threads:
            t.join()

        for pid, s in results["send"].items():
            self._send_streams[pid] = s
        for pid, s in results["recv"].items():
            self._recv_streams[pid] = s

        self._emit("connected", {"my_id": my_id})

    def _emit(self, event: str, details: dict):
        if self._on_event:
            self._on_event(event, details)

    @staticmethod
    def _which_port(port_base: int, from_id: int, to_id: int, num_parties: int) -> int:
        return port_base + from_id * num_parties + to_id

    @property
    def bytes_sent(self) -> int:
        return self._bytes_sent

    def add_bytes_sent(self, n: int):
        self._bytes_sent += n

    def send_bytes(self, to_id: int, data: bytes):
        s = self._send_streams[to_id]
        for i in range(0, len(data), CHUNK_SIZE):
            s.sendall(data[i:i + CHUNK_SIZE])
        self._bytes_sent += len(data)

    def recv_bytes(self, from_id: int, nbytes: int) -> bytes:
        s = self._recv_streams[from_id]
        buf = bytearray()
        while len(buf) < nbytes:
            chunk = s.recv(min(CHUNK_SIZE, nbytes - len(buf)))
            if not chunk:
                raise ConnectionError("Connection closed")
            buf.extend(chunk)
        return bytes(buf)

    def send_bytes_to_other(self, data: bytes):
        self.send_bytes(1 - self.my_id, data)

    def recv_bytes_from_other(self, nbytes: int) -> bytes:
        return self.recv_bytes(1 - self.my_id, nbytes)

    def send_recv_concurrent(self, other_id: int, send_data: bytes, recv_nbytes: int) -> bytes:
        """Send and receive concurrently using threads."""
        result = [None]

        def send_fn():
            self.send_bytes(other_id, send_data)

        def recv_fn():
            result[0] = self.recv_bytes(other_id, recv_nbytes)

        t_send = threading.Thread(target=send_fn)
        t_recv = threading.Thread(target=recv_fn)
        t_send.start()
        t_recv.start()
        t_send.join()
        t_recv.join()
        return result[0]
