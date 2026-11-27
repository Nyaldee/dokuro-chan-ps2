"""Les textes de l'exécutable à des adresses (lecture seule ; segment chargé
en 0x00100000, au décalage 0x80 du fichier), en Shift-JIS, jusqu'au premier
zéro : les phrases des menus et des questions (« セーブ#nロード… » en
0x363fc0), que le pseudo-C ne montre que par leur adresse.
Usage : python -I exe_text.py SLPM_661.86 <adresse hex> [<adresse hex>…]"""
import sys

LOAD = 0x00100000 - 0x80

sys.stdout.reconfigure(encoding="utf-8")
exe = open(sys.argv[1], "rb").read()
for arg in sys.argv[2:]:
    at = int(arg, 16)
    raw = exe[at - LOAD:at - LOAD + 512].split(b"\0")[0]
    print(f"{at:08x}  {raw.decode('cp932', 'replace')!r}")
