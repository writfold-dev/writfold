"""Exercise presentation keys through an actual terminal.

Every wait is on the child's output or its exit, never on a wall clock: the
first frame is awaited until it says `read-only`, and after `q` the terminal
is read until the child closes it, so a child working through queued keys
can never block on a full pty. With WATCH_PTY_BURST=1 every key is written
without waiting for the terminal to go quiet (the queued case); otherwise
keys are paced by 150 ms of quiet output.
"""
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

pace = 0.0 if os.environ.get("WATCH_PTY_BURST") == "1" else 0.15

def read_some(timeout):
    """Read what the terminal has; False once the child has closed it."""
    ready, _, _ = select.select([master], [], [], timeout)
    if not ready:
        return child.poll() is None
    try:
        chunk = os.read(master, 65536)
    except OSError as error:
        if error.errno == errno.EIO:
            return False
        raise
    if not chunk:
        return False
    seen.extend(chunk)
    return True

def receive(first=False):
    """The first frame: until it says read-only. Later: until quiet."""
    quiet = time.monotonic() + (0.15 if first else pace)
    while True:
        before = len(seen)
        if not read_some(0.05):
            if first and b"read-only" not in seen:
                raise RuntimeError("terminal closed before the first frame")
            return
        if len(seen) != before:
            quiet = time.monotonic() + (0.15 if first else pace)
        if time.monotonic() > quiet and (not first or b"read-only" in seen):
            return

def drain():
    """Read until the child closes the terminal, then reap it."""
    while read_some(1.0):
        pass
    return child.wait()

try:
    receive(True)
    for key in [b"l", b"j", b"k", b"n", b"N", b".", b"l", b"j", b"k",
                b"d", b"d", b"h", b"d", b"\x1b[C", b"\x1b[D", b"\r", b"\x1b", b"x", b"X",
                b"x", b"c", b"?", b"\x1b", b"r", b"r", b"r",
                b"m", b"j", b"l", b"d", b"d", b"h", b"2", b"3", b"4", b"a", b"v", b"v",
                b"5", b"n", b"N", b"1", b"?", b"\x1b", b"]", b"[", b"r", b"f", b"q"]:
        os.write(master, key)
        receive()
    # The child may still be drawing queued views when q arrives: keep
    # reading, so it never blocks on a full terminal, until it exits.
    assert drain() == 0
    plain = re.sub(rb"\x1b\[[0-?]*[ -/]*[@-~]", b"", seen)
    compact = b"".join(plain.split())
    assert b"nextsteptopickup,asrecorded" in compact, plain.decode("utf-8", "replace")
    assert b"dexactwordingccommandsggates" in compact, plain.decode("utf-8", "replace")
    assert b"watchneverrunsthem" in compact, plain.decode("utf-8", "replace")
    assert b"dlessggates" in compact, plain.decode("utf-8", "replace")
    assert b"lastcheckpoint:" in compact, plain.decode("utf-8", "replace")
    assert b"knowledgebase" in compact, plain.decode("utf-8", "replace")
    assert b"mruns" in compact and b"mkb" in compact, plain.decode("utf-8", "replace")
    assert b"\x1b[?1049l" in seen
finally:
    if child.poll() is None:
        child.kill()
        child.wait()
    os.close(master)
