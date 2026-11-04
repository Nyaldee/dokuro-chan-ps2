"""Cherche dans un exécutable ELF (lecture seule) les mots de 32 bits égaux
aux adresses données (pointeurs : tables de fonctions, de données) et
affiche l'adresse où chacun est rangé, avec les mots voisins.
Usage : python -I find_word.py exe adresse-hexa…"""
import struct
import sys

data = open(sys.argv[1], "rb").read()
phoff, phentsize, phnum = struct.unpack_from("<I", data, 0x1C)[0], *struct.unpack_from("<HH", data, 0x2A)
segments = []
for i in range(phnum):
    _, offset, vaddr, _, filesz, _, _, _ = struct.unpack_from("<8I", data, phoff + i * phentsize)
    segments.append((offset, vaddr, filesz))


def vaddr_of(off):
    for o, v, n in segments:
        if o <= off < o + n:
            return v + off - o
    return None


for arg in sys.argv[2:]:
    target = struct.pack("<I", int(arg, 16))
    pos = data.find(target)
    while pos != -1:
        # Hors des segments chargés (informations de débogage) : ignoré.
        if pos % 4 == 0 and vaddr_of(pos) is not None:
            around = struct.unpack_from("<8I", data, max(0, pos - 12))
            print(f"{arg} à {vaddr_of(pos):08x} : " + " ".join(f"{w:08x}" for w in around))
        pos = data.find(target, pos + 1)
