from matplotlib import pyplot as plt
import time
import subprocess

commands = {'Python': 'python btree.py', 'Neptune': 'neptune-cli btree.np',
            'Wren': 'wren_cli btree.wren', 'NodeJS(JIT off)': 'node --jitless btree.js',
            'Ruby': 'ruby btree.rb',  'Lua': 'lua btree.lua'}
results = []
for command in commands.values():
    start = time.time()
    subprocess.run(command, shell=True)
    results.append(time.time()-start)

fig, ax = plt.subplots()
ax.barh(list(commands.keys()), results)
plt.tight_layout()
ax.set_title('Btree benchmark')
plt.savefig('results.png')
