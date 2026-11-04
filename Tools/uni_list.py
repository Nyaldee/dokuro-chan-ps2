"""Contenu d'un paquet UNI2 de l'ISO (lecture seule) : pour chaque entrée,
son numéro, sa place et sa taille (secteurs de 2048 octets depuis la fin
du répertoire), le mot de drapeaux, et le début de ses données.
Usage : python -I uni_list.py image.iso secteur-du-paquet"""
import struct
import sys

SECTOR = 2048

iso, lba = sys.argv[1], int(sys.argv[2])
with open(iso, "rb") as f:
    f.seek(lba * SECTOR)
    head = f.read(SECTOR)
    # Le mot en 0x10 : les secteurs d'en-tête (en-tête et répertoire, 2 ou 3).
    sectors = struct.unpack_from("<I", head, 0x10)[0]
    f.seek(lba * SECTOR)
    head = f.read(SECTOR * sectors)
    magic, _, count = head[:4], *struct.unpack_from("<HH", head, 4)[:1], struct.unpack_from("<I", head, 8)[0]
    print(f"{magic} {count} entrées ; en-tête {head[12:32].hex(' ')}")
    for i in range(count):
        index, start, size, flags = struct.unpack_from("<IIII", head, SECTOR + 16 * i)
        f.seek((lba + sectors + start) * SECTOR)
        data = f.read(48)
        text = "".join(chr(b) if 32 <= b < 127 else "." for b in data)
        print(f"{index:3d} début {start:6d} taille {size:5d} drapeaux {flags:08x}  {data[:16].hex(' ')}  {text}")
