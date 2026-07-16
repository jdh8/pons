/* Shim for running BEN's vendored _dds3.so (built on glibc >= 2.38) on this
 * glibc 2.35 box. The extension's ONLY post-2.34 import is
 * __isoc23_strtol@GLIBC_2.38 (C23-semantics strtol; the C23 delta is 0b
 * binary-prefix parsing, which DDS never feeds it).
 *
 * One-time setup (see docs/ben-gen-design.md):
 *   gcc -shared -fPIC -O2 -o ~/ben/bin/dds3-linux/dds3/libisoc23shim.so \
 *       vendor/ben/isoc23-shim.c
 *   cp ~/ben/bin/dds3-linux/dds3/_dds3.so{,.orig}
 *   uvx patchelf --clear-symbol-version __isoc23_strtol \
 *       --add-needed libisoc23shim.so --set-rpath '$ORIGIN' \
 *       ~/ben/bin/dds3-linux/dds3/_dds3.so
 *   ./fix-dds3-verneed.py ~/ben/bin/dds3-linux/dds3/_dds3.so
 *
 * ponytail: forwarding shim, rebuild _dds3.so from the DDS repo (bazel,
 * glibc<=2.35 container) if the pinned BEN tag ever needs more 2.38 symbols.
 */
#include <stdlib.h>

long __isoc23_strtol(const char *nptr, char **endptr, int base)
{
    return strtol(nptr, endptr, base);
}
