"""Dev201b cleanup: only own scratch and old inactive incremental session dirs."""
import json
import os
from pathlib import Path
import shutil
import stat
import time

ROOT = Path('/home/lunarpulse/dev_ws')
EVID = ROOT / '_bmad-output/implementation-artifacts/20-1b-vetting-yank-policy-and-quarantine-evidence/landing-1/ac15/review-corrections'
SCRATCH = Path('/tmp/201b-ac15-corrections')
INCREMENTAL = (ROOT / 'target/debug/incremental').resolve()
started = time.time()
cutoff_ns = time.time_ns() - 2 * 60 * 60 * 1_000_000_000
report = {'started_unix': started, 'incremental_root': str(INCREMENTAL), 'scratch': str(SCRATCH), 'cutoff_ns': cutoff_ns, 'proc_scan': {'compiler_processes': [], 'incremental_references': [], 'permission_denied': []}}
compiler_names = {'cargo', 'rustc', 'rustdoc', 'xtask', 'cargo-clippy', 'clippy-driver', 'cargo-public-api', 'cargo-public-ap'}
references = set()
for process in os.scandir('/proc'):
    if not process.name.isdigit():
        continue
    pid = process.name
    directory = Path(process.path)
    try:
        argv = (directory / 'cmdline').read_bytes().split(b'\0')
        name = os.path.basename(os.fsdecode(argv[0])) if argv and argv[0] else ''
        comm = (directory / 'comm').read_text().strip()
        if name in compiler_names or comm in compiler_names:
            report['proc_scan']['compiler_processes'].append({'pid': int(pid), 'name': name, 'comm': comm, 'argv': [os.fsdecode(arg) for arg in argv if arg]})
    except FileNotFoundError:
        continue
    except PermissionError:
        report['proc_scan']['permission_denied'].append(f'{pid}/cmdline-or-comm')
    links = []
    for key in ('cwd', 'exe'):
        try:
            links.append(os.readlink(directory / key))
        except FileNotFoundError:
            pass
        except PermissionError:
            report['proc_scan']['permission_denied'].append(f'{pid}/{key}')
    try:
        with os.scandir(directory / 'fd') as descriptors:
            for descriptor in descriptors:
                try:
                    links.append(os.readlink(descriptor.path))
                except FileNotFoundError:
                    pass
                except PermissionError:
                    report['proc_scan']['permission_denied'].append(f'{pid}/fd/{descriptor.name}')
    except FileNotFoundError:
        pass
    except PermissionError:
        report['proc_scan']['permission_denied'].append(f'{pid}/fd')
    try:
        for line in (directory / 'maps').read_text().splitlines():
            fields = line.split(None, 5)
            if len(fields) == 6:
                links.append(fields[5])
    except FileNotFoundError:
        pass
    except PermissionError:
        report['proc_scan']['permission_denied'].append(f'{pid}/maps')
    for link in links:
        if not link.startswith('/') or '/incremental/' not in link:
            continue
        normalized = str(Path(link.removesuffix(' (deleted)')).resolve(strict=False))
        if normalized.startswith(str(INCREMENTAL) + '/'):
            references.add(normalized)
            report['proc_scan']['incremental_references'].append({'pid': int(pid), 'path': normalized})
(EVID / 'cleanup-proc-scan.json').write_text(json.dumps(report['proc_scan'], indent=2) + '\n')
if report['proc_scan']['compiler_processes']:
    raise RuntimeError('Refusing cleanup while any visible compiler/xtask process remains')
if not INCREMENTAL.is_dir() or INCREMENTAL.name != 'incremental' or INCREMENTAL.parent.name != 'debug':
    raise RuntimeError('Unexpected incremental root; nothing removed')

def latest_in_tree(path):
    newest = path.lstat().st_mtime_ns
    for directory, dirs, files in os.walk(path, followlinks=False):
        for name in dirs + files:
            newest = max(newest, (Path(directory) / name).lstat().st_mtime_ns)
    return newest

sessions = []
for crate in os.scandir(INCREMENTAL):
    if not crate.is_dir(follow_symlinks=False):
        continue
    siblings = []
    for entry in os.scandir(crate.path):
        if not entry.name.startswith('s-') or not entry.is_dir(follow_symlinks=False):
            continue
        path = Path(entry.path)
        item = {'path': str(path), 'cache': crate.name, 'keep_reasons': []}
        try:
            item['latest_mtime_ns'] = latest_in_tree(path)
            if path.lstat().st_uid != os.geteuid():
                item['keep_reasons'].append('different-owner')
            if item['latest_mtime_ns'] >= cutoff_ns:
                item['keep_reasons'].append('touched-within-two-hours')
        except (OSError, PermissionError) as error:
            item['latest_mtime_ns'] = 0
            item['keep_reasons'].append('unreadable:' + type(error).__name__)
        if any(reference == str(path) or reference.startswith(str(path) + '/') or reference.startswith(str(path) + '.') for reference in references):
            item['keep_reasons'].append('open-process-reference')
        siblings.append(item)
    if siblings:
        newest = max(siblings, key=lambda item: item['latest_mtime_ns'])
        newest['keep_reasons'].append('newest-per-crate-cache-feature')
        sessions.extend(siblings)
selected = [Path(item['path']) for item in sessions if not item['keep_reasons']]
for path in selected:
    if path.parent.parent != INCREMENTAL or path.is_symlink() or not path.name.startswith('s-'):
        raise RuntimeError('Candidate escaped the allowed session shape')
    if latest_in_tree(path) >= cutoff_ns:
        raise RuntimeError('Candidate became recent; refusing all deletion')
report['sessions'] = sessions
report['selected_session_count'] = len(selected)
report['retained_session_count'] = len(sessions) - len(selected)
if SCRATCH.is_symlink() or SCRATCH.parent != Path('/tmp') or SCRATCH.name != '201b-ac15-corrections':
    raise RuntimeError('Unexpected own scratch path')
removals = selected + ([SCRATCH] if SCRATCH.exists() else [])

# Count inode allocations, not each hard-linked pathname as separately freed.
inodes = {}
for root in removals:
    for directory, dirs, files in os.walk(root, followlinks=False):
        for path in [Path(directory)] + [Path(directory) / name for name in dirs + files if not (Path(directory) / name).is_dir() or (Path(directory) / name).is_symlink()]:
            info = path.lstat()
            key = (info.st_dev, info.st_ino)
            item = inodes.setdefault(key, {'selected_links': 0, 'nlink': info.st_nlink, 'size': info.st_size, 'blocks': info.st_blocks, 'directory': stat.S_ISDIR(info.st_mode)})
            item['selected_links'] += 1
report['unique_removed_inode_logical_bytes'] = sum(item['size'] for item in inodes.values())
report['fully_unlinked_inode_allocated_bytes'] = sum(item['blocks'] * 512 for item in inodes.values() if item['directory'] or item['selected_links'] >= item['nlink'])
report['retained_hardlinked_inode_count'] = sum(not item['directory'] and item['selected_links'] < item['nlink'] for item in inodes.values())
filesystems = {}
for path in (INCREMENTAL, SCRATCH if SCRATCH.exists() else SCRATCH.parent):
    info = path.stat()
    if info.st_dev not in filesystems:
        available = os.statvfs(path)
        filesystems[info.st_dev] = {'probe': str(path), 'available_before_bytes': available.f_bavail * available.f_frsize}
report['filesystems'] = filesystems
(EVID / 'cleanup-plan.json').write_text(json.dumps(report, indent=2) + '\n')
report['removed_paths'] = []
for path in removals:
    shutil.rmtree(path)
    report['removed_paths'].append(str(path))
for filesystem in filesystems.values():
    probe = Path(filesystem['probe'])
    while not probe.exists():
        probe = probe.parent
    available = os.statvfs(probe)
    filesystem['available_after_bytes'] = available.f_bavail * available.f_frsize
    filesystem['observed_available_delta_bytes'] = filesystem['available_after_bytes'] - filesystem['available_before_bytes']
report['elapsed_seconds'] = round(time.time() - started, 3)
report['limitations'] = 'Filesystem deltas can include unrelated activity; inode block accounting is not an exclusive-extent/reflink measurement. Permission-denied proc links are recorded, not invented as inspected. No deps/build/fingerprint, lockfiles, newest session, recent session or Main cleanup report was removed.'
(EVID / 'cleanup.json').write_text(json.dumps(report, indent=2) + '\n')
print(json.dumps({key: report[key] for key in ['selected_session_count', 'retained_session_count', 'unique_removed_inode_logical_bytes', 'fully_unlinked_inode_allocated_bytes', 'retained_hardlinked_inode_count', 'filesystems', 'elapsed_seconds']}, indent=2))
