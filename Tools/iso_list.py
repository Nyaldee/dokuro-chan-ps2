"""Liste les fichiers d'une image ISO 9660 (lecture seule) : secteur de
début, taille, chemin. Usage : python -I iso_list.py image.iso"""
import struct
import sys

SECTOR = 2048


def entries(f, lba, size, path):
    f.seek(lba * SECTOR)
    data = f.read(size)
    pos = 0
    while pos < len(data):
        n = data[pos]
        if n == 0:
            # Fin des entrées de ce secteur : le suivant.
            pos = (pos // SECTOR + 1) * SECTOR
            continue
        rec = data[pos:pos + n]
        start, length = struct.unpack_from("<I", rec, 2)[0], struct.unpack_from("<I", rec, 10)[0]
        flags, name_len = rec[25], rec[32]
        name = rec[33:33 + name_len]
        pos += n
        if name in (b"\x00", b"\x01"):
            continue
        name = name.decode("ascii", "replace").split(";")[0]
        full = f"{path}/{name}"
        if flags & 2:
            yield from entries(f, start, length, full)
        else:
            yield start, length, full


def files(f):
    """Les fichiers de l'image ouverte : (secteur, taille, chemin)."""
    f.seek(16 * SECTOR)
    pvd = f.read(SECTOR)
    root = pvd[156:156 + 34]
    lba, size = struct.unpack_from("<I", root, 2)[0], struct.unpack_from("<I", root, 10)[0]
    yield from entries(f, lba, size, "")


if __name__ == "__main__":
    with open(sys.argv[1], "rb") as f:
        for start, length, path in files(f):
            print(f"{start:8d} {length:11d} {path}")
