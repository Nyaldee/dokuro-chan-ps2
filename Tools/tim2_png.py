"""Une image TIM2 (format de Sony), sortie d'un paquet par uni_extract.py,
en PNG (il faut Pillow). En-tête de 16 octets (« TIM2 », version,
alignement, nombre d'images), puis pour chaque image un en-tête de 48
octets (tailles totale, de la palette, des pixels ; taille de l'en-tête ;
nombre de couleurs ; sorte de palette ; sorte de pixels : 1 16 bits, 2 24
bits, 3 32 bits, 4 4 bits, 5 8 bits ; largeur, hauteur), les pixels, puis
la palette (RGBA, alpha PS2 : 0x80 = opaque). Une palette de 256 couleurs
sans le bit 0x80 de sa sorte est rangée en CSM1 : dans chaque bloc de 32
couleurs, les huit du 2e groupe et du 3e sont échangées.
Usage : python -I tim2_png.py entrée.bin sortie.png"""
import struct
import sys

from PIL import Image


def decode(data):
    """La première image d'un TIM2 en RGBA ; ValueError si elle ne se lit pas."""
    if data[:4] != b"TIM2":
        raise ValueError("pas une image TIM2")
    at = 16 if data[5] == 0 else 128
    total, clut_size, image_size, header_size, colors, _, _, clut_type, kind, w, h = struct.unpack_from("<IIIHHBBBBHH", data, at)
    pixels = data[at + header_size:at + header_size + image_size]
    clut = data[at + header_size + image_size:at + header_size + image_size + clut_size]

    def color(raw, k):
        if clut_type & 0x3F == 3:
            r, g, b, a = raw[k * 4:k * 4 + 4]
        else:
            v = struct.unpack_from("<H", raw, k * 2)[0]
            r, g, b, a = (v & 31) << 3, (v >> 5 & 31) << 3, (v >> 10 & 31) << 3, 0x80 if v & 0x8000 else 0
        return r, g, b, min(255, a * 255 // 128)

    palette = [color(clut, k) for k in range(colors)]
    if colors == 256 and not clut_type & 0x80:
        fixed = list(palette)
        for block in range(0, 256, 32):
            fixed[block + 8:block + 16], fixed[block + 16:block + 24] = palette[block + 16:block + 24], palette[block + 8:block + 16]
        palette = fixed
    img = Image.new("RGBA", (w, h))
    if kind == 5:
        img.putdata([palette[p] for p in pixels[:w * h]])
    elif kind == 4:
        img.putdata([palette[(pixels[i // 2] >> (4 * (i & 1))) & 15] for i in range(w * h)])
    elif kind == 3:
        img.putdata([color(pixels, i) for i in range(w * h)])
    else:
        raise ValueError(f"sorte de pixels {kind} non lue")
    return img


if __name__ == "__main__":
    try:
        decode(open(sys.argv[1], "rb").read()).save(sys.argv[2])
    except ValueError as e:
        sys.exit(str(e))
