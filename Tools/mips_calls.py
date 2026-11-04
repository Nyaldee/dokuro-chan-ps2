"""Les instructions qui préparent des appels, sur une plage d'adresses de
l'exécutable : lui, addiu, ori, les copies de registres et les jal. Sert à
retrouver les arguments d'un appel que Ghidra n'a pas su lire (par exemple
le style et la priorité perdus d'un AD_WinMake). Lecture seule.
Usage : python -I mips_calls.py SLPM_661.86 <début hex> <fin hex>"""
import struct
import sys

# L'exécutable est chargé en 0x00100000, au décalage 0x80 du fichier.
LOAD = 0x00100000 - 0x80
REGS = ["zero", "at", "v0", "v1", "a0", "a1", "a2", "a3", "t0", "t1", "t2", "t3", "t4", "t5", "t6", "t7",
        "s0", "s1", "s2", "s3", "s4", "s5", "s6", "s7", "t8", "t9", "k0", "k1", "gp", "sp", "fp", "ra"]

exe = open(sys.argv[1], "rb").read()
start, end = int(sys.argv[2], 16), int(sys.argv[3], 16)
for at in range(start, end, 4):
    w = struct.unpack_from("<I", exe, at - LOAD)[0]
    op, rs, rt, imm = w >> 26, (w >> 21) & 31, (w >> 16) & 31, w & 0xFFFF
    signed = imm - 0x10000 if imm & 0x8000 else imm
    if op == 3:
        print(f"{at:08x}  jal    {(w & 0x3FFFFFF) << 2:08x}")
    elif op == 9:
        print(f"{at:08x}  addiu  {REGS[rt]}, {REGS[rs]}, {signed}")
    elif op == 13:
        print(f"{at:08x}  ori    {REGS[rt]}, {REGS[rs]}, 0x{imm:x}")
    elif op == 15:
        print(f"{at:08x}  lui    {REGS[rt]}, 0x{imm:x}")
    elif op == 0 and (w & 63) in (0x21, 0x25, 0x2D):
        print(f"{at:08x}  move   {REGS[(w >> 11) & 31]}, {REGS[rs]}, {REGS[rt]}")
