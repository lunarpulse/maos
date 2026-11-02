#!/usr/bin/env python3
"""Capture independent pre-repair witnesses without overlapping Cargo processes."""
import pathlib
import subprocess
import sys
here=pathlib.Path(__file__).resolve().parent
base=['/home/lunarpulse/.cargo/bin/cargo','nextest','run','--locked','--manifest-path','/home/lunarpulse/dev_ws/Cargo.toml','--no-fail-fast','--test-threads','1']
for label,args in [('review-red-install',['-p','maos-cli','--test','spirit_install_20_1']),('review-red-boot',['-p','maos-bin','--test','installed_spirit_dispatch_20_1']),('review-red-release',['-p','xtask','--test','release_build_integrity_20_1a','--test','registry_exit_20_1'])]:
    subprocess.run([sys.executable,str(here/'capture.py'),label,*base,*args],check=False)
