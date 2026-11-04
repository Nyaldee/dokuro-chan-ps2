"""Données de l'exécutable (lecture seule ; segment chargé en 0x00100000,
au décalage 0x80 du fichier) : une table à une adresse, lue en mots de
16 bits signés (« h ») ou de 32 bits (« i »), par lignes.
Usage : python -I exe_table.py exe adresse-hexa nombre-de-lignes
        valeurs-par-ligne [h|i]"""
import struct
import sys

LOAD, OFFSET = 0x00100000, 0x80

data = open(sys.argv[1], "rb").read()
at, rows, per = int(sys.argv[2], 16), int(sys.argv[3]), int(sys.argv[4])
kind = sys.argv[5] if len(sys.argv) > 5 else "h"
size = 2 if kind == "h" else 4
for r in range(rows):
    a = at + r * per * size
    vals = struct.unpack_from("<" + kind * per, data, a - LOAD + OFFSET)
    print(f"{a:08x} : " + " ".join(f"{v:6d}" for v in vals))
