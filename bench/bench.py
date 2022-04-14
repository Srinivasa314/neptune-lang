
import subprocess
from matplotlib import pyplot as plt

benchmarks = ['ack', 'btree', 'collatz', 'fib', 'nbody',
              'nested', 'selection-sort', 'sieve', 'spect']

languages = {'Neptune': ['../neptune-cli/target/release/neptune-cli', '.np'],
             'NodeJS(JIT off)': ['node', '-jitless', '.js'],
             'Ruby': ['ruby', '.rb'],  'Lua': ['lua', '.lua'], 'Node(JIT)': ['node', '.js'], }

results = {}
for benchmark in benchmarks:
    results[benchmark] = {}

for benchmark in benchmarks:
    for lang in languages:
        results[benchmark][lang] = float(subprocess.run(languages[lang][:-1]+[benchmark + languages[lang][-1]],
                                                        stdout=subprocess.PIPE, stderr=subprocess.DEVNULL).stdout.decode('utf-8').split('\n')[-2])

for benchmark in benchmarks:
    fig, ax = plt.subplots()
    langs = list(languages.keys())
    langs.sort(key=lambda l: results[benchmark][l])
    ax.barh(langs, [results[benchmark][l]for l in langs])
    plt.tight_layout()
    ax.set_title(benchmark)
    plt.xlabel('Time in milliseconds')
    plt.savefig(f'{benchmark}.png', bbox_inches="tight")
    pass
