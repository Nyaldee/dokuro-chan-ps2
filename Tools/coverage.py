"""Bilan de couverture : les fonctions de l'exécutable (exportées par
ExportFunctions.java) que personne n'a encore lues ni citées. Une fonction
est « vue » si elle a été décompilée (en-tête « // nom @ adresse » dans
decompiled/ ou Tools/work/) ou si le portage la nomme (src/, docs/). Seules
comptent celles qu'on atteint depuis main ou depuis une table de fonctions,
par les appels connus de Ghidra. Les bibliothèques de Sony (sans nom, ou
hors du code du jeu) sont à part. Lecture seule.
Usage : python -I coverage.py functions.tsv racine_du_dépôt"""
import collections
import os
import re
import sys

tsv, root = sys.argv[1], sys.argv[2]
funcs = {}
for line in open(tsv, encoding='utf-8'):
    addr, name, size, callees, data = line.rstrip('\n').split('\t')
    funcs[addr] = (name, int(size), [c for c in callees.split(',') if c], data == '1')

# Ce qui a été décompilé, et les mots qu'emploie le portage.
seen = set()
words = set()
for folder, pattern in (('decompiled', '.c'), ('Tools/work', '.c'), ('src', '.rs'), ('docs', '.md')):
    for base, _, files in os.walk(os.path.join(root, folder)):
        for f in files:
            if not f.endswith(pattern):
                continue
            text = open(os.path.join(base, f), encoding='utf-8', errors='replace').read()
            seen.update(re.findall(r'^// \w+ @ ([0-9a-f]{8})', text, re.M))
            words.update(re.findall(r'[A-Za-z_]\w+', text))

# Le code du jeu : nommé (symboles de débogage) et hors des bibliothèques.
def library(addr, name):
    a = int(addr, 16)
    return name.startswith('FUN_') or 0x110000 <= a < 0x150000

# Atteintes depuis main et les tables de fonctions.
roots = [a for a, f in funcs.items() if f[0] == 'main' or f[3]]
reach, todo = set(), list(roots)
while todo:
    a = todo.pop()
    if a in reach or a not in funcs:
        continue
    reach.add(a)
    todo.extend(funcs[a][2])

callers = collections.Counter(c for f in funcs.values() for c in set(f[2]))
missing = [(a, f) for a, f in funcs.items()
           if a in reach and not library(a, f[0]) and a not in seen and f[0] not in words]
game = [a for a, f in funcs.items() if a in reach and not library(a, f[0])]
print(f'{len(funcs)} fonctions ; {len(game)} du jeu atteintes ; {len(missing)} jamais lues ni citées')
groups = collections.defaultdict(list)
for a, (name, size, _, data) in missing:
    prefix = re.match(r'_*[A-Za-z]*_?[A-Z]?[a-z]*', name).group(0)
    groups[prefix].append((a, name, size, callers[a], data))
for prefix, items in sorted(groups.items(), key=lambda g: -sum(i[2] for i in g[1])):
    total = sum(i[2] for i in items)
    print(f'\n{prefix} : {len(items)} fonctions, {total} octets')
    for a, name, size, n, data in sorted(items):
        print(f'  {a}  {name:40} {size:6} octets  {n} appelants{"  (table)" if data else ""}')
