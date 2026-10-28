# usage: artifact_member.py ARTIFACT_ID MEMBER — print one member of a run artifact zip via HTTP Range reads (no full download)
import io, os, sys, subprocess, urllib.request, zipfile
aid, member = sys.argv[1], sys.argv[2]
tok = open(os.path.expanduser("~/.config/maos-agent/gh-ci-read.token")).read().strip()
hdrs = subprocess.run(["curl", "--http1.1", "-sS", "-o", "/dev/null", "-D", "-", "-H", f"Authorization: Bearer {tok}",
                       f"https://api.github.com/repos/lunarpulse/maos/actions/artifacts/{aid}/zip"], capture_output=True, text=True, check=True).stdout
url = next(l.split(":", 1)[1].strip() for l in hdrs.splitlines() if l.lower().startswith("location:"))

class RangeFile(io.RawIOBase):
    def __init__(self, url):
        self.url, self.pos = url, 0
        with urllib.request.urlopen(urllib.request.Request(url, headers={"Range": "bytes=0-0"})) as r:
            self.size = int(r.headers["Content-Range"].rsplit("/", 1)[1])
    def seekable(self): return True
    def readable(self): return True
    def tell(self): return self.pos
    def seek(self, off, whence=0):
        self.pos = {0: off, 1: self.pos + off, 2: self.size + off}[whence]
        return self.pos
    def readinto(self, b):
        n = min(len(b), self.size - self.pos)
        if n <= 0: return 0
        with urllib.request.urlopen(urllib.request.Request(self.url, headers={"Range": f"bytes={self.pos}-{self.pos + n - 1}"})) as r:
            data = r.read()
        b[:len(data)] = data
        self.pos += len(data)
        return len(data)

with zipfile.ZipFile(io.BufferedReader(RangeFile(url), 1 << 16)) as z:
    print("members:", sorted(z.namelist()), file=sys.stderr)
    sys.stdout.write(z.read(member).decode())
