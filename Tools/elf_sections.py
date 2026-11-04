"""Lecture seule : sections d'un exécutable ELF 32 bits (nom, type, adresse,
taille), pour savoir où sont ses symboles et ses informations de débogage.

Usage : python -I elf_sections.py <exécutable>
"""

import struct
import sys

data = open(sys.argv[1], "rb").read()
shoff, = struct.unpack_from("<I", data, 0x20)
shentsize, shnum, shstrndx = struct.unpack_from("<HHH", data, 0x2E)
sections = [struct.unpack_from("<IIIIIIIIII", data, shoff + i * shentsize) for i in range(shnum)]
names_off = sections[shstrndx][4]


def name(off):
    end = data.index(b"\0", names_off + off)
    return data[names_off + off:end].decode("ascii", "replace")


for s in sections:
    print(f"{name(s[0]):<20} type {s[1]:#010x} adresse {s[3]:#010x} décalage {s[4]:#09x} taille {s[5]}")
