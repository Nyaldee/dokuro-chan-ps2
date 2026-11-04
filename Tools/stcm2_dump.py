"""Désassemble un script STCM2 du jeu (une entrée de SCRIPT.UNI, sortie par
uni_extract.py) en texte lisible. Lecture seule.

Format (d'après STCM2_Run et GetFunctionVariable de l'exécutable) :
- en-tête de 0x20 octets (« STCM2 File Make By Minku 06.0 »), puis la place
  et le nombre des points d'entrée (0x20, 0x24) ;
- « GLOBAL_DATA » : les variables du fichier (des données) ;
- « CODE_START_ » : les actions, à la suite. Une action : appel (1 : le mot
  suivant est l'adresse d'un sous-programme du fichier), instruction (son
  numéro dans la table 0x35bf60, ou 0xffffff00 à 0xffffff14 pour les
  calculs), nombre de paramètres, longueur totale ; puis les paramètres
  (12 octets chacun) et les données qu'ils montrent ;
- un paramètre : 0xff000000 + n, la valeur n ; 0xffffff41 ou 0xffffff40 et
  une adresse, un saut ; 0xffffff00 + n, le n-ième argument du
  sous-programme ; 0xffffff20 + n, un registre du fil ; sinon l'adresse d'une
  donnée (et deux indices de tableau) ;
- une donnée : sorte (0 entier, 1 flottant), taille en mots, 1, taille en
  octets, puis la valeur (un texte en Shift-JIS, souvent) ;
- « EXPORT_DATA » : les points d'entrée (nom sur 32 octets, adresse).

Usage : python -I stcm2_dump.py script.bin [table.txt] > script.txt
(table.txt : la table des instructions, sortie de stcm2_table.py)"""
import struct
import sys

sys.stdout.reconfigure(encoding="utf-8")
data = open(sys.argv[1], "rb").read()
names = {}
if len(sys.argv) > 2:
    for line in open(sys.argv[2], encoding="utf-8-sig"):
        part = line.split()
        names[int(part[0], 16)] = part[2].replace("STCM2_", "")

# Les instructions faites par STCM2_Run lui-même.
BUILTIN = {0: "return", 1: "start_thread", 2: "set_result", 4: "yield", 5: "start_search",
           6: "if_int", 7: "goto", 8: "random", 9: "loop", 0xD: "if_float",
           0x32: "wait_frames"}
CALC = {0xFFFFFF00: "add", 0xFFFFFF01: "sub", 0xFFFFFF02: "mul", 0xFFFFFF03: "div",
        0xFFFFFF04: "mod", 0xFFFFFF05: "shl", 0xFFFFFF06: "shr", 0xFFFFFF07: "and",
        0xFFFFFF08: "xor", 0xFFFFFF09: "or", 0xFFFFFF0A: "+=", 0xFFFFFF0B: "-=",
        0xFFFFFF0C: "*=", 0xFFFFFF0D: "/=", 0xFFFFFF0E: "%=", 0xFFFFFF0F: "<<=",
        0xFFFFFF10: ">>=", 0xFFFFFF11: "&=", 0xFFFFFF12: "^=", 0xFFFFFF13: "|=",
        0xFFFFFF14: "="}


def u32(at):
    return struct.unpack_from("<I", data, at)[0]


def value(at):
    """Une donnée : sa sorte et sa valeur lisible."""
    kind, _, _, size = struct.unpack_from("<IIII", data, at)
    raw = data[at + 16:at + 16 + size]
    # Un mot qui commence par un caractère double et finit par deux zéros
    # est un texte court (« 桜 »).
    short_text = size == 4 and (0x81 <= raw[0] <= 0x9F or 0xE0 <= raw[0] <= 0xFC) and raw[2:] == b"\0\0"
    if size == 4 and not short_text:
        n = struct.unpack("<i", raw)[0]
        return f"{n}" if kind == 0 else f"{struct.unpack('<f', raw)[0]:g}f"
    text = raw.split(b"\0")[0]
    try:
        s = text.decode("cp932")
        if s and all(c.isprintable() or c == "　" for c in s):
            return repr(s)
    except UnicodeDecodeError:
        pass
    return f"[{size} octets]"


def index(w):
    if w >= 0xFF000000:
        return str(w - 0xFF000000) if w < 0xFFFFFF00 else f"?{w:x}"
    return f"@{w:x}"


def param(at, globals_end):
    v0, v1, v2 = struct.unpack_from("<III", data, at)
    if v0 == 0xFFFFFF41 or v0 == 0xFFFFFF40:
        return f"->{v1:x}"
    if v0 >= 0xFFFFFF20 and v0 < 0xFFFFFF28:
        return f"r{v0 - 0xFFFFFF20}"
    if v0 >= 0xFFFFFF00:
        return f"arg{v0 - 0xFFFFFF00}"
    if v0 >= 0xFF000000:
        n = v0 - 0xFF000000
        return str(n - 0x1000000 if n >= 0x800000 else n)
    where = "g" if v0 < globals_end else "d"
    tail = ""
    if v1 != 0xFF000000 or v2 != 0xFF000000:
        tail = f"[{index(v1)},{index(v2)}]"
    return f"{where}{v0:x}{tail}={value(v0)}"


code = data.index(b"CODE_START_") + 12
globals_end = code
export_at, export_count = u32(0x20), u32(0x24)
exports = {}
for k in range(export_count):
    e = export_at + 0x28 * k
    exports[u32(e + 0x24)] = data[e + 4:e + 0x24].split(b"\0")[0].decode("cp932", "replace")
end = data.index(b"EXPORT_DATA", code)

# Les variables globales.
at = data.index(b"GLOBAL_DATA") + 12
while at < code - 12:
    size = u32(at + 12)
    if size == 0 and u32(at + 4) == 0:
        break
    print(f"g{at:x} = {value(at)}")
    at += 16 + size

# Les cibles de sauts, pour les étiqueter.
targets = set()
at = code
while at < end:
    call, op, n, length = struct.unpack_from("<IIII", data, at)
    if length == 0:
        break
    if call == 1:
        targets.add(op)
    for k in range(n):
        v0, v1, _ = struct.unpack_from("<III", data, at + 16 + 12 * k)
        if v0 in (0xFFFFFF40, 0xFFFFFF41):
            targets.add(v1)
    at += length

at = code
while at < end:
    call, op, n, length = struct.unpack_from("<IIII", data, at)
    if length == 0:
        break
    if at in exports:
        print(f"\n== {exports[at]} ==")
    elif at in targets:
        print(f"\n-- {at:x} --")
    params = ", ".join(param(at + 16 + 12 * k, globals_end) for k in range(n))
    if call == 1:
        name = f"call {exports.get(op, f'{op:x}')}"
    elif op in CALC:
        name = CALC[op]
    else:
        name = names.get(op) or BUILTIN.get(op) or f"op{op:x}"
    print(f"{at:6x}  {name}({params})")
    at += length
