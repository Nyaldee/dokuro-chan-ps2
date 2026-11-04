"""Lecture seule : mots d'instruction MIPS de l'exécutable à des adresses
données (segment chargé en 0x00100000, au décalage 0x80 du fichier), avec
leur code d'opération : pour voir ce que Ghidra ne sait pas lire.

Usage : python -I peek_words.py <exécutable> <adresse hex> [<adresse hex>…]
"""

import struct
import sys

LOAD, OFFSET = 0x00100000, 0x80
# Codes d'opération propres au R5900 (EE de la PS2).
R5900 = {0x1C: "MMI", 0x1E: "lq", 0x1F: "sq"}

data = open(sys.argv[1], "rb").read()
for arg in sys.argv[2:]:
    at = int(arg, 16)
    word, = struct.unpack_from("<I", data, at - LOAD + OFFSET)
    op = word >> 26
    extra = ""
    if op == 0x11:
        extra = f" COP1 fmt {(word >> 21) & 0x1F:#x} fonction {word & 0x3F:#x}"
    print(f"{at:08x} : {word:08x} op {op:#04x} {R5900.get(op, '')}{extra}")
