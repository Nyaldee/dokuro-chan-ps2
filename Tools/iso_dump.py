"""Octets d'un fichier de l'ISO (lecture seule), en hexadécimal : à partir
d'un décalage, sur une longueur. Usage :
python -I iso_dump.py image.iso secteur décalage longueur"""
import sys

SECTOR = 2048

iso, lba, offset, length = sys.argv[1], int(sys.argv[2]), int(sys.argv[3], 0), int(sys.argv[4], 0)
with open(iso, "rb") as f:
    f.seek(lba * SECTOR + offset)
    data = f.read(length)
for i in range(0, len(data), 16):
    row = data[i:i + 16]
    text = "".join(chr(b) if 32 <= b < 127 else "." for b in row)
    print(f"{offset + i:08x}  {row.hex(' '):47}  {text}")
