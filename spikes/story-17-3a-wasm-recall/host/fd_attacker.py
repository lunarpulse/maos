#!/usr/bin/env python3
import ctypes
import errno
import json
import os
import sys

pid = int(sys.argv[1])
fd = int(sys.argv[2])
path = f"/proc/{pid}/fd/{fd}"
result = {"path": path}
for name, flags in (("read-only", os.O_RDONLY | os.O_NONBLOCK), ("path-only", os.O_PATH)):
    try:
        opened = os.open(path, flags)
    except OSError as error:
        result[f"proc_{name}"] = {"ok": False, "errno": error.errno, "error": error.strerror}
    else:
        os.close(opened)
        result[f"proc_{name}"] = {"ok": True}
try:
    pidfd = os.pidfd_open(pid)
except OSError as error:
    result["pidfd_open"] = {"ok": False, "errno": error.errno, "error": error.strerror}
else:
    libc = ctypes.CDLL(None, use_errno=True)
    duplicated = libc.syscall(438, pidfd, fd, 0)  # x86_64 SYS_pidfd_getfd
    if duplicated == -1:
        error = ctypes.get_errno()
        result["pidfd_getfd"] = {"ok": False, "errno": error, "error": os.strerror(error)}
    else:
        os.close(duplicated)
        result["pidfd_getfd"] = {"ok": True}
    os.close(pidfd)
print(json.dumps(result, sort_keys=True))
