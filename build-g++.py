#!/usr/bin/env python3
import subprocess
import sys
import shutil
import os

if shutil.which('g++') is None:
    sys.exit(f'g++ not found')
os.environ['CXX'] = 'g++'
os.environ['CXXFLAGS'] = '-flto'
os.environ['RUSTFLAGS'] = "-C linker=gcc"
subprocess.run('cargo build --release', shell=True)
