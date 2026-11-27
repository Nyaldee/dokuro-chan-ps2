"""Extrait une entrée d'un fichier VBG de l'ISO (lecture seule de l'ISO) en
WAV : ADPCM PS2 (blocs de 16 octets, 28 échantillons), mono ou stéréo
entrelacé. Répertoire VBG : en-tête d'un secteur (« VBG », nombre
d'entrées, taille du répertoire en secteurs à 0x10), puis des entrées de
16 octets (numéro, secteur, taille en secteurs, taille en octets) ; les
données suivent le répertoire.
Usage : python -I vbg_wav.py image.iso secteur-du-vbg numéro sortie.wav
        [canaux] [entrelacement] [fréquence] [secondes]"""
import struct
import sys
import wave

SECTOR = 2048
COEFS = [(0, 0), (60, 0), (115, -52), (98, -55), (122, -60)]


def decode(blocks):
    """Échantillons 16 bits d'une suite de blocs ADPCM PS2 (un canal)."""
    out = []
    h1 = h2 = 0
    for b in range(0, len(blocks) - 15, 16):
        shift, filt = blocks[b] & 0x0F, min(blocks[b] >> 4, 4)
        c1, c2 = COEFS[filt]
        for byte in blocks[b + 2:b + 16]:
            for nib in (byte & 0x0F, byte >> 4):
                s = (nib - 16 if nib >= 8 else nib) << 12 >> shift
                s += (h1 * c1 + h2 * c2 + 32) >> 6
                s = max(-32768, min(32767, s))
                h2, h1 = h1, s
                out.append(s)
    return out


def entry(iso, lba, number):
    with open(iso, "rb") as f:
        f.seek(lba * SECTOR)
        head = f.read(SECTOR)
        count, dir_sectors = struct.unpack_from("<I", head, 8)[0], struct.unpack_from("<I", head, 0x10)[0]
        f.seek((lba + 1) * SECTOR)
        directory = f.read(dir_sectors * SECTOR)
        for i in range(count):
            n, start, _, size = struct.unpack_from("<IIII", directory, 16 * i)
            if n == number:
                f.seek((lba + 1 + dir_sectors + start) * SECTOR)
                return f.read(size)
    raise SystemExit(f"entrée {number} introuvable")


if __name__ == "__main__":
    iso, lba, number, out = sys.argv[1], int(sys.argv[2]), int(sys.argv[3]), sys.argv[4]
    channels = int(sys.argv[5]) if len(sys.argv) > 5 else 1
    inter = int(sys.argv[6], 0) if len(sys.argv) > 6 else 0x10
    rate = int(sys.argv[7]) if len(sys.argv) > 7 else 44100
    seconds = float(sys.argv[8]) if len(sys.argv) > 8 else 0
    data = entry(iso, lba, number)
    if seconds:
        data = data[:int(seconds * rate / 28 * 16 * channels) // (inter * channels) * inter * channels + inter * channels]
    chans = [bytearray() for _ in range(channels)]
    for i in range(0, len(data), inter):
        chans[(i // inter) % channels] += data[i:i + inter]
    pcm = [decode(bytes(c)) for c in chans]
    n = min(len(p) for p in pcm)
    with wave.open(out, "wb") as w:
        w.setnchannels(channels)
        w.setsampwidth(2)
        w.setframerate(rate)
        w.writeframes(b"".join(struct.pack("<" + "h" * channels, *(p[i] for p in pcm)) for i in range(n)))
    print(f"{len(data)} octets, {n} échantillons par canal, {n / rate:.1f} s")
