"""Banque de sons Sony (.hd/.bd) de SOUND.UNI (lecture seule de l'ISO) :
pour chaque zone du programme 0, la note, le jeu d'échantillons,
l'échantillon VAG, sa fréquence, son bouclage et les volumes ; avec un
dossier de sortie, chaque échantillon en WAV.
Usage : python -I hd_list.py image.iso secteur-de-SOUND.UNI [dossier]"""
import os
import struct
import sys
import wave

# Le décodeur ADPCM du dossier de l'outil (python -I ne l'ajoute pas).
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from vbg_wav import decode  # noqa: E402

SECTOR = 2048

iso, lba = sys.argv[1], int(sys.argv[2])
out = sys.argv[3] if len(sys.argv) > 3 else None
with open(iso, "rb") as f:
    f.seek(lba * SECTOR)
    head = f.read(SECTOR * 2)
    entries = {}
    for k in range(struct.unpack_from("<I", head, 8)[0]):
        index, start, size, _ = struct.unpack_from("<IIII", head, SECTOR + 16 * k)
        f.seek((lba + 2 + start) * SECTOR)
        entries[index] = f.read(size * SECTOR)
hd, bd = entries[2], entries[3]


def chunk(name):
    pos = hd.find(name)
    count = struct.unpack_from("<I", hd, pos + 12)[0] + 1
    return pos, [struct.unpack_from("<I", hd, pos + 16 + 4 * i)[0] for i in range(count)]


vpos, voffs = chunk(b"IECSigaV")
vags = []
for o in voffs:
    at, rate, loop = struct.unpack_from("<IHB", hd, vpos + o)
    vags.append((at, rate, loop))
spos, soffs = chunk(b"IECSlpmS")
ssp, ssoffs = chunk(b"IECStesS")
ppos, poffs = chunk(b"IECSgorP")
prog = ppos + poffs[0]
nsplit, split_size, pvol = hd[prog + 4], hd[prog + 5], hd[prog + 6]
print(f"programme 0 : {nsplit} zones, volume {pvol}")
for s in range(nsplit):
    sp = prog + 0x24 + s * split_size
    sset, low, high = struct.unpack_from("<H", hd, sp)[0], hd[sp + 2], hd[sp + 4]
    svol = hd[sp + 0x10]
    ss = ssp + ssoffs[sset]
    smpl = struct.unpack_from("<H", hd, ss + 4)[0]
    sm = spos + soffs[smpl]
    vag = struct.unpack_from("<H", hd, sm)[0]
    base, mvol = hd[sm + 0xB], hd[sm + 0x10]
    at, rate, loop = vags[vag]
    end = vags[vag + 1][0] if vag + 1 < len(vags) else len(bd)
    print(f"note {low:3d}-{high:3d} (son {low - 11:3d}) : jeu {sset:2d} échantillon {smpl:2d} vag {vag:2d} "
          f"{rate} Hz boucle {loop} note de base {base} volumes zone {svol} échantillon {mvol} "
          f"{(end - at) / 16 * 28 / rate:.2f} s")
    if out:
        os.makedirs(out, exist_ok=True)
        pcm = decode(bd[at:end])
        with wave.open(os.path.join(out, f"se{low - 11:03d}.wav"), "wb") as w:
            w.setnchannels(1)
            w.setsampwidth(2)
            w.setframerate(rate)
            w.writeframes(struct.pack("<" + "h" * len(pcm), *pcm))
