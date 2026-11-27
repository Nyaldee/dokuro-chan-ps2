"""La table des messages de l'écran de la carte mémoire (lecture seule de
l'exécutable) : en 0x1dfbe0, une entrée de 0x1c octets par message
(mcsEnableWindow(n)) — l'adresse du texte, sa sorte (0 : rien à attendre,
1 : question はい / いいえ, 2 : attendre ○), puis pour ○ (ou はい) et pour ✕
(ou いいえ) un code d'action (cRam00365a14 : 1 l'action de prc_ynNextMain
ou prc_pwNextMain, 5 la sortie…) et l'état suivant de l'automate
(PTR_mcsmIdle_001e06e0), enfin un mot que mcsMakeYesNoWindow ignore (la
question part toujours sur いいえ).
Usage : python -I mcs_messages.py SLPM_661.86 [nombre d'entrées]"""
import struct
import sys

LOAD = 0x00100000 - 0x80
TABLE = 0x1DFBE0

sys.stdout.reconfigure(encoding="utf-8")
exe = open(sys.argv[1], "rb").read()
count = int(sys.argv[2]) if len(sys.argv) > 2 else 0x62


def text(addr):
    if not addr:
        return ""
    raw = exe[addr - LOAD:addr - LOAD + 512].split(b"\0")[0]
    return raw.decode("cp932", "replace")


for n in range(count):
    msg, kind, ok_code, ok_next, no_code, no_next, default = struct.unpack_from("<7i", exe, TABLE - LOAD + n * 0x1C)
    print(f"{n:#04x} sorte {kind}  ○ {ok_code}→{ok_next}  ✕ {no_code}→{no_next}  ({default})  {text(msg)!r}")
