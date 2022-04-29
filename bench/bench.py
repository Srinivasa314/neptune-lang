
import subprocess
from matplotlib import pyplot as plt

benchmarks = ['btree', 'collatz', 'fib', 'nbody',
              'nested', 'selection-sort', 'sieve', 'spect']

languages = {'Neptune': ['neptune-cli', '.np'],
             'NodeJS(JIT off)': ['node', '-jitless', '.js'],
             'Ruby': ['ruby', '.rb'],  'Lua': ['lua', '.lua'], 'NodeJS(JIT)': ['node', '.js'], 'LuaJIT(JIT off)': ['luajit', '-joff', '.lua'], 'LuaJIT(JIT)': ['luajit', '.lua']}

results = {}
for benchmark in benchmarks:
    results[benchmark] = {}

results['Geometric Mean'] = {}
for lang in languages:
    results['Geometric Mean'][lang] = 1.0

for benchmark in benchmarks:
    for lang in languages:
        results[benchmark][lang] = float(subprocess.run(languages[lang][:-1]+[benchmark + languages[lang][-1]],
                                                        stdout=subprocess.PIPE, stderr=subprocess.DEVNULL).stdout.decode('utf-8').split('\n')[-2])
        results['Geometric Mean'][lang] *= results[benchmark][lang]

for lang in languages:
    results['Geometric Mean'][lang] **= (1/len(benchmarks))
    print(lang, results['Geometric Mean'][lang])

benchmarks.append('Geometric Mean')
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
