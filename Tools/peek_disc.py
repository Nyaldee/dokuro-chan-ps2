"""Lecture seule : taille, début, signatures (ELF, TIM2) et chaînes lisibles de
fichiers du disque de « Game ni Natta yo! Dokuro-chan » (PS2) ; rien n'est
exécuté. Sert à savoir si un fichier contient du code ou des données.

Usage : python -I peek_disc.py <fichier> [<fichier>…]
"""

import re
import sys

KEYS = re.compile(rb"(?i)bat|timing|power|angle|mini|\.c\b|\.cpp|func")

for path in sys.argv[1:]:
    data = open(path, "rb").read()
    print(f"== {path.replace(chr(92), '/').rsplit('/', 1)[-1]} : {len(data)} octets, début {data[:16].hex(' ')}")
    for magic, name in [(b"\x7fELF", "ELF"), (b"TIM2", "TIM2")]:
        print(f"   {name} : {data.count(magic)}")
    strings = re.findall(rb"[\x20-\x7e]{6,}", data)
    print(f"   chaînes ASCII (6+) : {len(strings)}")
    for s in [s for s in strings if KEYS.search(s)][:40]:
        print("     ", s.decode("ascii"))
