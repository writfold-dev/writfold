"""Exercise presentation keys through an actual terminal, with bounded reads."""
import errno
import fcntl
import os
import pty
import select
import re
import struct
import subprocess
import sys
import termios
import time

master, slave = pty.openpty()
fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 40, 160, 0, 0))
child = subprocess.Popen([sys.argv[1], "watch", "--run", sys.argv[2]],
                         stdin=slave, stdout=slave, stderr=slave)
os.close(slave)
seen = bytearray()

def receive(first=False):
    deadline = time.monotonic() + 20
    quiet = time.monotonic() + 0.15
    while time.monotonic() < deadline:
        ready, _, _ = select.select([master], [], [], 0.05)
        if ready:
            try:
                chunk = os.read(master, 65536)
            except OSError as error:
                if error.errno == errno.EIO:
                    return
                raise
            if not chunk:
                return
            seen.extend(chunk)
            quiet = time.monotonic() + 0.15
        if time.monotonic() > quiet and (not first or b"read-only" in seen):
            return
    raise RuntimeError("terminal did not produce the expected frame")

try:
    receive(True)
    for key in [b"l", b"j", b"k", b"n", b"N", b".", b"l", b"j", b"k",
                b"h", b"\x1b[C", b"\x1b[D", b"\r", b"\x1b", b"x", b"X",
                b"x", b"c", b"?", b"\x1b", b"r", b"r", b"r", b"q"]:
        os.write(master, key)
        receive()
    assert child.wait(timeout=20) == 0
    receive()
    plain = re.sub(rb"\x1b\[[0-?]*[ -/]*[@-~]", b"", seen)
    compact = b"".join(plain.split())
    assert b"recordedposition" in compact, plain.decode("utf-8", "replace")
    assert b"fullreceipts" in compact, plain.decode("utf-8", "replace")
    assert b"\x1b[?1049l" in seen
finally:
    if child.poll() is None:
        child.kill()
        child.wait()
    os.close(master)
