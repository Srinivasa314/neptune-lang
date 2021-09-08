#!/usr/bin/env python3
import subprocess
import sys
import shutil
import os

version = next(filter(lambda x: x.startswith('LLVM version'), subprocess.check_output(
    "rustc -vV", shell=True, text=True).split('\n'))).split(' ')[-1].split('.')[0]
clang = f'clang-{version}'
if shutil.which(clang) is None:
    sys.exit(f'{clang} not found')
os.environ['CXX'] = clang
os.environ['CXXFLAGS'] = '-flto=thin'
os.environ['RUSTFLAGS'] = f'-Clinker-plugin-lto -Clinker={clang} -Clink-arg=-fuse-ld=lld'
subprocess.run('cargo run --release', shell=True)

