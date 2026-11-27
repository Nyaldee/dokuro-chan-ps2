"""Sort les entrées d'un paquet UNI2 de l'ISO, telles quelles, dans un
dossier : une entrée par fichier (« 012.bin », à sa taille exacte, le
quatrième mot du répertoire). Lecture seule de l'ISO.
Usage : python -I uni_extract.py image.iso UNION/SCRIPT.UNI dossier"""
import os
import struct
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
from iso_list import SECTOR, files  # noqa: E402

iso, path, out = sys.argv[1], sys.argv[2], sys.argv[3]
os.makedirs(out, exist_ok=True)
with open(iso, "rb") as f:
    found = [lba for lba, _, p in files(f) if p.lower() == "/" + path.lower().lstrip("/")]
    if not found:
        sys.exit(f"{path} est introuvable.")
    lba = found[0]
    f.seek(lba * SECTOR)
    head = f.read(SECTOR)
    # Le mot en 0x10 : les secteurs d'en-tête (en-tête et répertoire, 2 ou 3).
    sectors = struct.unpack_from("<I", head, 0x10)[0]
    f.seek(lba * SECTOR)
    head = f.read(SECTOR * sectors)
    if head[:4] != b"UNI2":
        sys.exit(f"{path} n'est pas un paquet UNI2.")
    count = struct.unpack_from("<I", head, 8)[0]
    for k in range(count):
        index, start, _, size = struct.unpack_from("<IIII", head, SECTOR + 16 * k)
        f.seek((lba + sectors + start) * SECTOR)
        with open(os.path.join(out, f"{index:03d}.bin"), "wb") as o:
            o.write(f.read(size))
print(f"{count} entrées dans {out}")
