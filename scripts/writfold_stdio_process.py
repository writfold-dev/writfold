"""Bounded, retained pipe executions for the control-only stdio qualification."""
from __future__ import annotations

import os
from pathlib import Path
import selectors
import signal
import subprocess
import time
from typing import Callable

from writfold_conformance_process import record, write_json


class Pipes:
    def __init__(self, directory: Path):
        self.directory = directory
        self.records = []

    def run(self, name: str, argv: list[str], actions: list[tuple[str, object]], *,
            cwd: Path, env: dict[str, str], timeout: int = 15,
            limit: int = 2 * 1024 * 1024,
            ready: Callable[[bytes], None] | None = None) -> tuple[bytes, bytes, int]:
        prefix = self.directory / f"{len(self.records):03d}-{name}"
        paths = {key: Path(f"{prefix}.{key}") for key in ("stdin", "stdout", "stderr")}
        planned = b"".join(value for op, value in actions if op == "write")
        if len(planned) > limit:
            raise ValueError("Input exceeds execution budget")
        paths["stdin"].write_bytes(planned)
        entry = dict(name=name, argv=argv, cwd=str(cwd), environment=env,
                     start_ns=time.time_ns(), timeout_seconds=timeout, byte_limit=limit,
                     actions=[(op, len(value) if op == "write" else value) for op, value in actions],
                     exit_code=None, timed_out=False, input_bytes_written=0, events=[])
        self.records.append(entry)
        start = time.monotonic_ns()
        child = None
        buffers = {"stdout": bytearray(), "stderr": bytearray()}
        selector = selectors.DefaultSelector()
        try:
            child = subprocess.Popen(argv, cwd=cwd, env=env, stdin=subprocess.PIPE,
                                     stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                                     start_new_session=True)
            for key in ("stdin", "stdout", "stderr"):
                os.set_blocking(getattr(child, key).fileno(), False)
            for key in buffers:
                selector.register(getattr(child, key), selectors.EVENT_READ, key)
            index, pending = 0, b""
            deadline = time.monotonic() + timeout
            while True:
                if time.monotonic() >= deadline:
                    entry["timed_out"] = True
                    raise TimeoutError(name)
                while not pending and index < len(actions):
                    op, value = actions[index]
                    if op == "ready":
                        if buffers["stdout"].count(b"\n") < value:
                            break
                        if ready is None:
                            raise ValueError("Readiness requires independent response validation")
                        ready(bytes(buffers["stdout"]))
                    elif op == "write":
                        pending = value
                        if pending:
                            selector.register(child.stdin, selectors.EVENT_WRITE, "stdin")
                    elif op == "eof":
                        child.stdin.close()
                    elif op == "close-output":
                        selector.unregister(child.stdout)
                        child.stdout.close()
                    elif op == "signal":
                        os.killpg(child.pid, value)
                    else:
                        raise ValueError(f"Unknown pipe action {op}")
                    entry["events"].append(dict(action=index, elapsed_ns=time.monotonic_ns() - start))
                    index += 1
                if child.poll() is not None and not selector.get_map():
                    if index != len(actions) or pending:
                        raise ValueError("Process exited before all actions were exercised")
                    break
                for key, _ in selector.select(min(0.1, max(0, deadline - time.monotonic()))):
                    if key.data == "stdin":
                        try:
                            count = os.write(key.fd, pending[:4096])
                        except BrokenPipeError:
                            raise ValueError("Process stopped before planned input was delivered") from None
                        pending = pending[count:]
                        entry["input_bytes_written"] += count
                        if not pending:
                            selector.unregister(child.stdin)
                    else:
                        data = os.read(key.fd, 65536)
                        if not data:
                            selector.unregister(key.fileobj)
                            key.fileobj.close()
                        else:
                            remaining = limit - len(buffers[key.data])
                            buffers[key.data].extend(data[:remaining])
                            if len(data) > remaining:
                                entry["output_limit_exceeded"] = key.data
                                raise ValueError(f"{key.data} exceeded bounded capture")
            entry["exit_code"] = child.wait()
            return bytes(buffers["stdout"]), bytes(buffers["stderr"]), child.returncode
        except BaseException as exc:
            entry["error"] = f"{type(exc).__name__}: {exc}"
            if child is not None:
                try:
                    os.killpg(child.pid, signal.SIGKILL)
                except ProcessLookupError:
                    pass
            raise
        finally:
            if child is not None:
                if child.poll() is None:
                    try:
                        os.killpg(child.pid, signal.SIGKILL)
                    except ProcessLookupError:
                        pass
                entry["exit_code"] = child.wait()
                for stream in (child.stdin, child.stdout, child.stderr):
                    if not stream.closed:
                        stream.close()
            selector.close()
            for key, data in buffers.items():
                paths[key].write_bytes(data)
            entry.update(end_ns=time.time_ns(), duration_ns=time.monotonic_ns() - start,
                         artifacts={key: record(path) for key, path in paths.items()})
            write_json(Path(f"{prefix}.command.json"), entry)
