"""Les numéros d'entrée d'un fichier VBG de l'ISO (lecture seule), regroupés
par milliers : pour savoir où sont les voix (SE.VBG : 1 à 999 le prologue,
10000 à 67999 les chapitres et les mini-jeux, 90000 les bruitages en flux),
les musiques (BGM.VBG : n × 10)… Avec un numéro, sa place et sa taille.
Usage : python -I vbg_list.py image.iso UNION/SE.VBG [numéro]"""
import collections
import os
import struct
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
from iso_list import SECTOR, files  # noqa: E402

iso, path = sys.argv[1], sys.argv[2]
with open(iso, "rb") as f:
    found = [lba for lba, _, p in files(f) if p.lower() == "/" + path.lower().lstrip("/")]
    if not found:
        sys.exit(f"{path} est introuvable.")
    f.seek(found[0] * SECTOR)
    head = f.read(SECTOR)
    if head[:3] != b"VBG":
        sys.exit(f"{path} n'est pas un fichier VBG.")
    count, dir_sectors = struct.unpack_from("<I", head, 8)[0], struct.unpack_from("<I", head, 0x10)[0]
    directory = f.read(dir_sectors * SECTOR)
entries = [struct.unpack_from("<IIII", directory, 16 * k) for k in range(count)]
if len(sys.argv) > 3:
    wanted = int(sys.argv[3])
    for n, start, sectors, size in entries:
        if n == wanted:
            print(f"{n} : secteur {start} (après le répertoire), {sectors} secteurs, {size} octets")
    sys.exit()
groups = collections.Counter(n // 1000 for n, _, _, _ in entries)
print(f"{count} entrées")
for k in sorted(groups):
    numbers = [n for n, _, _, _ in entries if n // 1000 == k]
    print(f"{k * 1000:6d}… : {groups[k]:4d} entrées ({min(numbers)} à {max(numbers)})")
