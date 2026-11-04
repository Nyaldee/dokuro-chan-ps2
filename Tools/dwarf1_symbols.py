"""Lecture seule : symboles de l'exécutable PS2 tirés de sa section .debug
(DWARF 1, que Ghidra ne lit pas) : fonctions (nom, début, fin) et variables
globales (nom, adresse), une par ligne :

    func <nom> <début hex> <fin hex>
    var <nom> <adresse hex>

DWARF 1 : une suite d'entrées (longueur sur 4 octets, étiquette sur 2, puis
des attributs : code sur 2 octets dont les 4 bits bas donnent la forme).

Usage : python -I dwarf1_symbols.py <exécutable> <fichier de sortie>
"""

import struct
import sys

TAG_GLOBAL_SUBROUTINE = 0x0006
TAG_SUBROUTINE = 0x0014
TAG_GLOBAL_VARIABLE = 0x0004
TAG_LOCAL_VARIABLE = 0x0005
AT_NAME = 0x0038
AT_LOW_PC = 0x0111
AT_HIGH_PC = 0x0121
AT_LOCATION = 0x0023
OP_ADDR = 0x03


def section(data, wanted):
    shoff, = struct.unpack_from("<I", data, 0x20)
    shentsize, shnum, shstrndx = struct.unpack_from("<HHH", data, 0x2E)
    heads = [struct.unpack_from("<IIIIIIIIII", data, shoff + i * shentsize) for i in range(shnum)]
    names = heads[shstrndx][4]
    for h in heads:
        end = data.index(b"\0", names + h[0])
        if data[names + h[0]:end] == wanted:
            return data[h[4]:h[4] + h[5]]
    raise SystemExit(f"section {wanted!r} absente")


def attributes(die):
    """Attributs d'une entrée : {code: valeur}."""
    out, i = {}, 0
    while i + 2 <= len(die):
        code, = struct.unpack_from("<H", die, i)
        i += 2
        form = code & 0xF
        if form in (1, 2, 6):  # adresse, référence, donnée 4 octets
            out[code] = struct.unpack_from("<I", die, i)[0]
            i += 4
        elif form == 5:
            out[code] = struct.unpack_from("<H", die, i)[0]
            i += 2
        elif form == 7:
            out[code] = struct.unpack_from("<Q", die, i)[0]
            i += 8
        elif form == 3:
            n, = struct.unpack_from("<H", die, i)
            out[code] = die[i + 2:i + 2 + n]
            i += 2 + n
        elif form == 4:
            n, = struct.unpack_from("<I", die, i)
            out[code] = die[i + 4:i + 4 + n]
            i += 4 + n
        elif form == 8:
            end = die.index(b"\0", i)
            out[code] = die[i:end].decode("latin-1")
            i = end + 1
        else:
            break
    return out


def main():
    data = open(sys.argv[1], "rb").read()
    debug = section(data, b".debug")
    lines = []
    pos = 0
    while pos + 4 <= len(debug):
        length, = struct.unpack_from("<I", debug, pos)
        if length < 8:
            pos += max(length, 4)
            continue
        tag, = struct.unpack_from("<H", debug, pos + 4)
        attrs = attributes(debug[pos + 6:pos + length])
        name = attrs.get(AT_NAME)
        if name and tag in (TAG_GLOBAL_SUBROUTINE, TAG_SUBROUTINE) and AT_LOW_PC in attrs:
            lines.append(f"func {name} {attrs[AT_LOW_PC]:08x} {attrs.get(AT_HIGH_PC, attrs[AT_LOW_PC]):08x}")
        elif name and tag in (TAG_GLOBAL_VARIABLE, TAG_LOCAL_VARIABLE):
            loc = attrs.get(AT_LOCATION)
            if isinstance(loc, bytes) and len(loc) == 5 and loc[0] == OP_ADDR:
                lines.append(f"var {name} {struct.unpack_from('<I', loc, 1)[0]:08x}")
        pos += length
    with open(sys.argv[2], "w", encoding="utf-8") as f:
        f.write("\n".join(lines) + "\n")
    funcs = sum(1 for l in lines if l.startswith("func"))
    print(f"{funcs} fonctions, {len(lines) - funcs} variables -> {sys.argv[2]}")


if __name__ == "__main__":
    main()
