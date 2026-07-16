#!/usr/bin/env python3
"""Rewrite _dds3.so's strong GLIBC_2.38 verneed entry to GLIBC_2.34.

Run AFTER `patchelf --clear-symbol-version __isoc23_strtol` (see
isoc23-shim.c): the entry then references no symbol, but ld.so still
validates every strong verneed against libc at load. Same-length string
swap + vna_hash fixup; asserts both patterns are unique in the file.

Usage: fix-dds3-verneed.py ~/ben/bin/dds3-linux/dds3/_dds3.so
"""
import sys


def elf_hash(s: str) -> int:
    h = 0
    for c in s.encode():
        h = (h << 4) + c
        g = h & 0xF0000000
        if g:
            h ^= g >> 24
        h &= ~g & 0xFFFFFFFF
    return h


path = sys.argv[1]
data = bytearray(open(path, 'rb').read())

for old, new in [
    (b'GLIBC_2.38\x00', b'GLIBC_2.34\x00'),
    (elf_hash('GLIBC_2.38').to_bytes(4, 'little'),
     elf_hash('GLIBC_2.34').to_bytes(4, 'little')),
]:
    n = data.count(old)
    assert n == 1, f"expected 1 occurrence of {old!r}, got {n}"
    i = data.index(old)
    data[i:i + len(old)] = new

open(path, 'wb').write(data)
print(f"patched {path}: GLIBC_2.38 verneed -> GLIBC_2.34")
