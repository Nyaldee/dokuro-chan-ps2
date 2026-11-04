"""Exporte en PNG toutes les images du jeu : chaque paquet UNI2 de l'ISO
(UNION/*.UNI), ses images ART2 et TIM2, dans un sous-dossier à son nom
(« CHARA/012_nom.png » ; un TIM2 n'a pas de nom). Les autres entrées
(scripts, sons) sont comptées et laissées. Lecture seule de l'ISO ; il
faut Pillow.
Usage : python -I sprites_png.py image.iso dossier-de-sortie"""
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
from iso_list import files  # noqa: E402
from tim2_png import decode as tim2_image  # noqa: E402
from uni_png import art2_image, art2_name, entries  # noqa: E402

sys.stdout.reconfigure(encoding="utf-8")
iso, out = sys.argv[1], sys.argv[2]
total = 0
with open(iso, "rb") as f:
    packages = [(lba, path) for lba, _, path in files(f) if path.upper().endswith(".UNI")]
    for lba, path in packages:
        package = os.path.splitext(os.path.basename(path))[0]
        saved, others, failed = 0, 0, []
        for index, data in entries(f, lba):
            try:
                if data[:4] == b"ART2":
                    img, safe = art2_image(data), art2_name(data)[1]
                elif data[:4] == b"TIM2":
                    img, safe = tim2_image(data), "tim2"
                else:
                    others += 1
                    continue
            except ValueError as e:
                failed.append(f"{index} ({e})")
                continue
            os.makedirs(os.path.join(out, package), exist_ok=True)
            img.save(os.path.join(out, package, f"{index:03d}_{safe}.png"))
            saved += 1
        total += saved
        line = f"{package} : {saved} image{'s' if saved > 1 else ''}"
        if others:
            line += f", {others} autres entrées"
        if failed:
            line += f", non lues : {', '.join(failed)}"
        print(line)
print(f"{total} images dans {out}")
