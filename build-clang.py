#!/usr/bin/env python3
import subprocess
import sys
import shutil
import os

if shutil.which('clang++') is None:
    sys.exit(f'clang++ not found')
os.environ['CXX'] = 'clang++'
os.environ['AR'] = 'llvm-lib'
os.environ['RUSTFLAGS'] = "-C linker=lld-link"
os.environ['CXXFLAGS'] = '-flto=thin'
subprocess.run('cargo build --release', shell=True)
