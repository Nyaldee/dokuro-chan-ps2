"""La table des instructions des scripts STCM2 (0x35bf60 dans l'exécutable,
500 entrées, lue par STCM2_Run) : pour chaque numéro d'instruction qui a une
fonction, son adresse et son nom (symbols.txt). Les numéros sans fonction
sont faits par STCM2_Run lui-même (sauts, calculs…). Lecture seule.
Usage : python -I stcm2_table.py SLPM_661.86 symbols.txt"""
import struct
import sys

# L'exécutable est chargé en 0x00100000, au décalage 0x80 du fichier.
LOAD = 0x00100000 - 0x80
TABLE = 0x0035BF60
COUNT = 500

sys.stdout.reconfigure(encoding="utf-8")
exe = open(sys.argv[1], "rb").read()
names = {}
for line in open(sys.argv[2], encoding="utf-8"):
    part = line.split()
    if len(part) >= 3 and part[0] == "func":
        names.setdefault(int(part[2], 16), part[1])

for k in range(COUNT):
    w = struct.unpack_from("<I", exe, TABLE - LOAD + 4 * k)[0]
    if w:
        print(f"0x{k:03x} {w:08x} {names.get(w, '?')}")
