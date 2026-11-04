"""Lecture seule : noms de fonctions (symboles C++, « Mini_BatProc__Fv ») et
fichiers sources (« \\sources\\minigames\\mini_bat.cpp ») cités dans
l'exécutable du jeu (SLPM_661.86), filtrés par un motif ; rien n'est exécuté.

Usage : python -I peek_symbols.py <exécutable> <motif (expression régulière)>
"""

import re
import sys

data = open(sys.argv[1], "rb").read()
pattern = re.compile(sys.argv[2].encode(), re.I)
names = sorted({s.decode("ascii") for s in re.findall(rb"[A-Za-z_][A-Za-z0-9_]{3,}(?:__[A-Za-z0-9_]*)?", data) if pattern.search(s)})
print(f"{len(names)} noms :")
for n in names:
    print(" ", n)
sources = sorted({s.decode("ascii") for s in re.findall(rb"\\sources\\[\x20-\x7e]+?\.(?:cpp|c|h)", data)})
print(f"{len(sources)} fichiers sources :")
for s in sources:
    print(" ", s)
