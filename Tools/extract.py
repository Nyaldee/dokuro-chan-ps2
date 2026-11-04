"""Sort de l'ISO originale (à la racine du dépôt) l'exécutable du jeu,
SLPM_661.86, dans Tools/work (ignoré par git) : Ghidra le lit là,
le lanceur de Ghidra refusant les parenthèses du nom de l'ISO. Lecture seule
de l'ISO ; rien n'est exécuté.
Usage : python -I extract.py"""
import os
import sys

# Le lecteur d'ISO du dossier de l'outil (python -I ne l'ajoute pas).
HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
from iso_list import SECTOR, files  # noqa: E402

ROOT = os.path.normpath(os.path.join(HERE, ".."))
WORK = os.path.join(HERE, "work")
EXE = "SLPM_661.86"

isos = [n for n in os.listdir(ROOT) if n.lower().endswith(".iso")]
if not isos:
    sys.exit("Aucune ISO à la racine du dépôt.")
with open(os.path.join(ROOT, isos[0]), "rb") as f:
    found = [(lba, size) for lba, size, path in files(f) if path == "/" + EXE]
    if not found:
        sys.exit(f"{EXE} est introuvable dans {isos[0]}.")
    lba, size = found[0]
    f.seek(lba * SECTOR)
    data = f.read(size)
os.makedirs(WORK, exist_ok=True)
with open(os.path.join(WORK, EXE), "wb") as out:
    out.write(data)
print(f"{EXE} : {size} octets dans Tools/work")
