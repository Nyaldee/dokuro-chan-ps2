"""Exporte en PNG les images ART2 d'un paquet UNI2 de l'ISO (lecture seule
de l'ISO) : en-tête de 0x20 octets (« ART2 », bits par pixel, largeur,
hauteur, nom), pixels indexés, palette RGBA (alpha PS2 : 0x80 = opaque),
rangée comme dans la mémoire de la PS2 (CSM1 : dans chaque bloc de 32
couleurs, les huit du 2e groupe et du 3e sont échangées).
Usage : python -I uni_png.py image.iso secteur-du-paquet dossier-de-sortie"""
import os
import struct
import sys

from PIL import Image

SECTOR = 2048


def csm1(i):
    """Place réelle de la couleur i dans une palette PS2 de 256 couleurs."""
    return i ^ 0x18 if (i & 0x18) in (0x08, 0x10) else i


def art2_name(data):
    """Le nom d'une image ART2, et sa part ASCII pour un nom de fichier (les
    noms sont en Shift-JIS)."""
    name = data[16:32].split(b"\0")[0].decode("cp932", "replace")
    return name, "".join(c for c in name if c.isascii() and (c.isalnum() or c in "-_")) or "img"


def art2_image(data):
    """Une image ART2 en RGBA ; ValueError si elle n'est pas en 8 bits."""
    bpp, w, h = struct.unpack_from("<III", data, 4)
    if bpp != 8:
        raise ValueError(f"{bpp} bits par pixel, non géré")
    pixels = data[0x20:0x20 + w * h]
    pal = data[0x20 + w * h:0x20 + w * h + 1024]
    colors = []
    for i in range(256):
        r, g, b, a = pal[4 * csm1(i):4 * csm1(i) + 4]
        colors.append((r, g, b, min(255, a * 2)))
    img = Image.new("RGBA", (w, h))
    img.putdata([colors[p] for p in pixels])
    return img


def entries(f, lba):
    """Les entrées d'un paquet UNI2 : numéro, données (à leur taille en
    secteurs)."""
    f.seek(lba * SECTOR)
    head = f.read(SECTOR)
    # Le mot en 0x10 : les secteurs d'en-tête (en-tête et répertoire, 2 ou 3).
    sectors = struct.unpack_from("<I", head, 0x10)[0]
    f.seek(lba * SECTOR)
    head = f.read(SECTOR * sectors)
    if head[:4] != b"UNI2":
        return
    count = struct.unpack_from("<I", head, 8)[0]
    for k in range(count):
        index, start, size, _ = struct.unpack_from("<IIII", head, SECTOR + 16 * k)
        f.seek((lba + sectors + start) * SECTOR)
        yield index, f.read(size * SECTOR)


if __name__ == "__main__":
    sys.stdout.reconfigure(encoding="utf-8")
    iso, lba, out = sys.argv[1], int(sys.argv[2]), sys.argv[3]
    os.makedirs(out, exist_ok=True)
    with open(iso, "rb") as f:
        for index, data in entries(f, lba):
            if data[:4] != b"ART2":
                print(f"{index} : pas une image ({data[:4]})")
                continue
            name, safe = art2_name(data)
            try:
                img = art2_image(data)
            except ValueError as e:
                print(f"{index} {name} : {e}")
                continue
            img.save(os.path.join(out, f"{index:03d}_{safe}.png"))
            print(f"{index} {name} : {img.width} x {img.height}")
