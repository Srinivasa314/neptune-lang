#!/usr/bin/env python3
import subprocess
import sys
import shutil
import os
import platform

if shutil.which('clang++') is None:
    sys.exit(f'clang++ not found')
os.environ['CXX'] = 'clang++'
if platform.system()=='Windows':
    os.environ['AR'] = 'llvm-lib'
if platform.system()=='Windows':
    os.environ['RUSTFLAGS'] = '-Clinker=lld-link'
elif platform.system()=='Linux':
    os.environ['RUSTFLAGS']= '-Clink-arg=-fuse-ld=lld'
os.environ['CXXFLAGS'] = '-flto=thin'
subprocess.run('cargo build --release', shell=True)
